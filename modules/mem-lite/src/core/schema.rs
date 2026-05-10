use rusqlite::Connection;

pub fn bootstrap(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS semantic_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            source TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS episodic_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            content TEXT NOT NULL,
            event_kind TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS procedural_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_semantic_memories_project_created_at
            ON semantic_memories(project_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_episodic_memories_project_created_at
            ON episodic_memories(project_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_procedural_memories_project_created_at
            ON procedural_memories(project_id, created_at DESC);
        "#,
    )
}
