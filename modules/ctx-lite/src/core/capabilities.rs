use std::collections::{BTreeSet, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShellCapabilityId {
    GitInspect,
    DockerInspect,
    DockerLogs,
    NpmWorkflow,
    CargoWorkflow,
    PythonTesting,
    RubyTesting,
    DangerousFilesystemWrite,
    DangerousNetwork,
    DangerousProcessControl,
}

impl ShellCapabilityId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GitInspect => "git.inspect",
            Self::DockerInspect => "docker.inspect",
            Self::DockerLogs => "docker.logs",
            Self::NpmWorkflow => "npm.workflow",
            Self::CargoWorkflow => "cargo.workflow",
            Self::PythonTesting => "python.testing",
            Self::RubyTesting => "ruby.testing",
            Self::DangerousFilesystemWrite => "dangerous.filesystem_write",
            Self::DangerousNetwork => "dangerous.network",
            Self::DangerousProcessControl => "dangerous.process_control",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "git.inspect" => Some(Self::GitInspect),
            "docker.inspect" => Some(Self::DockerInspect),
            "docker.logs" => Some(Self::DockerLogs),
            "npm.workflow" => Some(Self::NpmWorkflow),
            "cargo.workflow" => Some(Self::CargoWorkflow),
            "python.testing" => Some(Self::PythonTesting),
            "ruby.testing" => Some(Self::RubyTesting),
            "dangerous.filesystem_write" => Some(Self::DangerousFilesystemWrite),
            "dangerous.network" => Some(Self::DangerousNetwork),
            "dangerous.process_control" => Some(Self::DangerousProcessControl),
            _ => None,
        }
    }

    fn allowlist_patterns(self) -> &'static [&'static str] {
        match self {
            Self::GitInspect => &[
                "git rev-parse --show-toplevel",
                "git status --short",
                "git status --branch --short",
                "git ls-files",
                "git diff --stat",
                "git log --oneline -n 20",
            ],
            Self::DockerInspect => &[
                "docker ps",
                "docker compose ps",
                "docker inspect ...",
                "docker compose config",
                "docker version",
            ],
            Self::DockerLogs => &["docker logs ...", "docker compose logs ..."],
            Self::NpmWorkflow => &[
                "npm test",
                "npm run build",
                "npm run lint",
                "npm run typecheck",
            ],
            Self::CargoWorkflow => &[
                "cargo test ...",
                "cargo build",
                "cargo check",
                "cargo fmt --check",
                "cargo clippy --all-targets --all-features",
            ],
            Self::PythonTesting => &[
                "python --version",
                "python -m pytest ...",
                "python3 --version",
                "python3 -m pytest ...",
            ],
            Self::RubyTesting => &["ruby --version", "bundle exec rspec ..."],
            Self::DangerousFilesystemWrite
            | Self::DangerousNetwork
            | Self::DangerousProcessControl => &[],
        }
    }
}

pub const ORDERED_SHELL_CAPABILITIES: [ShellCapabilityId; 10] = [
    ShellCapabilityId::GitInspect,
    ShellCapabilityId::DockerInspect,
    ShellCapabilityId::DockerLogs,
    ShellCapabilityId::NpmWorkflow,
    ShellCapabilityId::CargoWorkflow,
    ShellCapabilityId::PythonTesting,
    ShellCapabilityId::RubyTesting,
    ShellCapabilityId::DangerousFilesystemWrite,
    ShellCapabilityId::DangerousNetwork,
    ShellCapabilityId::DangerousProcessControl,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellCapabilityProfile {
    Safe,
    Balanced,
    Dangerous,
}

impl ShellCapabilityProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Balanced => "balanced",
            Self::Dangerous => "dangerous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "safe" => Some(Self::Safe),
            "balanced" => Some(Self::Balanced),
            "dangerous" => Some(Self::Dangerous),
            _ => None,
        }
    }

    fn baseline_capabilities(self) -> &'static [ShellCapabilityId] {
        match self {
            Self::Safe => &[
                ShellCapabilityId::GitInspect,
                ShellCapabilityId::DockerInspect,
            ],
            Self::Balanced => &[
                ShellCapabilityId::GitInspect,
                ShellCapabilityId::DockerInspect,
                ShellCapabilityId::DockerLogs,
                ShellCapabilityId::NpmWorkflow,
                ShellCapabilityId::CargoWorkflow,
                ShellCapabilityId::PythonTesting,
                ShellCapabilityId::RubyTesting,
            ],
            Self::Dangerous => &ORDERED_SHELL_CAPABILITIES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPolicyInputs {
    pub profile: ShellCapabilityProfile,
    pub allow_capabilities: Vec<String>,
    pub deny_capabilities: Vec<String>,
    pub allowlist_additions: Vec<String>,
}

impl Default for ShellPolicyInputs {
    fn default() -> Self {
        Self {
            profile: ShellCapabilityProfile::Safe,
            allow_capabilities: Vec::new(),
            deny_capabilities: Vec::new(),
            allowlist_additions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveShellPolicy {
    pub active_profile: ShellCapabilityProfile,
    pub active_capabilities: Vec<ShellCapabilityId>,
    pub denied_capabilities: Vec<ShellCapabilityId>,
    pub allowlist_patterns: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPolicyResolveError {
    pub reason: String,
}

impl fmt::Display for ShellPolicyResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for ShellPolicyResolveError {}

pub fn resolve_shell_policy(
    inputs: &ShellPolicyInputs,
) -> Result<EffectiveShellPolicy, ShellPolicyResolveError> {
    let mut active_capabilities: BTreeSet<ShellCapabilityId> = inputs
        .profile
        .baseline_capabilities()
        .iter()
        .copied()
        .collect();

    for capability in parse_capabilities(&inputs.allow_capabilities, "allow_capabilities")? {
        active_capabilities.insert(capability);
    }

    let denied_capabilities = parse_capabilities(&inputs.deny_capabilities, "deny_capabilities")?;
    for capability in &denied_capabilities {
        active_capabilities.remove(capability);
    }

    let active_capabilities: Vec<ShellCapabilityId> = ORDERED_SHELL_CAPABILITIES
        .iter()
        .copied()
        .filter(|capability| active_capabilities.contains(capability))
        .collect();

    let mut seen_patterns = HashSet::new();
    let mut allowlist_patterns = Vec::new();
    for capability in &active_capabilities {
        for pattern in capability.allowlist_patterns() {
            let pattern = (*pattern).to_string();
            if seen_patterns.insert(pattern.clone()) {
                allowlist_patterns.push(pattern);
            }
        }
    }

    for pattern in &inputs.allowlist_additions {
        if seen_patterns.insert(pattern.clone()) {
            allowlist_patterns.push(pattern.clone());
        }
    }

    Ok(EffectiveShellPolicy {
        active_profile: inputs.profile,
        active_capabilities,
        denied_capabilities,
        allowlist_patterns,
    })
}

fn parse_capabilities(
    raw_capabilities: &[String],
    field: &str,
) -> Result<Vec<ShellCapabilityId>, ShellPolicyResolveError> {
    let mut seen = BTreeSet::new();
    let mut parsed = Vec::new();

    for raw_capability in raw_capabilities {
        let capability = ShellCapabilityId::parse(raw_capability.trim()).ok_or_else(|| {
            ShellPolicyResolveError {
                reason: format!(
                    "unknown shell capability id `{}` in `{field}`",
                    raw_capability.trim()
                ),
            }
        })?;

        if seen.insert(capability) {
            parsed.push(capability);
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_shell_policy, ShellCapabilityId, ShellCapabilityProfile, ShellPolicyInputs,
    };

    #[test]
    fn balanced_profile_contains_safe_plus_build_test_and_log_groups() {
        let resolved = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Balanced,
            ..ShellPolicyInputs::default()
        })
        .expect("balanced profile should resolve");

        assert_eq!(
            resolved.active_capabilities,
            vec![
                ShellCapabilityId::GitInspect,
                ShellCapabilityId::DockerInspect,
                ShellCapabilityId::DockerLogs,
                ShellCapabilityId::NpmWorkflow,
                ShellCapabilityId::CargoWorkflow,
                ShellCapabilityId::PythonTesting,
                ShellCapabilityId::RubyTesting,
            ]
        );
        assert!(resolved
            .allowlist_patterns
            .contains(&"npm run build".to_string()));
        assert!(resolved
            .allowlist_patterns
            .contains(&"docker logs ...".to_string()));
    }

    #[test]
    fn deny_removes_profile_default_capability() {
        let resolved = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Balanced,
            deny_capabilities: vec![ShellCapabilityId::DockerLogs.as_str().to_string()],
            ..ShellPolicyInputs::default()
        })
        .expect("policy should resolve");

        assert!(!resolved
            .active_capabilities
            .contains(&ShellCapabilityId::DockerLogs));
        assert_eq!(
            resolved.denied_capabilities,
            vec![ShellCapabilityId::DockerLogs]
        );
        assert!(!resolved
            .allowlist_patterns
            .contains(&"docker logs ...".to_string()));
    }

    #[test]
    fn custom_raw_allowlist_is_appended() {
        let resolved = resolve_shell_policy(&ShellPolicyInputs {
            allowlist_additions: vec![
                "git ls-files".to_string(),
                "echo hello".to_string(),
                "echo hello".to_string(),
            ],
            ..ShellPolicyInputs::default()
        })
        .expect("policy should resolve");

        assert_eq!(
            resolved
                .allowlist_patterns
                .iter()
                .filter(|pattern| pattern.as_str() == "git ls-files")
                .count(),
            1
        );
        assert_eq!(
            resolved
                .allowlist_patterns
                .iter()
                .filter(|pattern| pattern.as_str() == "echo hello")
                .count(),
            1
        );
    }

    #[test]
    fn explicit_allow_capability_extends_safe_baseline() {
        let resolved = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            allow_capabilities: vec![ShellCapabilityId::DockerLogs.as_str().to_string()],
            ..ShellPolicyInputs::default()
        })
        .expect("policy should resolve");

        assert!(resolved
            .active_capabilities
            .contains(&ShellCapabilityId::DockerLogs));
        assert!(resolved
            .allowlist_patterns
            .contains(&"docker logs ...".to_string()));
    }

    #[test]
    fn unknown_capability_returns_error() {
        let error = resolve_shell_policy(&ShellPolicyInputs {
            allow_capabilities: vec!["not.real".to_string()],
            ..ShellPolicyInputs::default()
        })
        .expect_err("invalid capability should error");

        assert_eq!(
            error.reason,
            "unknown shell capability id `not.real` in `allow_capabilities`"
        );
    }
}
