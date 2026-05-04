use std::path::PathBuf;

use crate::core::capabilities::ShellCapabilityId;
use crate::core::config::AppConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckSeverity {
    Ok,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub severity: CheckSeverity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub overall_severity: CheckSeverity,
}

pub struct DoctorService;

impl DoctorService {
    pub fn run(config: &AppConfig) -> DoctorReport {
        let mut checks = vec![];
        let mut max_severity = CheckSeverity::Ok;

        // Perform all checks
        let check = Self::check_project_root_exists(config);
        if check.severity == CheckSeverity::Error {
            max_severity = CheckSeverity::Error;
        }
        checks.push(check);

        let check = Self::check_storage_sanity(config);
        if check.severity == CheckSeverity::Error {
            max_severity = CheckSeverity::Error;
        } else if check.severity == CheckSeverity::Warning && max_severity != CheckSeverity::Error {
            max_severity = CheckSeverity::Warning;
        }
        checks.push(check);

        let check = Self::check_shell_policy_presence(config);
        if check.severity == CheckSeverity::Error {
            max_severity = CheckSeverity::Error;
        } else if check.severity == CheckSeverity::Warning && max_severity != CheckSeverity::Error {
            max_severity = CheckSeverity::Warning;
        }
        checks.push(check);

        let check = Self::check_effective_shell_policy(config);
        if check.severity == CheckSeverity::Error {
            max_severity = CheckSeverity::Error;
        } else if check.severity == CheckSeverity::Warning && max_severity != CheckSeverity::Error {
            max_severity = CheckSeverity::Warning;
        }
        checks.push(check);

        DoctorReport {
            checks,
            overall_severity: max_severity,
        }
    }

    fn check_project_root_exists(config: &AppConfig) -> DoctorCheck {
        if !config.project_root.exists() {
            DoctorCheck {
                name: "project_root_exists".to_string(),
                severity: CheckSeverity::Error,
                message: format!(
                    "Project root does not exist: {}",
                    config.project_root.display()
                ),
            }
        } else {
            DoctorCheck {
                name: "project_root_exists".to_string(),
                severity: CheckSeverity::Ok,
                message: format!("Project root exists: {}", config.project_root.display()),
            }
        }
    }

    fn check_storage_sanity(config: &AppConfig) -> DoctorCheck {
        let network_roots: Vec<&PathBuf> = config
            .allowed_roots
            .iter()
            .filter(|root| {
                let root_str = root.to_string_lossy();
                root_str.contains("//") || root_str.contains("\\\\")
            })
            .collect();

        if !network_roots.is_empty() {
            DoctorCheck {
                name: "storage_sanity".to_string(),
                severity: CheckSeverity::Warning,
                message: format!(
                    "allowed_roots contains potential network paths: {}",
                    network_roots
                        .iter()
                        .map(|r| r.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        } else {
            DoctorCheck {
                name: "storage_sanity".to_string(),
                severity: CheckSeverity::Ok,
                message: "All allowed_roots are local paths".to_string(),
            }
        }
    }

    fn check_shell_policy_presence(config: &AppConfig) -> DoctorCheck {
        match config.effective_shell_whitelist() {
            Ok(allowlist) if allowlist.is_empty() => DoctorCheck {
                name: "shell_policy_presence".to_string(),
                severity: CheckSeverity::Warning,
                message: "Shell whitelist is empty".to_string(),
            },
            Ok(allowlist) => DoctorCheck {
                name: "shell_policy_presence".to_string(),
                severity: CheckSeverity::Ok,
                message: format!("Shell allowlist has {} entries", allowlist.len()),
            },
            Err(error) => DoctorCheck {
                name: "shell_policy_presence".to_string(),
                severity: CheckSeverity::Error,
                message: format!("Failed to resolve effective shell allowlist: {error}"),
            },
        }
    }

    fn check_effective_shell_policy(config: &AppConfig) -> DoctorCheck {
        let shell_enabled = config.shell_enabled;

        match config.resolve_shell_policy() {
            Ok(policy) => {
                let active_ids: Vec<&str> = policy
                    .active_capabilities
                    .iter()
                    .map(|c| c.as_str())
                    .collect();
                let denied_ids: Vec<&str> = policy
                    .denied_capabilities
                    .iter()
                    .map(|c| c.as_str())
                    .collect();

                let mut parts = vec![
                    format!("shell_enabled={shell_enabled}"),
                    format!("profile={}", policy.active_profile.as_str()),
                    format!("active=[{}]", active_ids.join(", ")),
                ];
                if !denied_ids.is_empty() {
                    parts.push(format!("denied=[{}]", denied_ids.join(", ")));
                }
                if !config.shell_policy.allowlist_additions.is_empty() {
                    parts.push(format!(
                        "custom_patterns=[{}]",
                        config.shell_policy.allowlist_additions.join(", ")
                    ));
                }

                DoctorCheck {
                    name: "shell_effective_policy".to_string(),
                    severity: CheckSeverity::Ok,
                    message: parts.join("; "),
                }
            }
            Err(error) => DoctorCheck {
                name: "shell_effective_policy".to_string(),
                severity: CheckSeverity::Error,
                message: format!("shell_enabled={shell_enabled}; policy resolve error: {error}"),
            },
        }
    }

    /// Derives a summary of active capability IDs matched from a raw allowlist.
    /// Used when running in legacy/backward-compatible mode (no explicit capability policy).
    pub fn infer_capabilities_from_allowlist(allowlist: &[String]) -> Vec<ShellCapabilityId> {
        use crate::core::capabilities::ORDERED_SHELL_CAPABILITIES;
        ORDERED_SHELL_CAPABILITIES
            .iter()
            .copied()
            .filter(|cap| {
                cap.allowlist_patterns().iter().any(|pattern| {
                    allowlist
                        .iter()
                        .any(|entry| entry.contains(pattern.trim_end_matches(" ...")))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn doctor_detects_missing_project_root() {
        let config = AppConfig {
            project_root: PathBuf::from("/nonexistent/path/does/not/exist"),
            allowed_roots: vec![PathBuf::from(".")],
            shell_enabled: false,
            shell_whitelist: vec!["git status".to_string()],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        assert_eq!(report.overall_severity, CheckSeverity::Error);
        let root_check = report
            .checks
            .iter()
            .find(|c| c.name == "project_root_exists")
            .unwrap();
        assert_eq!(root_check.severity, CheckSeverity::Error);
    }

    #[test]
    fn doctor_verifies_project_root_exists() {
        let config = AppConfig::default();

        let report = DoctorService::run(&config);

        let root_check = report
            .checks
            .iter()
            .find(|c| c.name == "project_root_exists")
            .unwrap();
        assert_eq!(root_check.severity, CheckSeverity::Ok);
    }

    #[test]
    fn doctor_warns_if_allowed_roots_contain_network_paths() {
        let config = AppConfig {
            project_root: PathBuf::from("."),
            allowed_roots: vec![PathBuf::from("//server/share/path")],
            shell_enabled: false,
            shell_whitelist: vec!["git status".to_string()],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        let storage_check = report
            .checks
            .iter()
            .find(|c| c.name == "storage_sanity")
            .unwrap();
        assert_eq!(storage_check.severity, CheckSeverity::Warning);
    }

    #[test]
    fn doctor_verifies_shell_policy_allowlist_is_non_empty() {
        let config = AppConfig {
            project_root: PathBuf::from("."),
            allowed_roots: vec![PathBuf::from(".")],
            shell_enabled: false,
            shell_whitelist: vec!["git status".to_string()],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        let policy_check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_policy_presence")
            .unwrap();
        assert_eq!(policy_check.severity, CheckSeverity::Ok);
    }

    #[test]
    fn doctor_warns_if_shell_policy_allowlist_is_empty() {
        let config = AppConfig {
            project_root: PathBuf::from("."),
            allowed_roots: vec![PathBuf::from(".")],
            shell_enabled: false,
            shell_whitelist: vec![],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        let policy_check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_policy_presence")
            .unwrap();
        assert_eq!(policy_check.severity, CheckSeverity::Warning);
    }

    #[test]
    fn doctor_uses_effective_allowlist_when_policy_is_explicit() {
        use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};

        let config = AppConfig {
            project_root: PathBuf::from("."),
            allowed_roots: vec![PathBuf::from(".")],
            shell_enabled: true,
            shell_whitelist: vec![],
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Safe,
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        let policy_check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_policy_presence")
            .unwrap();
        assert_eq!(policy_check.severity, CheckSeverity::Ok);
        assert!(policy_check.message.contains("Shell allowlist has"));
    }

    #[test]
    fn doctor_reports_ok_when_all_checks_pass() {
        let config = AppConfig::default();

        let report = DoctorService::run(&config);

        assert_eq!(report.overall_severity, CheckSeverity::Ok);
        assert!(report
            .checks
            .iter()
            .all(|c| c.severity == CheckSeverity::Ok));
    }

    #[test]
    fn doctor_combines_multiple_checks_into_report() {
        let config = AppConfig {
            project_root: PathBuf::from("."),
            allowed_roots: vec![PathBuf::from("//network/path"), PathBuf::from(".")],
            shell_enabled: false,
            shell_whitelist: vec!["git status".to_string()],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        assert!(report.checks.len() >= 3);
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "project_root_exists"));
        assert!(report.checks.iter().any(|c| c.name == "storage_sanity"));
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "shell_policy_presence"));
    }

    #[test]
    fn doctor_error_takes_precedence_over_warning() {
        let config = AppConfig {
            project_root: PathBuf::from("/nonexistent/path"),
            allowed_roots: vec![PathBuf::from("//network/path")],
            shell_enabled: false,
            shell_whitelist: vec!["git status".to_string()],
            shell_policy: Default::default(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        assert_eq!(report.overall_severity, CheckSeverity::Error);
    }

    #[test]
    fn doctor_reports_effective_shell_policy_check() {
        let config = AppConfig::default();

        let report = DoctorService::run(&config);

        let policy_check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_effective_policy")
            .expect("report should include shell_effective_policy check");
        assert_eq!(policy_check.severity, CheckSeverity::Ok);
        assert!(policy_check.message.contains("profile=safe"));
        assert!(policy_check.message.contains("shell_enabled=false"));
        assert!(policy_check.message.contains("active=["));
    }

    #[test]
    fn doctor_effective_policy_includes_active_capability_ids() {
        use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
        let config = AppConfig {
            shell_enabled: true,
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Balanced,
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            ..AppConfig::default()
        };

        let report = DoctorService::run(&config);

        let check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_effective_policy")
            .unwrap();
        assert!(check.message.contains("profile=balanced"));
        assert!(check.message.contains("shell_enabled=true"));
        assert!(check.message.contains("git.inspect"));
        assert!(check.message.contains("npm.build"));
    }

    #[test]
    fn doctor_effective_policy_reports_denied_capabilities() {
        use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
        let config = AppConfig {
            shell_enabled: true,
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Balanced,
                deny_capabilities: vec!["docker.logs".to_string()],
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            ..AppConfig::default()
        };

        let report = DoctorService::run(&config);

        let check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_effective_policy")
            .unwrap();
        assert!(check.message.contains("denied=[docker.logs]"));
    }

    #[test]
    fn doctor_effective_policy_reports_custom_allowlist_patterns() {
        use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
        let config = AppConfig {
            shell_enabled: true,
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Safe,
                allowlist_additions: vec!["echo hello".to_string()],
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            ..AppConfig::default()
        };

        let report = DoctorService::run(&config);

        let check = report
            .checks
            .iter()
            .find(|c| c.name == "shell_effective_policy")
            .unwrap();
        assert!(check.message.contains("custom_patterns=[echo hello]"));
    }
}
