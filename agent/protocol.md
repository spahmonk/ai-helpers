# Agent Module Protocol

This document defines the standard for adding new helper modules to ai-helpers so any AI agent can install and configure them automatically.

---

## Module Directory Structure

```
agent/modules/<module-name>/
├── manifest.json          ← Required: machine-readable metadata
└── skills/
    ├── <module>-setup.md  ← Required: rigid setup skill (step-by-step install + config)
    └── <module>-usage.md  ← Required: usage optimization skill (token efficiency tips)
```

---

## manifest.json Schema

```jsonc
{
  "name": "string",          // module name, lowercase, kebab-case
  "version": "string",       // semver
  "description": "string",   // one-line description
  "homepage": "string",      // URL to repo or docs

  "install": {
    "linux":   "string",     // shell command to install on Linux
    "macos":   "string",     // shell command to install on macOS
    "windows": "string",     // PowerShell command to install on Windows
    "npm":     "string"      // npm fallback (cross-platform)
  },

  "verify": "string",        // command to run to confirm installation succeeded

  "mcp_config": {
    // Template for MCP server config entry — {{shell_profile}} is a placeholder
    "claude_desktop": { "command": "...", "args": ["..."] },
    "copilot_cli":    { "command": "...", "args": ["..."], "fileExtensions": {} }
  },

  "config_paths": {
    // Where each agent platform stores its config file, per OS
    "claude_desktop": { "macos": "...", "windows": "...", "linux": "..." },
    "copilot_cli":    { "all": "..." },
    "cursor":         { "all": "..." },
    "windsurf":       { "all": "..." }
  },

  "skills": [
    {
      "id":   "string",      // unique skill ID
      "name": "string",      // human-readable name
      "url":  "string",      // raw GitHub URL to the skill .md file
      "type": "setup|usage"  // skill category
    }
  ]
}
```

---

## Setup Skill Requirements

The setup skill (`<module>-setup.md`) MUST:

1. Use the superpowers frontmatter format (`---\nname: ...\ndescription: ...\n---`)
2. Follow a **rigid step-by-step algorithm** — no improvisation
3. Ask user for consent **before** writing any config file
4. Support all 3 target agent platforms: Claude Code, Copilot CLI, Cursor/Windsurf
5. Support all 3 OSes: Linux, macOS, Windows
6. End by registering the usage skill

---

## Usage Skill Requirements

The usage skill (`<module>-usage.md`) MUST:

1. Describe when to use each exposed tool/command
2. Provide a **token optimization workflow** (preferred operation order)
3. Cover batching patterns to reduce round-trips
4. Document any constraints (path jail, shell policy, etc.)

---

## Adding a New Module Checklist

- [ ] Create `agent/modules/<name>/` directory
- [ ] Write `manifest.json` following the schema above
- [ ] Write `<name>-setup.md` setup skill
- [ ] Write `<name>-usage.md` usage skill
- [ ] Add module entry to `AGENT.md` modules table
- [ ] Test: simulate an agent following the setup skill end-to-end
