# ⚡ Quick Start Guide

Install and start using `ctx-lite` in 3 minutes.

## 1️⃣ Installation

### Linux / macOS
```bash
curl -fsSL https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.sh | bash
```

**Install without sudo** (custom directory):
```bash
curl -fsSL https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.sh \
  | CTX_LITE_INSTALL_DIR=$HOME/.local/bin bash
```

### Windows (PowerShell)
```powershell
powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))"
```

### Via npm (Node.js 18+)
```bash
npm install -g @spahmonk/ctx-lite
```

## 2️⃣ Verify Installation

```bash
ctx-lite --version
ctx-lite --help
```

You should see the version number and a list of available commands.

## 3️⃣ First Use

### Read a file
```bash
ctx-lite read src/main.rs
```

### Show directory tree
```bash
ctx-lite tree ./src
```

### Search in code
```bash
# Search everywhere in the current directory
ctx-lite search "function_name"

# Scope the search to a subdirectory
ctx-lite search "TODO" ./src
```

### Run diagnostics
```bash
ctx-lite doctor
```

## 4️⃣ MCP Server Mode

ctx-lite can act as an MCP server for AI tools (Claude Desktop, GitHub Copilot, etc.):

```bash
# Auto-configure MCP for installed AI tools
ctx-lite setup-mcp

# Or run the MCP server manually (pipe JSON-RPC via stdin/stdout)
ctx-lite --mcp --allow .
```

## 📚 Next Steps

- **All commands**: `ctx-lite --help`
- **MCP integration guide**: [MCP_INTEGRATION.md](MCP_INTEGRATION.md)
- **Uninstall**: see [Uninstall / Cleanup](#5️⃣-uninstall--cleanup) below
- **Installation issues?**: see [Troubleshooting](#troubleshooting) below

## 5️⃣ Uninstall / Cleanup

### Remove the installed binary

**Linux/macOS (install.sh):**
```bash
sudo rm -f /usr/local/bin/ctx-lite
```

If you installed to a custom directory via `CTX_LITE_INSTALL_DIR`, remove `ctx-lite` from that directory instead.

**Windows (install.ps1):**
```powershell
Remove-Item "$env:ProgramFiles\ctx-lite" -Recurse -Force
```

Also remove `%ProgramFiles%\ctx-lite` from your **User PATH** if it was added.

**npm:**
```bash
npm uninstall -g @spahmonk/ctx-lite
```

### Full cleanup (local data / cache)

**Linux/macOS:**
```bash
rm -rf ~/.ctx-lite ~/.ctx-lite-cache
```

**Windows:**
```powershell
Remove-Item "$HOME\.ctx-lite","$HOME\.ctx-lite-cache" -Recurse -Force -ErrorAction SilentlyContinue
```

## 🆘 Troubleshooting

### "ctx-lite: command not found"

**Linux/macOS:**
```bash
# Add to PATH for the current session
export PATH="$PATH:/usr/local/bin"
# Add to ~/.bashrc or ~/.zshrc for a permanent effect
```

**Windows:**
- Restart PowerShell/cmd after installation
- Or manually add `%ProgramFiles%\ctx-lite` to PATH

### Download fails

Make sure:
1. Internet is working: `ping github.com`
2. curl is installed: `curl --version`
3. The release exists: https://github.com/spahmonk/ai-helpers/releases

### Permission denied

**Linux/macOS:**
```bash
sudo chmod +x /usr/local/bin/ctx-lite
```

**Windows:**
Run PowerShell as Administrator and retry the installation.

### First `npm` run is slow (2-3 seconds)

This is normal — the npm package downloads the native binary on first use and caches it at `~/.ctx-lite-cache/`. Subsequent calls are instant.

---

**You're all set!** Use `ctx-lite` to extract code context for your AI assistant. 🚀
