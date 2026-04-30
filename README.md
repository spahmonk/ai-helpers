# ai-helpers

Workspace bootstrap for the local-only `ctx-lite` module.

## Baseline commands

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo build --workspace`

## Bootstrap TDD record

- Test written first: `modules/ctx-lite/tests/cli_help.rs` (`ctx-lite --help` exits 0 and prints usage text).
- Red check command: `cargo test -p ctx-lite binary_shows_help_text` before implementing the binary help path; expected failure because `ctx-lite --help` behavior was not implemented yet.
- Green check command: `cargo test -p ctx-lite binary_shows_help_text` after adding the minimal `help_text()` + `main.rs` help handling; expected pass.
