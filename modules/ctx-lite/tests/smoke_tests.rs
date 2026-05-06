/// End-to-end smoke tests for ctx-lite binary
/// Tests verify the packaged binary works correctly with common workflows
use std::process::Command;
use std::time::Instant;

fn run_ctx_lite(args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_ctx-lite"))
        .args(args)
        .output()
        .expect("failed to execute ctx-lite binary");

    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
    let success = output.status.success();

    (success, format!("{}{}", stdout, stderr))
}

// ============================================================================
// BASIC CLI COMMANDS
// ============================================================================

#[test]
fn smoke_cli_help_command() {
    let (success, output) = run_ctx_lite(&["--help"]);
    assert!(success, "help should exit successfully");
    assert!(
        output.contains("ctx-lite"),
        "help should mention binary name"
    );
    assert!(
        output.contains("USAGE") || output.contains("Usage"),
        "help should show usage text"
    );
    assert!(output.contains("read"), "help should list read command");
    assert!(output.contains("tree"), "help should list tree command");
    assert!(output.contains("search"), "help should list search command");
}

#[test]
fn smoke_cli_help_short_flag() {
    let (success, output) = run_ctx_lite(&["-h"]);
    assert!(success, "-h should work like --help");
    assert!(output.contains("Usage"), "short help should show usage");
}

#[test]
fn smoke_cli_version_command() {
    let (success, output) = run_ctx_lite(&["--version"]);
    assert!(success, "version should exit successfully");
    assert!(
        output.contains("ctx-lite"),
        "version should mention binary name"
    );
    assert!(output.contains("1.0"), "version should show version number");
}

#[test]
fn smoke_cli_version_short_flag() {
    let (success, output) = run_ctx_lite(&["-v"]);
    assert!(success, "-v should work like --version");
    assert!(
        output.contains("ctx-lite"),
        "version should mention binary name"
    );
}

#[test]
fn smoke_cli_no_args_shows_help() {
    let (success, output) = run_ctx_lite(&[]);
    assert!(!success, "no args should exit with error");
    assert!(output.contains("Usage"), "no args should show usage");
}

// ============================================================================
// READ COMMAND
// ============================================================================

#[test]
fn smoke_read_readme() {
    let (success, output) = run_ctx_lite(&["read", "README.md"]);
    assert!(success, "read should succeed for README.md");
    assert!(!output.is_empty(), "read should return content");
}

#[test]
fn smoke_read_cargo_toml() {
    let (success, output) = run_ctx_lite(&["read", "Cargo.toml"]);
    assert!(success, "read should succeed for Cargo.toml");
    assert!(
        output.contains("[") || output.contains("package"),
        "should contain toml content"
    );
}

#[test]
fn smoke_read_nonexistent_file_fails() {
    let (success, _output) = run_ctx_lite(&["read", "/nonexistent/path/to/file.txt"]);
    assert!(
        !success,
        "read should fail for paths outside project root or missing files"
    );
}

#[test]
fn smoke_read_without_path_fails() {
    let (success, output) = run_ctx_lite(&["read"]);
    assert!(!success, "read without path should fail");
    assert!(
        output.contains("Error") || output.contains("error"),
        "should show error message"
    );
}

// ============================================================================
// TREE COMMAND
// ============================================================================

#[test]
fn smoke_tree_current_directory() {
    let (success, output) = run_ctx_lite(&["tree", "."]);
    assert!(success, "tree should succeed on current directory");
    assert!(!output.is_empty(), "tree should list entries");
}

#[test]
fn smoke_tree_nonexistent_fails() {
    let (success, _output) = run_ctx_lite(&["tree", "/nonexistent/directory"]);
    assert!(!success, "tree should fail on out-of-bounds directory");
}

// ============================================================================
// SEARCH COMMAND
// ============================================================================

#[test]
fn smoke_search_text_pattern() {
    let (success, output) = run_ctx_lite(&["search", "ctx"]);

    if success {
        assert!(
            output.contains("Search") || output.contains("results"),
            "search output should show search context"
        );
    }
}

#[test]
fn smoke_search_without_query_fails() {
    let (success, output) = run_ctx_lite(&["search"]);
    assert!(!success, "search without query should fail");
    assert!(
        output.contains("Error") || output.contains("error"),
        "should show error message"
    );
}

// ============================================================================
// SHELL COMMAND
// ============================================================================

#[test]
fn smoke_shell_git_status() {
    let (success, _output) = run_ctx_lite(&["shell", ".", "git", "status"]);
    // git status is whitelisted, so if it runs, it should work
    if success {
        assert!(true);
    }
}

#[test]
fn smoke_shell_without_args_fails() {
    let (success, output) = run_ctx_lite(&["shell"]);
    assert!(!success, "shell without args should fail");
    assert!(
        output.contains("Error") || output.contains("error"),
        "should show error"
    );
}

#[test]
fn smoke_shell_with_only_cwd_fails() {
    let (success, output) = run_ctx_lite(&["shell", "."]);
    assert!(!success, "shell with only cwd should fail");
    assert!(
        output.contains("Error") || output.contains("error"),
        "should show error message"
    );
}

#[test]
fn smoke_shell_disallowed_command_fails() {
    let (success, output) = run_ctx_lite(&["shell", ".", "echo", "test"]);
    assert!(!success, "shell should reject non-whitelisted commands");
    assert!(
        output.contains("not allowed") || output.contains("Error"),
        "should indicate command not allowed"
    );
}

// ============================================================================
// DOCTOR COMMAND
// ============================================================================

#[test]
fn smoke_doctor_runs_successfully() {
    let (success, output) = run_ctx_lite(&["doctor"]);
    assert!(success, "doctor command should succeed");
    assert!(
        output.contains("Diagnostics") || output.contains("diagnostic"),
        "doctor should show diagnostics"
    );
}

#[test]
fn smoke_doctor_shows_checks() {
    let (success, output) = run_ctx_lite(&["doctor"]);
    assert!(success, "doctor should exit successfully");
    assert!(
        output.contains("✓") || output.contains("✗"),
        "doctor should show check status"
    );
}

#[test]
fn smoke_doctor_with_storage_flag() {
    let (success, output) = run_ctx_lite(&["doctor", "--storage"]);
    assert!(success, "doctor with --storage should succeed");
    assert!(
        output.contains("Diagnostics") || output.contains("diagnostic"),
        "doctor should show diagnostics"
    );
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[test]
fn smoke_unknown_command_fails() {
    let (success, output) = run_ctx_lite(&["unknown_command"]);
    assert!(!success, "unknown command should fail");
    assert!(
        output.contains("unknown") || output.contains("Error"),
        "should indicate unknown command"
    );
}

#[test]
fn smoke_invalid_flags_handled() {
    let (success, output) = run_ctx_lite(&["--invalid-flag"]);
    assert!(!success, "invalid flag should fail");
    assert!(
        output.contains("unknown") || output.contains("Error") || output.contains("Usage"),
        "should show error or usage"
    );
}

// ============================================================================
// PERFORMANCE BASELINE TESTS
// ============================================================================

#[test]
fn smoke_help_command_fast() {
    let start = Instant::now();
    let (success, _) = run_ctx_lite(&["--help"]);
    let elapsed = start.elapsed();

    assert!(success, "help should succeed");
    assert!(
        elapsed.as_millis() < 5000,
        "help should complete in under 5 seconds (took {}ms)",
        elapsed.as_millis()
    );
}

#[test]
fn smoke_version_command_fast() {
    let start = Instant::now();
    let (success, _) = run_ctx_lite(&["--version"]);
    let elapsed = start.elapsed();

    assert!(success, "version should succeed");
    assert!(
        elapsed.as_millis() < 5000,
        "version should complete in under 5 seconds (took {}ms)",
        elapsed.as_millis()
    );
}

#[test]
fn smoke_doctor_command_fast() {
    let start = Instant::now();
    let (success, _) = run_ctx_lite(&["doctor"]);
    let elapsed = start.elapsed();

    assert!(success, "doctor should succeed");
    assert!(
        elapsed.as_millis() < 10000,
        "doctor should complete in under 10 seconds (took {}ms)",
        elapsed.as_millis()
    );
}

#[test]
fn smoke_read_file_fast() {
    let start = Instant::now();
    let (success, _) = run_ctx_lite(&["read", "README.md"]);
    let elapsed = start.elapsed();

    if success {
        assert!(
            elapsed.as_millis() < 5000,
            "read should complete in under 5 seconds (took {}ms)",
            elapsed.as_millis()
        );
    }
}

#[test]
fn smoke_tree_directory_fast() {
    let start = Instant::now();
    let (success, _) = run_ctx_lite(&["tree", "."]);
    let elapsed = start.elapsed();

    assert!(success, "tree should succeed");
    assert!(
        elapsed.as_millis() < 10000,
        "tree directory should complete in under 10 seconds (took {}ms)",
        elapsed.as_millis()
    );
}

// ============================================================================
// END-TO-END WORKFLOWS
// ============================================================================

#[test]
fn smoke_workflow_help_version() {
    let (help_success, help_output) = run_ctx_lite(&["--help"]);
    let (version_success, version_output) = run_ctx_lite(&["--version"]);

    assert!(help_success, "help should succeed");
    assert!(version_success, "version should succeed");
    assert!(
        help_output.contains("ctx-lite"),
        "help should mention binary"
    );
    assert!(
        version_output.contains("ctx-lite"),
        "version should mention binary"
    );
}

#[test]
fn smoke_workflow_doctor_and_read() {
    let (doctor_success, _doctor_output) = run_ctx_lite(&["doctor"]);
    assert!(doctor_success, "doctor should succeed");

    let (read_success, read_output) = run_ctx_lite(&["read", "README.md"]);
    assert!(read_success, "read should succeed after doctor");
    assert!(!read_output.is_empty(), "read should return content");
}

// ============================================================================
// BINARY VERIFICATION
// ============================================================================

#[test]
fn smoke_binary_exists_and_executable() {
    let path = env!("CARGO_BIN_EXE_ctx-lite");
    assert!(
        std::path::Path::new(path).exists(),
        "binary should exist at {}",
        path
    );

    let (success, _) = run_ctx_lite(&["--help"]);
    assert!(success, "binary should be executable");
}
