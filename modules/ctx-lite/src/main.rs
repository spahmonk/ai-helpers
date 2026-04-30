use ctx_lite::app::cli::CliAdapter;
use ctx_lite::app::contracts::{
    DoctorRequest, DoctorResponse, DoctorService, ReadRequestNormalized, ReadResponse, ReadService,
    SearchRequestNormalized, SearchResponse, SearchService, ServiceError, ShellRequestNormalized,
    ShellResponse, ShellService, TreeRequestNormalized, TreeResponse, TreeService,
};
use ctx_lite::core::config::AppConfig;
use ctx_lite::core::doctor::{CheckSeverity, DoctorService as DoctorServiceImpl};
use ctx_lite::core::fs::{FileReader, TreeBuilder};
use ctx_lite::core::search::SearchService as SearchServiceImpl;
use ctx_lite::core::security::path_jail::PathJail;
use ctx_lite::core::shell::ShellExecutor;

// Wrapper types to implement the service traits with the core services

#[derive(Clone)]
struct ReadServiceAdapter {
    file_reader: FileReader,
}

impl ReadService for ReadServiceAdapter {
    fn read(&self, request: ReadRequestNormalized) -> Result<ReadResponse, ServiceError> {
        self.file_reader.read(request)
    }
}

#[derive(Clone)]
struct TreeServiceAdapter {
    tree_builder: TreeBuilder,
}

impl TreeService for TreeServiceAdapter {
    fn tree(&self, request: TreeRequestNormalized) -> Result<TreeResponse, ServiceError> {
        self.tree_builder.tree(request)
    }
}

#[derive(Clone)]
struct SearchServiceAdapter {
    search_service: SearchServiceImpl,
}

impl SearchService for SearchServiceAdapter {
    fn search(&self, request: SearchRequestNormalized) -> Result<SearchResponse, ServiceError> {
        self.search_service.search(request)
    }
}

#[derive(Clone)]
struct ShellServiceAdapter {
    executor: ShellExecutor,
}

impl ShellService for ShellServiceAdapter {
    fn shell(&self, request: ShellRequestNormalized) -> Result<ShellResponse, ServiceError> {
        let output = self
            .executor
            .execute(&request.command, &request.cwd)
            .map_err(|e| ServiceError::internal(&e.reason))?;
        Ok(ShellResponse {
            command: request.command.rendered(),
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
        })
    }
}

#[derive(Clone)]
struct DoctorServiceAdapter {
    config: AppConfig,
}

impl DoctorService for DoctorServiceAdapter {
    fn doctor(&self, _request: DoctorRequest) -> Result<DoctorResponse, ServiceError> {
        let report = DoctorServiceImpl::run(&self.config);
        Ok(DoctorResponse {
            checks: report
                .checks
                .into_iter()
                .map(|check| {
                    let passed = check.severity == CheckSeverity::Ok;
                    ctx_lite::app::contracts::DoctorCheck {
                        name: check.name,
                        passed,
                        detail: Some(check.message),
                    }
                })
                .collect(),
        })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Check for --mcp flag first (should be run in MCP server mode)
    if !args.is_empty() && args[0] == "--mcp" {
        handle_mcp_mode();
    }

    let mut config = AppConfig::default();
    config.shell_enabled = true;

    let jail = PathJail::from_config(&config).unwrap_or_else(|e| {
        eprintln!("Error: {}", e.message);
        std::process::exit(1);
    });

    let read_adapter = ReadServiceAdapter {
        file_reader: FileReader::new(jail.clone()),
    };

    let tree_adapter = TreeServiceAdapter {
        tree_builder: TreeBuilder::new(jail.clone()),
    };

    let search_adapter = SearchServiceAdapter {
        search_service: SearchServiceImpl::new(jail.clone()),
    };

    let shell_adapter = ShellServiceAdapter {
        executor: ShellExecutor::new(config.max_shell_output_bytes, jail.clone()),
    };

    let doctor_adapter = DoctorServiceAdapter {
        config: config.clone(),
    };

    let cli = CliAdapter::new(
        config,
        read_adapter,
        tree_adapter,
        search_adapter,
        shell_adapter,
        doctor_adapter,
    );

    let result = cli.run(args);
    print!("{}", result.output);
    std::process::exit(result.exit_code);
}

/// Handle MCP server mode: reads JSON-RPC requests from stdin
fn handle_mcp_mode() -> ! {
    use ctx_lite::app::mcp::McpAdapter;

    let mut config = AppConfig::default();
    config.shell_enabled = true;

    let jail = PathJail::from_config(&config).unwrap_or_else(|e| {
        eprintln!("Error: {}", e.message);
        std::process::exit(1);
    });

    let read_adapter = ReadServiceAdapter {
        file_reader: FileReader::new(jail.clone()),
    };

    let tree_adapter = TreeServiceAdapter {
        tree_builder: TreeBuilder::new(jail.clone()),
    };

    let search_adapter = SearchServiceAdapter {
        search_service: SearchServiceImpl::new(jail.clone()),
    };

    let shell_adapter = ShellServiceAdapter {
        executor: ShellExecutor::new(config.max_shell_output_bytes, jail.clone()),
    };

    let doctor_adapter = DoctorServiceAdapter {
        config: config.clone(),
    };

    let mut mcp = McpAdapter::new(
        config,
        read_adapter,
        tree_adapter,
        search_adapter,
        shell_adapter,
        doctor_adapter,
    );

    match mcp.run() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("MCP Error: {}", e);
            std::process::exit(1);
        }
    }
}
