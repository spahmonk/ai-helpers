use ctx_lite::app::cli::{parse_leading_process_args, CliAdapter};
use ctx_lite::app::contracts::{
    DoctorRequest, DoctorResponse, DoctorService, ReadRequestNormalized, ReadResponse, ReadService,
    SearchRequestNormalized, SearchResponse, SearchService, ServiceError, ShellRequestNormalized,
    ShellResponse, ShellService, TreeRequestNormalized, TreeResponse, TreeService,
};
use ctx_lite::core::capabilities::{resolve_shell_policy, ShellPolicyInputs};
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
            overall_severity: match report.overall_severity {
                CheckSeverity::Ok => "ok".to_string(),
                CheckSeverity::Warning => "warning".to_string(),
                CheckSeverity::Error => "error".to_string(),
            },
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
    let startup_args = parse_startup_args(&args).unwrap_or_else(|error| {
        eprintln!("Error: {error}");
        std::process::exit(1);
    });

    let mut config = AppConfig::default();
    config.shell_enabled = true;
    config.shell_policy = startup_args.shell_policy;

    if startup_args.run_mcp {
        handle_mcp_mode(config);
    }

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

    let result = cli.run(startup_args.cli_args);
    print!("{}", result.output);
    std::process::exit(result.exit_code);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartupArgs {
    run_mcp: bool,
    shell_policy: ShellPolicyInputs,
    cli_args: Vec<String>,
}

fn parse_startup_args(args: &[String]) -> Result<StartupArgs, String> {
    let parsed = parse_leading_process_args(args)?;
    resolve_shell_policy(&parsed.shell_policy).map_err(|error| error.reason)?;
    Ok(StartupArgs {
        run_mcp: parsed.run_mcp,
        shell_policy: parsed.shell_policy,
        cli_args: parsed.passthrough_args,
    })
}

/// Handle MCP server mode: reads JSON-RPC requests from stdin
fn handle_mcp_mode(config: AppConfig) -> ! {
    use ctx_lite::app::mcp::McpAdapter;

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

#[cfg(test)]
mod tests {
    use super::parse_startup_args;
    use ctx_lite::core::capabilities::ShellCapabilityProfile;

    #[test]
    fn startup_arg_parser_supports_mcp_beyond_first_position() {
        let parsed = parse_startup_args(&[
            "--shell-profile".to_string(),
            "balanced".to_string(),
            "--mcp".to_string(),
            "--allow-capability".to_string(),
            "npm.build,cargo.build".to_string(),
        ])
        .expect("startup args should parse");

        assert!(parsed.run_mcp);
        assert!(parsed.shell_policy.explicit_policy);
        assert_eq!(
            parsed.shell_policy.profile,
            ShellCapabilityProfile::Balanced
        );
        assert_eq!(
            parsed.shell_policy.allow_capabilities,
            vec!["npm.build".to_string(), "cargo.build".to_string()]
        );
        assert!(parsed.cli_args.is_empty());
    }

    #[test]
    fn startup_arg_parser_rejects_invalid_capability_before_runtime() {
        let error = parse_startup_args(&[
            "--mcp".to_string(),
            "--allow-capability".to_string(),
            "not.real".to_string(),
        ])
        .expect_err("invalid capability should fail during startup parsing");

        assert_eq!(
            error,
            "unknown shell capability id `not.real` in `allow_capabilities`"
        );
    }
}
