use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};

use crate::core::embed::{cosine_similarity, validate_embedding, Embedder};
use crate::core::store::{MemoryLevel, StoreError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchInput {
    pub query: String,
    pub limit: usize,
    pub level: Option<MemoryLevel>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub level: MemoryLevel,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub score: f32,
}

#[derive(Clone, Debug)]
struct Candidate {
    title: String,
    content: String,
    tags: Vec<String>,
    created_at: String,
    lexical_score: f32,
    vector_score: f32,
    total_score: f32,
}

pub fn search_semantic(
    conn: &Connection,
    project_id: &str,
    input: SearchInput,
    embedder: Option<&dyn Embedder>,
) -> Result<Vec<SearchHit>, StoreError> {
    let query_terms = tokenize_list(&input.query);

    if query_terms.is_empty() || input.limit == 0 {
        return Ok(Vec::new());
    }

    let fts_query = build_fts_query(&query_terms);
    let query_embedding = match embedder {
        Some(provider) => match provider.embed(&input.query) {
            Ok(embedding) => {
                validate_embedding(&embedding)?;
                Some(embedding)
            }
            Err(crate::core::embed::EmbedError::Unavailable(_)) => None,
            Err(error) => return Err(error.into()),
        },
        None => None,
    };

    let required_tags = normalized_tag_set(&input.tags);
    let vector_scores = vector_candidate_scores(
        conn,
        project_id,
        &required_tags,
        query_embedding.as_deref(),
        candidate_limit(input.limit),
    )?;

    let mut statement = conn.prepare(
        "
        SELECT sm.id, sm.title, sm.content, sm.tags_json, sm.created_at
        FROM semantic_fts
        JOIN semantic_memories sm ON sm.id = semantic_fts.memory_id
        WHERE semantic_fts MATCH ?1
          AND sm.project_id = ?2
        ORDER BY sm.created_at DESC
        LIMIT ?3
        ",
    )?;

    let rows = statement.query_map(params![fts_query, project_id, candidate_limit(input.limit)], |row| {
        let memory_id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        let tags_json: String = row.get(3)?;
        let created_at: String = row.get(4)?;

        Ok((memory_id, title, content, tags_json, created_at))
    })?;

    let mut candidates = HashMap::new();

    for row in rows {
        let (memory_id, title, content, tags_json, created_at) = row?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;

        if !matches_required_tags(&required_tags, &tags) {
            continue;
        }

        let lexical_score = lexical_score(&query_terms, &title, &content, &tags);

        if lexical_score <= 0.0 {
            continue;
        }

        candidates.insert(
            memory_id.clone(),
            Candidate {
                title,
                content,
                tags,
                created_at,
                lexical_score,
                vector_score: vector_scores
                    .get(&memory_id)
                    .map(|candidate| candidate.vector_score)
                    .unwrap_or(0.0),
                total_score: 0.0,
            },
        );
    }

    if query_embedding.is_some() {
        for (memory_id, vector_candidate) in vector_scores {
            candidates
                .entry(memory_id)
                .and_modify(|candidate| {
                    candidate.vector_score = candidate.vector_score.max(vector_candidate.vector_score);
                })
                .or_insert(vector_candidate);
        }
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    let newest_first = sorted_indices_by_recency(&candidates);

    for (rank, index) in newest_first.iter().enumerate() {
        let recency_boost = ((candidates.len() - rank) as f32) / (candidates.len().max(1) as f32);
        let candidate = &mut candidates[*index];
        candidate.total_score =
            candidate.lexical_score * 1000.0 + candidate.vector_score * 100.0 + recency_boost;
    }

    candidates.sort_by(|left, right| {
        right
            .total_score
            .partial_cmp(&left.total_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| left.title.cmp(&right.title))
    });

    Ok(candidates
        .into_iter()
        .take(input.limit)
        .map(|candidate| SearchHit {
            level: MemoryLevel::Semantic,
            title: Some(candidate.title),
            content: candidate.content,
            tags: candidate.tags,
            created_at: candidate.created_at,
            score: candidate.total_score,
        })
        .collect())
}

fn vector_candidate_scores(
    conn: &Connection,
    project_id: &str,
    required_tags: &HashSet<String>,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Result<HashMap<String, Candidate>, StoreError> {
    let Some(query_embedding) = query_embedding else {
        return Ok(HashMap::new());
    };

    let mut statement = conn.prepare(
        "
        SELECT sm.id, sm.title, sm.content, sm.tags_json, sm.created_at, se.embedding_json
        FROM semantic_memories sm
        JOIN semantic_embeddings se ON se.memory_id = sm.id
        WHERE sm.project_id = ?1
        ",
    )?;

    let rows = statement.query_map(params![project_id], |row| {
        let memory_id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let content: String = row.get(2)?;
        let tags_json: String = row.get(3)?;
        let created_at: String = row.get(4)?;
        let embedding_json: String = row.get(5)?;

        Ok((memory_id, title, content, tags_json, created_at, embedding_json))
    })?;

    let mut candidates = Vec::new();

    for row in rows {
        let (memory_id, title, content, tags_json, created_at, embedding_json) = row?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;

        if !matches_required_tags(required_tags, &tags) {
            continue;
        }

        let stored_embedding: Vec<f32> = match serde_json::from_str(&embedding_json) {
            Ok(embedding) => embedding,
            Err(_) => continue,
        };

        if validate_embedding(&stored_embedding).is_err() {
            continue;
        }

        let vector_score = match cosine_similarity(query_embedding, &stored_embedding) {
            Ok(score) if score > 0.0 => score,
            Ok(_) => continue,
            Err(_) => continue,
        };

        candidates.push((
            memory_id,
            Candidate {
                title,
                content,
                tags,
                created_at,
                lexical_score: 0.0,
                vector_score,
                total_score: 0.0,
            },
        ));
    }

    candidates.sort_by(|left, right| {
        right
            .1
            .vector_score
            .partial_cmp(&left.1.vector_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.1.created_at.cmp(&left.1.created_at))
            .then_with(|| left.1.title.cmp(&right.1.title))
    });
    candidates.truncate(limit);

    Ok(candidates.into_iter().collect())
}

fn candidate_limit(limit: usize) -> usize {
    (limit.max(1) * 8).min(256)
}

fn build_fts_query(terms: &[String]) -> String {
    terms
        .iter()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn lexical_score(query_terms: &[String], title: &str, content: &str, tags: &[String]) -> f32 {
    let title_terms = tokenize(title);
    let content_terms = tokenize(content);
    let tag_terms = tags
        .iter()
        .flat_map(|tag| tokenize(tag))
        .collect::<HashSet<String>>();

    let mut score = 0.0f32;

    for term in query_terms {
        if title_terms.contains(term) {
            score += 3.0;
        }

        if content_terms.contains(term) {
            score += 2.0;
        }

        if tag_terms.contains(term) {
            score += 2.0;
        }
    }

    score
}

fn matches_required_tags(required_tags: &HashSet<String>, candidate_tags: &[String]) -> bool {
    if required_tags.is_empty() {
        return true;
    }

    let candidate_tag_set = normalized_tag_set(candidate_tags);
    required_tags
        .iter()
        .all(|required_tag| candidate_tag_set.contains(required_tag))
}

fn normalized_tag_set(tags: &[String]) -> HashSet<String> {
    tags.iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn sorted_indices_by_recency(candidates: &[Candidate]) -> Vec<usize> {
    let mut indices = (0..candidates.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| candidates[*right].created_at.cmp(&candidates[*left].created_at));
    indices
}

fn tokenize(input: &str) -> HashSet<String> {
    tokenize_list(input).into_iter().collect()
}

fn tokenize_list(input: &str) -> Vec<String> {
    let mut tokens = input
        .split(|character: char| !character.is_alphanumeric())
        .filter_map(|term| normalize_term(term))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    tokens.sort();
    tokens
}

fn normalize_term(term: &str) -> Option<String> {
    let lowered = term.trim().to_ascii_lowercase();

    if lowered.is_empty() {
        return None;
    }

    let normalized = if lowered.len() > 3 && lowered.ends_with('s') {
        lowered.trim_end_matches('s').to_string()
    } else {
        lowered
    };

    if normalized.len() < 2 {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::{lexical_score, tokenize, tokenize_list};

    #[test]
    fn tokenize_normalizes_plural_words() {
        let tokens = tokenize("Drive-relative paths");

        assert!(tokens.contains("drive"));
        assert!(tokens.contains("relative"));
        assert!(tokens.contains("path"));
    }

    #[test]
    fn lexical_score_weights_title_and_content_hits() {
        let query = tokenize_list("windows path");
        let score = lexical_score(&query, "Windows note", "Path guidance", &[]);

        assert_eq!(score, 5.0);
    }
}
