use std::path::PathBuf;

use crate::core::capabilities::{
    resolve_shell_policy, EffectiveShellPolicy, ShellPolicyInputs, ShellPolicyResolveError,
};
use crate::core::shell::default_shell_allowlist;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub project_root: PathBuf,
    pub allowed_roots: Vec<PathBuf>,
    pub shell_enabled: bool,
    pub shell_whitelist: Vec<String>,
    pub shell_policy: ShellPolicyInputs,
    pub max_read_bytes: usize,
    pub max_shell_output_bytes: usize,
    pub memory_enabled: bool,
    pub redaction_enabled: bool,
}

impl AppConfig {
    pub fn resolve_shell_policy(&self) -> Result<EffectiveShellPolicy, ShellPolicyResolveError> {
        resolve_shell_policy(&self.shell_policy)
    }

    pub fn refresh_shell_whitelist_from_policy(
        &mut self,
    ) -> Result<EffectiveShellPolicy, ShellPolicyResolveError> {
        let effective_policy = self.resolve_shell_policy()?;
        self.shell_whitelist = effective_policy.allowlist_patterns.clone();
        Ok(effective_policy)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let project_root = PathBuf::from(".");
        let shell_policy = ShellPolicyInputs::default();
        let shell_whitelist = resolve_shell_policy(&shell_policy)
            .map(|effective_policy| effective_policy.allowlist_patterns)
            .unwrap_or_else(|_| default_shell_allowlist());

        Self {
            project_root: project_root.clone(),
            allowed_roots: vec![project_root],
            shell_enabled: false,
            shell_whitelist,
            shell_policy,
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
    use crate::core::capabilities::{
        resolve_shell_policy, ShellCapabilityProfile, ShellPolicyInputs,
    };
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

    #[test]
    fn default_shell_policy_inputs_are_safe() {
        let config = AppConfig::default();

        assert_eq!(config.shell_policy.profile, ShellCapabilityProfile::Safe);
        assert!(config.shell_policy.allow_capabilities.is_empty());
        assert!(config.shell_policy.deny_capabilities.is_empty());
        assert!(config.shell_policy.allowlist_additions.is_empty());

        let expected = resolve_shell_policy(&ShellPolicyInputs::default())
            .expect("default policy inputs should always resolve");
        assert_eq!(config.shell_whitelist, expected.allowlist_patterns);
    }
}
