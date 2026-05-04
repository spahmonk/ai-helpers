/// MCP setup: configures ctx-lite as an MCP server for various applications
use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum McpClient {
    ClaudeDesktop,
    CopilotCli,
}

impl McpClient {
    pub fn name(&self) -> &'static str {
        match self {
            McpClient::ClaudeDesktop => "Claude Desktop",
            McpClient::CopilotCli => "Copilot CLI",
        }
    }

    pub fn config_path(&self) -> Result<PathBuf, String> {
        match self {
            McpClient::ClaudeDesktop => Self::claude_config_path(),
            McpClient::CopilotCli => Self::copilot_config_path(),
        }
    }

    fn claude_config_path() -> Result<PathBuf, String> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
            Ok(PathBuf::from(format!(
                "{}/Library/Application Support/Claude/claude_desktop_config.json",
                home
            )))
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
            Ok(PathBuf::from(format!(
                "{}\\Claude\\claude_desktop_config.json",
                appdata
            )))
        }

        #[cfg(target_os = "linux")]
        {
            Err("Claude Desktop not supported on Linux".to_string())
        }
    }

    fn copilot_config_path() -> Result<PathBuf, String> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "HOME or USERPROFILE not set".to_string())?;

        Ok(PathBuf::from(format!("{}/.copilot/lsp-config.json", home)))
    }
}

pub struct McpSetup;

impl McpSetup {
    /// Setup MCP for Claude Desktop
    pub fn setup_claude_desktop() -> Result<SetupResult, String> {
        Self::setup_claude_desktop_with_policy(&ShellPolicyInputs::default())
    }

    pub fn setup_claude_desktop_with_policy(
        policy_inputs: &ShellPolicyInputs,
    ) -> Result<SetupResult, String> {
        let config_path = McpClient::ClaudeDesktop.config_path()?;
        Self::ensure_parent_dir(&config_path)?;

        // Read existing config or create new one
        let mut config = if config_path.exists() {
            Self::read_json_file(&config_path)?
        } else {
            serde_json::json!({})
        };

        // Create backup
        if config_path.exists() {
            let backup_path = Self::create_backup(&config_path)?;
            println!("  📦 Backup created: {}", backup_path.display());
        }

        // Ensure config is an object
        if !config.is_object() {
            config = serde_json::json!({});
        }

        // Deep merge: add/update ctx-lite in mcpServers without touching other servers
        if let Some(obj) = config.as_object_mut() {
            // Ensure mcpServers object exists
            if !obj.contains_key("mcpServers") {
                obj.insert("mcpServers".to_string(), serde_json::json!({}));
            }

            // Update or add ctx-lite server config only
            if let Some(mcp_servers) = obj.get_mut("mcpServers") {
                if let Some(servers_obj) = mcp_servers.as_object_mut() {
                    servers_obj.insert(
                        "ctx-lite".to_string(),
                        claude_ctx_lite_server_config(policy_inputs),
                    );
                }
            }
        }

        // Write config
        let json_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, json_str).map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(SetupResult {
            client: McpClient::ClaudeDesktop,
            config_path,
            message: "Claude Desktop configured successfully! Restart Claude Desktop to activate."
                .to_string(),
        })
    }

    /// Setup MCP for Copilot CLI (LSP mode)
    pub fn setup_copilot_cli() -> Result<SetupResult, String> {
        Self::setup_copilot_cli_with_policy(&ShellPolicyInputs::default())
    }

    pub fn setup_copilot_cli_with_policy(
        policy_inputs: &ShellPolicyInputs,
    ) -> Result<SetupResult, String> {
        let config_path = McpClient::CopilotCli.config_path()?;
        Self::ensure_parent_dir(&config_path)?;

        // Read existing config or create new one
        let mut config = if config_path.exists() {
            Self::read_json_file(&config_path)?
        } else {
            serde_json::json!({})
        };

        // Create backup
        if config_path.exists() {
            let backup_path = Self::create_backup(&config_path)?;
            println!("  📦 Backup created: {}", backup_path.display());
        }

        // Ensure config is an object
        if !config.is_object() {
            config = serde_json::json!({});
        }

        // Deep merge: add/update ctx-lite in lspServers without touching other servers
        if let Some(obj) = config.as_object_mut() {
            // Ensure lspServers object exists
            if !obj.contains_key("lspServers") {
                obj.insert("lspServers".to_string(), serde_json::json!({}));
            }

            // Update or add ctx-lite server config only
            if let Some(lsp_servers) = obj.get_mut("lspServers") {
                if let Some(servers_obj) = lsp_servers.as_object_mut() {
                    servers_obj.insert(
                        "ctx-lite".to_string(),
                        copilot_ctx_lite_server_config(policy_inputs),
                    );
                }
            }
        }

        // Write config
        let json_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, json_str).map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(SetupResult {
            client: McpClient::CopilotCli,
            config_path,
            message: "Copilot CLI configured successfully!".to_string(),
        })
    }

    /// Read JSON file
    fn read_json_file(path: &Path) -> Result<serde_json::Value, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in config: {}", e))
    }

    /// Ensure parent directory exists
    fn ensure_parent_dir(path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }
        Ok(())
    }

    /// Create backup of existing config
    fn create_backup(path: &Path) -> Result<PathBuf, String> {
        let backup_path = path.with_extension(format!(
            "json.backup.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs()
        ));

        fs::copy(path, &backup_path).map_err(|e| format!("Failed to create backup: {}", e))?;

        Ok(backup_path)
    }

    /// Run interactive setup
    pub fn run_interactive() -> Result<Vec<SetupResult>, String> {
        Self::run_interactive_with_policy(ShellPolicyInputs::default())
    }

    pub fn run_interactive_with_policy(
        policy_inputs: ShellPolicyInputs,
    ) -> Result<Vec<SetupResult>, String> {
        println!("\n🚀 Welcome to ctx-lite MCP Setup!\n");
        println!("This will configure ctx-lite as an MCP server for various applications.");
        println!("Your existing configurations will be backed up.\n");

        let mut results = Vec::new();

        // Try Claude Desktop
        match Self::setup_claude_desktop_with_policy(&policy_inputs) {
            Ok(result) => {
                println!("  ✓ {}: {}", result.client.name(), result.message);
                println!("    Config: {}", result.config_path.display());
                results.push(result);
            }
            Err(e) => {
                println!("  ✗ Claude Desktop: {}", e);
            }
        }

        // Try Copilot CLI
        match Self::setup_copilot_cli_with_policy(&policy_inputs) {
            Ok(result) => {
                println!("  ✓ {}: {}", result.client.name(), result.message);
                println!("    Config: {}", result.config_path.display());
                results.push(result);
            }
            Err(e) => {
                println!("  ✗ Copilot CLI: {}", e);
            }
        }

        if results.is_empty() {
            return Err("No MCP clients could be configured".to_string());
        }

        println!("\n✅ Setup complete!\n");
        Ok(results)
    }
}

pub struct SetupResult {
    pub client: McpClient,
    pub config_path: PathBuf,
    pub message: String,
}

fn build_setup_policy_args(inputs: &ShellPolicyInputs) -> Vec<String> {
    let mut args = Vec::new();

    if inputs.explicit_policy || inputs.profile != ShellCapabilityProfile::Safe {
        args.push("--shell-profile".to_string());
        args.push(inputs.profile.as_str().to_string());
    }

    if !inputs.allow_capabilities.is_empty() {
        args.push("--allow-capability".to_string());
        args.push(inputs.allow_capabilities.join(","));
    }

    if !inputs.deny_capabilities.is_empty() {
        args.push("--deny-capability".to_string());
        args.push(inputs.deny_capabilities.join(","));
    }

    for pattern in &inputs.allowlist_additions {
        args.push("--allow-command".to_string());
        args.push(pattern.clone());
    }

    args
}

fn claude_ctx_lite_server_config(policy_inputs: &ShellPolicyInputs) -> serde_json::Value {
    let mut args = vec![
        "-y".to_string(),
        "@spahmonk/ctx-lite".to_string(),
        "--mcp".to_string(),
    ];
    args.extend(build_setup_policy_args(policy_inputs));

    serde_json::json!({
        "command": "npx",
        "args": args
    })
}

fn copilot_ctx_lite_server_config(policy_inputs: &ShellPolicyInputs) -> serde_json::Value {
    serde_json::json!({
        "command": "ctx-lite",
        "args": build_setup_policy_args(policy_inputs),
        "fileExtensions": {
            ".rs": "rust",
            ".ts": "typescript",
            ".tsx": "typescript",
            ".js": "javascript",
            ".jsx": "javascript",
            ".py": "python",
            ".go": "go"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_setup_policy_args, claude_ctx_lite_server_config, copilot_ctx_lite_server_config,
    };
    use crate::core::capabilities::{ShellCapabilityProfile, ShellPolicyInputs};
    use serde_json::json;

    #[test]
    fn setup_policy_args_include_explicit_safe_profile() {
        let args = build_setup_policy_args(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        });

        assert_eq!(args, vec!["--shell-profile", "safe"]);
    }

    #[test]
    fn setup_policy_args_omit_default_implicit_safe_profile() {
        let args = build_setup_policy_args(&ShellPolicyInputs::default());

        assert!(args.is_empty());
    }

    #[test]
    fn setup_policy_args_are_deterministic_for_all_supported_flags() {
        let inputs = ShellPolicyInputs {
            profile: ShellCapabilityProfile::Balanced,
            allow_capabilities: vec!["npm.test".to_string(), "cargo.check".to_string()],
            deny_capabilities: vec!["docker.compose.logs".to_string(), "docker.logs".to_string()],
            allowlist_additions: vec!["echo hello".to_string(), "git show --stat".to_string()],
            explicit_policy: true,
        };

        let args = build_setup_policy_args(&inputs);

        assert_eq!(
            args,
            vec![
                "--shell-profile".to_string(),
                "balanced".to_string(),
                "--allow-capability".to_string(),
                "npm.test,cargo.check".to_string(),
                "--deny-capability".to_string(),
                "docker.compose.logs,docker.logs".to_string(),
                "--allow-command".to_string(),
                "echo hello".to_string(),
                "--allow-command".to_string(),
                "git show --stat".to_string(),
            ]
        );
    }

    #[test]
    fn claude_config_includes_explicit_safe_profile() {
        let config = claude_ctx_lite_server_config(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        });

        assert_eq!(
            config["args"],
            json!(["-y", "@spahmonk/ctx-lite", "--mcp", "--shell-profile", "safe"])
        );
    }

    #[test]
    fn claude_entry_includes_mcp_and_policy_args() {
        let inputs = ShellPolicyInputs {
            profile: ShellCapabilityProfile::Balanced,
            allow_capabilities: vec!["npm.test".to_string()],
            deny_capabilities: vec!["docker.compose.logs".to_string()],
            allowlist_additions: vec!["echo hello".to_string()],
            explicit_policy: true,
        };

        let entry = claude_ctx_lite_server_config(&inputs);

        assert_eq!(
            entry,
            json!({
                "command": "npx",
                "args": [
                    "-y",
                    "@spahmonk/ctx-lite",
                    "--mcp",
                    "--shell-profile",
                    "balanced",
                    "--allow-capability",
                    "npm.test",
                    "--deny-capability",
                    "docker.compose.logs",
                    "--allow-command",
                    "echo hello"
                ]
            })
        );
    }

    #[test]
    fn copilot_config_includes_explicit_safe_profile() {
        let config = copilot_ctx_lite_server_config(&ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            explicit_policy: true,
            ..ShellPolicyInputs::default()
        });

        assert_eq!(config["args"], json!(["--shell-profile", "safe"]));
    }

    #[test]
    fn copilot_entry_includes_policy_args_when_provided() {
        let inputs = ShellPolicyInputs {
            profile: ShellCapabilityProfile::Safe,
            allow_capabilities: vec!["npm.test".to_string()],
            deny_capabilities: vec!["docker.compose.logs".to_string()],
            allowlist_additions: vec!["echo hello".to_string()],
            explicit_policy: true,
        };

        let entry = copilot_ctx_lite_server_config(&inputs);
        let args = entry
            .get("args")
            .and_then(|value| value.as_array())
            .expect("copilot args should be an array");

        assert_eq!(
            args,
            &vec![
                json!("--shell-profile"),
                json!("safe"),
                json!("--allow-capability"),
                json!("npm.test"),
                json!("--deny-capability"),
                json!("docker.compose.logs"),
                json!("--allow-command"),
                json!("echo hello")
            ]
        );
    }

    #[test]
    fn no_policy_inputs_keep_default_entries_minimal() {
        let entry = copilot_ctx_lite_server_config(&ShellPolicyInputs::default());
        let args = entry
            .get("args")
            .and_then(|value| value.as_array())
            .expect("copilot args should be an array");
        assert!(args.is_empty());

        let claude_entry = claude_ctx_lite_server_config(&ShellPolicyInputs::default());
        assert_eq!(
            claude_entry,
            json!({
                "command": "npx",
                "args": ["-y", "@spahmonk/ctx-lite", "--mcp"]
            })
        );
    }
}
