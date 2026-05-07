---
name: ctx-lite-setup
description: Use this skill when the user asks to install or configure ctx-lite as an MCP server. Guides through installation, user consent, and MCP config setup for Claude Code, Copilot CLI, Cursor, and Windsurf.
---

# ctx-lite Setup

## Overview

This is a **rigid skill** — follow every step in order. Do not skip steps or improvise.

ctx-lite is a fast context extractor that acts as an MCP server. It gives AI agents tools to read, search, and compress code context with 87% compression ratio.

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
   - Claude Desktop: config path is `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows)
   - Cursor: config path is `~/.cursor/mcp.json`
   - Windsurf: config path is `~/.windsurf/mcp.json`
   - If you cannot detect, ask the user: *"Which AI tool are you configuring ctx-lite for? (Claude Desktop / Copilot CLI / Cursor / Windsurf)"*

### Phase 2 — Check Existing Installation

3. Run: `ctx-lite --version`
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
     npm install -g @spahmonk/ctx-lite
     ```

5. After installing, verify: `ctx-lite --version` — confirm it prints a version number.

### Phase 4 — User Questions (Ask One at a Time)

**Question 1 — Shell Profile**

Ask the user:

> *"Which shell access level do you want ctx-lite to have? Choose a profile:*
> - **safe** — Read-only inspection: git status/diff/log, docker logs, run tests. No side effects. *(Recommended for most users)*
> - **balanced** — Everything in safe, plus build and lint commands (cargo build, npm run build, cargo clippy, etc.)
> - **dangerous** — Everything in balanced, plus side-effectful commands like docker run, npm install, cargo run. Use only if you need it.
> - **none** — No shell access at all. ctx-lite can only read files and search.
>
> *What's your preference?*"

Wait for the user's answer. Store the chosen profile as `SHELL_PROFILE`.
- If the user says "none", omit `--shell-profile` arg and add `--no-shell` to args instead.
- If the user says nothing or "default", use `safe`.

**Question 2 — Directory Restrictions**

Ask the user:

> *"By default, ctx-lite can access all directories on your system. Do you want to restrict access to certain directories? For example, you might want to block access to your home directory's private folders.*
>
> *Reply with a list of directories to block (e.g., ~/secrets, ~/Documents/private), or say 'no restrictions' to keep full access."*

Wait for the user's answer. Store any blocked paths as `DENIED_ROOTS`.
- If they say "no restrictions" or similar, leave unrestricted.
- If they provide paths, add `--deny-root <path>` args for each path.

**Question 3 — Confirm Config**

Build the config snippet based on the platform detected in Phase 1:

**Claude Desktop:**
```json
{
  "mcpServers": {
    "ctx-lite": {
      "command": "npx",
      "args": ["-y", "@spahmonk/ctx-lite", "--mcp", "--shell-profile", "<SHELL_PROFILE>"]
    }
  }
}
```

**Copilot CLI** (merges into `lspServers` in `~/.copilot/lsp-config.json`):
```json
{
  "lspServers": {
    "ctx-lite": {
      "command": "ctx-lite",
      "args": ["--shell-profile", "<SHELL_PROFILE>"],
      "fileExtensions": {
        ".rs": "rust", ".ts": "typescript", ".tsx": "typescript",
        ".js": "javascript", ".jsx": "javascript", ".py": "python", ".go": "go"
      }
    }
  }
}
```

**Cursor / Windsurf** (merges into `mcpServers` in `~/.cursor/mcp.json` or `~/.windsurf/mcp.json`):
```json
{
  "mcpServers": {
    "ctx-lite": {
      "command": "npx",
      "args": ["-y", "@spahmonk/ctx-lite", "--mcp", "--shell-profile", "<SHELL_PROFILE>"]
    }
  }
}
```

Replace `<SHELL_PROFILE>` with the value from Question 1. If profile is "none", replace `--shell-profile <SHELL_PROFILE>` with `--no-shell`.

If the user specified `DENIED_ROOTS`, add `--deny-root <path>` entries to `args`.

Show the user:
> *"Here is the configuration I will write to `<CONFIG_FILE_PATH>`:"*
> *(display the JSON snippet)*
>
> *"Shall I apply this configuration? (yes / no)"*

Wait for explicit confirmation before proceeding.

### Phase 5 — Apply Config

6. **If the config file exists:** Create a backup by copying it to `<original>.bak` before modifying.

7. **Read existing config** (or start with `{}`). Deep-merge: add/update only the `ctx-lite` key inside `mcpServers` or `lspServers`. Do not touch other servers or keys.

8. **Write the updated config** to the file path.

9. Print: *"✅ Config written to `<CONFIG_FILE_PATH>`"*

### Phase 6 — Verify

10. Run: `ctx-lite doctor`
    - If it exits 0 → print the output and proceed
    - If it fails → show the error and suggest running `ctx-lite --version` to check installation

11. Tell the user:
    - **Claude Desktop / Cursor / Windsurf**: *"Restart the application to activate ctx-lite."*
    - **Copilot CLI**: *"ctx-lite is now configured. It will be available in your next session."*

### Phase 7 — Register Usage Skill

12. Fetch the usage skill:
    ```
    https://raw.githubusercontent.com/spahmonk/ai-helpers/main/agent/modules/ctx-lite/skills/ctx-lite-usage.md
    ```

13. Save it to your skills directory and register it. This skill teaches optimal usage patterns to minimize token consumption.

14. Print: *"🎉 ctx-lite is installed and configured! The usage optimization skill is now active."*

---

## Error Handling

| Error | Action |
|-------|--------|
| Install script fails | Try npm fallback: `npm install -g @spahmonk/ctx-lite` |
| `ctx-lite --version` fails after install | Ask user to restart terminal/shell, then retry |
| Config file path doesn't exist | Create parent directories with `mkdir -p` |
| JSON parse error in existing config | Inform user, offer to create a fresh config (with backup of broken file) |
| `ctx-lite doctor` fails | Show raw output, suggest re-running setup or filing an issue at https://github.com/spahmonk/ai-helpers/issues |
