# ctx-lite MCP Integration Guide

## What is MCP?

**Model Context Protocol (MCP)** is an open standard that enables AI applications to seamlessly integrate with external systems and tools. Think of it as USB-C for AI apps.

ctx-lite exposes a full MCP server that AI coding assistants (Claude, Copilot CLI, etc.) can use to efficiently extract and analyze code context.

## Installation

### Quick Setup

```bash
# Configure ctx-lite for Claude Desktop and/or Copilot CLI
ctx-lite setup-mcp --shell-profile balanced
```

This command:
- ✅ Detects your OS and available applications
- ✅ Creates backups of existing configs
- ✅ Registers ctx-lite as an MCP/LSP server
- ✅ Writes capability-policy args into the generated config when requested
- ✅ Does NOT overwrite other tools or servers

### Manual Setup

**Claude Desktop (macOS/Windows):**
```json
{
  "mcpServers": {
    "ctx-lite": {
      "command": "npx",
      "args": ["-y", "@spahmonk/ctx-lite", "--mcp", "--shell-profile", "balanced"]
    }
  }
}
```

**Copilot CLI (Linux/macOS/Windows):**
```json
{
  "lspServers": {
    "ctx-lite": {
      "command": "ctx-lite",
      "args": ["--shell-profile", "balanced"],
      "fileExtensions": {
        ".rs": "rust",
        ".ts": "typescript",
        ".js": "javascript",
        ".py": "python",
        ".go": "go"
      }
    }
  }
}
```

## Shell Capability Policy

ctx-lite exposes shell access through **named capabilities**. These map to the low-level raw allowlist, which remains the final execution boundary.

### Profiles

| Profile | What it enables |
| --- | --- |
| `safe` | inspect/log/test workflows only |
| `balanced` | `safe` plus build/lint/typecheck workflows |
| `dangerous` | side-effectful operations such as `docker run`, `docker build`, `docker compose up`, `docker exec`, `npm install`, `cargo run` |

`dangerous` is always opt-in.

### Runtime args

These args work both directly and through generated MCP setup:

```bash
ctx-lite --mcp \
  --shell-profile balanced \
  --allow-capability docker.logs,cargo.test \
  --deny-capability docker.compose.logs \
  --allow-command "git show --stat"
```

Supported policy flags:

- `--shell-profile <safe|balanced|dangerous>`
- `--allow-capability <csv>`
- `--deny-capability <csv>`
- `--allow-command <pattern>` (repeatable)

### Capability matrix

| Capability | Commands enabled | Profile |
| --- | --- | --- |
| `git.inspect` | `git rev-parse --show-toplevel`, `git status --short`, `git status --branch --short`, `git ls-files`, `git diff --stat`, `git log --oneline -n 20` | safe |
| `docker.inspect` | `docker ps`, `docker inspect ...`, `docker compose config`, `docker version` | safe |
| `docker.logs` | `docker logs ...` | safe |
| `docker.compose.ps` | `docker compose ps` | safe |
| `docker.compose.logs` | `docker compose logs ...` | safe |
| `npm.test` | `npm test` | safe |
| `cargo.test` | `cargo test ...` | safe |
| `python.pytest` | `python --version`, `python -m pytest ...` | safe |
| `python3.pytest` | `python3 --version`, `python3 -m pytest ...` | safe |
| `ruby.version` | `ruby --version` | safe |
| `ruby.rspec` | `bundle exec rspec ...` | safe |
| `npm.build` | `npm run build` | balanced |
| `npm.lint` | `npm run lint` | balanced |
| `npm.typecheck` | `npm run typecheck` | balanced |
| `cargo.build` | `cargo build` | balanced |
| `cargo.check` | `cargo check` | balanced |
| `cargo.fmt.check` | `cargo fmt --check` | balanced |
| `cargo.clippy` | `cargo clippy --all-targets --all-features` | balanced |
| `docker.run` | `docker run ...` | dangerous |
| `docker.build` | `docker build ...` | dangerous |
| `docker.compose.up` | `docker compose up ...` | dangerous |
| `docker.exec` | `docker exec ...` | dangerous |
| `npm.install` | `npm install ...` | dangerous |
| `cargo.run` | `cargo run ...` | dangerous |

## Available Tools

### 1. **search** - Fast code search
Find files and code patterns quickly using regex or text search.

```
search("function handleRequest")
search("error.*timeout", limit=50)
```

**Best for:**
- Locating functions, classes, error handlers
- Finding patterns across the codebase
- Understanding code relationships

### 2. **read** - Extract file contents
Read file contents with automatic truncation for large files.

```
read("path/to/file.rs")
read("src/main.rs", max_bytes=10000)
```

**Best for:**
- Getting exact code content after search
- Extracting specific implementations
- Understanding file structure

### 3. **tree** - Explore directory structure
List directory contents with configurable depth.

```
tree(".")
tree("src/", max_depth=2, include_hidden=true)
```

**Best for:**
- Understanding codebase organization
- Finding related files
- Exploring project structure

### 4. **shell** - Execute commands
Run only the commands allowed by the **effective capability policy**.

```
shell("git log --oneline -10")
shell("cargo test my_case", cwd="./modules/ctx-lite")
shell("docker compose logs api")
```

**Best for:**
- Viewing git history and diffs
- Running diagnostics, tests, and explicitly allowed builds
- Getting environment information

### 5. **doctor** - Diagnostic checks
Verify ctx-lite setup and configuration.

```
doctor()
doctor(include_storage=true, include_shell_policy=true)
```

**Best for:**
- Troubleshooting setup issues
- Verifying security policies
- Checking environment

## AI Agent Instructions

When you configure ctx-lite with MCP, AI agents receive detailed instructions on how to use it efficiently:

### Optimal Workflow

1. **Search First** - Use `search` to locate relevant code
2. **Understand Structure** - Use `tree` to see the layout
3. **Extract Context** - Use `read` to get file contents
4. **Check History** - Use `shell + git` for version control info
5. **Verify Setup** - Use `doctor` if something seems wrong

### Pro Tips

- **Search before reading**: Always search to know what you need
- **Batch your context**: Gather related files together
- **Prefer read/search/tree first**: shell is a secondary tool
- **Use only active shell capabilities**: instructions are generated from the effective profile
- **Respect limits**: Large files are auto-truncated for efficiency
- **Security first**: Paths are sandboxed (can't escape allowed dirs)

## Why AI Agents Love ctx-lite

✅ **Efficient context extraction**: Search + read = fast code understanding
✅ **Built-in instructions**: AI knows exactly how to use each tool
✅ **Security by default**: Path jails prevent accidental access
✅ **Performance optimized**: Parallel search, smart truncation
✅ **Policy-aware shell access**: Shell guidance matches the effective capability set
✅ **No installation needed**: Works with `npx` on any platform

## Examples

### Finding and Fixing a Bug

```
1. search("NullPointerException") → finds error location
2. read("path/to/exception.rs") → see the code
3. search("function that throws") → find related code
4. tree(".") → understand structure
5. shell("git blame path/to/file.rs") → see who changed it
6. shell("git log -p path/to/file.rs") → see history
```

### Understanding a Feature

```
1. search("feature_flag_name") → find where used
2. tree("path/to/feature") → see structure
3. read("main.rs", "handler.rs", "utils.rs") → extract context
4. shell("git log --oneline path/to/feature") → see history
```

### Code Review

```
1. search("modified_function") → find changes
2. read("relevant files") → understand changes
3. shell("git diff HEAD~1") → see exact changes
4. Analyze security, performance, correctness
```

## Doctor Output

`ctx-lite doctor` now reports the effective shell policy, including:

- whether shell is enabled
- active profile
- active capability IDs
- denied capability IDs
- custom raw allowlist patterns

Use it after `setup-mcp` or after changing process-level args to confirm what the runtime can actually execute.

## Configuration

### Claude Desktop Config Location
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

### Copilot CLI Config Location
- **All Platforms**: `~/.copilot/lsp-config.json`

## Troubleshooting

### Claude Desktop doesn't see ctx-lite

1. Verify config syntax: `cat ~/Library/Application\ Support/Claude/claude_desktop_config.json`
2. Check if `npx` is available: `which npx`
3. Restart Claude Desktop completely
4. Check logs: `~/Library/Logs/Claude/mcp*.log`

### Copilot CLI integration not working

1. Verify config: `cat ~/.copilot/lsp-config.json`
2. Check if `ctx-lite` is in PATH: `which ctx-lite`
3. Run diagnostics: `ctx-lite doctor`
4. Test MCP mode: `echo '{}' | ctx-lite --mcp`
5. If shell behavior is narrower/broader than expected, inspect the generated `args` array for `--shell-profile`, `--allow-capability`, `--deny-capability`, and `--allow-command`

## Privacy & Security

- ✅ All processing is local to your machine
- ✅ Paths are sandboxed - can't read outside allowed directories
- ✅ Shell commands are gated by capability policy and raw allowlist matching
- ✅ No data is sent to external services
- ✅ Backups are created before modifying configs

## Support

For issues or questions:
- 📖 Check the main [README.md](../README.md)
- 🐛 Open an issue on GitHub
- 💬 See [QUICK_START.md](../QUICK_START.md) for installation help
