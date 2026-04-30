use std::path::PathBuf;

use crate::core::shell::default_shell_allowlist;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub project_root: PathBuf,
    pub allowed_roots: Vec<PathBuf>,
    pub shell_enabled: bool,
    pub shell_whitelist: Vec<String>,
    pub max_read_bytes: usize,
    pub max_shell_output_bytes: usize,
    pub memory_enabled: bool,
    pub redaction_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let project_root = PathBuf::from(".");

        Self {
            project_root: project_root.clone(),
            allowed_roots: vec![project_root],
            shell_enabled: false,
            shell_whitelist: default_shell_allowlist(),
            max_read_bytes: 1_048_576,
            max_shell_output_bytes: 65_536,
            memory_enabled: false,
            redaction_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use std::path::PathBuf;

    #[test]
    fn defaults_are_safe() {
        let config = AppConfig::default();

        assert_eq!(config.project_root, PathBuf::from("."));
        assert_eq!(config.allowed_roots, vec![PathBuf::from(".")]);
        assert!(!config.shell_enabled);
        assert!(config
            .shell_whitelist
            .contains(&"git rev-parse --show-toplevel".to_string()));
        assert!(config.shell_whitelist.contains(&"git ls-files".to_string()));
        assert_eq!(config.max_read_bytes, 1_048_576);
        assert_eq!(config.max_shell_output_bytes, 65_536);
        assert!(!config.memory_enabled);
        assert!(config.redaction_enabled);
    }
}
