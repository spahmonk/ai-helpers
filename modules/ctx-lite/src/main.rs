fn main() {
    let help_requested = std::env::args()
        .skip(1)
        .any(|arg| arg == "-h" || arg == "--help");

    if help_requested {
        print!("{}", help_text());
        std::process::exit(0);
    }
}

fn help_text() -> String {
    format!(
        "ctx-lite {}\n\nUsage: ctx-lite [--help]\n\nOptions:\n  -h, --help    Print help\n",
        ctx_lite::version()
    )
}
