use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceError {
    pub message: String,
}

impl ServiceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ServiceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryLevel {
    Semantic,
    Episodic,
    Procedural,
}

impl MemoryLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "semantic" => Some(Self::Semantic),
            "episodic" => Some(Self::Episodic),
            "procedural" => Some(Self::Procedural),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Episodic => "episodic",
            Self::Procedural => "procedural",
        }
    }
}

impl fmt::Display for MemoryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitRequest {
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectInfoRequest {
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberRequest {
    pub content: String,
    pub title: Option<String>,
    pub level: MemoryLevel,
    pub tags: Vec<String>,
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: String,
    pub limit: usize,
    pub tags: Vec<String>,
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentRequest {
    pub limit: usize,
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsRequest {
    pub root: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectInfoResponse {
    pub project_id: String,
    pub database_path: PathBuf,
}

pub type InitResponse = ProjectInfoResponse;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberResponse {
    pub project_id: String,
    pub level: MemoryLevel,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub level: MemoryLevel,
    pub title: Option<String>,
    pub content: String,
    pub score: f32,
    pub created_at: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResponse {
    pub query: String,
    pub hits: Vec<SearchHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentMemory {
    pub level: MemoryLevel,
    pub title: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentResponse {
    pub memories: Vec<RecentMemory>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryStats {
    pub semantic_count: usize,
    pub episodic_count: usize,
    pub procedural_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsResponse {
    pub stats: MemoryStats,
}

pub trait InitService {
    fn init(&self, request: InitRequest) -> Result<InitResponse, ServiceError>;
}

pub trait ProjectInfoService {
    fn project_info(
        &self,
        request: ProjectInfoRequest,
    ) -> Result<ProjectInfoResponse, ServiceError>;
}

pub trait RememberService {
    fn remember(&self, request: RememberRequest) -> Result<RememberResponse, ServiceError>;
}

pub trait SearchService {
    fn search(&self, request: SearchRequest) -> Result<SearchResponse, ServiceError>;
}

pub trait RecentService {
    fn recent(&self, request: RecentRequest) -> Result<RecentResponse, ServiceError>;
}

pub trait StatsService {
    fn stats(&self, request: StatsRequest) -> Result<StatsResponse, ServiceError>;
}
