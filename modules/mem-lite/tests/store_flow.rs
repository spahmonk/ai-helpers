use std::thread;
use std::time::Duration;

use mem_lite::{MemoryLevel, MemorySource, MemoryStore, ProjectScope, RememberInput};
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

    thread::sleep(Duration::from_millis(5));

    store
        .remember(RememberInput {
            level: MemoryLevel::Procedural,
            title: "Second memory".into(),
            content: "Newer entry".into(),
            tags: vec![],
            source: MemorySource::Explicit,
        })
        .unwrap();

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
