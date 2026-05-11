use mem_lite::app::{CliAdapter, McpAdapter, MemoryServiceAdapter};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("mem-lite {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        _ => {}
    }

    let run_mcp = matches!(args.first().map(String::as_str), Some("--mcp"));
    let cli_args = if run_mcp {
        args.into_iter().skip(1).collect()
    } else {
        args
    };

    if run_mcp {
        let services = MemoryServiceAdapter::default();
        let mut mcp = McpAdapter::new(services);
        if let Err(error) = mcp.run() {
            eprintln!("MCP Error: {error}");
            std::process::exit(1);
        }
        return;
    }

    let services = MemoryServiceAdapter::default();
    let cli = CliAdapter::new(services);
    let result = cli.run(cli_args);

    if result.exit_code == 0 {
        print!("{}", result.output);
    } else {
        eprint!("{}", result.output);
    }

    std::process::exit(result.exit_code);
}
