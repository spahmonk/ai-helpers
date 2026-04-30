/// MCP setup: configures ctx-lite as an MCP server for various applications
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
                        serde_json::json!({
                            "command": "npx",
                            "args": ["-y", "@spahmonk/ctx-lite"]
                        }),
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
                        serde_json::json!({
                            "command": "ctx-lite",
                            "args": [],
                            "fileExtensions": {
                                ".rs": "rust",
                                ".ts": "typescript",
                                ".tsx": "typescript",
                                ".js": "javascript",
                                ".jsx": "javascript",
                                ".py": "python",
                                ".go": "go"
                            }
                        }),
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
        println!("\n🚀 Welcome to ctx-lite MCP Setup!\n");
        println!("This will configure ctx-lite as an MCP server for various applications.");
        println!("Your existing configurations will be backed up.\n");

        let mut results = Vec::new();

        // Try Claude Desktop
        match Self::setup_claude_desktop() {
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
        match Self::setup_copilot_cli() {
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
