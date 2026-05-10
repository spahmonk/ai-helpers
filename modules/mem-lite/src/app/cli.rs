use crate::app::contracts::{
    InitRequest, MemoryLevel, ProjectInfoRequest, RecentRequest, RememberRequest, SearchRequest,
    StatsRequest,
};
use crate::app::contracts::{
    InitService, ProjectInfoService, RecentService, RememberService, SearchService, StatsService,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliResult {
    pub output: String,
    pub exit_code: i32,
}

pub struct CliAdapter<S>
where
    S: InitService + ProjectInfoService + RememberService + SearchService + RecentService + StatsService,
{
    services: S,
}

impl<S> CliAdapter<S>
where
    S: InitService + ProjectInfoService + RememberService + SearchService + RecentService + StatsService,
{
    pub fn new(services: S) -> Self {
        Self { services }
    }

    pub fn run(&self, args: Vec<String>) -> CliResult {
        let Some(command) = args.first().map(String::as_str) else {
            return CliResult {
                output: help_text(),
                exit_code: 1,
            };
        };

        match command {
            "init" => self.handle_init(&args[1..]),
            "remember" => self.handle_remember(&args[1..]),
            "search" => self.handle_search(&args[1..]),
            "recent" => self.handle_recent(&args[1..]),
            "stats" => self.handle_stats(&args[1..]),
            "project-info" => self.handle_project_info(&args[1..]),
            "--help" | "-h" => CliResult {
                output: help_text(),
                exit_code: 0,
            },
            _ => CliResult {
                output: format!("Error: unknown command `{command}`\n\n{}", help_text()),
                exit_code: 1,
            },
        }
    }

    fn handle_init(&self, args: &[String]) -> CliResult {
        match parse_root_only(args).and_then(|root| {
            self.services
                .init(InitRequest { root })
                .map_err(|error| error.message)
        }) {
            Ok(response) => CliResult {
                output: format!(
                    "Initialized project {} at {}\n",
                    response.project_id,
                    response.database_path.display()
                ),
                exit_code: 0,
            },
            Err(message) => error_result(message),
        }
    }

    fn handle_remember(&self, args: &[String]) -> CliResult {
        let mut content = None;
        let mut title = None;
        let mut level = MemoryLevel::Semantic;
        let mut tags = Vec::new();
        let mut root = None;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--title" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --title".to_string());
                    };
                    title = Some(value.clone());
                    index += 2;
                }
                "--level" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --level".to_string());
                    };
                    level = match MemoryLevel::parse(value) {
                        Some(parsed) => parsed,
                        None => return error_result(format!("invalid level `{value}`")),
                    };
                    index += 2;
                }
                "--tags" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --tags".to_string());
                    };
                    tags = parse_csv(value);
                    index += 2;
                }
                "--root" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --root".to_string());
                    };
                    root = Some(value.clone());
                    index += 2;
                }
                token if token.starts_with("--") => {
                    return error_result(format!("unknown option `{token}`"));
                }
                token => {
                    if content.is_some() {
                        return error_result("remember accepts a single content argument".to_string());
                    }
                    content = Some(token.to_string());
                    index += 1;
                }
            }
        }

        let Some(content) = content else {
            return error_result("remember requires a content argument".to_string());
        };

        match self.services.remember(RememberRequest {
            content,
            title,
            level,
            tags,
            root,
        }) {
            Ok(response) => CliResult {
                output: format!(
                    "Stored {} memory in project {}\n",
                    response.level, response.project_id
                ),
                exit_code: 0,
            },
            Err(error) => error_result(error.message),
        }
    }

    fn handle_search(&self, args: &[String]) -> CliResult {
        let mut query = None;
        let mut limit = 10usize;
        let mut tags = Vec::new();
        let mut root = None;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--limit" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --limit".to_string());
                    };
                    limit = match value.parse::<usize>() {
                        Ok(parsed) => parsed,
                        Err(_) => return error_result(format!("invalid limit `{value}`")),
                    };
                    index += 2;
                }
                "--tags" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --tags".to_string());
                    };
                    tags = parse_csv(value);
                    index += 2;
                }
                "--root" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --root".to_string());
                    };
                    root = Some(value.clone());
                    index += 2;
                }
                token if token.starts_with("--") => {
                    return error_result(format!("unknown option `{token}`"));
                }
                token => {
                    if query.is_some() {
                        return error_result("search accepts a single query argument".to_string());
                    }
                    query = Some(token.to_string());
                    index += 1;
                }
            }
        }

        let Some(query) = query else {
            return error_result("search requires a query argument".to_string());
        };

        match self.services.search(SearchRequest {
            query: query.clone(),
            limit,
            tags,
            root,
        }) {
            Ok(response) => CliResult {
                output: format_search_response(&response.query, &response.hits),
                exit_code: 0,
            },
            Err(error) => error_result(error.message),
        }
    }

    fn handle_recent(&self, args: &[String]) -> CliResult {
        let mut limit = 20usize;
        let mut root = None;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--limit" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --limit".to_string());
                    };
                    limit = match value.parse::<usize>() {
                        Ok(parsed) => parsed,
                        Err(_) => return error_result(format!("invalid limit `{value}`")),
                    };
                    index += 2;
                }
                "--root" => {
                    let Some(value) = args.get(index + 1) else {
                        return error_result("missing value for --root".to_string());
                    };
                    root = Some(value.clone());
                    index += 2;
                }
                token if token.starts_with("--") => {
                    return error_result(format!("unknown option `{token}`"));
                }
                token => {
                    return error_result(format!("unexpected argument `{token}`"));
                }
            }
        }

        match self.services.recent(RecentRequest { limit, root }) {
            Ok(response) => CliResult {
                output: format_recent_response(&response.memories),
                exit_code: 0,
            },
            Err(error) => error_result(error.message),
        }
    }

    fn handle_stats(&self, args: &[String]) -> CliResult {
        match parse_root_only(args).and_then(|root| {
            self.services
                .stats(StatsRequest { root })
                .map_err(|error| error.message)
        }) {
            Ok(response) => CliResult {
                output: format!(
                    "semantic: {}\nepisodic: {}\nprocedural: {}\n",
                    response.stats.semantic_count,
                    response.stats.episodic_count,
                    response.stats.procedural_count
                ),
                exit_code: 0,
            },
            Err(message) => error_result(message),
        }
    }

    fn handle_project_info(&self, args: &[String]) -> CliResult {
        match parse_root_only(args).and_then(|root| {
            self.services
                .project_info(ProjectInfoRequest { root })
                .map_err(|error| error.message)
        }) {
            Ok(response) => CliResult {
                output: format!(
                    "project_id: {}\ndatabase_path: {}\n",
                    response.project_id,
                    response.database_path.display()
                ),
                exit_code: 0,
            },
            Err(message) => error_result(message),
        }
    }
}

fn parse_root_only(args: &[String]) -> Result<Option<String>, String> {
    let mut root = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("missing value for --root".to_string());
                };
                root = Some(value.clone());
                index += 2;
            }
            token if token.starts_with("--") => {
                return Err(format!("unknown option `{token}`"));
            }
            token => return Err(format!("unexpected argument `{token}`")),
        }
    }
    Ok(root)
}

fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn format_search_response(query: &str, hits: &[crate::app::contracts::SearchHit]) -> String {
    if hits.is_empty() {
        return format!("No memories found for `{query}`\n");
    }

    let mut out = format!("Results for `{query}`:\n");
    for (index, hit) in hits.iter().enumerate() {
        let title = hit.title.as_deref().unwrap_or("(untitled)");
        let tags = if hit.tags.is_empty() {
            String::from("none")
        } else {
            hit.tags.join(", ")
        };
        out.push_str(&format!(
            "{}. [{}] {} (score {:.3})\n   created: {}\n   tags: {}\n   content: {}\n",
            index + 1,
            hit.level,
            title,
            hit.score,
            hit.created_at,
            tags,
            hit.content
        ));
    }
    out
}

fn format_recent_response(memories: &[crate::app::contracts::RecentMemory]) -> String {
    if memories.is_empty() {
        return "No recent memories found\n".to_string();
    }

    let mut out = String::from("Recent memories:\n");
    for (index, memory) in memories.iter().enumerate() {
        let title = memory.title.as_deref().unwrap_or("(untitled)");
        out.push_str(&format!(
            "{}. [{}] {}\n   created: {}\n   content: {}\n",
            index + 1,
            memory.level,
            title,
            memory.created_at,
            memory.content
        ));
    }
    out
}

fn error_result(message: String) -> CliResult {
    CliResult {
        output: format!("Error: {message}\n"),
        exit_code: 1,
    }
}

fn help_text() -> String {
    "mem-lite commands:\n  init [--root <dir>]\n  remember <content> [--title <title>] [--level semantic|episodic|procedural] [--tags tag1,tag2] [--root <dir>]\n  search <query> [--limit N] [--tags tag1,tag2] [--root <dir>]\n  recent [--limit N] [--root <dir>]\n  stats [--root <dir>]\n  project-info [--root <dir>]\n".to_string()
}
