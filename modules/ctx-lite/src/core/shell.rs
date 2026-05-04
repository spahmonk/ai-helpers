use std::env;
use std::ffi::OsString;
use std::io::{self, Read};
use std::iter::Peekable;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::Chars;
use std::thread;

use crate::core::security::path_jail::{PathJail, PathJailError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedShellCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl NormalizedShellCommand {
    pub fn rendered(&self) -> String {
        let mut tokens = Vec::with_capacity(self.args.len() + 1);
        tokens.push(self.program.clone());
        tokens.extend(self.args.iter().cloned());
        render_tokens(&tokens)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPolicyError {
    pub reason: String,
}

pub fn default_shell_allowlist() -> Vec<String> {
    [
        "git rev-parse --show-toplevel",
        "git status --short",
        "git status --branch --short",
        "git ls-files",
        "git diff --stat",
        "git log --oneline -n 20",
        "docker ps",
        "docker compose ps",
        "docker logs ...",
        "docker compose logs ...",
        "docker inspect ...",
        "docker compose config",
        "docker version",
        "npm test",
        "npm run build",
        "npm run lint",
        "npm run typecheck",
        "cargo test ...",
        "cargo build",
        "cargo check",
        "cargo fmt --check",
        "cargo clippy --all-targets --all-features",
        "python --version",
        "python -m pytest ...",
        "python3 --version",
        "python3 -m pytest ...",
        "ruby --version",
        "bundle exec rspec ...",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct ShellExecutor {
    max_output_bytes: usize,
    cwd_jail: PathJail,
    environment: Vec<(OsString, OsString)>,
}

impl ShellExecutor {
    pub fn new(max_output_bytes: usize, cwd_jail: PathJail) -> Self {
        Self {
            max_output_bytes,
            cwd_jail,
            environment: build_safe_environment(),
        }
    }

    pub fn execute(
        &self,
        command: &NormalizedShellCommand,
        cwd: &Path,
    ) -> Result<ShellExecutionOutput, ShellPolicyError> {
        let resolved_cwd = self
            .cwd_jail
            .resolve(cwd)
            .map_err(shell_policy_from_path_jail)?;
        let mut child = Command::new(&command.program);

        // Harden git commands by disabling config-injection vectors
        let args = if command.program == "git" {
            augment_git_args(&command.args)
        } else {
            command.args.clone()
        };

        child
            .args(&args)
            .current_dir(resolved_cwd.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .envs(self.environment.iter().cloned());

        let mut child = child.spawn().map_err(|error| ShellPolicyError {
            reason: format!("failed to spawn shell command: {error}"),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| ShellPolicyError {
            reason: "failed to capture shell stdout".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| ShellPolicyError {
            reason: "failed to capture shell stderr".to_string(),
        })?;

        let stdout_limit = self.max_output_bytes;
        let stderr_limit = self.max_output_bytes;
        let stdout_handle = thread::spawn(move || capture_stream(stdout, stdout_limit));
        let stderr_handle = thread::spawn(move || capture_stream(stderr, stderr_limit));

        let status = child.wait().map_err(|error| ShellPolicyError {
            reason: format!("failed to wait for shell command: {error}"),
        })?;

        let stdout = stdout_handle
            .join()
            .map_err(|_| ShellPolicyError {
                reason: "stdout capture thread panicked".to_string(),
            })?
            .map_err(|error| ShellPolicyError {
                reason: format!("failed to capture shell stdout: {error}"),
            })?;
        let stderr = stderr_handle
            .join()
            .map_err(|_| ShellPolicyError {
                reason: "stderr capture thread panicked".to_string(),
            })?
            .map_err(|error| ShellPolicyError {
                reason: format!("failed to capture shell stderr: {error}"),
            })?;

        Ok(ShellExecutionOutput {
            stdout: render_captured_text(stdout),
            stderr: render_captured_text(stderr),
            exit_code: status.code(),
        })
    }
}

pub fn normalize_and_validate_command(
    input: &str,
    allowlist: &[String],
) -> Result<NormalizedShellCommand, ShellPolicyError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ShellPolicyError {
            reason: "shell command cannot be empty".to_string(),
        });
    }

    let tokens = tokenize(trimmed)?;
    classify_tokens(&tokens)?;

    if !allowlist
        .iter()
        .any(|entry| allowlist_matches(entry, &tokens))
    {
        return Err(ShellPolicyError {
            reason: "shell command is not allowed by the configured whitelist".to_string(),
        });
    }

    Ok(NormalizedShellCommand {
        program: tokens[0].clone(),
        args: tokens[1..].to_vec(),
    })
}

fn allowlist_matches(entry: &str, candidate: &[String]) -> bool {
    tokenize(entry.trim())
        .map(|allowed| match allowed.split_last() {
            Some((wildcard, prefix)) if wildcard == "..." => {
                candidate.len() >= prefix.len() && prefix == &candidate[..prefix.len()]
            }
            _ => allowed == candidate,
        })
        .unwrap_or(false)
}

fn tokenize(input: &str) -> Result<Vec<String>, ShellPolicyError> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut state = TokenState::Normal;

    while let Some(ch) = chars.next() {
        match state {
            TokenState::Normal => match ch {
                ' ' | '\t' => {
                    push_token(&mut tokens, &mut current);
                }
                '\n' | '\r' | '\0' => return Err(control_character_error()),
                '\'' => state = TokenState::SingleQuoted,
                '"' => state = TokenState::DoubleQuoted,
                '>' | '<' => return Err(redirect_error()),
                '|' | '&' | ';' => {
                    return Err(wrapper_error("shell chaining operators are not allowed"))
                }
                '`' => return Err(wrapper_error("command substitution is not allowed")),
                '$' => {
                    if matches!(chars.peek(), Some('(' | '{')) {
                        return Err(wrapper_error("command substitution is not allowed"));
                    }
                    current.push(ch);
                }
                '\\' => push_escaped_normal(&mut current, &mut chars)?,
                _ => current.push(ch),
            },
            TokenState::SingleQuoted => match ch {
                '\'' => state = TokenState::Normal,
                '\0' => return Err(control_character_error()),
                _ => current.push(ch),
            },
            TokenState::DoubleQuoted => match ch {
                '"' => state = TokenState::Normal,
                '\n' | '\r' | '\0' => return Err(control_character_error()),
                '`' => return Err(wrapper_error("command substitution is not allowed")),
                '$' => {
                    if matches!(chars.peek(), Some('(' | '{')) {
                        return Err(wrapper_error("command substitution is not allowed"));
                    }
                    current.push(ch);
                }
                '\\' => push_escaped_double_quoted(&mut current, &mut chars),
                _ => current.push(ch),
            },
        }
    }

    if !matches!(state, TokenState::Normal) {
        return Err(ShellPolicyError {
            reason: "shell command contains an unterminated quote".to_string(),
        });
    }

    push_token(&mut tokens, &mut current);

    if tokens.is_empty() {
        return Err(ShellPolicyError {
            reason: "shell command cannot be empty".to_string(),
        });
    }

    Ok(tokens)
}

fn push_escaped_normal(
    current: &mut String,
    chars: &mut Peekable<Chars<'_>>,
) -> Result<(), ShellPolicyError> {
    match chars.peek().copied() {
        Some(' ' | '\t' | '\'' | '"' | '\\') => {
            current.push(chars.next().expect("peeked escape target must exist"));
        }
        Some('\n' | '\r' | '\0') => return Err(control_character_error()),
        Some(_) | None => current.push('\\'),
    }

    Ok(())
}

fn push_escaped_double_quoted(current: &mut String, chars: &mut Peekable<Chars<'_>>) {
    match chars.peek().copied() {
        Some('"') | Some('\\') | Some('$') | Some('`') => {
            current.push(chars.next().expect("peeked escape target must exist"));
        }
        Some(_) | None => current.push('\\'),
    }
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn augment_git_args(args: &[String]) -> Vec<String> {
    // Prepend safety flags to disable git config injection vectors
    // These flags disable core.fsmonitor and diff.external which can execute arbitrary commands
    let mut augmented = vec![
        String::from("-c"),
        String::from("core.fsmonitor=false"),
        String::from("-c"),
        String::from("diff.external=false"),
    ];

    // Add --no-ext-diff for diff commands specifically
    if !args.is_empty() && args[0] == "diff" {
        augmented.push(String::from("--no-ext-diff"));
    }

    augmented.extend_from_slice(args);
    augmented
}

fn classify_tokens(tokens: &[String]) -> Result<(), ShellPolicyError> {
    let program = tokens[0].to_ascii_lowercase();

    if matches!(
        program.as_str(),
        "sh" | "bash"
            | "zsh"
            | "dash"
            | "fish"
            | "cmd"
            | "cmd.exe"
            | "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "env"
    ) {
        return Err(wrapper_error("shell wrapper commands are not allowed"));
    }

    if is_remote_fetch(&program, tokens) {
        return Err(ShellPolicyError {
            reason: "remote fetch commands are not allowed".to_string(),
        });
    }

    if is_file_write(&program, tokens) {
        return Err(ShellPolicyError {
            reason: "file write commands are not allowed".to_string(),
        });
    }

    Ok(())
}

fn is_remote_fetch(program: &str, tokens: &[String]) -> bool {
    if matches!(
        program,
        "curl" | "wget" | "ftp" | "scp" | "sftp" | "ssh" | "nc" | "ncat"
    ) {
        return true;
    }

    if program == "git" {
        return matches!(
            tokens.get(1).map(|token| token.as_str()),
            Some("clone" | "fetch" | "pull" | "push" | "remote" | "submodule")
        ) || tokens.iter().skip(1).any(|token| looks_like_remote(token));
    }

    tokens.iter().skip(1).any(|token| looks_like_remote(token))
}

fn is_file_write(program: &str, tokens: &[String]) -> bool {
    if matches!(
        program,
        "tee"
            | "touch"
            | "rm"
            | "mv"
            | "cp"
            | "mkdir"
            | "rmdir"
            | "install"
            | "dd"
            | "truncate"
            | "chmod"
            | "chown"
            | "ln"
    ) {
        return true;
    }

    if program == "sed"
        && tokens
            .iter()
            .skip(1)
            .any(|token| token == "-i" || token.starts_with("-i"))
    {
        return true;
    }

    if program == "git" {
        return matches!(
            tokens.get(1).map(|token| token.as_str()),
            Some(
                "add"
                    | "apply"
                    | "am"
                    | "checkout"
                    | "clean"
                    | "commit"
                    | "merge"
                    | "rebase"
                    | "reset"
                    | "restore"
                    | "revert"
                    | "stash"
                    | "switch"
            )
        );
    }

    false
}

fn looks_like_remote(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ssh://")
        || lower.starts_with("ftp://")
}

fn render_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| {
            if token.is_empty() {
                "''".to_string()
            } else if token.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '=')
            }) {
                token.clone()
            } else {
                format!("'{}'", token.replace('\'', r"'\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn build_safe_environment() -> Vec<(OsString, OsString)> {
    let mut environment = Vec::new();

    if let Some(path) = env::var_os("PATH") {
        environment.push((OsString::from("PATH"), path));
    }

    environment.push((OsString::from("LANG"), OsString::from("C")));
    environment.push((OsString::from("LC_ALL"), OsString::from("C")));
    environment.push((OsString::from("NO_COLOR"), OsString::from("1")));

    #[cfg(windows)]
    for key in ["PATHEXT", "SYSTEMROOT", "WINDIR"] {
        if let Some(value) = env::var_os(key) {
            environment.push((OsString::from(key), value));
        }
    }

    environment
}

#[derive(Debug)]
struct CapturedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

fn capture_stream<R>(mut reader: R, limit: usize) -> io::Result<CapturedOutput>
where
    R: Read,
{
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8_192];
    let mut truncated = false;

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(bytes.len());
        if remaining > 0 {
            let kept = remaining.min(read);
            bytes.extend_from_slice(&buffer[..kept]);
        }

        if read > remaining {
            truncated = true;
        }
    }

    Ok(CapturedOutput { bytes, truncated })
}

fn render_captured_text(output: CapturedOutput) -> String {
    let mut rendered = String::from_utf8_lossy(&output.bytes).into_owned();
    if output.truncated {
        rendered.push_str("[TRUNCATED]");
    }
    rendered
}

fn redirect_error() -> ShellPolicyError {
    ShellPolicyError {
        reason: "shell redirects are not allowed".to_string(),
    }
}

fn control_character_error() -> ShellPolicyError {
    ShellPolicyError {
        reason: "shell command contains control characters".to_string(),
    }
}

fn wrapper_error(reason: &str) -> ShellPolicyError {
    ShellPolicyError {
        reason: reason.to_string(),
    }
}

fn shell_policy_from_path_jail(error: PathJailError) -> ShellPolicyError {
    ShellPolicyError {
        reason: error.message,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenState {
    Normal,
    SingleQuoted,
    DoubleQuoted,
}

#[cfg(test)]
mod tests {
    use super::{
        build_safe_environment, default_shell_allowlist, normalize_and_validate_command,
        ShellExecutor,
    };
    use crate::core::{config::AppConfig, security::path_jail::PathJail};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate directory should have a parent")
            .parent()
            .expect("modules directory should have a parent")
            .to_path_buf()
    }

    #[test]
    fn quoted_tokens_normalize_before_allowlist_matching() {
        let normalized = normalize_and_validate_command(
            " git   rev-parse  \"--show-toplevel\" ",
            &[String::from("git rev-parse --show-toplevel")],
        )
        .expect("quoted tokens should normalize before allowlist matching");

        assert_eq!(normalized.rendered(), "git rev-parse --show-toplevel");
        assert_eq!(normalized.program, "git");
        assert_eq!(normalized.args, vec!["rev-parse", "--show-toplevel"]);
    }

    #[test]
    fn wildcard_allowlist_entries_accept_trailing_args() {
        let normalized = normalize_and_validate_command(
            "docker compose logs api",
            &[String::from("docker compose logs ...")],
        )
        .expect("allowlist entries should support safe command prefixes with trailing args");

        assert_eq!(normalized.program, "docker");
        assert_eq!(normalized.args, vec!["compose", "logs", "api"]);
    }

    #[test]
    fn default_allowlist_accepts_safe_dev_tooling_commands() {
        for command in [
            "docker logs ctx-lite",
            "docker compose logs api",
            "npm run build",
            "cargo test mcp_server_accepts_json_line_requests",
            "python -m pytest tests/mcp_stdio.rs",
            "ruby --version",
        ] {
            normalize_and_validate_command(command, &default_shell_allowlist()).unwrap_or_else(
                |error| panic!("{command:?} should be allowed by default: {}", error.reason),
            );
        }
    }

    #[test]
    fn injection_like_shapes_are_rejected_explicitly() {
        for (command, expected_reason) in [
            ("git status $(whoami)", "command substitution"),
            ("git status `whoami`", "command substitution"),
            ("git status; pwd", "chaining"),
            ("git status && pwd", "chaining"),
            ("git status || pwd", "chaining"),
        ] {
            let error = normalize_and_validate_command(command, &[String::from("git status")])
                .expect_err("injection-like syntax should be rejected");

            assert!(
                error.reason.contains(expected_reason),
                "unexpected shell error for {command:?}: {}",
                error.reason
            );
        }
    }

    #[test]
    fn windows_wrapper_commands_are_rejected_explicitly() {
        for command in [
            "cmd.exe /c dir",
            "powershell.exe -Command Get-ChildItem",
            "pwsh.exe -Command Get-ChildItem",
        ] {
            let error = normalize_and_validate_command(command, &[String::from("git status")])
                .expect_err("wrapper shell variants should be rejected");

            assert!(
                error.reason.contains("wrapper"),
                "unexpected shell error for {command:?}: {}",
                error.reason
            );
        }
    }

    #[test]
    fn write_commands_are_rejected_explicitly() {
        let error = normalize_and_validate_command("tee output.txt", &[String::from("git status")])
            .expect_err("file-writing commands should be denied explicitly");

        assert!(error.reason.contains("write"));
    }

    #[test]
    fn executor_runs_allowed_git_inspection_commands() {
        let command = normalize_and_validate_command(
            "git rev-parse --show-toplevel",
            &[String::from("git rev-parse --show-toplevel")],
        )
        .expect("git inspection command should normalize");
        // Canonicalize paths to handle Windows path variations
        let workspace = workspace_root()
            .canonicalize()
            .unwrap_or_else(|_| workspace_root());
        let config = AppConfig {
            project_root: workspace.clone(),
            allowed_roots: vec![workspace.clone()],
            ..AppConfig::default()
        };

        let output = ShellExecutor::new(
            1024,
            PathJail::from_config(&config).expect("workspace path jail should initialize"),
        )
        .execute(&command, &workspace)
        .expect("executor should run allowed git inspection command");

        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stderr, "");
        // Normalize both sides for comparison (handle symlinks and case differences)
        let actual = PathBuf::from(output.stdout.trim())
            .canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| output.stdout.trim().to_string());
        let expected = workspace.display().to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn executor_truncates_large_output_to_configured_limit() {
        let command =
            normalize_and_validate_command("git ls-files", &[String::from("git ls-files")])
                .expect("git ls-files should normalize");
        // Canonicalize paths to handle Windows path variations
        let workspace = workspace_root()
            .canonicalize()
            .unwrap_or_else(|_| workspace_root());
        let config = AppConfig {
            project_root: workspace.clone(),
            allowed_roots: vec![workspace.clone()],
            ..AppConfig::default()
        };

        let output = ShellExecutor::new(
            32,
            PathJail::from_config(&config).expect("workspace path jail should initialize"),
        )
        .execute(&command, &workspace)
        .expect("executor should capture output within a fixed budget");

        assert_eq!(output.exit_code, Some(0));
        assert!(
            output.stdout.ends_with("[TRUNCATED]"),
            "stdout should include truncation marker: {:?}",
            output.stdout
        );
        assert!(output.stdout.len() <= 43);
    }

    #[test]
    fn safe_environment_only_keeps_whitelisted_variables() {
        let environment = build_safe_environment();

        // Also allow Windows-specific environment variables
        let allowed_vars = if cfg!(windows) {
            vec![
                "PATH",
                "LANG",
                "LC_ALL",
                "NO_COLOR",
                "PATHEXT",
                "SYSTEMROOT",
                "WINDIR",
            ]
        } else {
            vec!["PATH", "LANG", "LC_ALL", "NO_COLOR"]
        };

        assert!(
            environment
                .iter()
                .all(|(key, _)| allowed_vars.contains(&key.to_str().unwrap_or(""))),
            "unexpected environment: {:?}",
            environment
        );
        assert!(
            environment.iter().any(|(key, _)| key == "PATH"),
            "safe environment should preserve PATH for command lookup"
        );
    }

    #[test]
    fn executor_rejects_symlink_escaped_cwd() {
        let fixture = ShellFixture::new("cwd-symlink-escape");
        if !fixture.supports_dir_symlinks() {
            return;
        }

        let command = normalize_and_validate_command(
            "git status --short",
            &[String::from("git status --short")],
        )
        .expect("git status should normalize");
        let executor = ShellExecutor::new(
            1024,
            PathJail::from_config(&fixture.config()).expect("fixture path jail should initialize"),
        );

        let error = executor
            .execute(&command, &fixture.repo_root.join("links/outside-dir"))
            .expect_err("executor should reject cwd paths that escape through symlinks");

        assert!(
            error.reason.contains("symlink"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    struct ShellFixture {
        root: PathBuf,
        repo_root: PathBuf,
        dir_symlink_supported: bool,
    }

    impl ShellFixture {
        fn new(name: &str) -> Self {
            let fixture_id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("runtime")
                .join(format!("shell-{name}-{}-{fixture_id}", std::process::id()));
            let repo_root = root.join("repo");
            let outside_root = root.join("outside");

            if root.exists() {
                remove_path(&root);
            }

            fs::create_dir_all(repo_root.join("links"))
                .expect("fixture links dir should be created");
            fs::create_dir_all(&outside_root).expect("fixture outside dir should be created");

            let dir_symlink_supported =
                create_dir_symlink(&outside_root, &repo_root.join("links/outside-dir"));

            Self {
                root,
                repo_root,
                dir_symlink_supported,
            }
        }

        fn config(&self) -> AppConfig {
            AppConfig {
                project_root: self.repo_root.clone(),
                allowed_roots: vec![self.repo_root.clone()],
                ..AppConfig::default()
            }
        }

        fn supports_dir_symlinks(&self) -> bool {
            self.dir_symlink_supported
        }
    }

    impl Drop for ShellFixture {
        fn drop(&mut self) {
            if self.root.exists() {
                remove_path(&self.root);
            }
        }
    }

    fn remove_path(path: &Path) {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
                fs::remove_dir_all(path).expect("fixture directory should be removed");
            }
            Ok(_) => {
                fs::remove_file(path).expect("fixture file should be removed");
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to inspect fixture path {}: {error}", path.display()),
        }
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).expect("fixture symlink should be created");
        true
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                false
            }
            Err(error) => panic!("fixture symlink should be created: {error}"),
        }
    }
}
