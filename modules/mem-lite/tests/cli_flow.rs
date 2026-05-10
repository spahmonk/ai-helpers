use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use mem_lite::app::{CliAdapter, MemoryServiceAdapter};

static WORKSPACE_SEQ: AtomicUsize = AtomicUsize::new(1);

fn test_home() -> PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("mem-lite-test-home");
        fs::create_dir_all(&path).unwrap();
        std::env::set_var("MEM_LITE_HOME", &path);
        path
    })
    .clone()
}

fn workspace_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("mem-lite-test-workspaces")
        .join(format!(
            "{}-{}",
            std::process::id(),
            WORKSPACE_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn run_cli(args: &[&str]) -> mem_lite::app::CliResult {
    let _ = test_home();
    let services = MemoryServiceAdapter::default();
    let cli = CliAdapter::new(services);
    cli.run(args.iter().map(|arg| arg.to_string()).collect())
}

#[test]
fn cli_init_creates_project() {
    let root = workspace_root();
    let result = run_cli(&["init", "--root", root.to_str().unwrap()]);
    assert_eq!(result.exit_code, 0);
    assert!(result.output.contains("Initialized project"));
}

#[test]
fn cli_remember_stores_entry() {
    let root = workspace_root();
    let remember = run_cli(&[
        "remember",
        "remembered entry",
        "--title",
        "cli test",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(remember.exit_code, 0);

    let recent = run_cli(&["recent", "--root", root.to_str().unwrap()]);
    assert_eq!(recent.exit_code, 0);
    assert!(recent.output.contains("remembered entry"));
}

#[test]
fn cli_search_returns_hits() {
    let root = workspace_root();
    let _ = run_cli(&[
        "remember",
        "searchable memory",
        "--title",
        "search title",
        "--root",
        root.to_str().unwrap(),
    ]);

    let search = run_cli(&["search", "searchable", "--root", root.to_str().unwrap()]);
    assert_eq!(search.exit_code, 0);
    assert!(search.output.contains("searchable memory"));
}

#[test]
fn cli_stats_shows_counts() {
    let root = workspace_root();
    let _ = run_cli(&[
        "remember",
        "stats entry",
        "--title",
        "stats title",
        "--root",
        root.to_str().unwrap(),
    ]);

    let stats = run_cli(&["stats", "--root", root.to_str().unwrap()]);
    assert_eq!(stats.exit_code, 0);
    assert!(stats.output.contains("semantic: 1"));
}

#[test]
fn cli_project_info_shows_project_id() {
    let root = workspace_root();
    let result = run_cli(&["project-info", "--root", root.to_str().unwrap()]);
    assert_eq!(result.exit_code, 0);

    let project_id = result
        .output
        .lines()
        .find_map(|line| line.strip_prefix("project_id: "))
        .unwrap();
    assert!(!project_id.trim().is_empty());
}

#[test]
fn cli_remember_uses_root_flag() {
    let root = workspace_root();
    let remember = run_cli(&[
        "remember",
        "root scoped memory",
        "--title",
        "root test",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(remember.exit_code, 0);

    let info = run_cli(&["project-info", "--root", root.to_str().unwrap()]);
    assert_eq!(info.exit_code, 0);
    assert!(info.output.contains("project_id: "));
}

#[test]
fn cli_error_on_invalid_level() {
    let root = workspace_root();
    let result = run_cli(&[
        "remember",
        "invalid level memory",
        "--level",
        "bad",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(result.exit_code, 1);
    assert!(result.output.contains("invalid level"));
}
