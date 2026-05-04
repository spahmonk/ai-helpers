use std::collections::{BTreeSet, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShellCapabilityId {
    GitInspect,
    DockerInspect,
    DockerLogs,
    DockerComposePs,
    DockerComposeLogs,
    NpmTest,
    NpmBuild,
    NpmLint,
    NpmTypecheck,
    CargoTest,
    CargoBuild,
    CargoCheck,
    CargoFmtCheck,
    CargoClippy,
    PythonPytest,
    Python3Pytest,
    RubyVersion,
    RubyRspec,
    DockerRun,
    DockerBuild,
    DockerComposeUp,
    DockerExec,
    NpmInstall,
    CargoRun,
}

impl ShellCapabilityId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GitInspect => "git.inspect",
            Self::DockerInspect => "docker.inspect",
            Self::DockerLogs => "docker.logs",
            Self::DockerComposePs => "docker.compose.ps",
            Self::DockerComposeLogs => "docker.compose.logs",
            Self::NpmTest => "npm.test",
            Self::NpmBuild => "npm.build",
            Self::NpmLint => "npm.lint",
            Self::NpmTypecheck => "npm.typecheck",
            Self::CargoTest => "cargo.test",
            Self::CargoBuild => "cargo.build",
            Self::CargoCheck => "cargo.check",
            Self::CargoFmtCheck => "cargo.fmt.check",
            Self::CargoClippy => "cargo.clippy",
            Self::PythonPytest => "python.pytest",
            Self::Python3Pytest => "python3.pytest",
            Self::RubyVersion => "ruby.version",
            Self::RubyRspec => "ruby.rspec",
            Self::DockerRun => "docker.run",
            Self::DockerBuild => "docker.build",
            Self::DockerComposeUp => "docker.compose.up",
            Self::DockerExec => "docker.exec",
            Self::NpmInstall => "npm.install",
            Self::CargoRun => "cargo.run",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "git.inspect" => Some(Self::GitInspect),
            "docker.inspect" => Some(Self::DockerInspect),
            "docker.logs" => Some(Self::DockerLogs),
            "docker.compose.ps" => Some(Self::DockerComposePs),
            "docker.compose.logs" => Some(Self::DockerComposeLogs),
            "npm.test" => Some(Self::NpmTest),
            "npm.build" => Some(Self::NpmBuild),
            "npm.lint" => Some(Self::NpmLint),
            "npm.typecheck" => Some(Self::NpmTypecheck),
            "cargo.test" => Some(Self::CargoTest),
            "cargo.build" => Some(Self::CargoBuild),
            "cargo.check" => Some(Self::CargoCheck),
            "cargo.fmt.check" => Some(Self::CargoFmtCheck),
            "cargo.clippy" => Some(Self::CargoClippy),
            "python.pytest" => Some(Self::PythonPytest),
            "python3.pytest" => Some(Self::Python3Pytest),
            "ruby.version" => Some(Self::RubyVersion),
            "ruby.rspec" => Some(Self::RubyRspec),
            "docker.run" => Some(Self::DockerRun),
            "docker.build" => Some(Self::DockerBuild),
            "docker.compose.up" => Some(Self::DockerComposeUp),
            "docker.exec" => Some(Self::DockerExec),
            "npm.install" => Some(Self::NpmInstall),
            "cargo.run" => Some(Self::CargoRun),
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
                "docker inspect ...",
                "docker compose config",
                "docker version",
            ],
            Self::DockerLogs => &["docker logs ..."],
            Self::DockerComposePs => &["docker compose ps"],
            Self::DockerComposeLogs => &["docker compose logs ..."],
            Self::NpmTest => &["npm test"],
            Self::NpmBuild => &["npm run build"],
            Self::NpmLint => &["npm run lint"],
            Self::NpmTypecheck => &["npm run typecheck"],
            Self::CargoTest => &["cargo test ..."],
            Self::CargoBuild => &["cargo build"],
            Self::CargoCheck => &["cargo check"],
            Self::CargoFmtCheck => &["cargo fmt --check"],
            Self::CargoClippy => &["cargo clippy --all-targets --all-features"],
            Self::PythonPytest => &["python --version", "python -m pytest ..."],
            Self::Python3Pytest => &["python3 --version", "python3 -m pytest ..."],
            Self::RubyVersion => &["ruby --version"],
            Self::RubyRspec => &["bundle exec rspec ..."],
            Self::DockerRun => &["docker run ..."],
            Self::DockerBuild => &["docker build ..."],
            Self::DockerComposeUp => &["docker compose up ..."],
            Self::DockerExec => &["docker exec ..."],
            Self::NpmInstall => &["npm install ..."],
            Self::CargoRun => &["cargo run ..."],
        }
    }
}

pub const ORDERED_SHELL_CAPABILITIES: [ShellCapabilityId; 24] = [
    ShellCapabilityId::GitInspect,
    ShellCapabilityId::DockerInspect,
    ShellCapabilityId::DockerLogs,
    ShellCapabilityId::DockerComposePs,
    ShellCapabilityId::DockerComposeLogs,
    ShellCapabilityId::NpmTest,
    ShellCapabilityId::NpmBuild,
    ShellCapabilityId::NpmLint,
    ShellCapabilityId::NpmTypecheck,
    ShellCapabilityId::CargoTest,
    ShellCapabilityId::CargoBuild,
    ShellCapabilityId::CargoCheck,
    ShellCapabilityId::CargoFmtCheck,
    ShellCapabilityId::CargoClippy,
    ShellCapabilityId::PythonPytest,
    ShellCapabilityId::Python3Pytest,
    ShellCapabilityId::RubyVersion,
    ShellCapabilityId::RubyRspec,
    ShellCapabilityId::DockerRun,
    ShellCapabilityId::DockerBuild,
    ShellCapabilityId::DockerComposeUp,
    ShellCapabilityId::DockerExec,
    ShellCapabilityId::NpmInstall,
    ShellCapabilityId::CargoRun,
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
                ShellCapabilityId::DockerLogs,
                ShellCapabilityId::DockerComposePs,
                ShellCapabilityId::DockerComposeLogs,
                ShellCapabilityId::NpmTest,
                ShellCapabilityId::CargoTest,
                ShellCapabilityId::PythonPytest,
                ShellCapabilityId::Python3Pytest,
                ShellCapabilityId::RubyVersion,
                ShellCapabilityId::RubyRspec,
            ],
            Self::Balanced => &[
                ShellCapabilityId::GitInspect,
                ShellCapabilityId::DockerInspect,
                ShellCapabilityId::DockerLogs,
                ShellCapabilityId::DockerComposePs,
                ShellCapabilityId::DockerComposeLogs,
                ShellCapabilityId::NpmTest,
                ShellCapabilityId::NpmBuild,
                ShellCapabilityId::NpmLint,
                ShellCapabilityId::NpmTypecheck,
                ShellCapabilityId::CargoTest,
                ShellCapabilityId::CargoBuild,
                ShellCapabilityId::CargoCheck,
                ShellCapabilityId::CargoFmtCheck,
                ShellCapabilityId::CargoClippy,
                ShellCapabilityId::PythonPytest,
                ShellCapabilityId::Python3Pytest,
                ShellCapabilityId::RubyVersion,
                ShellCapabilityId::RubyRspec,
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
    pub explicit_policy: bool,
}

impl Default for ShellPolicyInputs {
    fn default() -> Self {
        Self {
            profile: ShellCapabilityProfile::Safe,
            allow_capabilities: Vec::new(),
            deny_capabilities: Vec::new(),
            allowlist_additions: Vec::new(),
            explicit_policy: false,
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
    fn profiles_compose_capabilities_per_spec() {
        let safe =
            resolve_shell_policy(&ShellPolicyInputs::default()).expect("safe should resolve");
        assert!(safe
            .active_capabilities
            .contains(&ShellCapabilityId::GitInspect));
        assert!(safe
            .active_capabilities
            .contains(&ShellCapabilityId::DockerComposeLogs));
        assert!(safe
            .active_capabilities
            .contains(&ShellCapabilityId::NpmTest));
        assert!(!safe
            .active_capabilities
            .contains(&ShellCapabilityId::NpmBuild));
        assert!(!safe
            .active_capabilities
            .contains(&ShellCapabilityId::DockerRun));
        assert!(safe
            .allowlist_patterns
            .contains(&"docker compose config".to_string()));
        assert!(safe
            .allowlist_patterns
            .contains(&"python --version".to_string()));
        assert!(safe
            .allowlist_patterns
            .contains(&"python3 --version".to_string()));

        let balanced = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Balanced,
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        })
        .expect("balanced should resolve");
        assert!(balanced
            .active_capabilities
            .contains(&ShellCapabilityId::NpmBuild));
        assert!(balanced
            .active_capabilities
            .contains(&ShellCapabilityId::CargoClippy));
        assert!(!balanced
            .active_capabilities
            .contains(&ShellCapabilityId::DockerRun));

        let dangerous = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Dangerous,
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        })
        .expect("dangerous should resolve");
        assert!(dangerous
            .active_capabilities
            .contains(&ShellCapabilityId::DockerRun));
        assert!(dangerous
            .active_capabilities
            .contains(&ShellCapabilityId::NpmInstall));
        assert!(dangerous
            .active_capabilities
            .contains(&ShellCapabilityId::CargoRun));
    }

    #[test]
    fn resolution_order_applies_allow_then_deny_then_custom_additions() {
        let resolved = resolve_shell_policy(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            allow_capabilities: vec![
                ShellCapabilityId::NpmBuild.as_str().to_string(),
                ShellCapabilityId::DockerRun.as_str().to_string(),
            ],
            deny_capabilities: vec![ShellCapabilityId::DockerRun.as_str().to_string()],
            allowlist_additions: vec!["echo hello".to_string()],
            explicit_policy: true,
        })
        .expect("policy should resolve");

        assert!(resolved
            .active_capabilities
            .contains(&ShellCapabilityId::NpmBuild));
        assert!(!resolved
            .active_capabilities
            .contains(&ShellCapabilityId::DockerRun));
        assert!(resolved
            .allowlist_patterns
            .contains(&"npm run build".to_string()));
        assert!(!resolved
            .allowlist_patterns
            .contains(&"docker run ...".to_string()));
        assert!(resolved
            .allowlist_patterns
            .contains(&"echo hello".to_string()));
    }

    #[test]
    fn unknown_capability_returns_error() {
        let error = resolve_shell_policy(&ShellPolicyInputs {
            allow_capabilities: vec!["not.real".to_string()],
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        })
        .expect_err("invalid capability should error");

        assert_eq!(
            error.reason,
            "unknown shell capability id `not.real` in `allow_capabilities`"
        );
    }
}
