use std::path::PathBuf;

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
        if config.shell_whitelist.is_empty() {
            DoctorCheck {
                name: "shell_policy_presence".to_string(),
                severity: CheckSeverity::Warning,
                message: "Shell whitelist is empty".to_string(),
            }
        } else {
            DoctorCheck {
                name: "shell_policy_presence".to_string(),
                severity: CheckSeverity::Ok,
                message: format!(
                    "Shell allowlist has {} entries",
                    config.shell_whitelist.len()
                ),
            }
        }
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
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        };

        let report = DoctorService::run(&config);

        assert_eq!(report.overall_severity, CheckSeverity::Error);
    }
}
