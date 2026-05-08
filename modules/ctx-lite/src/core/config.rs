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

    pub fn effective_shell_whitelist(&self) -> Result<Vec<String>, ShellPolicyResolveError> {
        // Backward-compatible path: with no capability overrides, preserve the raw configured allowlist.
        if !self.shell_policy.explicit_policy && self.shell_policy == ShellPolicyInputs::default() {
            return Ok(self.shell_whitelist.clone());
        }

        let effective_policy = self.resolve_shell_policy()?;
        Ok(effective_policy.allowlist_patterns)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let shell_policy = ShellPolicyInputs::default();
        let shell_whitelist = default_shell_allowlist();

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
    use crate::core::shell::default_shell_allowlist;
    use std::path::PathBuf;

    #[test]
    fn defaults_are_safe() {
        let config = AppConfig::default();

        // project_root resolves to the actual working directory (always absolute)
        assert!(
            config.project_root.is_absolute(),
            "project_root should be absolute so absolute-path requests can be validated lexically"
        );
        assert_eq!(config.allowed_roots, vec![config.project_root.clone()]);
        assert!(!config.shell_enabled);
        assert_eq!(config.shell_whitelist, default_shell_allowlist());
        assert_eq!(config.max_read_bytes, 1_048_576);
        assert_eq!(config.max_shell_output_bytes, 65_536);
        assert!(!config.memory_enabled);
        assert!(config.redaction_enabled);
    }

    #[test]
    fn default_shell_policy_inputs_are_safe() {
        let config = AppConfig::default();

        assert_eq!(config.shell_policy.profile, ShellCapabilityProfile::Safe);
        assert!(!config.shell_policy.explicit_policy);
        assert!(config.shell_policy.allow_capabilities.is_empty());
        assert!(config.shell_policy.deny_capabilities.is_empty());
        assert!(config.shell_policy.allowlist_additions.is_empty());

        let resolved = resolve_shell_policy(&ShellPolicyInputs::default())
            .expect("default policy inputs should always resolve");
        assert_ne!(config.shell_whitelist, resolved.allowlist_patterns);
    }

    #[test]
    fn effective_shell_whitelist_uses_policy_when_overridden() {
        let config = AppConfig {
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Balanced,
                deny_capabilities: vec!["docker.logs".to_string()],
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            shell_whitelist: vec!["echo hello".to_string(), "npm test".to_string()],
            ..AppConfig::default()
        };

        let effective = config
            .effective_shell_whitelist()
            .expect("effective allowlist should resolve");

        assert!(effective.contains(&"npm run build".to_string()));
        assert!(!effective.contains(&"docker logs ...".to_string()));
        assert!(!effective.contains(&"echo hello".to_string()));
    }

    #[test]
    fn effective_shell_whitelist_preserves_backward_compatible_default() {
        let config = AppConfig::default();

        let effective = config
            .effective_shell_whitelist()
            .expect("default effective allowlist should resolve");

        assert_eq!(effective, default_shell_allowlist());
    }

    #[test]
    fn effective_shell_whitelist_uses_resolved_safe_profile_when_explicit() {
        let config = AppConfig {
            shell_policy: ShellPolicyInputs {
                profile: ShellCapabilityProfile::Safe,
                explicit_policy: true,
                ..ShellPolicyInputs::default()
            },
            shell_whitelist: vec!["echo hello".to_string()],
            ..AppConfig::default()
        };

        let effective = config
            .effective_shell_whitelist()
            .expect("explicit safe profile should resolve capabilities");

        assert!(effective.contains(&"docker compose config".to_string()));
        assert!(effective.contains(&"python --version".to_string()));
        assert!(effective.contains(&"python3 --version".to_string()));
        assert!(!effective.contains(&"echo hello".to_string()));
    }
}
