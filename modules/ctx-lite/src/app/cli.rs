/// CLI adapter: parses command-line arguments and dispatches to services
use crate::app::contracts::{
    DoctorRequest, DoctorService, ReadRequest, ReadService, SearchRequest, SearchService,
    ShellRequest, ShellService, TreeRequest, TreeService,
};
use crate::core::config::AppConfig;

/// CLI result type with exit code
pub struct CliResult {
    pub output: String,
    pub exit_code: i32,
}

/// CLI adapter for command-line argument parsing and dispatch
pub struct CliAdapter<Read, Tree, Search, Shell, Doctor>
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

impl<Read, Tree, Search, Shell, Doctor> CliAdapter<Read, Tree, Search, Shell, Doctor>
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

    /// Parse arguments and dispatch to appropriate service
    pub fn run(&self, args: Vec<String>) -> CliResult {
        if args.is_empty() {
            return CliResult {
                output: help_text(),
                exit_code: 1,
            };
        }

        match args[0].as_str() {
            "read" => self.handle_read(&args[1..]),
            "tree" => self.handle_tree(&args[1..]),
            "search" => self.handle_search(&args[1..]),
            "shell" => self.handle_shell(&args[1..]),
            "doctor" => self.handle_doctor(&args[1..]),
            "--help" | "-h" => CliResult {
                output: help_text(),
                exit_code: 0,
            },
            "--version" | "-v" => CliResult {
                output: format!("ctx-lite {}\n", crate::version()),
                exit_code: 0,
            },
            _ => CliResult {
                output: format!("Error: unknown command '{}'\n\n{}", args[0], help_text()),
                exit_code: 1,
            },
        }
    }

    fn handle_read(&self, args: &[String]) -> CliResult {
        if args.is_empty() {
            return CliResult {
                output: "Error: read requires a path argument\n".to_string(),
                exit_code: 1,
            };
        }

        // Parse flags
        let mut show_budget = true;
        let mut mode = None;
        let mut path = String::new();

        // Look for --no-budget and --mode flags
        for (i, arg) in args.iter().enumerate() {
            match arg.as_str() {
                "--no-budget" => show_budget = false,
                "--mode" => {
                    if i + 1 < args.len() {
                        let mode_str = &args[i + 1];
                        mode = match mode_str.as_str() {
                            "full" => Some(crate::app::contracts::ReadMode::Full),
                            "signatures" => Some(crate::app::contracts::ReadMode::Signatures),
                            "map" => Some(crate::app::contracts::ReadMode::Map),
                            "diff" => Some(crate::app::contracts::ReadMode::Diff),
                            _ => {
                                return CliResult {
                                    output: format!("Error: invalid mode '{}'. Valid options: full, signatures, map, diff\n", mode_str),
                                    exit_code: 1,
                                };
                            }
                        };
                    }
                }
                _ => {
                    // Treat non-flag arguments as path (first one encountered)
                    if !arg.starts_with("--") && path.is_empty() {
                        path = arg.clone();
                    }
                }
            }
        }

        if path.is_empty() {
            return CliResult {
                output: "Error: read requires a path argument\n".to_string(),
                exit_code: 1,
            };
        }

        let request = ReadRequest {
            path,
            max_bytes: None,
            mode,
        };

        match request.normalize(&self.config) {
            Ok(normalized) => match self.read.read(normalized) {
                Ok(response) => CliResult {
                    output: format_read_response(&response, show_budget),
                    exit_code: 0,
                },
                Err(err) => CliResult {
                    output: format!("Error: {}\n", err.message),
                    exit_code: 2,
                },
            },
            Err(err) => CliResult {
                output: format!("Error: {}\n", err.reason),
                exit_code: 1,
            },
        }
    }

    fn handle_tree(&self, args: &[String]) -> CliResult {
        let path = if args.is_empty() {
            None
        } else {
            Some(args[0].clone())
        };

        let request = TreeRequest {
            path: path.unwrap_or_default(),
            max_depth: None,
            include_hidden: false,
        };

        match request.normalize(&self.config) {
            Ok(normalized) => match self.tree.tree(normalized) {
                Ok(response) => CliResult {
                    output: format_tree_response(&response),
                    exit_code: 0,
                },
                Err(err) => CliResult {
                    output: format!("Error: {}\n", err.message),
                    exit_code: 2,
                },
            },
            Err(err) => CliResult {
                output: format!("Error: {}\n", err.reason),
                exit_code: 1,
            },
        }
    }

    fn handle_search(&self, args: &[String]) -> CliResult {
        if args.is_empty() {
            return CliResult {
                output: "Error: search requires a query argument\n".to_string(),
                exit_code: 1,
            };
        }

        let query_start = if args[0] == "--mode" {
            if args.len() < 3 {
                return CliResult {
                    output: "Error: search with --mode requires a mode and query argument\n"
                        .to_string(),
                    exit_code: 1,
                };
            }
            2
        } else {
            0
        };

        let request = SearchRequest {
            query: args[query_start].clone(),
            limit: None,
        };

        let normalized = request.normalize(&self.config);
        match self.search.search(normalized) {
            Ok(response) => CliResult {
                output: format_search_response(&response),
                exit_code: 0,
            },
            Err(err) => CliResult {
                output: format!("Error: {}\n", err.message),
                exit_code: 2,
            },
        }
    }

    fn handle_shell(&self, args: &[String]) -> CliResult {
        if args.len() < 2 {
            return CliResult {
                output: "Error: shell requires cwd and command arguments\n".to_string(),
                exit_code: 1,
            };
        }

        let cwd = Some(args[0].clone());
        let command = args[1..].join(" ");

        let request = ShellRequest { command, cwd };

        match request.normalize(&self.config) {
            Ok(normalized) => match self.shell.shell(normalized) {
                Ok(response) => CliResult {
                    output: format_shell_response(&response),
                    exit_code: 0,
                },
                Err(err) => CliResult {
                    output: format!("Error: {}\n", err.message),
                    exit_code: 2,
                },
            },
            Err(err) => CliResult {
                output: format!("Error: {}\n", err.reason),
                exit_code: 1,
            },
        }
    }

    fn handle_doctor(&self, args: &[String]) -> CliResult {
        let _include_storage = args.iter().any(|a| a == "--storage");
        let _include_shell_policy = args.iter().any(|a| a == "--shell-policy");

        let request = DoctorRequest {
            include_storage: _include_storage,
            include_shell_policy: _include_shell_policy,
        };

        match self.doctor.doctor(request) {
            Ok(response) => CliResult {
                output: format_doctor_response(&response),
                exit_code: 0,
            },
            Err(err) => CliResult {
                output: format!("Error: {}\n", err.message),
                exit_code: 2,
            },
        }
    }
}

fn help_text() -> String {
    format!(
        "ctx-lite {}\n\nUsage: ctx-lite <COMMAND> [OPTIONS] [ARGS]\n\nCommands:\n  read <path>              Read file at path\n  tree [path]              List directory tree\n  search <query>           Search for text/regex\n  shell <cwd> <command>    Execute whitelisted command\n  doctor                   Run diagnostics\n  --help, -h               Show this help message\n  --version, -v            Show version\n\nOptions for read command:\n  --mode <mode>            Compression mode: full, signatures, map, diff\n  --no-budget              Suppress budget information in output\n",
        crate::version()
    )
}

fn format_read_response(response: &crate::app::contracts::ReadResponse, show_budget: bool) -> String {
    use crate::core::budget::BudgetStatus;

    let mode_str = format!("{:?}", response.mode);
    
    let mode_indicator = if response.is_auto_selected {
        format!("{} (auto-selected)", mode_str)
    } else {
        mode_str
    };

    let compression_str = if response.compression_percent > 0 {
        format!(" [{}% compression]", response.compression_percent)
    } else {
        String::new()
    };

    let mut output = format!(
        "File: {}\nMode: {}{}\n",
        response.path.display(),
        mode_indicator,
        compression_str
    );

    if show_budget {
        let percentage = if response.max_tokens > 0 {
            (response.tokens_consumed * 100) / response.max_tokens
        } else {
            0
        };

        let status_str = match response.budget_status {
            BudgetStatus::Ok => "Ok".to_string(),
            BudgetStatus::WarningThreshold => "Warning".to_string(),
            BudgetStatus::Exceeded => "Exceeded".to_string(),
        };

        output.push_str(&format!(
            "Budget: {}/{} tokens ({}%, status: {})\n",
            response.tokens_consumed, response.max_tokens, percentage, status_str
        ));
    }

    output.push('\n');
    output.push_str(&response.content);
    output.push('\n');

    output
}

fn format_tree_response(response: &crate::app::contracts::TreeResponse) -> String {
    let mut output = format!("{}\n", response.root.display());
    for entry in &response.entries {
        let prefix = "  ".repeat(entry.depth);
        let marker = if entry.is_directory { "/" } else { "" };
        output.push_str(&format!(
            "{}{}{}\n",
            prefix,
            entry.path.file_name().unwrap_or_default().to_string_lossy(),
            marker
        ));
    }
    output
}

fn format_search_response(response: &crate::app::contracts::SearchResponse) -> String {
    let mut output = format!("Search results for: {}\n", response.query);
    for hit in &response.hits {
        output.push_str(&format!(
            "{}:{}: {}\n",
            hit.path.display(),
            hit.line_number,
            hit.line
        ));
    }
    output
}

fn format_shell_response(response: &crate::app::contracts::ShellResponse) -> String {
    format!(
        "$ {}\n{}{}",
        response.command, response.stdout, response.stderr
    )
}

fn format_doctor_response(response: &crate::app::contracts::DoctorResponse) -> String {
    let mut output = "Diagnostics:\n".to_string();
    for check in &response.checks {
        let status = if check.passed { "✓" } else { "✗" };
        output.push_str(&format!("  {} {}", status, check.name));
        if let Some(detail) = &check.detail {
            output.push_str(&format!(": {}", detail));
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::contracts::{
        DoctorCheck, DoctorResponse, ReadMode, ReadResponse, SearchHit, SearchResponse,
        ServiceError, ShellResponse, TreeEntry, TreeResponse,
    };
    use crate::core::budget::BudgetStatus;
    use std::path::PathBuf;

    // Mock services for testing
    struct MockReadService;
    impl ReadService for MockReadService {
        fn read(
            &self,
            _request: crate::app::contracts::ReadRequestNormalized,
        ) -> Result<ReadResponse, ServiceError> {
            Ok(ReadResponse {
                path: PathBuf::from("test.txt"),
                content: "test content".to_string(),
                bytes_read: 12,
                truncated: false,
                mode: ReadMode::Full,
                compression_percent: 0,
                is_auto_selected: false,
                tokens_consumed: 100,
                max_tokens: 1000,
                budget_status: BudgetStatus::Ok,
            })
        }
    }

    struct MockTreeService;
    impl TreeService for MockTreeService {
        fn tree(
            &self,
            request: crate::app::contracts::TreeRequestNormalized,
        ) -> Result<TreeResponse, ServiceError> {
            Ok(TreeResponse {
                root: request.path,
                entries: vec![
                    TreeEntry {
                        path: PathBuf::from("file.txt"),
                        is_directory: false,
                        depth: 1,
                    },
                    TreeEntry {
                        path: PathBuf::from("subdir"),
                        is_directory: true,
                        depth: 1,
                    },
                ],
            })
        }
    }

    struct MockSearchService;
    impl SearchService for MockSearchService {
        fn search(
            &self,
            request: crate::app::contracts::SearchRequestNormalized,
        ) -> Result<SearchResponse, ServiceError> {
            Ok(SearchResponse {
                query: request.query,
                hits: vec![SearchHit {
                    path: PathBuf::from("test.txt"),
                    line_number: 1,
                    line: "matching line".to_string(),
                }],
            })
        }
    }

    struct MockShellService;
    impl ShellService for MockShellService {
        fn shell(
            &self,
            request: crate::app::contracts::ShellRequestNormalized,
        ) -> Result<ShellResponse, ServiceError> {
            Ok(ShellResponse {
                command: request.command.rendered(),
                stdout: "output".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
            })
        }
    }

    struct MockDoctorService;
    impl DoctorService for MockDoctorService {
        fn doctor(&self, _request: DoctorRequest) -> Result<DoctorResponse, ServiceError> {
            Ok(DoctorResponse {
                checks: vec![DoctorCheck {
                    name: "test_check".to_string(),
                    passed: true,
                    detail: None,
                }],
            })
        }
    }

    fn create_test_adapter() -> CliAdapter<
        MockReadService,
        MockTreeService,
        MockSearchService,
        MockShellService,
        MockDoctorService,
    > {
        let mut config = AppConfig::default();
        config.shell_enabled = true;
        CliAdapter::new(
            config,
            MockReadService,
            MockTreeService,
            MockSearchService,
            MockShellService,
            MockDoctorService,
        )
    }

    #[test]
    fn test_parse_read_command_with_path() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["read".to_string(), "test.txt".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("test content"));
    }

    #[test]
    fn test_parse_tree_command_with_path() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["tree".to_string(), ".".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("file.txt"));
    }

    #[test]
    fn test_parse_search_command_with_query() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["search".to_string(), "test".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("matching line"));
    }

    #[test]
    fn test_parse_search_with_mode_literal() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec![
            "search".to_string(),
            "--mode".to_string(),
            "literal".to_string(),
            "test".to_string(),
        ]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("matching line"));
    }

    #[test]
    fn test_parse_shell_command_with_cwd_and_args() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec![
            "shell".to_string(),
            ".".to_string(),
            "git".to_string(),
            "status".to_string(),
            "--short".to_string(),
        ]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("output"));
    }

    #[test]
    fn test_parse_doctor_command() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["doctor".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("Diagnostics"));
    }

    #[test]
    fn test_invalid_command_returns_usage_error() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["invalid".to_string()]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("unknown command"));
    }

    #[test]
    fn test_help_flag_shows_help_text() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["--help".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("Usage"));
        assert!(result.output.contains("read"));
        assert!(result.output.contains("tree"));
        assert!(result.output.contains("search"));
        assert!(result.output.contains("shell"));
        assert!(result.output.contains("doctor"));
    }

    #[test]
    fn test_version_flag_shows_version() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["--version".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("ctx-lite"));
        assert!(result.output.contains("0.1.0"));
    }

    #[test]
    fn test_read_without_path_returns_error() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["read".to_string()]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn test_search_without_query_returns_error() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["search".to_string()]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn test_shell_without_cwd_returns_error() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["shell".to_string(), "ls".to_string()]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("Error"));
    }

    #[test]
    fn test_empty_args_shows_help_with_error_code() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec![]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("Usage"));
    }

    #[test]
    fn test_h_flag_shows_help_text() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["-h".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("Usage"));
    }

    #[test]
    fn test_v_flag_shows_version() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["-v".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("0.1.0"));
    }

    #[test]
    fn test_read_shows_budget_info_by_default() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["read".to_string(), "test.txt".to_string()]);
        assert_eq!(result.exit_code, 0);
        // Budget info should be displayed by default
        assert!(result.output.contains("Budget:"));
        assert!(result.output.contains("tokens"));
        assert!(result.output.contains("status:"));
        assert!(result.output.contains("Ok"));
    }

    #[test]
    fn test_read_hides_budget_with_no_budget_flag() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["read".to_string(), "--no-budget".to_string(), "test.txt".to_string()]);
        assert_eq!(result.exit_code, 0);
        // Budget info should be hidden with --no-budget flag
        assert!(!result.output.contains("Budget:"));
        // But content should still be there
        assert!(result.output.contains("test content"));
    }

    #[test]
    fn test_read_with_mode_shows_auto_selected_indicator() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["read".to_string(), "--mode".to_string(), "signatures".to_string(), "test.txt".to_string()]);
        assert_eq!(result.exit_code, 0);
        // Mode is not auto-selected when explicitly provided
        assert!(!result.output.contains("auto-selected"));
    }

    // Tests for service error handling
    struct MockReadServiceError;
    impl ReadService for MockReadServiceError {
        fn read(
            &self,
            _request: crate::app::contracts::ReadRequestNormalized,
        ) -> Result<ReadResponse, ServiceError> {
            Err(ServiceError::internal("file not found"))
        }
    }

    struct MockSearchServiceError;
    impl SearchService for MockSearchServiceError {
        fn search(
            &self,
            _request: crate::app::contracts::SearchRequestNormalized,
        ) -> Result<SearchResponse, ServiceError> {
            Err(ServiceError::internal("search failed"))
        }
    }

    #[test]
    fn test_read_service_error_returns_exit_code_2() {
        let config = AppConfig::default();
        let adapter = CliAdapter::new(
            config,
            MockReadServiceError,
            MockTreeService,
            MockSearchService,
            MockShellService,
            MockDoctorService,
        );
        let result = adapter.run(vec!["read".to_string(), "test.txt".to_string()]);
        assert_eq!(result.exit_code, 2);
        assert!(result.output.contains("file not found"));
    }

    #[test]
    fn test_search_service_error_returns_exit_code_2() {
        let config = AppConfig::default();
        let adapter = CliAdapter::new(
            config,
            MockReadService,
            MockTreeService,
            MockSearchServiceError,
            MockShellService,
            MockDoctorService,
        );
        let result = adapter.run(vec!["search".to_string(), "query".to_string()]);
        assert_eq!(result.exit_code, 2);
        assert!(result.output.contains("search failed"));
    }

    #[test]
    fn test_tree_without_path_uses_default() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec!["tree".to_string()]);
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("file.txt"));
    }

    #[test]
    fn test_search_with_mode_requires_query() {
        let adapter = create_test_adapter();
        let result = adapter.run(vec![
            "search".to_string(),
            "--mode".to_string(),
            "literal".to_string(),
        ]);
        assert_eq!(result.exit_code, 1);
        assert!(result.output.contains("Error"));
    }
}
