pub mod cli;
pub mod contracts;
pub mod mcp;

use std::path::PathBuf;

use crate::core::project::ProjectScope;
use crate::core::retrieval::SearchInput;
use crate::core::store::{
    MemoryLevel as CoreMemoryLevel, MemorySource, MemoryStore, RememberInput,
};

pub use cli::{CliAdapter, CliResult};
pub use contracts::*;
pub use mcp::McpAdapter;

#[derive(Clone, Copy, Default)]
pub struct MemoryServiceAdapter;

impl MemoryServiceAdapter {
    fn resolve_scope(root: Option<&str>) -> Result<ProjectScope, ServiceError> {
        let workspace_root = resolve_workspace_root(root)?;
        ProjectScope::from_workspace_root(&workspace_root).map_err(|error| {
            ServiceError::new(format!("{error}"))
        })
    }

    fn open_store(root: Option<&str>) -> Result<(ProjectScope, MemoryStore), ServiceError> {
        let scope = Self::resolve_scope(root)?;
        let store = MemoryStore::open(scope.clone())
            .map_err(|error| ServiceError::new(error.to_string()))?;
        Ok((scope, store))
    }

    fn summarize_title(title: Option<String>, content: &str) -> String {
        let trimmed_title = title.unwrap_or_default().trim().to_string();
        if !trimmed_title.is_empty() {
            return trimmed_title;
        }

        let line = content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("untitled");

        line.chars().take(80).collect()
    }
}

impl InitService for MemoryServiceAdapter {
    fn init(&self, request: InitRequest) -> Result<InitResponse, ServiceError> {
        let (scope, _store) = Self::open_store(request.root.as_deref())?;
        Ok(ProjectInfoResponse {
            project_id: scope.project_id,
            database_path: scope.database_path,
        })
    }
}

impl ProjectInfoService for MemoryServiceAdapter {
    fn project_info(
        &self,
        request: ProjectInfoRequest,
    ) -> Result<ProjectInfoResponse, ServiceError> {
        let scope = Self::resolve_scope(request.root.as_deref())?;
        Ok(ProjectInfoResponse {
            project_id: scope.project_id,
            database_path: scope.database_path,
        })
    }
}

impl RememberService for MemoryServiceAdapter {
    fn remember(&self, request: RememberRequest) -> Result<RememberResponse, ServiceError> {
        let (scope, store) = Self::open_store(request.root.as_deref())?;
        let title = Self::summarize_title(request.title, &request.content);
        let input = RememberInput {
            level: match request.level {
                MemoryLevel::Semantic => CoreMemoryLevel::Semantic,
                MemoryLevel::Episodic => CoreMemoryLevel::Episodic,
                MemoryLevel::Procedural => CoreMemoryLevel::Procedural,
            },
            title: title.clone(),
            content: request.content,
            tags: request.tags,
            source: MemorySource::Explicit,
        };

        store
            .remember(input)
            .map_err(|error| ServiceError::new(error.to_string()))?;

        Ok(RememberResponse {
            project_id: scope.project_id,
            level: request.level,
            title,
        })
    }
}

impl CaptureBatchService for MemoryServiceAdapter {
    fn capture_batch(
        &self,
        request: CaptureBatchRequest,
    ) -> Result<CaptureBatchResponse, ServiceError> {
        let (_scope, store) = Self::open_store(request.root.as_deref())?;
        let mut stored = 0usize;

        for entry in request.entries {
            let input = RememberInput {
                level: match entry.level.as_memory_level() {
                    MemoryLevel::Semantic => CoreMemoryLevel::Semantic,
                    MemoryLevel::Episodic => CoreMemoryLevel::Episodic,
                    MemoryLevel::Procedural => CoreMemoryLevel::Procedural,
                },
                title: Self::summarize_title(entry.title, &entry.content),
                content: entry.content,
                tags: entry.tags,
                source: MemorySource::Explicit,
            };

            store
                .remember(input)
                .map_err(|error| ServiceError::new(error.to_string()))?;
            stored += 1;
        }

        Ok(CaptureBatchResponse { stored })
    }
}

impl SearchService for MemoryServiceAdapter {
    fn search(&self, request: SearchRequest) -> Result<SearchResponse, ServiceError> {
        let (_scope, store) = Self::open_store(request.root.as_deref())?;
        let hits = store
            .search(SearchInput {
                query: request.query.clone(),
                limit: request.limit,
                level: Some(CoreMemoryLevel::Semantic),
                tags: request.tags,
            })
            .map_err(|error| ServiceError::new(error.to_string()))?
            .into_iter()
            .map(|hit| SearchHit {
                id: format!("{}:{}", hit.created_at, hit.title.clone().unwrap_or_default()),
                level: MemoryLevel::Semantic,
                title: hit.title,
                content: hit.content,
                score: hit.score,
                created_at: hit.created_at,
                tags: hit.tags,
            })
            .collect();

        Ok(SearchResponse {
            query: request.query,
            hits,
        })
    }
}

impl RecentService for MemoryServiceAdapter {
    fn recent(&self, request: RecentRequest) -> Result<RecentResponse, ServiceError> {
        let (_scope, store) = Self::open_store(request.root.as_deref())?;
        let memories = store
            .recent(request.limit)
            .map_err(|error| ServiceError::new(error.to_string()))?
            .into_iter()
            .map(|memory| RecentMemory {
                level: match memory.level {
                    CoreMemoryLevel::Semantic => MemoryLevel::Semantic,
                    CoreMemoryLevel::Episodic => MemoryLevel::Episodic,
                    CoreMemoryLevel::Procedural => MemoryLevel::Procedural,
                },
                title: Some(memory.title),
                content: memory.content,
                created_at: memory.created_at,
            })
            .collect();

        Ok(RecentResponse { memories })
    }
}

impl StatsService for MemoryServiceAdapter {
    fn stats(&self, request: StatsRequest) -> Result<StatsResponse, ServiceError> {
        let (_scope, store) = Self::open_store(request.root.as_deref())?;
        let stats = store
            .stats()
            .map_err(|error| ServiceError::new(error.to_string()))?;

        Ok(StatsResponse {
            stats: MemoryStats {
                semantic_count: stats.semantic_count,
                episodic_count: stats.episodic_count,
                procedural_count: stats.procedural_count,
            },
        })
    }
}

impl ProjectSummaryService for MemoryServiceAdapter {
    fn project_summary(
        &self,
        request: ProjectSummaryRequest,
    ) -> Result<ProjectSummaryResponse, ServiceError> {
        let (scope, store) = Self::open_store(request.root.as_deref())?;
        let stats = store
            .stats()
            .map_err(|error| ServiceError::new(error.to_string()))?;
        let recent = store
            .recent(10)
            .map_err(|error| ServiceError::new(error.to_string()))?;

        let mut summary = format!(
            "Project: {}\nWorkspace: {}\nMemory: {} semantic, {} episodic, {} procedural entries\n\nRecent:",
            scope.project_id,
            scope.workspace_root.display(),
            stats.semantic_count,
            stats.episodic_count,
            stats.procedural_count
        );

        if recent.is_empty() {
            summary.push_str("\n- none");
        } else {
            for memory in recent {
                let level = match memory.level {
                    CoreMemoryLevel::Semantic => "semantic",
                    CoreMemoryLevel::Episodic => "episodic",
                    CoreMemoryLevel::Procedural => "procedural",
                };
                let title = memory.title.trim();
                let title = if title.is_empty() { "(untitled)" } else { title };
                let snippet = memory
                    .content
                    .split_whitespace()
                    .take(18)
                    .collect::<Vec<_>>()
                    .join(" ");
                if snippet.is_empty() {
                    summary.push_str(&format!(
                        "\n- [{}] \"{}\" ({})",
                        level, title, memory.created_at
                    ));
                } else {
                    summary.push_str(&format!(
                        "\n- [{}] \"{}\" ({}) — {}",
                        level, title, memory.created_at, snippet
                    ));
                }
            }
        }

        Ok(ProjectSummaryResponse { summary })
    }
}

fn resolve_workspace_root(root: Option<&str>) -> Result<PathBuf, ServiceError> {
    let root = match root {
        Some(value) if !value.trim().is_empty() => PathBuf::from(value.trim()),
        Some(_) => return Err(ServiceError::new("root cannot be empty")),
        None => std::env::current_dir().map_err(|error| ServiceError::new(error.to_string()))?,
    };

    if root.is_absolute() {
        Ok(root)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .map_err(|error| ServiceError::new(error.to_string()))
    }
}
