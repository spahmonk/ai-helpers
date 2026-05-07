---
name: ctx-lite-usage
description: Use this skill when working with ctx-lite MCP tools. Provides token-efficient patterns for reading code context, searching files, and running shell commands. Apply this before exploring any codebase.
---

# ctx-lite Usage Optimization

## Overview

ctx-lite exposes 5 MCP tools for code context extraction. Using them in the right order and with appropriate scope dramatically reduces token consumption compared to reading files directly.

**Core principle: Narrow before you read. Tree → Search → Targeted read.**

---

## Available Tools

| Tool | Purpose | Token Cost |
|------|---------|-----------|
| `ctx_tree` | Show directory structure with file sizes | Very low |
| `ctx_search` | Search by text/regex, optionally scoped to a path | Low |
| `ctx_read` | Read and compress a single file | Medium (compressed) |
| `ctx_shell` | Run allowed shell commands | Varies |
| `ctx_doctor` | Diagnose MCP server state | Low |

---

## Token-Efficient Workflow

### Step 1 — Always start with `ctx_tree`

Before reading any file, call `ctx_tree` on the relevant directory:

```
ctx_tree("./src", depth=2)
```

This gives you the structure and file sizes in one low-cost call. Use it to:
- Identify relevant files before reading them
- Understand directory organization
- Avoid reading large files accidentally

**Never call `ctx_read` on a directory or multiple files without scoping first.**

### Step 2 — Use `ctx_search` to find relevant code

Once you know the structure, search for specific symbols, patterns, or text:

```
ctx_search("function_name", "./src")
ctx_search("TODO|FIXME", "./src/core")
ctx_search("impl.*Trait")
```

Search returns: file path, line number, matching line — without loading full file content. This is 10-50x cheaper than reading files and grepping manually.

**Tips:**
- Always scope search to the smallest relevant directory
- Use regex for flexible matching
- Search for function names, struct names, error messages — not for vague concepts

### Step 3 — Read only what you need

After identifying the exact files and lines you need:

```
ctx_read("./src/core/search/mod.rs")
```

ctx-lite applies compression automatically (87% average reduction). Do not read files you haven't identified as relevant in Steps 1-2.

**Batching:** If you need multiple files, call `ctx_read` for all of them in a single response (parallel tool calls). Never make sequential read calls when files are independent.

---

## Shell Commands (`ctx_shell`)

Shell access depends on the configured profile. Check active profile with `ctx_doctor`.

### Safe profile (default)
```
ctx_shell("git status --short")
ctx_shell("git diff --stat")
ctx_shell("git log --oneline -n 20")
ctx_shell("docker logs <container>")
ctx_shell("npm test")
ctx_shell("cargo test")
```

### Balanced profile
Everything in safe, plus:
```
ctx_shell("cargo build")
ctx_shell("npm run build")
ctx_shell("cargo clippy --all-targets")
```

### Dangerous profile
Everything in balanced, plus:
```
ctx_shell("docker run ...")
ctx_shell("npm install ...")
ctx_shell("cargo run ...")
```

**Rule:** If a shell command is available, prefer it over reading config files. `git status` is faster and cheaper than `ctx_read(".git/index")`.

---

## Depth and Scope Guidelines

| Scenario | Recommended call |
|----------|-----------------|
| Explore entire project | `ctx_tree(".", depth=2)` |
| Explore a module | `ctx_tree("./src/module", depth=3)` |
| Find a function | `ctx_search("fn function_name", "./src")` |
| Find all usages of a type | `ctx_search("TypeName", "./src")` |
| Read a small config file | `ctx_read("./config.json")` |
| Read a large source file | `ctx_read("./src/large_module.rs")` — compression handles it |
| Check project health | `ctx_shell("cargo test -- --test-threads=1")` or `ctx_doctor()` |

---

## Batching Patterns

### Parallel reads (do this)
Call multiple `ctx_read` in one response:
```
ctx_read("src/main.rs")
ctx_read("src/lib.rs")
ctx_read("Cargo.toml")
```
All three execute in parallel — one round-trip.

### Anti-pattern (avoid)
```
// Turn 1: ctx_read("src/main.rs")
// Turn 2: ctx_read("src/lib.rs")   ← wasted round-trip
// Turn 3: ctx_read("Cargo.toml")   ← wasted round-trip
```

### Parallel search + read (do this)
When you know you'll need a file after searching:
```
ctx_search("error handling")
ctx_read("src/errors.rs")   ← if you already know this file is relevant
```

---

## Path Jail

ctx-lite enforces path restrictions based on configuration. If you get a "path not allowed" error:
1. Run `ctx_doctor()` to see the active path restrictions
2. The restricted paths were configured by the user during setup
3. Do not attempt to work around restrictions — inform the user and ask them to adjust config if needed

---

## Context Budget Awareness

ctx-lite compresses aggressively, but for very large codebases:
- Use `ctx_tree` to identify the 3-5 most relevant files rather than reading 20
- Search before reading — one search query can replace reading 10 files
- Prefer `ctx_shell("git log --oneline -n 10")` over reading CHANGELOG manually

---

## Diagnostics

If any tool returns unexpected results:
```
ctx_doctor()
```
This shows: installed version, active shell profile, path restrictions, MCP server health.
