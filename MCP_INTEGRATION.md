# ctx-lite MCP Integration Guide

## What is MCP?

**Model Context Protocol (MCP)** is an open standard that enables AI applications to seamlessly integrate with external systems and tools. Think of it as USB-C for AI apps.

ctx-lite exposes a full MCP server that AI coding assistants (Claude, Copilot CLI, etc.) can use to efficiently extract and analyze code context.

## Installation

### Quick Setup

```bash
# Configure ctx-lite for Claude Desktop and/or Copilot CLI
ctx-lite setup-mcp
```

This command:
- ✅ Detects your OS and available applications
- ✅ Creates backups of existing configs
- ✅ Registers ctx-lite as an MCP/LSP server
- ✅ Does NOT overwrite other tools or servers

### Manual Setup

**Claude Desktop (macOS/Windows):**
```json
{
  "mcpServers": {
    "ctx-lite": {
      "command": "npx",
      "args": ["-y", "@spahmonk/ctx-lite"]
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
      "args": [],
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
Run whitelisted shell commands (git, npm, cargo, python, etc.).

```
shell("git log --oneline -10")
shell("npm test", cwd="./packages/core")
shell("git diff HEAD~1")
```

**Best for:**
- Viewing git history and diffs
- Running tests and builds
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
- **Use git commands**: Shell access to git is powerful
- **Respect limits**: Large files are auto-truncated for efficiency
- **Security first**: Paths are sandboxed (can't escape allowed dirs)

## Why AI Agents Love ctx-lite

✅ **Efficient context extraction**: Search + read = fast code understanding
✅ **Built-in instructions**: AI knows exactly how to use each tool
✅ **Security by default**: Path jails prevent accidental access
✅ **Performance optimized**: Parallel search, smart truncation
✅ **Git integration**: Full shell access for blame, diff, log
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

## Privacy & Security

- ✅ All processing is local to your machine
- ✅ Paths are sandboxed - can't read outside allowed directories
- ✅ Shell commands are whitelisted for safety
- ✅ No data is sent to external services
- ✅ Backups are created before modifying configs

## Support

For issues or questions:
- 📖 Check the main [README.md](../README.md)
- 🐛 Open an issue on GitHub
- 💬 See [QUICK_START.md](../QUICK_START.md) for installation help
