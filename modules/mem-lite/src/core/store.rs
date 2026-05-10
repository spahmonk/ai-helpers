use std::error::Error;
use std::fmt;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::core::project::ProjectScope;
use crate::core::schema;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
    Time(std::time::SystemTimeError),
    InvalidInput(&'static str),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(error) => error.fmt(f),
            StoreError::Sql(error) => error.fmt(f),
            StoreError::Json(error) => error.fmt(f),
            StoreError::Time(error) => error.fmt(f),
            StoreError::InvalidInput(message) => f.write_str(message),
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

pub struct MemoryStore {
    scope: ProjectScope,
    conn: Connection,
}

impl MemoryStore {
    pub fn open(scope: ProjectScope) -> Result<Self, StoreError> {
        if let Some(parent) = scope.database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&scope.database_path)?;
        schema::bootstrap(&conn)?;

        Ok(Self { scope, conn })
    }

    pub fn remember(&self, input: RememberInput) -> Result<(), StoreError> {
        let id = next_id()?;
        let timestamp = now_string()?;

        match input.level {
            MemoryLevel::Semantic => {
                self.conn.execute(
                    "INSERT INTO semantic_memories (
                        id, project_id, title, content, tags_json, created_at, updated_at, source
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        id,
                        self.scope.project_id,
                        input.title,
                        input.content,
                        serde_json::to_string(&input.tags)?,
                        timestamp,
                        timestamp,
                        input.source.as_str(),
                    ],
                )?;
            }
            MemoryLevel::Episodic => {
                if !input.tags.is_empty() {
                    return Err(StoreError::InvalidInput(
                        "episodic memories do not support tags",
                    ));
                }

                self.conn.execute(
                    "INSERT INTO episodic_memories (
                        id, project_id, content, event_kind, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        id,
                        self.scope.project_id,
                        input.content,
                        input.title,
                        timestamp,
                    ],
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
                        input.title,
                        input.content,
                        serde_json::to_string(&input.tags)?,
                        timestamp,
                        timestamp,
                    ],
                )?;
            }
        }

        Ok(())
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
