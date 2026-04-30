use serde_json::{json, Value};
use std::io::{self, Read, Write};

use crate::app::contracts::{
    DoctorRequest, DoctorService, ReadRequest, ReadService, SearchRequest, SearchService,
    ShellRequest, ShellService, TreeRequest, TreeService,
};
use crate::core::config::AppConfig;

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
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;

        if buffer.trim().is_empty() {
            return Ok(());
        }

        let request: Value = serde_json::from_str(&buffer)?;
        let response = self.handle_request(&request)?;
        let output = serde_json::to_string(&response)?;
        io::stdout().write_all(output.as_bytes())?;
        io::stdout().flush()?;

        Ok(())
    }

    fn handle_request(&self, request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or("Missing method")?;

        match method {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_call_tool(request),
            _ => Err("Unknown method".into()),
        }
    }

    fn handle_initialize(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ctx-lite-mcp",
                    "version": "0.1.0"
                }
            }
        }))
    }

    fn handle_list_tools(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "jsonrpc": "2.0",
            "result": {
                "tools": [
                    {
                        "name": "read",
                        "description": "Read file contents",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to file"
                                },
                                "max_bytes": {
                                    "type": "integer",
                                    "description": "Maximum bytes to read (optional)"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "tree",
                        "description": "List directory tree",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to directory"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum depth (optional)"
                                },
                                "include_hidden": {
                                    "type": "boolean",
                                    "description": "Include hidden files (optional)"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "search",
                        "description": "Search files",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Result limit (optional)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "shell",
                        "description": "Execute shell command",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": {
                                    "type": "string",
                                    "description": "Command to execute"
                                },
                                "cwd": {
                                    "type": "string",
                                    "description": "Working directory (optional)"
                                }
                            },
                            "required": ["command"]
                        }
                    },
                    {
                        "name": "doctor",
                        "description": "Run diagnostic checks",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "include_storage": {
                                    "type": "boolean",
                                    "description": "Include storage checks (optional)"
                                },
                                "include_shell_policy": {
                                    "type": "boolean",
                                    "description": "Include shell policy checks (optional)"
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
                "content": response.content,
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
}
