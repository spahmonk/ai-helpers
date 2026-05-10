use std::error::Error;
use std::fmt;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::core::embed::{validate_embedding, EmbedError, Embedder};
use crate::core::project::ProjectScope;
use crate::core::retrieval::{search_semantic, SearchHit, SearchInput};
use crate::core::schema;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
const SEMANTIC_EMBEDDER_IDENTITY_KEY: &str = "semantic_embedder_identity";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryLevel {
    Semantic,
    Episodic,
    Procedural,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemorySource {
    Explicit,
}

impl MemorySource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberInput {
    pub level: MemoryLevel,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source: MemorySource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentMemory {
    pub level: MemoryLevel,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryStats {
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub procedural_count: usize,
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Embed(EmbedError),
    Time(std::time::SystemTimeError),
    InvalidInput(&'static str),
    EmbedderIdentityMismatch { expected: String, actual: String },
    MissingEmbedderIdentity,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(error) => error.fmt(f),
            StoreError::Sql(error) => error.fmt(f),
            StoreError::Json(error) => error.fmt(f),
            StoreError::Embed(error) => error.fmt(f),
            StoreError::Time(error) => error.fmt(f),
            StoreError::InvalidInput(message) => f.write_str(message),
            StoreError::EmbedderIdentityMismatch { expected, actual } => {
                write!(
                    f,
                    "embedder identity mismatch: store vectors use '{expected}', but '{actual}' was requested"
                )
            }
            StoreError::MissingEmbedderIdentity => f.write_str(
                "stored embeddings exist but embedder identity metadata is missing; clear and rebuild embeddings before vector search or vector persistence",
            ),
        }
    }
}

impl Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<std::time::SystemTimeError> for StoreError {
    fn from(value: std::time::SystemTimeError) -> Self {
        Self::Time(value)
    }
}

impl From<EmbedError> for StoreError {
    fn from(value: EmbedError) -> Self {
        Self::Embed(value)
    }
}

pub struct MemoryStore {
    scope: ProjectScope,
    conn: Connection,
    embedder: Option<Arc<dyn Embedder>>,
}

impl MemoryStore {
    pub fn open(scope: ProjectScope) -> Result<Self, StoreError> {
        Self::open_with_optional_embedder(scope, None)
    }

    pub fn open_with_embedder(
        scope: ProjectScope,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, StoreError> {
        Self::open_with_optional_embedder(scope, Some(embedder))
    }

    fn open_with_optional_embedder(
        scope: ProjectScope,
        embedder: Option<Arc<dyn Embedder>>,
    ) -> Result<Self, StoreError> {
        if let Some(parent) = scope.database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&scope.database_path)?;
        schema::bootstrap(&conn)?;

        Ok(Self {
            scope,
            conn,
            embedder,
        })
    }

    pub fn remember(&self, input: RememberInput) -> Result<(), StoreError> {
        let id = next_id()?;
        let timestamp = now_string()?;
        let RememberInput {
            level,
            title,
            content,
            tags,
            source,
        } = input;

        match level {
            MemoryLevel::Semantic => {
                let tags_json = serde_json::to_string(&tags)?;
                let searchable_text = format!("{title}\n{content}\n{}", tags.join(" "));
                let mut embedder_identity: Option<&'static str> = None;
                let mut should_record_embedder_identity = false;
                let maybe_embedding = match self.embedder.as_deref() {
                    Some(embedder) => match embedder.embed(&searchable_text) {
                        Ok(embedding) => {
                            validate_embedding(&embedding)?;
                            embedder_identity = Some(embedder.identity());
                            should_record_embedder_identity =
                                self.ensure_embedder_identity_for_write(embedder.identity())?;
                            Some(embedding)
                        }
                        Err(EmbedError::Unavailable(_)) => None,
                        Err(error) => return Err(error.into()),
                    },
                    None => None,
                };

                let tx = self.conn.unchecked_transaction()?;
                tx.execute(
                    "INSERT INTO semantic_memories (
                        id, project_id, title, content, tags_json, created_at, updated_at, source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        id,
                        self.scope.project_id,
                        title,
                        content,
                        tags_json,
                        timestamp,
                        timestamp,
                        source.as_str(),
                    ],
                )?;
                tx.execute(
                    "INSERT INTO semantic_fts (memory_id, title, content, tags) VALUES (?1, ?2, ?3, ?4)",
                    params![id, title, content, tags.join(" ")],
                )?;

                if let Some(embedding) = maybe_embedding {
                    tx.execute(
                        "INSERT OR REPLACE INTO semantic_embeddings (
                            memory_id, embedding_json, created_at
                        ) VALUES (?1, ?2, ?3)",
                        params![id, serde_json::to_string(&embedding)?, timestamp],
                    )?;

                    if should_record_embedder_identity {
                        tx.execute(
                            "INSERT INTO store_metadata (key, value)
                             VALUES (?1, ?2)
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                            params![
                                SEMANTIC_EMBEDDER_IDENTITY_KEY,
                                embedder_identity.expect("embedder identity set when recording")
                            ],
                        )?;
                    }
                }

                tx.commit()?;
            }
            MemoryLevel::Episodic => {
                self.conn.execute(
                    "INSERT INTO episodic_memories (
                        id, project_id, content, event_kind, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, self.scope.project_id, content, title, timestamp,],
                )?;
            }
            MemoryLevel::Procedural => {
                self.conn.execute(
                    "INSERT INTO procedural_memories (
                        id, project_id, name, content, tags_json, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id,
                        self.scope.project_id,
                        title,
                        content,
                        serde_json::to_string(&tags)?,
                        timestamp,
                        timestamp,
                    ],
                )?;
            }
        }

        Ok(())
    }

    pub fn search(&self, input: SearchInput) -> Result<Vec<SearchHit>, StoreError> {
        match input.level {
            Some(MemoryLevel::Episodic) | Some(MemoryLevel::Procedural) => Err(
                StoreError::InvalidInput("search currently supports semantic memories only"),
            ),
            Some(MemoryLevel::Semantic) | None => {
                let query_embedding = match self.embedder.as_deref() {
                    Some(embedder) => match embedder.embed(&input.query) {
                        Ok(embedding) => {
                            validate_embedding(&embedding)?;

                            if self.any_semantic_embeddings_exist()? {
                                self.ensure_embedder_identity_for_search(embedder.identity())?;
                            }

                            Some(embedding)
                        }
                        Err(EmbedError::Unavailable(_)) => None,
                        Err(error) => return Err(error.into()),
                    },
                    None => None,
                };

                search_semantic(
                    &self.conn,
                    &self.scope.project_id,
                    input,
                    query_embedding.as_deref(),
                )
            }
        }
    }

    pub fn backfill_embeddings(&self) -> Result<usize, StoreError> {
        let embedder = self.embedder.as_deref().ok_or(StoreError::InvalidInput(
            "backfill_embeddings requires an embedder",
        ))?;

        let pending_rows = self.pending_semantic_backfill_rows()?;
        if pending_rows.is_empty() {
            return Ok(0);
        }

        let should_record_embedder_identity =
            self.ensure_embedder_identity_for_write(embedder.identity())?;
        let mut pending_embeddings = Vec::with_capacity(pending_rows.len());

        for (memory_id, title, content, tags_json, created_at) in pending_rows {
            let tags: Vec<String> = serde_json::from_str(&tags_json)?;
            let searchable_text = format!("{title}\n{content}\n{}", tags.join(" "));
            let embedding = embedder.embed(&searchable_text)?;
            validate_embedding(&embedding)?;
            pending_embeddings.push((memory_id, serde_json::to_string(&embedding)?, created_at));
        }

        let tx = self.conn.unchecked_transaction()?;

        for (memory_id, embedding_json, created_at) in &pending_embeddings {
            tx.execute(
                "INSERT OR IGNORE INTO semantic_embeddings (
                    memory_id, embedding_json, created_at
                ) VALUES (?1, ?2, ?3)",
                params![memory_id, embedding_json, created_at],
            )?;
        }

        if should_record_embedder_identity {
            tx.execute(
                "INSERT INTO store_metadata (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![SEMANTIC_EMBEDDER_IDENTITY_KEY, embedder.identity()],
            )?;
        }

        tx.commit()?;
        Ok(pending_embeddings.len())
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<RecentMemory>, StoreError> {
        let mut statement = self.conn.prepare(
            "
            SELECT level, title, content, created_at
            FROM (
                SELECT 'semantic' AS level, title, content, created_at FROM semantic_memories WHERE project_id = ?1
                UNION ALL
                SELECT 'episodic' AS level, event_kind AS title, content, created_at FROM episodic_memories WHERE project_id = ?1
                UNION ALL
                SELECT 'procedural' AS level, name AS title, content, created_at FROM procedural_memories WHERE project_id = ?1
            )
            ORDER BY created_at DESC
            LIMIT ?2
            ",
        )?;

        let rows = statement.query_map(params![self.scope.project_id, limit as i64], |row| {
            Ok(RecentMemory {
                level: parse_level(row.get::<_, String>(0)?),
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let recent = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(recent)
    }

    pub fn stats(&self) -> Result<MemoryStats, StoreError> {
        Ok(MemoryStats {
            episodic_count: query_count(&self.conn, &self.scope.project_id, "episodic_memories")?,
            semantic_count: query_count(&self.conn, &self.scope.project_id, "semantic_memories")?,
            procedural_count: query_count(
                &self.conn,
                &self.scope.project_id,
                "procedural_memories",
            )?,
        })
    }

    fn ensure_embedder_identity_for_write(&self, identity: &str) -> Result<bool, StoreError> {
        match self.recorded_embedder_identity()? {
            Some(recorded_identity) if recorded_identity == identity => Ok(false),
            Some(recorded_identity) => Err(StoreError::EmbedderIdentityMismatch {
                expected: recorded_identity,
                actual: identity.to_string(),
            }),
            None if self.any_semantic_embeddings_exist()? => {
                Err(StoreError::MissingEmbedderIdentity)
            }
            None => Ok(true),
        }
    }

    fn ensure_embedder_identity_for_search(&self, identity: &str) -> Result<(), StoreError> {
        match self.recorded_embedder_identity()? {
            Some(recorded_identity) if recorded_identity == identity => Ok(()),
            Some(recorded_identity) => Err(StoreError::EmbedderIdentityMismatch {
                expected: recorded_identity,
                actual: identity.to_string(),
            }),
            None => Err(StoreError::MissingEmbedderIdentity),
        }
    }

    fn recorded_embedder_identity(&self) -> Result<Option<String>, StoreError> {
        let identity = self
            .conn
            .query_row(
                "SELECT value FROM store_metadata WHERE key = ?1",
                params![SEMANTIC_EMBEDDER_IDENTITY_KEY],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(identity)
    }

    fn any_semantic_embeddings_exist(&self) -> Result<bool, StoreError> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM semantic_embeddings", [], |row| {
                row.get::<_, i64>(0)
            })?;
        Ok(count > 0)
    }
    fn pending_semantic_backfill_rows(
        &self,
    ) -> Result<Vec<(String, String, String, String, String)>, StoreError> {
        let mut statement = self.conn.prepare(
            "
            SELECT sm.id, sm.title, sm.content, sm.tags_json, sm.created_at
            FROM semantic_memories sm
            LEFT JOIN semantic_embeddings se ON se.memory_id = sm.id
            WHERE sm.project_id = ?1
              AND se.memory_id IS NULL
            ",
        )?;

        let rows = statement.query_map(params![self.scope.project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn query_count(conn: &Connection, project_id: &str, table: &str) -> Result<usize, StoreError> {
    let sql = match table {
        "episodic_memories" => "SELECT COUNT(*) FROM episodic_memories WHERE project_id = ?1",
        "semantic_memories" => "SELECT COUNT(*) FROM semantic_memories WHERE project_id = ?1",
        "procedural_memories" => "SELECT COUNT(*) FROM procedural_memories WHERE project_id = ?1",
        _ => unreachable!("unsupported memory table"),
    };
    let count = conn.query_row(&sql, params![project_id], |row| row.get::<_, i64>(0))?;
    Ok(count as usize)
}

fn next_id() -> Result<String, StoreError> {
    let tick = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "{}-{:08x}-{tick:016x}",
        now_string()?,
        std::process::id()
    ))
}

fn now_string() -> Result<String, StoreError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!("{now:020}"))
}

fn parse_level(value: String) -> MemoryLevel {
    match value.as_str() {
        "semantic" => MemoryLevel::Semantic,
        "episodic" => MemoryLevel::Episodic,
        "procedural" => MemoryLevel::Procedural,
        _ => MemoryLevel::Semantic,
    }
}

#[cfg(test)]
mod tests {
    use super::next_id;

    #[test]
    fn generated_ids_include_process_identifier() {
        let id = next_id().unwrap();
        let pid = std::process::id();

        assert!(id.contains(&format!("-{pid:08x}-")));
    }
}
