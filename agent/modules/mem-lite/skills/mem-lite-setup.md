---
name: mem-lite-setup
description: Use this skill when the user asks to install or configure mem-lite as an MCP server. Guides through installation, user consent, and MCP config setup for Claude Desktop, Copilot CLI, Cursor, and Windsurf.
---

# mem-lite Setup

## Overview

This is a **rigid skill** — follow every step in order. Do not skip steps or improvise.

mem-lite is a project-scoped AI memory module. It stores architectural decisions, episodic events, and procedures in a local SQLite database, one DB per project directory. It exposes an MCP server so AI agents can remember and recall information scoped to the current project.

**Key design:** Memory is isolated per project. The project root (the directory you pass via `--root`, or `$PWD`) determines which DB is accessed. Two different projects never share memory.

---

## HARD GATE

**Do NOT write any config file until you have:**
1. Shown the user the exact config snippet that will be written
2. Received explicit confirmation ("yes", "apply", "go ahead", or equivalent)

---

## Algorithm

### Phase 1 — Detect Environment

1. **Detect OS:**
   - Linux: `uname -s` → "Linux"
   - macOS: `uname -s` → "Darwin"
   - Windows: check `$env:OS` or `%OS%` → "Windows_NT"

2. **Detect agent platform** (which app is running you):
   - Copilot CLI: config path is `~/.copilot/lsp-config.json`
   - Claude Desktop: config path varies by OS (see config_paths in manifest)
   - Cursor: `~/.cursor/mcp.json`
   - Windsurf: `~/.codeium/windsurf/mcp_config.json`
   - If unclear, ask the user: *"Which AI tool are you configuring mem-lite for? (Claude Desktop / Copilot CLI / Cursor / Windsurf)"*

### Phase 2 — Check Existing Installation

3. Run: `mem-lite --version`
   - If it succeeds → print the version and skip to Phase 4 (skip install)
   - If it fails → proceed to Phase 3

### Phase 3 — Install

4. Choose install command based on OS:
   - **Linux/macOS:**
     ```bash
     curl -fsSL https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.sh | bash
     ```
   - **Windows (PowerShell):**
     ```powershell
     iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))
     ```
   - **npm fallback (all platforms, requires Node.js 18+):**
     ```bash
     npm install -g @spahmonk/mem-lite
     ```

5. Verify: run `mem-lite --version` again. If it still fails, report the error to the user and stop.

### Phase 4 — Show Config Snippet

6. Build the MCP config snippet for the user's platform:

   **Claude Desktop / Cursor / Windsurf:**
   ```json
   {
     "mcpServers": {
       "mem-lite": {
         "command": "npx",
         "args": ["-y", "@spahmonk/mem-lite", "--mcp"]
       }
     }
   }
   ```

   **Copilot CLI** (`~/.copilot/lsp-config.json`):
   ```json
   {
     "servers": {
       "mem-lite": {
         "command": "mem-lite",
         "args": ["--mcp"]
       }
     }
   }
   ```

7. Show the user exactly what will be written and where. Then ask:
   *"Shall I write this config to `<path>`? (yes/no)"*

   **Wait for explicit confirmation before proceeding.**

### Phase 5 — Write Config

8. If confirmed:
   - Read the existing config file (or start from `{}` if absent)
   - Merge the `mem-lite` entry into `mcpServers` / `servers` — do not remove existing entries
   - Write the updated JSON back to disk

9. Tell the user: *"mem-lite MCP server is configured. Restart your AI agent to apply the change."*

### Phase 6 — Register Usage Skill

10. Fetch and activate the usage skill:
    ```
    https://raw.githubusercontent.com/spahmonk/ai-helpers/main/agent/modules/mem-lite/skills/mem-lite-usage.md
    ```
    Tell the user: *"Usage skill loaded. I'll now follow mem-lite best practices for storing and recalling project memory."*

---

## Notes

- mem-lite stores data at `~/.mem-lite/projects/<project_id>/memory.sqlite` by default.
- Each project directory gets its own isolated DB — memory never leaks between projects.
- No cloud, no Docker, no Python required. Pure local binary.
- The `--root` flag (CLI) or `root` parameter (MCP tools) overrides the project root if needed.
