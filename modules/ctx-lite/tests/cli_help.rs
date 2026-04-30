use std::process::Command;

#[test]
fn binary_shows_help_text() {
    let output = Command::new(env!("CARGO_BIN_EXE_ctx-lite"))
        .arg("--help")
        .output()
        .expect("binary should run");

    assert!(output.status.success(), "help should exit successfully");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be utf8");
    assert!(
        stdout.contains("ctx-lite"),
        "help should mention the binary name"
    );
    assert!(
        stdout.contains("USAGE") || stdout.contains("Usage"),
        "help should show usage text"
    );
}
