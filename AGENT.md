# 🤖 ai-helpers — Agent Integration Guide

This repository provides **installable AI helper modules** that any agent can integrate on behalf of a user. When a user says *"install this tool"* and shares this repo link, follow the protocol below.

---

## Available Modules

| Module | Description | Manifest |
|--------|-------------|---------|
| [ctx-lite](agent/modules/ctx-lite/manifest.json) | Fast context extractor — 87% compression, MCP server, cross-platform | [manifest.json](agent/modules/ctx-lite/manifest.json) |
| [mem-lite](agent/modules/mem-lite/manifest.json) | Project-scoped AI memory — semantic/episodic/procedural, hybrid search, local SQLite, MCP server, cross-platform | [manifest.json](agent/modules/mem-lite/manifest.json) |

---

## Universal Integration Protocol (5 Steps)

### Step 1 — Read the manifest

Fetch the module's `manifest.json` from this repository. It contains installation commands, MCP config templates, and skill download URLs.

```
https://raw.githubusercontent.com/spahmonk/ai-helpers/main/agent/modules/<module>/manifest.json
```

### Step 2 — Check if already installed

Run the `verify` command from the manifest. If it succeeds, skip to Step 4.

### Step 3 — Install

Use the install command matching the user's OS:
- **Linux/macOS**: shell script command from `install.linux` / `install.macos`
- **Windows**: PowerShell command from `install.windows`
- **Fallback**: npm command from `install.npm`

### Step 4 — Download and run the setup skill

Fetch `ctx-lite-setup.md` from the manifest's `skills` list. This skill:
1. Guides you through asking the user 3 configuration questions
2. Generates the correct MCP config for their agent platform
3. Applies it to the right config file
4. Verifies the server starts correctly

> **Important:** The setup skill asks the user for consent before writing any config file. Do not skip this step.

### Step 5 — Register the usage skill

After setup, download `ctx-lite-usage.md` from the skills list and register it in your active skills. This skill teaches optimal usage patterns that minimize token consumption.

---

## For Module Authors

See [agent/protocol.md](agent/protocol.md) for the standard on adding new modules to this repository.

---

## Quick Reference — ctx-lite

**What it does:** Extracts compressed code context for AI assistants. Reduces context size by 87% on average.

**MCP tools exposed:**
- `ctx_read` — Read and compress a file
- `ctx_tree` — Show directory tree with compression metadata
- `ctx_search` — Search by text/regex (path-scoped)
- `ctx_shell` — Run allowed shell commands
- `ctx_doctor` — Diagnostics

**Token optimization strategy:** `ctx_tree` → `ctx_search` → targeted `ctx_read` (never read entire directories)
