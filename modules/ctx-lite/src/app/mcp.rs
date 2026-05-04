use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};

use crate::app::contracts::{
    DoctorRequest, DoctorService, ReadRequest, ReadService, SearchRequest, SearchService,
    ShellRequest, ShellService, TreeRequest, TreeService,
};
use crate::core::capabilities::ShellCapabilityId;
use crate::core::config::AppConfig;

enum MessageFormat {
    ContentLength,
    JsonLine,
}

pub struct McpAdapter<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    config: AppConfig,
    read: Read,
    tree: Tree,
    search: Search,
    shell: Shell,
    doctor: Doctor,
}

impl<Read, Tree, Search, Shell, Doctor> McpAdapter<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    pub fn new(
        config: AppConfig,
        read: Read,
        tree: Tree,
        search: Search,
        shell: Shell,
        doctor: Doctor,
    ) -> Self {
        Self {
            config,
            read,
            tree,
            search,
            shell,
            doctor,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        while let Some((request, format)) = Self::read_message(&mut reader)? {
            if request.get("id").is_none() {
                continue;
            }

            let response = self.handle_request(&request)?;
            Self::write_message(&mut writer, &response, format)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn handle_request(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or("Missing method")?;

        let mut response = match method {
            "initialize" => self.handle_initialize(request),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(request),
            _ => Err("Unknown method".into()),
        }?;

        if let (Some(id), Some(object)) = (request.get("id"), response.as_object_mut()) {
            object.insert("id".to_string(), id.clone());
        }

        Ok(response)
    }

    fn read_message<R: BufRead>(
        reader: &mut R,
    ) -> Result<Option<(Value, MessageFormat)>, Box<dyn std::error::Error>> {
        loop {
            let mut first_line = String::new();
            let bytes = reader.read_line(&mut first_line)?;

            if bytes == 0 {
                return Ok(None);
            }

            let trimmed = first_line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('{') {
                return Ok(Some((
                    serde_json::from_str(trimmed)?,
                    MessageFormat::JsonLine,
                )));
            }

            let mut content_length = None;
            Self::capture_content_length(&first_line, &mut content_length)?;

            loop {
                let mut line = String::new();
                let bytes = reader.read_line(&mut line)?;

                if bytes == 0 {
                    return Ok(None);
                }

                if line.trim_end_matches(['\r', '\n']).is_empty() {
                    break;
                }

                Self::capture_content_length(&line, &mut content_length)?;
            }

            let content_length = content_length.ok_or("Missing Content-Length header")?;
            let mut body = vec![0_u8; content_length];
            reader.read_exact(&mut body)?;

            return Ok(Some((
                serde_json::from_slice(&body)?,
                MessageFormat::ContentLength,
            )));
        }
    }

    fn capture_content_length(
        line: &str,
        content_length: &mut Option<usize>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("Content-Length") {
                *content_length = Some(value.trim().parse::<usize>()?);
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

    fn text_content(text: String) -> Value {
        json!([
            {
                "type": "text",
                "text": text
            }
        ])
    }

    fn handle_initialize(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let protocol_version = request
            .get("params")
            .and_then(|params| params.get("protocolVersion"))
            .and_then(|version| version.as_str())
            .unwrap_or("2024-11-05");

        let instructions = self.build_instructions();

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": protocol_version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ctx-lite-mcp",
                    "version": "0.1.0"
                },
                "instructions": instructions
            }
        }))
    }

    fn build_instructions(&self) -> String {
        let shell_section = if !self.config.shell_enabled {
            "### 4. **shell** - Execute commands\n- Shell execution is disabled in this configuration\n- Use search, read, and tree for code exploration instead\n".to_string()
        } else {
            match self.config.resolve_shell_policy() {
                Ok(policy) => {
                    let caps = &policy.active_capabilities;
                    let mut lines = vec![
                        "### 4. **shell** - Execute whitelisted shell commands".to_string(),
                        format!("- Active profile: **{}**", policy.active_profile.as_str()),
                        "- Only explicitly allowed commands run; shell wrappers, redirects, and chaining remain blocked".to_string(),
                    ];

                    if caps.contains(&ShellCapabilityId::GitInspect) {
                        lines.push("- Git inspection: `git status --short`, `git diff --stat`, `git log --oneline -n 20`, `git ls-files`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerInspect) {
                        lines.push("- Docker inspection: `docker ps`, `docker inspect <name>`, `docker compose config`, `docker version`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerLogs) {
                        lines.push("- Docker logs: `docker logs <name>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerComposePs) || caps.contains(&ShellCapabilityId::DockerComposeLogs) {
                        lines.push("- Compose diagnostics: `docker compose ps`, `docker compose logs <svc>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::NpmTest) {
                        lines.push("- npm test: `npm test`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::NpmBuild) || caps.contains(&ShellCapabilityId::NpmLint) || caps.contains(&ShellCapabilityId::NpmTypecheck) {
                        lines.push("- npm build/lint/typecheck: `npm run build`, `npm run lint`, `npm run typecheck`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::NpmInstall) {
                        lines.push("- npm install: `npm install`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::CargoTest) {
                        lines.push("- Cargo test: `cargo test <filter>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::CargoBuild) || caps.contains(&ShellCapabilityId::CargoCheck) || caps.contains(&ShellCapabilityId::CargoClippy) || caps.contains(&ShellCapabilityId::CargoFmtCheck) {
                        lines.push("- Cargo build/check/clippy/fmt: `cargo build`, `cargo check`, `cargo clippy --all-targets --all-features`, `cargo fmt --check`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::CargoRun) {
                        lines.push("- Cargo run: `cargo run`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::PythonPytest) || caps.contains(&ShellCapabilityId::Python3Pytest) {
                        lines.push("- Python tests: `python -m pytest <path>`, `python3 -m pytest <path>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::RubyRspec) {
                        lines.push("- Ruby/RSpec: `bundle exec rspec <path>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerRun) {
                        lines.push("- Docker run: `docker run <image>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerBuild) {
                        lines.push("- Docker build: `docker build <path>`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerComposeUp) {
                        lines.push("- Docker compose up: `docker compose up`".to_string());
                    }
                    if caps.contains(&ShellCapabilityId::DockerExec) {
                        lines.push("- Docker exec: `docker exec <container> <cmd>`".to_string());
                    }
                    if !policy.allowlist_patterns.is_empty() {
                        lines.push(format!("- Custom allowlist: {} additional pattern(s)", policy.allowlist_patterns.len()));
                    }

                    lines.join("\n")
                }
                Err(_) => {
                    "### 4. **shell** - Execute whitelisted shell commands\n- Use for git inspection, docker diagnostics, and build/test workflows\n- Whitelist-protected for security\n- Returns stdout, stderr, exit code".to_string()
                }
            }
        };

        format!(
            "# ctx-lite: Fast Context Extraction for AI Coding\n\n\
             ## Overview\n\
             ctx-lite is a high-performance context extractor and compression tool optimized for AI coding assistants. \
             Use these tools to efficiently gather and analyze code context.\n\n\
             ## Best Practices\n\n\
             ### 1. **search** - Find code quickly\n\
             - Use FIRST to locate relevant files and code patterns\n\
             - Search for: function names, error messages, patterns, variable names\n\
             - Efficient: searches in parallel, returns line numbers\n\
             - Example: `search \"function handleRequest\"` before reading files\n\n\
             ### 2. **read** - Extract file contents\n\
             - Use AFTER search to get file contents\n\
             - Respects security boundaries (can't read outside allowed paths)\n\
             - Automatically truncates large files\n\
             - For large codebases: read multiple small files vs one huge file\n\n\
             ### 3. **tree** - Understand structure\n\
             - Use to explore codebase organization\n\
             - Shows directory structure with depth control\n\
             - Fast way to find related files\n\
             - Use max_depth=2-3 for large codebases\n\n\
             {shell_section}\n\n\
             ### 5. **doctor** - Diagnose environment\n\
             - Use to verify setup is correct\n\
             - Checks: security policies, shell access, storage\n\
             - Run when troubleshooting issues\n\n\
             ## Performance Tips\n\n\
             - **search first**: Always search before reading to know what you need\n\
             - **tree for structure**: Use tree to understand layout before diving in\n\
             - **batch reads**: Read related files together to build context\n\
             - **use shell selectively**: Prefer the smallest safe command that answers the question\n\
             - **max_depth**: Limit tree depth for large codebases (2-3 levels)\n\n\
             ## Limitations\n\n\
             - **Security**: Respects path jail - cannot read outside allowed directories\n\
             - **Size**: Large files are automatically truncated (user-configurable)\n\
             - **Whitelist**: Shell commands must match the configured allowlist\n\
             - **Frequency**: Queries are fast but batching reduces overhead\n"
        )
    }

    fn effective_shell_tool_description(&self) -> String {
        if !self.config.shell_enabled {
            return "Execute shell commands - currently disabled in this configuration.".to_string();
        }
        match self.config.resolve_shell_policy() {
            Ok(policy) => {
                let caps = &policy.active_capabilities;
                let mut classes = Vec::new();
                if caps.contains(&ShellCapabilityId::GitInspect) {
                    classes.push("safe git inspection");
                }
                if caps.contains(&ShellCapabilityId::DockerInspect) || caps.contains(&ShellCapabilityId::DockerLogs) {
                    classes.push("docker diagnostics");
                }
                if caps.contains(&ShellCapabilityId::NpmTest) || caps.contains(&ShellCapabilityId::NpmBuild) || caps.contains(&ShellCapabilityId::NpmLint) {
                    classes.push("npm build/test workflows");
                }
                if caps.contains(&ShellCapabilityId::NpmInstall) {
                    classes.push("npm install");
                }
                if caps.contains(&ShellCapabilityId::CargoTest) || caps.contains(&ShellCapabilityId::CargoBuild) {
                    classes.push("cargo build/test");
                }
                if caps.contains(&ShellCapabilityId::CargoRun) {
                    classes.push("cargo run");
                }
                if caps.contains(&ShellCapabilityId::PythonPytest) || caps.contains(&ShellCapabilityId::Python3Pytest) {
                    classes.push("python pytest");
                }
                if caps.contains(&ShellCapabilityId::RubyRspec) {
                    classes.push("ruby/rspec");
                }
                if caps.contains(&ShellCapabilityId::DockerRun) || caps.contains(&ShellCapabilityId::DockerBuild) || caps.contains(&ShellCapabilityId::DockerComposeUp) {
                    classes.push("docker run/build/compose");
                }

                if classes.is_empty() {
                    format!(
                        "Execute whitelisted shell commands - profile: {}. Whitelist-protected for security.",
                        policy.active_profile.as_str()
                    )
                } else {
                    format!(
                        "Execute whitelisted shell commands - profile: {}. Active command classes: {}. Whitelist-protected; shell wrappers, redirects, and chaining are blocked.",
                        policy.active_profile.as_str(),
                        classes.join(", ")
                    )
                }
            }
            Err(_) => {
                "Execute whitelisted shell commands - Use for git, docker, npm, cargo, python, and ruby workflows. Whitelist-protected for security.".to_string()
            }
        }
    }

    fn handle_list_tools(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let shell_desc = self.effective_shell_tool_description();
        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": [
                    {
                        "name": "read",
                        "description": "Read file contents - Use AFTER search to get exact file content. Fast and efficient for extracting code context. Respects security boundaries.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "File path to read (relative or absolute)"
                                },
                                "max_bytes": {
                                    "type": "integer",
                                    "description": "Maximum bytes to read - optional, useful for limiting output from large files"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "tree",
                        "description": "List directory structure - Use to explore codebase organization. Shows depth and file types. Use max_depth=2-3 for large codebases.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Directory path to explore (current dir if omitted)"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum depth to show (2-3 recommended for large codebases)"
                                },
                                "include_hidden": {
                                    "type": "boolean",
                                    "description": "Include hidden files (.git, .env, etc)"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "search",
                        "description": "Search files by pattern or text - Use FIRST to locate relevant code. Fast parallel search. Returns line numbers and context. Use before read.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search pattern - supports regex. Search for: function names, error messages, patterns, variables"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Max results to return (default: 50)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "shell",
                        "description": shell_desc,
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": {
                                    "type": "string",
                                    "description": "Command to execute - must match the configured allowlist"
                                },
                                "cwd": {
                                    "type": "string",
                                    "description": "Working directory for command (optional)"
                                }
                            },
                            "required": ["command"]
                        }
                    },
                    {
                        "name": "doctor",
                        "description": "Run diagnostic checks - Verifies ctx-lite setup, security policies, shell access, storage. Use to troubleshoot issues.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "include_storage": {
                                    "type": "boolean",
                                    "description": "Include storage/cache diagnostics"
                                },
                                "include_shell_policy": {
                                    "type": "boolean",
                                    "description": "Include shell security policy checks"
                                }
                            }
                        }
                    }
                ]
            }
        }))
    }

    fn handle_call_tool(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let name = request
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .ok_or("Missing tool name")?;

        let arguments = request
            .get("params")
            .and_then(|p| p.get("arguments"))
            .ok_or("Missing arguments")?;

        match name {
            "read" => self.call_read_tool(arguments),
            "tree" => self.call_tree_tool(arguments),
            "search" => self.call_search_tool(arguments),
            "shell" => self.call_shell_tool(arguments),
            "doctor" => self.call_doctor_tool(arguments),
            _ => Err("Unknown tool".into()),
        }
    }

    fn call_read_tool(&self, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing path")?
            .to_string();

        let max_bytes = arguments
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let request = ReadRequest { path, max_bytes };
        let normalized = request.normalize(&self.config).map_err(|e| e.reason)?;
        let response = self.read.read(normalized).map_err(|e| e.message)?;

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": Self::text_content(response.content.clone()),
                "truncated": response.truncated
            }
        }))
    }

    fn call_tree_tool(&self, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing path")?
            .to_string();

        let max_depth = arguments
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let include_hidden = arguments
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let request = TreeRequest {
            path,
            max_depth,
            include_hidden,
        };
        let normalized = request.normalize(&self.config).map_err(|e| e.reason)?;
        let response = self.tree.tree(normalized).map_err(|e| e.message)?;

        let entries: Vec<Value> = response
            .entries
            .iter()
            .map(|e| {
                json!({
                    "path": e.path.to_string_lossy(),
                    "is_directory": e.is_directory,
                    "depth": e.depth
                })
            })
            .collect();

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": Self::text_content(
                    serde_json::to_string(&json!({
                        "root": response.root.to_string_lossy(),
                        "entries": entries,
                    }))?
                ),
                "root": response.root.to_string_lossy(),
                "entries": entries
            }
        }))
    }

    fn call_search_tool(&self, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing query")?
            .to_string();

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let request = SearchRequest { query, limit };
        let normalized = request.normalize(&self.config);
        let response = self.search.search(normalized).map_err(|e| e.message)?;

        let hits: Vec<Value> = response
            .hits
            .iter()
            .map(|h| {
                json!({
                    "path": h.path.to_string_lossy(),
                    "line_number": h.line_number,
                    "line": h.line
                })
            })
            .collect();

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": Self::text_content(
                    serde_json::to_string(&json!({
                        "query": response.query,
                        "hits": hits,
                    }))?
                ),
                "query": response.query,
                "hits": hits
            }
        }))
    }

    fn call_shell_tool(&self, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing command")?
            .to_string();

        let cwd = arguments
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let request = ShellRequest { command, cwd };
        let normalized = request.normalize(&self.config).map_err(|e| e.reason)?;
        let response = self.shell.shell(normalized).map_err(|e| e.message)?;

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": Self::text_content(
                    serde_json::to_string(&json!({
                        "command": response.command,
                        "stdout": response.stdout,
                        "stderr": response.stderr,
                        "exit_code": response.exit_code,
                    }))?
                ),
                "command": response.command,
                "stdout": response.stdout,
                "stderr": response.stderr,
                "exit_code": response.exit_code
            }
        }))
    }

    fn call_doctor_tool(&self, arguments: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let include_storage = arguments
            .get("include_storage")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let include_shell_policy = arguments
            .get("include_shell_policy")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let request = DoctorRequest {
            include_storage,
            include_shell_policy,
        };

        let response = self.doctor.doctor(request).map_err(|e| e.message)?;

        let checks: Vec<Value> = response
            .checks
            .iter()
            .map(|c| {
                json!({
                    "name": c.name,
                    "passed": c.passed,
                    "detail": c.detail
                })
            })
            .collect();

        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "content": Self::text_content(
                    serde_json::to_string(&json!({
                        "checks": checks,
                    }))?
                ),
                "checks": checks
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock service implementations for testing
    struct MockReadService;
    impl ReadService for MockReadService {
        fn read(
            &self,
            _request: crate::app::contracts::ReadRequestNormalized,
        ) -> Result<crate::app::contracts::ReadResponse, crate::app::contracts::ServiceError>
        {
            Ok(crate::app::contracts::ReadResponse {
                path: std::path::PathBuf::from("/test/file.txt"),
                content: "test content".to_string(),
                bytes_read: 12,
                truncated: false,
            })
        }
    }

    struct MockTreeService;
    impl TreeService for MockTreeService {
        fn tree(
            &self,
            request: crate::app::contracts::TreeRequestNormalized,
        ) -> Result<crate::app::contracts::TreeResponse, crate::app::contracts::ServiceError>
        {
            Ok(crate::app::contracts::TreeResponse {
                root: request.path,
                entries: vec![],
            })
        }
    }

    struct MockSearchService;
    impl SearchService for MockSearchService {
        fn search(
            &self,
            request: crate::app::contracts::SearchRequestNormalized,
        ) -> Result<crate::app::contracts::SearchResponse, crate::app::contracts::ServiceError>
        {
            Ok(crate::app::contracts::SearchResponse {
                query: request.query,
                hits: vec![],
            })
        }
    }

    struct MockShellService;
    impl ShellService for MockShellService {
        fn shell(
            &self,
            request: crate::app::contracts::ShellRequestNormalized,
        ) -> Result<crate::app::contracts::ShellResponse, crate::app::contracts::ServiceError>
        {
            Ok(crate::app::contracts::ShellResponse {
                command: request.command.rendered(),
                stdout: "shell output".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
            })
        }
    }

    struct MockDoctorService;
    impl DoctorService for MockDoctorService {
        fn doctor(
            &self,
            _request: DoctorRequest,
        ) -> Result<crate::app::contracts::DoctorResponse, crate::app::contracts::ServiceError>
        {
            Ok(crate::app::contracts::DoctorResponse {
                checks: vec![crate::app::contracts::DoctorCheck {
                    name: "test_check".to_string(),
                    passed: true,
                    detail: None,
                }],
            })
        }
    }

    fn test_config() -> AppConfig {
        let mut config = AppConfig::default();
        config.shell_enabled = true;
        config
    }

    fn create_adapter() -> McpAdapter<
        MockReadService,
        MockTreeService,
        MockSearchService,
        MockShellService,
        MockDoctorService,
    > {
        McpAdapter::new(
            test_config(),
            MockReadService,
            MockTreeService,
            MockSearchService,
            MockShellService,
            MockDoctorService,
        )
    }

    #[test]
    fn test_parse_initialize_request() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_response_has_correct_structure() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        });

        let response = adapter.handle_request(&request).unwrap();
        assert!(response.get("jsonrpc").is_some());
        assert_eq!(response.get("jsonrpc").unwrap().as_str(), Some("2.0"));
        assert!(response.get("result").is_some());

        let result = response.get("result").unwrap();
        assert!(result.get("serverInfo").is_some());
        assert!(result.get("capabilities").is_some());
    }

    #[test]
    fn test_server_info_has_name_and_version() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        });

        let response = adapter.handle_request(&request).unwrap();
        let server_info = response.get("result").unwrap().get("serverInfo").unwrap();
        assert_eq!(
            server_info.get("name").unwrap().as_str(),
            Some("ctx-lite-mcp")
        );
        assert!(server_info.get("version").is_some());
    }

    #[test]
    fn test_parse_list_tools_request() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_tools_returns_all_tools() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = adapter.handle_request(&request).unwrap();
        let tools = response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array());

        assert!(tools.is_some());
        let tools_arr = tools.unwrap();
        assert_eq!(tools_arr.len(), 5); // read, tree, search, shell, doctor

        let tool_names: Vec<&str> = tools_arr
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(tool_names.contains(&"read"));
        assert!(tool_names.contains(&"tree"));
        assert!(tool_names.contains(&"search"));
        assert!(tool_names.contains(&"shell"));
        assert!(tool_names.contains(&"doctor"));
    }

    #[test]
    fn test_tool_definitions_have_input_schemas() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = adapter.handle_request(&request).unwrap();
        let tools = response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .unwrap();

        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }

    #[test]
    fn test_parse_call_read_tool_request() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "read",
                "arguments": {
                    "path": "."
                }
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_tool_returns_content() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "read",
                "arguments": {
                    "path": "."
                }
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        assert!(response.get("result").is_some());
        let result = response.get("result").unwrap();
        assert!(result.get("content").is_some());
        assert!(result.get("truncated").is_some());
    }

    #[test]
    fn test_search_tool_returns_hits() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": "test"
                }
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        let result = response.get("result").unwrap();
        assert!(result.get("query").is_some());
        assert!(result.get("hits").is_some());
        assert!(result.get("hits").unwrap().is_array());
    }

    #[test]
    fn test_tree_tool_returns_entries() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "tree",
                "arguments": {
                    "path": "."
                }
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        let result = response.get("result").unwrap();
        assert!(result.get("root").is_some());
        assert!(result.get("entries").is_some());
        assert!(result.get("entries").unwrap().is_array());
    }

    #[test]
    fn test_shell_tool_returns_output() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "shell",
                "arguments": {
                    "command": "git status --short"
                }
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        let result = response.get("result").unwrap();
        assert!(result.get("command").is_some());
        assert!(result.get("stdout").is_some());
        assert!(result.get("exit_code").is_some());
    }

    #[test]
    fn test_doctor_tool_returns_checks() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "doctor",
                "arguments": {}
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        let result = response.get("result").unwrap();
        assert!(result.get("checks").is_some());
        assert!(result.get("checks").unwrap().is_array());
    }

    #[test]
    fn test_response_format_is_valid_jsonrpc() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/list"
        });

        let response = adapter.handle_request(&request).unwrap();
        assert_eq!(response.get("jsonrpc").unwrap().as_str(), Some("2.0"));
        assert!(response.get("result").is_some());
    }

    #[test]
    fn test_missing_method_returns_error() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 9
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_tool_name_returns_error() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "arguments": {}
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_method_returns_error() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "unknown/method"
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_tool_returns_error() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool",
                "arguments": {}
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_parameters() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "tools/call",
            "params": {
                "name": "read",
                "arguments": {}
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_tool_with_limit_parameter() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": "test",
                    "limit": 50
                }
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tree_tool_with_max_depth_parameter() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "tools/call",
            "params": {
                "name": "tree",
                "arguments": {
                    "path": ".",
                    "max_depth": 5,
                    "include_hidden": true
                }
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_tool_with_cwd_parameter() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "tools/call",
            "params": {
                "name": "shell",
                "arguments": {
                    "command": "git status --short",
                    "cwd": "."
                }
            }
        });

        let result = adapter.handle_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_response_echoes_request_id() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "initialize"
        });

        let response = adapter.handle_request(&request).unwrap();
        assert_eq!(response.get("id"), Some(&json!(42)));
    }

    #[test]
    fn test_initialize_response_uses_client_protocol_version() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25"
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        assert_eq!(
            response
                .get("result")
                .and_then(|result| result.get("protocolVersion")),
            Some(&json!("2025-11-25"))
        );
    }

    #[test]
    fn test_read_tool_returns_mcp_content_blocks() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "tools/call",
            "params": {
                "name": "read",
                "arguments": {
                    "path": "."
                }
            }
        });

        let response = adapter.handle_request(&request).unwrap();
        let blocks = response
            .get("result")
            .and_then(|result| result.get("content"))
            .and_then(|content| content.as_array())
            .expect("tool result should expose MCP content blocks");

        assert_eq!(blocks[0].get("type"), Some(&json!("text")));
        assert!(blocks[0]
            .get("text")
            .and_then(|text| text.as_str())
            .unwrap_or_default()
            .contains("test content"));
    }

    #[test]
    fn test_instructions_omit_dangerous_commands_in_safe_profile() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 44,
            "method": "initialize",
            "params": {}
        });

        let response = adapter.handle_request(&request).unwrap();
        let instructions = response
            .get("result")
            .and_then(|r| r.get("instructions"))
            .and_then(|i| i.as_str())
            .expect("instructions should be present");

        assert!(!instructions.contains("docker run"), "safe profile should not mention docker run");
        assert!(!instructions.contains("npm install"), "safe profile should not mention npm install");
        assert!(!instructions.contains("cargo run"), "safe profile should not mention cargo run");
        assert!(instructions.contains("git"), "safe profile should mention git");
    }

    #[test]
    fn test_shell_tool_description_reflects_active_profile() {
        let adapter = create_adapter();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 45,
            "method": "tools/list"
        });

        let response = adapter.handle_request(&request).unwrap();
        let shell_tool = response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .and_then(|tools| tools.iter().find(|t| t.get("name") == Some(&json!("shell"))))
            .expect("shell tool should be present");

        let description = shell_tool
            .get("description")
            .and_then(|d| d.as_str())
            .expect("shell tool description should be present");

        assert!(description.contains("safe"), "shell description should mention the active profile");
    }

    #[test]
    fn test_instructions_mention_docker_logs_when_docker_logs_enabled() {
        use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
        let config = AppConfig {
            shell_enabled: true,
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Safe,
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            ..AppConfig::default()
        };
        let adapter = McpAdapter::new(
            config,
            MockReadService,
            MockTreeService,
            MockSearchService,
            MockShellService,
            MockDoctorService,
        );
        let request = json!({ "jsonrpc": "2.0", "id": 46, "method": "initialize", "params": {} });
        let response = adapter.handle_request(&request).unwrap();
        let instructions = response
            .get("result")
            .and_then(|r| r.get("instructions"))
            .and_then(|i| i.as_str())
            .unwrap();
        assert!(instructions.contains("docker logs"), "safe profile includes docker.logs capability");
    }
}
