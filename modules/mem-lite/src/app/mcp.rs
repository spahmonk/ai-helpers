use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};

use crate::app::contracts::{
    CaptureBatchEntry, CaptureBatchRequest, CaptureBatchService, InitService, MemoryLevel,
    ProjectInfoRequest, ProjectInfoService, ProjectSummaryRequest, ProjectSummaryService,
    RecentRequest, RecentService, RememberRequest, RememberService, SearchRequest, SearchService,
    StatsRequest, StatsService,
};

#[derive(Clone, Copy)]
enum MessageFormat {
    ContentLength,
    JsonLine,
}

enum ReadMessageError {
    Io(io::Error),
    Parse {
        format: MessageFormat,
        message: String,
        recoverable: bool,
    },
}

pub struct McpAdapter<S>
where
    S: InitService
        + ProjectInfoService
        + RememberService
        + SearchService
        + RecentService
        + StatsService
        + CaptureBatchService
        + ProjectSummaryService,
{
    services: S,
}

impl<S> McpAdapter<S>
where
    S: InitService
        + ProjectInfoService
        + RememberService
        + SearchService
        + RecentService
        + StatsService
        + CaptureBatchService
        + ProjectSummaryService,
{
    pub fn new(services: S) -> Self {
        Self { services }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        loop {
            let (request, format) = match Self::read_message(&mut reader) {
                Ok(Some(message)) => message,
                Ok(None) => break,
                Err(ReadMessageError::Parse {
                    format,
                    message,
                    recoverable,
                }) => {
                    let response = Self::error_response(None, -32700, message);
                    Self::write_message(&mut writer, &response, format)?;
                    if recoverable {
                        continue;
                    }
                    break;
                }
                Err(ReadMessageError::Io(error)) => return Err(Box::new(error)),
            };

            if request.get("id").is_none() {
                continue;
            }

            let response = match self.handle_request(&request) {
                Ok(response) => response,
                Err(error) => Self::error_response(
                    request.get("id").cloned(),
                    Self::error_code(&*error),
                    error.to_string(),
                ),
            };
            Self::write_message(&mut writer, &response, format)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn handle_request(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| boxed_error("Missing method"))?;

        let mut response = match method {
            "initialize" => self.handle_initialize(request),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(request),
            _ => Err(boxed_error("Unknown method")),
        }?;

        if let (Some(id), Some(object)) = (request.get("id"), response.as_object_mut()) {
            object.insert("id".to_string(), id.clone());
        }

        Ok(response)
    }

    fn read_message<R: BufRead>(
        reader: &mut R,
    ) -> Result<Option<(Value, MessageFormat)>, ReadMessageError> {
        loop {
            let mut first_line = String::new();
            let bytes = reader.read_line(&mut first_line).map_err(ReadMessageError::Io)?;
            if bytes == 0 {
                return Ok(None);
            }

            let trimmed = first_line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('{') {
                let request =
                    serde_json::from_str(trimmed).map_err(|error| ReadMessageError::Parse {
                        format: MessageFormat::JsonLine,
                        message: error.to_string(),
                        recoverable: true,
                    })?;
                return Ok(Some((request, MessageFormat::JsonLine)));
            }

            let mut content_length = None;
            Self::capture_content_length(&first_line, &mut content_length)?;

            loop {
                let mut line = String::new();
                let bytes = reader.read_line(&mut line).map_err(ReadMessageError::Io)?;
                if bytes == 0 {
                    return Ok(None);
                }
                if line.trim_end_matches(['\r', '\n']).is_empty() {
                    break;
                }
                Self::capture_content_length(&line, &mut content_length)?;
            }

            let content_length = content_length.ok_or_else(|| ReadMessageError::Parse {
                format: MessageFormat::ContentLength,
                message: "Missing Content-Length header".to_string(),
                recoverable: false,
            })?;

            let mut body = vec![0_u8; content_length];
            reader.read_exact(&mut body).map_err(ReadMessageError::Io)?;
            let request =
                serde_json::from_slice(&body).map_err(|error| ReadMessageError::Parse {
                    format: MessageFormat::ContentLength,
                    message: error.to_string(),
                    recoverable: true,
                })?;

            return Ok(Some((request, MessageFormat::ContentLength)));
        }
    }

    fn capture_content_length(
        line: &str,
        content_length: &mut Option<usize>,
    ) -> Result<(), ReadMessageError> {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("Content-Length") {
                *content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                    ReadMessageError::Parse {
                        format: MessageFormat::ContentLength,
                        message: error.to_string(),
                        recoverable: false,
                    }
                })?);
            }
        }
        Ok(())
    }

    fn write_message<W: Write>(
        writer: &mut W,
        response: &Value,
        format: MessageFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(response)?;
        match format {
            MessageFormat::ContentLength => {
                write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
                writer.write_all(&body)?;
            }
            MessageFormat::JsonLine => {
                writer.write_all(&body)?;
                writer.write_all(b"\n")?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    fn error_code(error: &dyn std::error::Error) -> i64 {
        match error.to_string().as_str() {
            "Unknown method" | "Unknown tool" => -32601,
            "Missing method" | "Missing tool name" | "Missing arguments" => -32600,
            _ => -32603,
        }
    }

    fn error_response(id: Option<Value>, code: i64, message: String) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": {
                "code": code,
                "message": message
            }
        })
    }

    fn handle_initialize(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let protocol_version = request
            .get("params")
            .and_then(|params| params.get("protocolVersion"))
            .and_then(|value| value.as_str())
            .unwrap_or("2024-11-05");

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": protocol_version,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "mem-lite-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }))
    }

    fn handle_list_tools(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": [
                    tool_schema("remember", "Store a new memory", json!({
                        "type": "object",
                        "properties": {
                            "content": { "type": "string" },
                            "title": { "type": "string" },
                            "level": { "type": "string", "enum": ["semantic", "episodic", "procedural"], "default": "semantic" },
                            "tags": { "type": "array", "items": { "type": "string" } },
                            "root": { "type": "string" }
                        },
                        "required": ["content"]
                    })),
                    tool_schema("capture_batch", "Store a batch of memories", json!({
                        "type": "object",
                        "properties": {
                            "entries": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "level": { "type": "string", "enum": ["semantic", "episodic", "procedural"] },
                                        "title": { "type": ["string", "null"] },
                                        "content": { "type": "string" },
                                        "tags": { "type": "array", "items": { "type": "string" } }
                                    },
                                    "required": ["level", "content"]
                                }
                            },
                            "root": { "type": "string" }
                        },
                        "required": ["entries"]
                    })),
                    tool_schema("search", "Search semantic memories", json!({
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" },
                            "limit": { "type": "integer", "default": 10 },
                            "tags": { "type": "array", "items": { "type": "string" } },
                            "root": { "type": "string" }
                        },
                        "required": ["query"]
                    })),
                    tool_schema("recent", "List recent memories", json!({
                        "type": "object",
                        "properties": {
                            "limit": { "type": "integer", "default": 20 },
                            "root": { "type": "string" }
                        }
                    })),
                    tool_schema("stats", "Get memory counts", json!({
                        "type": "object",
                        "properties": {
                            "root": { "type": "string" }
                        }
                    })),
                    tool_schema("project_info", "Get project info", json!({
                        "type": "object",
                        "properties": {
                            "root": { "type": "string" }
                        }
                    })),
                    tool_schema("project_summary", "Summarize project memories", json!({
                        "type": "object",
                        "properties": {
                            "root": { "type": "string" }
                        }
                    })),
                ]
            }
        }))
    }

    fn handle_call_tool(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let params = request
            .get("params")
            .ok_or_else(|| boxed_error("Missing arguments"))?;
        let name = params
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| boxed_error("Missing tool name"))?;
        let arguments = params
            .get("arguments")
            .and_then(|value| value.as_object())
            .or_else(|| params.as_object())
            .ok_or_else(|| boxed_error("Missing arguments"))?;

        // Unknown tool name is a protocol error → propagate as Err so run() emits a
        // JSON-RPC error object (code -32601) rather than a successful isError response.
        let call_result: Result<String, String> = match name {
            "remember" => self.call_remember(arguments),
            "capture_batch" => self.call_capture_batch(arguments),
            "search" => self.call_search(arguments),
            "recent" => self.call_recent(arguments),
            "stats" => self.call_stats(arguments),
            "project_info" => self.call_project_info(arguments),
            "project_summary" => self.call_project_summary(arguments),
            _ => return Err(boxed_error("Unknown tool")),
        };

        match call_result {
            Ok(text) => Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{ "type": "text", "text": text }]
                }
            })),
            Err(message) => Ok(json!({
                "jsonrpc": "2.0",
                "result": {
                    "content": [{ "type": "text", "text": message }],
                    "isError": true
                }
            })),
        }
    }

    fn call_remember(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let content = argument_string(arguments, "content")?;
        let title = arguments
            .get("title")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let level = arguments
            .get("level")
            .and_then(|value| value.as_str())
            .and_then(MemoryLevel::parse)
            .unwrap_or(MemoryLevel::Semantic);
        let tags = argument_strings(arguments, "tags")?;
        let root = argument_optional_string(arguments, "root")?;

        let response = self
            .services
            .remember(RememberRequest {
                content,
                title,
                level,
                tags,
                root,
            })
            .map_err(|error| error.message)?;

        Ok(format!(
            "Stored {} memory in project {}",
            response.level, response.project_id
        ))
    }

    fn call_capture_batch(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let entries_value = arguments
            .get("entries")
            .ok_or_else(|| "Missing entries".to_string())?;
        let entries: Vec<CaptureBatchEntry> = serde_json::from_value(entries_value.clone())
            .map_err(|error| error.to_string())?;
        let root = argument_optional_string(arguments, "root")?;
        let response = self
            .services
            .capture_batch(CaptureBatchRequest { entries, root })
            .map_err(|error| error.message)?;

        Ok(format!("Captured {} memories", response.stored))
    }

    fn call_search(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let query = argument_string(arguments, "query")?;
        let limit = argument_usize(arguments, "limit").unwrap_or(10);
        let tags = argument_strings(arguments, "tags")?;
        let root = argument_optional_string(arguments, "root")?;

        let response = self
            .services
            .search(SearchRequest {
                query: query.clone(),
                limit,
                tags,
                root,
            })
            .map_err(|error| error.message)?;

        Ok(format_search_text(&response.query, &response.hits))
    }

    fn call_recent(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let limit = argument_usize(arguments, "limit").unwrap_or(20);
        let root = argument_optional_string(arguments, "root")?;
        let response = self
            .services
            .recent(RecentRequest { limit, root })
            .map_err(|error| error.message)?;

        Ok(format_recent_text(&response.memories))
    }

    fn call_stats(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let root = argument_optional_string(arguments, "root")?;
        let response = self
            .services
            .stats(StatsRequest { root })
            .map_err(|error| error.message)?;

        Ok(format!(
            "semantic: {}\nepisodic: {}\nprocedural: {}",
            response.stats.semantic_count,
            response.stats.episodic_count,
            response.stats.procedural_count
        ))
    }

    fn call_project_info(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let root = argument_optional_string(arguments, "root")?;
        let response = self
            .services
            .project_info(ProjectInfoRequest { root })
            .map_err(|error| error.message)?;

        Ok(format!(
            "project_id: {}\ndatabase_path: {}",
            response.project_id,
            response.database_path.display()
        ))
    }

    fn call_project_summary(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, String> {
        let root = argument_optional_string(arguments, "root")?;
        let response = self
            .services
            .project_summary(ProjectSummaryRequest { root })
            .map_err(|error| error.message)?;

        Ok(response.summary)
    }
}

fn tool_schema(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn boxed_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

fn argument_string(
    arguments: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .ok_or_else(|| format!("Missing {key}"))
}

fn argument_optional_string(
    arguments: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| format!("invalid {key}")),
    }
}

fn argument_usize(arguments: &serde_json::Map<String, Value>, key: &str) -> Option<usize> {
    arguments.get(key).and_then(|value| value.as_u64()).map(|value| value as usize)
}

fn argument_strings(
    arguments: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    match arguments.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(|item| item.to_string())
                    .ok_or_else(|| format!("invalid {key}"))
            })
            .collect(),
        Some(_) => Err(format!("invalid {key}")),
        None => Ok(Vec::new()),
    }
}

fn format_search_text(query: &str, hits: &[crate::app::contracts::SearchHit]) -> String {
    if hits.is_empty() {
        return format!("No memories found for `{query}`");
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
    out.trim_end().to_string()
}

fn format_recent_text(memories: &[crate::app::contracts::RecentMemory]) -> String {
    if memories.is_empty() {
        return "No recent memories found".to_string();
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
    out.trim_end().to_string()
}
