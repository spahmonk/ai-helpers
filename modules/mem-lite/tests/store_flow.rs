use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use mem_lite::{
    EmbedError, Embedder, MemoryLevel, MemorySource, MemoryStore, ProjectScope, RememberInput,
    SearchInput,
};
use rusqlite::{params, Connection};
use tempfile::TempDir;

struct MemoryFixture {
    _workspace: TempDir,
    scope: ProjectScope,
}

impl MemoryFixture {
    fn new() -> Self {
        let workspace = tempfile::tempdir().unwrap();
        let scope = ProjectScope {
            workspace_root: workspace.path().to_path_buf(),
            project_id: "test-project".into(),
            database_path: workspace.path().join("memory.sqlite"),
        };

        Self {
            _workspace: workspace,
            scope,
        }
    }

    fn store(&self) -> MemoryStore {
        MemoryStore::open(self.scope.clone()).unwrap()
    }

    fn store_with_embedder(&self, embedder: Arc<dyn Embedder>) -> MemoryStore {
        MemoryStore::open_with_embedder(self.scope.clone(), embedder).unwrap()
    }
}

struct TestEmbedder;

impl Embedder for TestEmbedder {
    fn identity(&self) -> &'static str {
        "test-embedder"
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        let lowered = input.to_ascii_lowercase();

        if lowered.contains("platform note a") || lowered.contains("windows") {
            Ok(vec![1.0, 0.0])
        } else if lowered.contains("platform note b") || lowered.contains("linux") {
            Ok(vec![0.0, 1.0])
        } else {
            Ok(vec![0.5, 0.5])
        }
    }
}

#[derive(Debug)]
struct UnavailableEmbedder;

impl std::fmt::Display for UnavailableEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("embeddings unavailable")
    }
}

impl Error for UnavailableEmbedder {}

impl Embedder for UnavailableEmbedder {
    fn identity(&self) -> &'static str {
        "unavailable-embedder"
    }

    fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbedError> {
        Err(EmbedError::unavailable(UnavailableEmbedder))
    }
}

struct InvalidEmbedder;

impl Embedder for InvalidEmbedder {
    fn identity(&self) -> &'static str {
        "invalid-embedder"
    }

    fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbedError> {
        Err(EmbedError::InvalidVector("bad embedding"))
    }
}

struct EmptyVectorEmbedder;

impl Embedder for EmptyVectorEmbedder {
    fn identity(&self) -> &'static str {
        "empty-vector-embedder"
    }

    fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(vec![])
    }
}

struct AlternateEmbedder;

impl Embedder for AlternateEmbedder {
    fn identity(&self) -> &'static str {
        "alternate-embedder"
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        let lowered = input.to_ascii_lowercase();

        if lowered.contains("platform note a") || lowered.contains("windows") {
            Ok(vec![0.0, 1.0])
        } else if lowered.contains("platform note b") || lowered.contains("linux") {
            Ok(vec![1.0, 0.0])
        } else {
            Ok(vec![0.5, 0.5])
        }
    }
}

struct CountingEmbedder {
    calls: Arc<Mutex<Vec<String>>>,
}

impl CountingEmbedder {
    fn new() -> (Arc<Self>, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Arc::new(Self {
                calls: calls.clone(),
            }),
            calls,
        )
    }
}

impl Embedder for CountingEmbedder {
    fn identity(&self) -> &'static str {
        "counting-embedder"
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        self.calls.lock().unwrap().push(input.to_string());
        TestEmbedder.embed(input)
    }
}

fn explicit_semantic(title: &str, content: &str) -> RememberInput {
    RememberInput {
        level: MemoryLevel::Semantic,
        title: title.into(),
        content: content.into(),
        tags: vec![],
        source: MemorySource::Explicit,
    }
}

fn set_semantic_created_at(scope: &ProjectScope, title: &str, created_at: &str) {
    let conn = Connection::open(&scope.database_path).unwrap();
    conn.execute(
        "UPDATE semantic_memories SET created_at = ?1, updated_at = ?1 WHERE project_id = ?2 AND title = ?3",
        params![created_at, scope.project_id, title],
    )
    .unwrap();
}

fn set_semantic_embedding_json(scope: &ProjectScope, title: &str, embedding_json: &str) {
    let conn = Connection::open(&scope.database_path).unwrap();
    conn.execute(
        "
        UPDATE semantic_embeddings
        SET embedding_json = ?1
        WHERE memory_id = (
            SELECT id
            FROM semantic_memories
            WHERE project_id = ?2 AND title = ?3
        )
        ",
        params![embedding_json, scope.project_id, title],
    )
    .unwrap();
}

fn semantic_embedding_count(scope: &ProjectScope, title: &str) -> i64 {
    let conn = Connection::open(&scope.database_path).unwrap();
    conn.query_row(
        "
        SELECT COUNT(*)
        FROM semantic_embeddings
        WHERE memory_id = (
            SELECT id
            FROM semantic_memories
            WHERE project_id = ?1 AND title = ?2
        )
        ",
        params![scope.project_id, title],
        |row| row.get::<_, i64>(0),
    )
    .unwrap()
}

fn metadata_value(scope: &ProjectScope, key: &str) -> Option<String> {
    let conn = Connection::open(&scope.database_path).unwrap();
    conn.query_row(
        "SELECT value FROM store_metadata WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

fn set_procedural_created_at(scope: &ProjectScope, name: &str, created_at: &str) {
    let conn = Connection::open(&scope.database_path).unwrap();
    conn.execute(
        "UPDATE procedural_memories SET created_at = ?1, updated_at = ?1 WHERE project_id = ?2 AND name = ?3",
        params![created_at, scope.project_id, name],
    )
    .unwrap();
}

#[test]
fn remember_persists_semantic_and_procedural_entries() {
    let fixture = MemoryFixture::new();
    let store = fixture.store();

    store
        .remember(RememberInput {
            level: MemoryLevel::Semantic,
            title: "Path jail rule".into(),
            content: "Absolute paths are allowed when inside project root".into(),
            tags: vec!["security".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    store
        .remember(RememberInput {
            level: MemoryLevel::Procedural,
            title: "Test loop".into(),
            content: "Run targeted tests before package tests".into(),
            tags: vec!["workflow".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    drop(store);

    let stats = fixture.store().stats().unwrap();
    assert_eq!(stats.semantic_count, 1);
    assert_eq!(stats.procedural_count, 1);
    assert_eq!(stats.episodic_count, 0);
}

#[test]
fn recent_returns_newest_entries_first() {
    let fixture = MemoryFixture::new();
    let store = fixture.store();

    store
        .remember(RememberInput {
            level: MemoryLevel::Semantic,
            title: "First memory".into(),
            content: "Older entry".into(),
            tags: vec![],
            source: MemorySource::Explicit,
        })
        .unwrap();

    store
        .remember(RememberInput {
            level: MemoryLevel::Procedural,
            title: "Second memory".into(),
            content: "Newer entry".into(),
            tags: vec![],
            source: MemorySource::Explicit,
        })
        .unwrap();

    set_semantic_created_at(&fixture.scope, "First memory", "00000000000000000001");
    set_procedural_created_at(&fixture.scope, "Second memory", "00000000000000000002");

    let recent = store.recent(2).unwrap();

    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].title, "Second memory");
    assert_eq!(recent[0].level, MemoryLevel::Procedural);
    assert_eq!(recent[1].title, "First memory");
    assert_eq!(recent[1].level, MemoryLevel::Semantic);
}

#[test]
fn stats_count_each_level_independently() {
    let fixture = MemoryFixture::new();
    let store = fixture.store();

    store
        .remember(RememberInput {
            level: MemoryLevel::Semantic,
            title: "Semantic".into(),
            content: "Meaningful fact".into(),
            tags: vec!["fact".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    store
        .remember(RememberInput {
            level: MemoryLevel::Procedural,
            title: "Procedure".into(),
            content: "Do the safe thing".into(),
            tags: vec!["process".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    store
        .remember(RememberInput {
            level: MemoryLevel::Episodic,
            title: "Incident".into(),
            content: "Observed a store write".into(),
            tags: vec![],
            source: MemorySource::Explicit,
        })
        .unwrap();

    let stats = store.stats().unwrap();

    assert_eq!(stats.semantic_count, 1);
    assert_eq!(stats.procedural_count, 1);
    assert_eq!(stats.episodic_count, 1);
}

#[test]
fn search_prefers_same_project_fact_and_recent_keyword_match() {
    let fixture = MemoryFixture::new();

    fixture
        .store()
        .remember(explicit_semantic(
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
        ))
        .unwrap();

    let hits = fixture
        .store()
        .search(SearchInput {
            query: "drive-relative path on windows".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Windows root hint"));
}

#[test]
fn search_uses_embedding_signal_when_lexical_scores_tie() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    store
        .remember(explicit_semantic(
            "Platform note A",
            "Rooted path behavior should be explained clearly",
        ))
        .unwrap();

    store
        .remember(explicit_semantic(
            "Platform note B",
            "Rooted path behavior should be explained clearly",
        ))
        .unwrap();

    set_semantic_created_at(&fixture.scope, "Platform note A", "00000000000000000001");
    set_semantic_created_at(&fixture.scope, "Platform note B", "00000000000000000002");

    let hits = store
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 2,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Platform note A"));
}

#[test]
fn search_keeps_older_stronger_lexical_match_after_scoring() {
    let fixture = MemoryFixture::new();
    let store = fixture.store();

    store
        .remember(explicit_semantic(
            "Windows drive relative path note",
            "Windows drive relative path behavior needs careful explanation",
        ))
        .unwrap();

    for index in 0..8 {
        store
            .remember(explicit_semantic(
                &format!("Recent weak lexical note {index}"),
                "Path reminder for users",
            ))
            .unwrap();
    }

    set_semantic_created_at(
        &fixture.scope,
        "Windows drive relative path note",
        "00000000000000000001",
    );
    for index in 0..8 {
        set_semantic_created_at(
            &fixture.scope,
            &format!("Recent weak lexical note {index}"),
            &format!("{:020}", index + 2),
        );
    }

    let hits = store
        .search(SearchInput {
            query: "windows drive relative path".into(),
            limit: 1,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(
        hits[0].title.as_deref(),
        Some("Windows drive relative path note")
    );
}

#[test]
fn search_returns_vector_only_matches_without_lexical_overlap() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    store
        .remember(RememberInput {
            level: MemoryLevel::Semantic,
            title: "Platform note A".into(),
            content: "Rooted behavior should be explained clearly".into(),
            tags: vec!["platform".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    store
        .remember(RememberInput {
            level: MemoryLevel::Semantic,
            title: "Platform note B".into(),
            content: "Rooted behavior should be explained clearly".into(),
            tags: vec!["server".into()],
            source: MemorySource::Explicit,
        })
        .unwrap();

    let hits = store
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec!["platform".into()],
        })
        .unwrap();

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title.as_deref(), Some("Platform note A"));
}

#[test]
fn search_falls_back_to_lexical_ranking_when_embeddings_are_unavailable() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(UnavailableEmbedder));

    store
        .remember(explicit_semantic(
            "Drive-relative note",
            "Drive-relative paths should be documented for Windows users",
        ))
        .unwrap();

    let hits = store
        .search(SearchInput {
            query: "drive-relative windows path".into(),
            limit: 3,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Drive-relative note"));
}

#[test]
fn search_is_scoped_to_the_requesting_project() {
    let fixture = MemoryFixture::new();
    let shared_path = fixture.scope.database_path.clone();
    let first_scope = ProjectScope {
        workspace_root: fixture.scope.workspace_root.join("one"),
        project_id: "project-one".into(),
        database_path: shared_path.clone(),
    };
    let second_scope = ProjectScope {
        workspace_root: fixture.scope.workspace_root.join("two"),
        project_id: "project-two".into(),
        database_path: shared_path,
    };

    MemoryStore::open(first_scope.clone())
        .unwrap()
        .remember(explicit_semantic(
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
        ))
        .unwrap();

    let hits = MemoryStore::open(second_scope)
        .unwrap()
        .search(SearchInput {
            query: "drive-relative path on windows".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert!(hits.is_empty());
}

#[test]
fn search_rejects_non_semantic_levels() {
    let fixture = MemoryFixture::new();
    let error = fixture
        .store()
        .search(SearchInput {
            query: "incident".into(),
            limit: 5,
            level: Some(MemoryLevel::Episodic),
            tags: vec![],
        })
        .unwrap_err();

    assert!(error.to_string().contains("semantic"));
}

#[test]
fn search_surfaces_non_unavailable_embedder_errors() {
    let fixture = MemoryFixture::new();
    fixture
        .store()
        .remember(explicit_semantic(
            "Drive-relative note",
            "Drive-relative paths should be documented for Windows users",
        ))
        .unwrap();

    let store = fixture.store_with_embedder(Arc::new(InvalidEmbedder));
    let error = store
        .search(SearchInput {
            query: "drive-relative windows path".into(),
            limit: 3,
            level: None,
            tags: vec![],
        })
        .unwrap_err();

    assert!(error.to_string().contains("bad embedding"));
}

#[test]
fn search_skips_malformed_stored_embeddings_instead_of_failing() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    store
        .remember(explicit_semantic(
            "Drive-relative note",
            "Drive-relative paths should be documented for Windows users",
        ))
        .unwrap();

    store
        .remember(explicit_semantic(
            "Broken embedding note",
            "Drive-relative paths should be documented for Windows users",
        ))
        .unwrap();

    set_semantic_embedding_json(&fixture.scope, "Broken embedding note", "[]");

    let hits = store
        .search(SearchInput {
            query: "drive-relative windows path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert!(hits
        .iter()
        .any(|hit| hit.title.as_deref() == Some("Drive-relative note")));
}

#[test]
fn remember_rejects_invalid_embeddings_before_persisting() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(EmptyVectorEmbedder));

    let error = store
        .remember(explicit_semantic(
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
        ))
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("embedding vectors must not be empty"));
    assert_eq!(fixture.store().stats().unwrap().semantic_count, 0);
}

#[test]
fn remember_populates_fts_and_embedding_storage() {
    let fixture = MemoryFixture::new();
    fixture
        .store_with_embedder(Arc::new(TestEmbedder))
        .remember(explicit_semantic(
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
        ))
        .unwrap();

    let conn = Connection::open(&fixture.scope.database_path).unwrap();
    let fts_count = conn
        .query_row("SELECT COUNT(*) FROM semantic_fts", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    let embedding_count = conn
        .query_row("SELECT COUNT(*) FROM semantic_embeddings", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

    assert_eq!(fts_count, 1);
    assert_eq!(embedding_count, 1);
}

#[test]
fn search_falls_back_to_lexical_ranking_after_reopen_without_embedder() {
    let fixture = MemoryFixture::new();
    fixture
        .store_with_embedder(Arc::new(TestEmbedder))
        .remember(explicit_semantic(
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
        ))
        .unwrap();

    let hits = fixture
        .store_with_embedder(Arc::new(UnavailableEmbedder))
        .search(SearchInput {
            query: "drive-relative path on windows".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Windows root hint"));
}

#[test]
fn search_remains_read_only_when_embeddings_are_missing() {
    let fixture = MemoryFixture::new();
    fixture
        .store()
        .remember(explicit_semantic(
            "Drive-relative platform note",
            "Drive-relative behavior should be explained clearly",
        ))
        .unwrap();

    assert_eq!(
        semantic_embedding_count(&fixture.scope, "Drive-relative platform note"),
        0
    );

    let (embedder, calls) = CountingEmbedder::new();
    let hits = fixture
        .store_with_embedder(embedder)
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(
        hits[0].title.as_deref(),
        Some("Drive-relative platform note")
    );
    assert_eq!(
        semantic_embedding_count(&fixture.scope, "Drive-relative platform note"),
        0
    );
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["windows drive-relative path"]
    );
}

#[test]
fn remember_records_embedder_identity_after_successful_semantic_write() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    store
        .remember(explicit_semantic(
            "Identity note",
            "Semantic embeddings should persist cleanly",
        ))
        .unwrap();

    assert_eq!(semantic_embedding_count(&fixture.scope, "Identity note"), 1);
    assert_eq!(
        metadata_value(&fixture.scope, "semantic_embedder_identity").as_deref(),
        Some("test-embedder")
    );
}

#[test]
fn backfill_embeddings_on_empty_store_leaves_identity_unset() {
    let fixture = MemoryFixture::new();
    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    let inserted = store.backfill_embeddings().unwrap();

    assert_eq!(inserted, 0);
    assert_eq!(
        metadata_value(&fixture.scope, "semantic_embedder_identity"),
        None
    );
}

#[test]
fn backfill_embeddings_populates_missing_embeddings_after_reopen() {
    let fixture = MemoryFixture::new();
    fixture
        .store()
        .remember(explicit_semantic(
            "Platform note A",
            "Rooted behavior should be explained clearly",
        ))
        .unwrap();

    let store = fixture.store_with_embedder(Arc::new(TestEmbedder));

    let before_hits = store
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();
    assert!(before_hits.is_empty());
    assert_eq!(
        semantic_embedding_count(&fixture.scope, "Platform note A"),
        0
    );

    let inserted = store.backfill_embeddings().unwrap();

    assert_eq!(inserted, 1);
    assert_eq!(
        semantic_embedding_count(&fixture.scope, "Platform note A"),
        1
    );
    assert_eq!(
        metadata_value(&fixture.scope, "semantic_embedder_identity").as_deref(),
        Some("test-embedder")
    );

    let after_hits = store
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();
    assert_eq!(after_hits[0].title.as_deref(), Some("Platform note A"));
}

#[test]
fn search_rejects_mismatched_embedder_identity() {
    let fixture = MemoryFixture::new();
    fixture
        .store_with_embedder(Arc::new(TestEmbedder))
        .remember(explicit_semantic(
            "Platform note A",
            "Rooted behavior should be explained clearly",
        ))
        .unwrap();

    let error = fixture
        .store_with_embedder(Arc::new(AlternateEmbedder))
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap_err();

    assert!(error.to_string().contains("embedder identity mismatch"));
    assert!(error.to_string().contains("test-embedder"));
    assert!(error.to_string().contains("alternate-embedder"));
}

#[test]
fn search_rejects_mismatched_embedder_identity_across_projects() {
    let fixture = MemoryFixture::new();
    let shared_path = fixture.scope.database_path.clone();
    let first_scope = ProjectScope {
        workspace_root: fixture.scope.workspace_root.join("one"),
        project_id: "project-one".into(),
        database_path: shared_path.clone(),
    };
    let second_scope = ProjectScope {
        workspace_root: fixture.scope.workspace_root.join("two"),
        project_id: "project-two".into(),
        database_path: shared_path,
    };

    MemoryStore::open_with_embedder(first_scope, Arc::new(TestEmbedder))
        .unwrap()
        .remember(explicit_semantic(
            "Platform note A",
            "Rooted behavior should be explained clearly",
        ))
        .unwrap();

    let error = MemoryStore::open_with_embedder(second_scope, Arc::new(AlternateEmbedder))
        .unwrap()
        .search(SearchInput {
            query: "windows drive-relative path".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap_err();

    assert!(error.to_string().contains("embedder identity mismatch"));
    assert!(error.to_string().contains("test-embedder"));
    assert!(error.to_string().contains("alternate-embedder"));
}

#[test]
fn bootstrap_backfills_fts_for_existing_semantic_rows() {
    let fixture = MemoryFixture::new();
    let conn = Connection::open(&fixture.scope.database_path).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE semantic_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            source TEXT NOT NULL
        );
        ",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO semantic_memories (
            id, project_id, title, content, tags_json, created_at, updated_at, source
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            "legacy-memory",
            fixture.scope.project_id,
            "Windows root hint",
            "Drive-relative paths like \\.aws should be explained clearly",
            "[]",
            "00000000000000000001",
            "00000000000000000001",
            "explicit",
        ],
    )
    .unwrap();
    drop(conn);

    let hits = fixture
        .store()
        .search(SearchInput {
            query: "drive-relative path on windows".into(),
            limit: 5,
            level: None,
            tags: vec![],
        })
        .unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Windows root hint"));
}
