use std::path::{Component, Path, PathBuf};

use crate::core::{
    config::AppConfig,
    shell::{normalize_and_validate_command, NormalizedShellCommand},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError {
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadRequest {
    pub path: String,
    pub max_bytes: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadRequestNormalized {
    pub path: PathBuf,
    pub max_bytes: usize,
}

impl ReadRequest {
    pub fn normalize(self, config: &AppConfig) -> Result<ReadRequestNormalized, ContractError> {
        Ok(ReadRequestNormalized {
            path: normalize_required_path(&self.path, config)?,
            max_bytes: self
                .max_bytes
                .unwrap_or(config.max_read_bytes)
                .min(config.max_read_bytes),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadResponse {
    pub path: PathBuf,
    pub content: String,
    pub bytes_read: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRequest {
    pub path: String,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRequestNormalized {
    pub path: PathBuf,
    pub max_depth: usize,
    pub include_hidden: bool,
}

impl TreeRequest {
    pub fn normalize(self, config: &AppConfig) -> Result<TreeRequestNormalized, ContractError> {
        Ok(TreeRequestNormalized {
            path: normalize_optional_path(Some(&self.path), config)?,
            max_depth: self.max_depth.unwrap_or(3).min(32),
            include_hidden: self.include_hidden,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub is_directory: bool,
    pub depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeResponse {
    pub root: PathBuf,
    pub entries: Vec<TreeEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    /// Optional path to restrict the search scope (defaults to project root)
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequestNormalized {
    pub query: String,
    pub limit: usize,
    pub path: PathBuf,
}

impl SearchRequest {
    pub fn normalize(self, config: &AppConfig) -> Result<SearchRequestNormalized, ContractError> {
        Ok(SearchRequestNormalized {
            query: self.query.trim().to_string(),
            limit: self.limit.unwrap_or(20).min(100),
            path: normalize_optional_path(self.path.as_deref(), config)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResponse {
    pub query: String,
    pub hits: Vec<SearchHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellRequest {
    pub command: String,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellRequestNormalized {
    pub command: NormalizedShellCommand,
    pub cwd: PathBuf,
}

impl ShellRequest {
    pub fn normalize(self, config: &AppConfig) -> Result<ShellRequestNormalized, ContractError> {
        if !config.shell_enabled {
            return Err(ContractError {
                reason: "shell execution is disabled by default".to_string(),
            });
        }

        let effective_allowlist =
            config
                .effective_shell_whitelist()
                .map_err(|error| ContractError {
                    reason: error.reason,
                })?;
        let command = normalize_and_validate_command(&self.command, &effective_allowlist).map_err(
            |error| ContractError {
                reason: error.reason,
            },
        )?;

        Ok(ShellRequestNormalized {
            command,
            cwd: normalize_optional_path(self.cwd.as_deref(), config)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellResponse {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorRequest {
    pub include_storage: bool,
    pub include_shell_policy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorResponse {
    pub checks: Vec<DoctorCheck>,
    pub overall_severity: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServiceErrorKind {
    Unsupported,
    Unavailable,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceError {
    pub kind: ServiceErrorKind,
    pub message: String,
}

impl ServiceError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: ServiceErrorKind::Unsupported,
            message: message.into(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            kind: ServiceErrorKind::Unavailable,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: ServiceErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub trait ReadService {
    fn read(&self, request: ReadRequestNormalized) -> Result<ReadResponse, ServiceError>;
}

pub trait TreeService {
    fn tree(&self, request: TreeRequestNormalized) -> Result<TreeResponse, ServiceError>;
}

pub trait SearchService {
    fn search(&self, request: SearchRequestNormalized) -> Result<SearchResponse, ServiceError>;
}

pub trait ShellService {
    fn shell(&self, request: ShellRequestNormalized) -> Result<ShellResponse, ServiceError>;
}

pub trait DoctorService {
    fn doctor(&self, request: DoctorRequest) -> Result<DoctorResponse, ServiceError>;
}

fn normalize_required_path(input: &str, config: &AppConfig) -> Result<PathBuf, ContractError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ContractError {
            reason: "path cannot be empty".to_string(),
        });
    }

    normalize_path_candidate(Path::new(trimmed), config)
}

fn normalize_optional_path(
    input: Option<&str>,
    config: &AppConfig,
) -> Result<PathBuf, ContractError> {
    let trimmed = input.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        normalize_default_path(config)
    } else {
        normalize_path_candidate(Path::new(trimmed), config)
    }
}

fn normalize_default_path(config: &AppConfig) -> Result<PathBuf, ContractError> {
    let project_root = normalize_config_root(&config.project_root)?;
    if configured_allowed_roots(config, &project_root)?
        .iter()
        .any(|root| path_is_within_root(&project_root, root))
    {
        Ok(project_root)
    } else {
        Err(ContractError {
            reason: format!(
                "path must stay within configured allowed roots after lexical normalization: {}",
                config.project_root.display()
            ),
        })
    }
}

fn normalize_path_candidate(path: &Path, config: &AppConfig) -> Result<PathBuf, ContractError> {
    let project_root = normalize_config_root(&config.project_root)?;
    let candidate = if path.is_absolute() {
        lexical_normalize_absolute(path)?
    } else {
        lexical_join_with_base(&project_root, path)?
    };

    let allowed_roots = configured_allowed_roots(config, &project_root)?;
    if allowed_roots
        .iter()
        .any(|root| path_is_within_root(&candidate, root))
    {
        Ok(candidate)
    } else {
        let roots_display = allowed_roots
            .iter()
            .map(|r| format!("'{}'", r.display()))
            .collect::<Vec<_>>()
            .join(", ");
        let hint = build_path_hint(path, &candidate);
        Err(ContractError {
            reason: format!(
                "path '{}' resolves to '{}' which is outside the configured allowed root(s): {}{}",
                path.display(),
                candidate.display(),
                roots_display,
                hint,
            ),
        })
    }
}

/// Returns an optional hint string when the resolved path differs meaningfully from the input,
/// e.g. a Windows drive-relative path (\.aws → C:\.aws) that is likely a typo.
fn build_path_hint(requested: &Path, resolved: &Path) -> String {
    // On Windows, a path starting with \ but no drive letter is drive-relative.
    // The user probably wanted a relative path (drop the leading \).
    let req_str = requested.to_string_lossy();
    let res_str = resolved.to_string_lossy();
    if req_str.starts_with('\\') && !req_str.starts_with("\\\\") && res_str.contains(":\\") {
        if let Some(name) = requested.file_name() {
            return format!(
                ". Tip: '{}' is a drive-relative path; use '.{}{}' for a path relative to your current directory",
                requested.display(),
                std::path::MAIN_SEPARATOR,
                name.to_string_lossy()
            );
        }
    }
    String::new()
}

fn normalize_config_root(root: &Path) -> Result<PathBuf, ContractError> {
    if root.is_absolute() {
        lexical_normalize_absolute(root)
    } else {
        lexical_join_with_base(&PathBuf::new(), root)
    }
}

fn configured_allowed_roots(
    config: &AppConfig,
    project_root: &Path,
) -> Result<Vec<PathBuf>, ContractError> {
    if config.allowed_roots.is_empty() {
        return Ok(vec![project_root.to_path_buf()]);
    }

    config
        .allowed_roots
        .iter()
        .map(|root| normalize_allowed_root(root, project_root))
        .collect()
}

fn normalize_allowed_root(root: &Path, project_root: &Path) -> Result<PathBuf, ContractError> {
    let normalized = normalize_config_root(root)?;
    if normalized.is_absolute()
        || normalized == project_root
        || normalized.starts_with(project_root)
    {
        Ok(normalized)
    } else {
        lexical_join_with_base(project_root, &normalized)
    }
}

fn lexical_normalize_absolute(path: &Path) -> Result<PathBuf, ContractError> {
    lexical_join_with_base(&PathBuf::from(std::path::MAIN_SEPARATOR.to_string()), path)
}

fn lexical_join_with_base(base: &Path, requested: &Path) -> Result<PathBuf, ContractError> {
    let (mut normalized, floor, is_absolute) = normalize_base(base)?;

    for component in requested.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if path_depth(&normalized) > floor {
                    normalized.pop();
                } else {
                    return Err(ContractError {
                        reason: format!(
                            "path must stay within configured allowed roots after lexical normalization: {}",
                            requested.display()
                        ),
                    });
                }
            }
            Component::RootDir => {
                normalized = rooted_path(&normalized);
            }
            Component::Prefix(prefix) => {
                normalized = PathBuf::from(prefix.as_os_str());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else if is_absolute {
        Ok(ensure_absolute_root(normalized))
    } else {
        Ok(normalized)
    }
}

fn normalize_base(base: &Path) -> Result<(PathBuf, usize, bool), ContractError> {
    let mut normalized = PathBuf::new();
    let mut is_absolute = false;

    for component in base.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if normalized.pop() {
                    continue;
                }
                return Err(ContractError {
                    reason: format!("configured root is not lexically safe: {}", base.display()),
                });
            }
            Component::RootDir => {
                normalized = rooted_path(&normalized);
                is_absolute = true;
            }
            Component::Prefix(prefix) => {
                normalized = PathBuf::from(prefix.as_os_str());
            }
        }
    }

    let floor = path_depth(&normalized);
    Ok((normalized, floor, is_absolute))
}

fn ensure_absolute_root(path: PathBuf) -> PathBuf {
    if path.as_os_str().is_empty() {
        rooted_path(&PathBuf::new())
    } else {
        path
    }
}

fn rooted_path(prefix: &Path) -> PathBuf {
    let mut rooted = PathBuf::from(prefix);
    let root = std::path::MAIN_SEPARATOR.to_string();
    if rooted.as_os_str().is_empty() {
        PathBuf::from(root)
    } else {
        rooted.push(Path::new(&root));
        rooted
    }
}

fn path_depth(path: &Path) -> usize {
    path.components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count()
}

fn path_is_within_root(path: &Path, root: &Path) -> bool {
    if root.as_os_str().is_empty() || root == Path::new(".") {
        !path.is_absolute()
    } else {
        path == root || path.starts_with(root)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DoctorCheck, DoctorRequest, DoctorResponse, DoctorService, ReadRequest,
        ReadRequestNormalized, ReadResponse, ReadService, SearchRequest, SearchRequestNormalized,
        SearchResponse, SearchService, ServiceError, ServiceErrorKind, ShellRequest,
        ShellRequestNormalized, ShellResponse, ShellService, TreeEntry, TreeRequest,
        TreeRequestNormalized, TreeResponse, TreeService,
    };
    use crate::app::AppServices;
    use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
    use crate::core::config::AppConfig;
    use std::path::PathBuf;

    #[test]
    fn read_and_tree_requests_are_normalized_lexically() {
        let config = AppConfig {
            project_root: PathBuf::from("workspace"),
            allowed_roots: vec![PathBuf::from("workspace")],
            max_read_bytes: 128,
            ..AppConfig::default()
        };

        let read = ReadRequest {
            path: "  ./docs/../README.md  ".into(),
            max_bytes: Some(512),
        }
        .normalize(&config)
        .expect("read request should normalize");
        assert_eq!(read.path, PathBuf::from("workspace/README.md"));
        assert_eq!(read.max_bytes, 128);

        let tree = TreeRequest {
            path: "  src/./app/../core  ".into(),
            max_depth: Some(100),
            include_hidden: true,
        }
        .normalize(&config)
        .expect("tree request should normalize");
        assert_eq!(tree.path, PathBuf::from("workspace/src/core"));
        assert_eq!(tree.max_depth, 32);
        assert!(tree.include_hidden);
    }

    #[test]
    fn read_and_tree_requests_reject_lexical_escape() {
        let config = AppConfig {
            project_root: PathBuf::from("workspace"),
            allowed_roots: vec![PathBuf::from("workspace")],
            ..AppConfig::default()
        };

        let read_error = ReadRequest {
            path: " ../../etc/passwd ".into(),
            max_bytes: None,
        }
        .normalize(&config)
        .expect_err("read request should reject paths outside the lexical root");
        assert!(
            read_error.reason.contains("allowed roots"),
            "unexpected read error: {}",
            read_error.reason
        );

        let tree_error = TreeRequest {
            path: "../secrets".into(),
            max_depth: None,
            include_hidden: false,
        }
        .normalize(&config)
        .expect_err("tree request should reject paths outside the lexical root");
        assert!(
            tree_error.reason.contains("allowed roots"),
            "unexpected tree error: {}",
            tree_error.reason
        );
    }

    #[test]
    fn search_requests_trim_outer_whitespace_without_rewriting_query() {
        let config = AppConfig::default();

        let search = SearchRequest {
            query: "  app   config\tquery  ".into(),
            limit: None,
            path: None,
        }
        .normalize(&config)
        .expect("normalization should succeed");
        assert_eq!(search.query, "app   config\tquery");
        assert_eq!(search.limit, 20);
    }

    #[test]
    fn shell_requests_are_rejected_by_default() {
        let config = AppConfig::default();

        let request = ShellRequest {
            command: "echo hello".into(),
            cwd: None,
        };

        assert!(request.normalize(&config).is_err());
    }

    #[test]
    fn shell_requests_require_whitelisted_command() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["pwd".into()],
            ..AppConfig::default()
        };

        let error = ShellRequest {
            command: "echo hello".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("shell request should reject commands outside the whitelist");

        assert!(
            error.reason.contains("whitelist"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn shell_requests_use_effective_policy_allowlist_when_policy_is_overridden() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["echo hello".into()],
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Safe,
                allow_capabilities: vec!["npm.build".into()],
                ..ShellPolicyInputs::default()
            },
            ..AppConfig::default()
        };

        let denied = ShellRequest {
            command: "echo hello".into(),
            cwd: None,
        }
        .normalize(&config);
        assert!(denied.is_err());

        let allowed = ShellRequest {
            command: "npm run build".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect("policy override should control shell normalization");

        assert_eq!(allowed.command.rendered(), "npm run build");
    }

    #[test]
    fn shell_requests_normalize_tokens_before_whitelist_match() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git rev-parse --show-toplevel".into()],
            ..AppConfig::default()
        };

        let normalized = ShellRequest {
            command: "  git   rev-parse   \"--show-toplevel\"  ".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect("shell command should normalize before allowlist matching");

        assert_eq!(normalized.command.program, "git");
        assert_eq!(
            normalized.command.args,
            vec!["rev-parse", "--show-toplevel"]
        );
    }

    #[test]
    fn shell_requests_reject_redirects_with_explicit_reason() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git status".into()],
            ..AppConfig::default()
        };

        let error = ShellRequest {
            command: "git status > status.txt".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("redirects should be rejected explicitly");

        assert!(
            error.reason.contains("redirect"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn shell_requests_reject_wrapper_shells_with_explicit_reason() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git status".into()],
            ..AppConfig::default()
        };

        let error = ShellRequest {
            command: "sh -c 'git status'".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("shell wrappers should be rejected explicitly");

        assert!(
            error.reason.contains("wrapper"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn shell_requests_reject_remote_fetches_with_explicit_reason() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git status".into()],
            ..AppConfig::default()
        };

        let error = ShellRequest {
            command: "curl https://example.com".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("remote fetches should be rejected explicitly");

        assert!(
            error.reason.contains("remote fetch"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn shell_requests_reject_file_write_commands_with_explicit_reason() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git status".into()],
            ..AppConfig::default()
        };

        let error = ShellRequest {
            command: "tee status.txt".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("file-writing commands should be rejected explicitly");

        assert!(
            error.reason.contains("write"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn shell_requests_preserve_internal_whitespace_and_normalize_cwd() {
        let config = AppConfig {
            project_root: PathBuf::from("workspace"),
            allowed_roots: vec![PathBuf::from("workspace")],
            shell_enabled: true,
            shell_whitelist: vec!["printf 'a   b'".into()],
            ..AppConfig::default()
        };

        let request = ShellRequest {
            command: "  printf 'a   b'  ".into(),
            cwd: Some(" ./bin/../scripts ".into()),
        };

        let normalized = request
            .normalize(&config)
            .expect("shell request should normalize");

        assert_eq!(normalized.command.program, "printf");
        assert_eq!(normalized.command.args, vec!["a   b"]);
        assert_eq!(normalized.cwd, PathBuf::from("workspace/scripts"));
    }

    #[test]
    fn shell_requests_reject_injection_like_shapes_with_explicit_reasons() {
        let config = AppConfig {
            shell_enabled: true,
            shell_whitelist: vec!["git status".into()],
            ..AppConfig::default()
        };

        for (command, expected_reason) in [
            ("git status $(whoami)", "command substitution"),
            ("git status `whoami`", "command substitution"),
            ("git status; pwd", "chaining"),
            ("git status && pwd", "chaining"),
            ("git status || pwd", "chaining"),
        ] {
            let error = ShellRequest {
                command: command.into(),
                cwd: None,
            }
            .normalize(&config)
            .expect_err("injection-like shell syntax should be rejected explicitly");

            assert!(
                error.reason.contains(expected_reason),
                "unexpected shell error for {command:?}: {}",
                error.reason
            );
        }
    }

    #[test]
    fn shell_requests_reject_lexical_escape_in_cwd() {
        let config = AppConfig {
            project_root: PathBuf::from("workspace"),
            allowed_roots: vec![PathBuf::from("workspace")],
            shell_enabled: true,
            shell_whitelist: vec!["echo ok".into()],
            ..AppConfig::default()
        };

        let request = ShellRequest {
            command: "echo ok".into(),
            cwd: Some("../outside".into()),
        };

        let error = request
            .normalize(&config)
            .expect_err("shell cwd should reject paths outside the lexical root");

        assert!(
            error.reason.contains("allowed roots"),
            "unexpected shell error: {}",
            error.reason
        );
    }

    #[test]
    fn optional_paths_reject_project_root_outside_allowed_roots() {
        let config = AppConfig {
            project_root: PathBuf::from("workspace"),
            allowed_roots: vec![PathBuf::from("workspace/src")],
            shell_enabled: true,
            shell_whitelist: vec!["pwd".into()],
            ..AppConfig::default()
        };

        let tree_error = TreeRequest {
            path: "   ".into(),
            max_depth: None,
            include_hidden: false,
        }
        .normalize(&config)
        .expect_err("empty tree path should reject project root outside allowed roots");
        assert!(
            tree_error.reason.contains("allowed roots"),
            "unexpected tree error: {}",
            tree_error.reason
        );

        let shell_error = ShellRequest {
            command: "pwd".into(),
            cwd: None,
        }
        .normalize(&config)
        .expect_err("empty shell cwd should reject project root outside allowed roots");
        assert!(
            shell_error.reason.contains("allowed roots"),
            "unexpected shell error: {}",
            shell_error.reason
        );
    }

    #[test]
    fn relative_allowed_roots_are_resolved_from_project_root() {
        let config = AppConfig {
            project_root: PathBuf::from("/repo"),
            allowed_roots: vec![PathBuf::from("src")],
            ..AppConfig::default()
        };

        let read = ReadRequest {
            path: "src/./lib.rs".into(),
            max_bytes: None,
        }
        .normalize(&config)
        .expect("relative allowed root should be resolved from project root");

        assert_eq!(read.path, PathBuf::from("/repo/src/lib.rs"));
    }

    #[test]
    fn shared_services_can_return_runtime_errors() {
        struct ReadStub;
        impl ReadService for ReadStub {
            fn read(&self, _request: ReadRequestNormalized) -> Result<ReadResponse, ServiceError> {
                Err(ServiceError::internal("read failed"))
            }
        }

        struct TreeStub;
        impl TreeService for TreeStub {
            fn tree(&self, request: TreeRequestNormalized) -> Result<TreeResponse, ServiceError> {
                Ok(TreeResponse {
                    root: request.path,
                    entries: vec![TreeEntry {
                        path: PathBuf::from("workspace/src"),
                        is_directory: true,
                        depth: 1,
                    }],
                })
            }
        }

        struct SearchStub;
        impl SearchService for SearchStub {
            fn search(
                &self,
                request: SearchRequestNormalized,
            ) -> Result<SearchResponse, ServiceError> {
                Ok(SearchResponse {
                    query: request.query,
                    hits: Vec::new(),
                })
            }
        }

        struct ShellStub;
        impl ShellService for ShellStub {
            fn shell(
                &self,
                request: ShellRequestNormalized,
            ) -> Result<ShellResponse, ServiceError> {
                Ok(ShellResponse {
                    command: request.command.rendered(),
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: Some(0),
                })
            }
        }

        struct DoctorStub;
        impl DoctorService for DoctorStub {
            fn doctor(&self, request: DoctorRequest) -> Result<DoctorResponse, ServiceError> {
                Ok(DoctorResponse {
                    overall_severity: "ok".into(),
                    checks: vec![DoctorCheck {
                        name: "storage".into(),
                        passed: request.include_storage,
                        detail: None,
                    }],
                })
            }
        }

        let services = AppServices::new(ReadStub, TreeStub, SearchStub, ShellStub, DoctorStub);
        let error = services
            .read
            .read(ReadRequestNormalized {
                path: PathBuf::from("workspace/file.txt"),
                max_bytes: 64,
            })
            .expect_err("read service should surface runtime errors");
        assert_eq!(error.kind, ServiceErrorKind::Internal);
        assert_eq!(error.message, "read failed");
    }

    #[cfg(windows)]
    #[test]
    fn windows_absolute_request_paths_keep_drive_prefix_during_normalization() {
        let config = AppConfig {
            project_root: PathBuf::from(r"C:\repo"),
            allowed_roots: vec![PathBuf::from(r"C:\repo")],
            ..AppConfig::default()
        };

        let read = ReadRequest {
            path: r" C:\repo\src\..\lib.rs ".into(),
            max_bytes: None,
        }
        .normalize(&config)
        .expect("windows absolute path should keep its drive prefix");

        assert_eq!(read.path, PathBuf::from(r"C:\repo\lib.rs"));
    }
}
