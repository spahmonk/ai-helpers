---
name: mem-lite-usage
description: Use this skill when working with mem-lite MCP tools. Provides patterns for storing and retrieving project-scoped memory efficiently. Apply this at the start of any session where project memory is relevant.
---

# mem-lite Usage Optimization

## Overview

mem-lite gives AI agents persistent, project-scoped memory via 6 MCP tools. Memory is stored in a local SQLite DB — one DB per project directory — so nothing leaks between projects.

**Core principle: Remember decisions explicitly. Search before re-deriving. Capture in batches.**

---

## Available MCP Tools

| Tool | Purpose | When to use |
|------|---------|-------------|
| `remember` | Store a single memory entry | After a key decision or event |
| `search` | Search semantic memories by meaning | Before starting work to recall prior context |
| `recent` | List recent entries across all levels | To catch up on what happened in a session |
| `stats` | Count entries per memory level | Quick sanity check |
| `project_info` | Show project_id and DB path | Confirm which project DB is active |
| `capture_batch` | Store multiple entries at once | After a long session; capturing many facts |
| `project_summary` | Offline summary of stored memory | At session start; to orient yourself |

---

## Memory Levels

mem-lite has three memory levels. Choose the right one:

| Level | What goes here | Examples |
|-------|---------------|---------|
| `semantic` | Factual knowledge, decisions, architecture | "We use SQLite for storage", "Auth uses JWT" |
| `episodic` | Events, actions taken, what happened | "Refactored the auth module", "Fixed UNC path bug" |
| `procedural` | How to do things, workflows, instructions | "To release: run cargo build then npm publish" |

---

## Efficient Workflow

### At session start — recall before deriving

Always search memory before spending tokens on re-analysis:

```
search("architecture decisions")
search("known bugs")
project_summary()
```

This costs a few tokens but prevents re-deriving things the project already knows.

### During a session — remember decisions as they happen

After any architectural decision, important finding, or action taken:

```
remember(
  content="Chose rusqlite over sqlx because sqlx requires async runtime",
  level="semantic",
  title="DB driver decision",
  tags=["rust", "database"]
)
```

**Do not batch explicit decisions** — store them immediately so they are available if the session resets.

### At session end — batch-capture episodic events

For a sequence of actions taken during the session, use `capture_batch` instead of N individual `remember` calls:

```
capture_batch([
  {"level":"episodic","title":"Fixed UNC path bug","content":"Replaced to_string_lossy with lossless byte hashing on Windows paths"},
  {"level":"episodic","title":"Added backfill API","content":"Implemented explicit backfill_embeddings() to avoid hidden writes in search()"},
  {"level":"procedural","title":"Run tests","content":"cargo test -p mem-lite runs all 38 unit and integration tests"}
])
```

This is more efficient than N separate `remember` calls.

---

## Project Scoping

**Every MCP call is scoped to a project root.** By default this is the working directory of the MCP server process. You can override it per-call:

```
search(query="authentication", root="/path/to/project-a")
remember(content="...", root="/path/to/project-a")
```

Use `project_info` to confirm which DB is currently active before storing important data.

---

## What NOT to store

- Do NOT store information that belongs in source code or documentation
- Do NOT store temporary debugging output or ephemeral test results
- Do NOT store PII or secrets
- Prefer `semantic` level for facts that should survive long-term; `episodic` for short-term event log

---

## Token efficiency tips

- Prefer `search` over `recent` when you have a specific question — search uses FTS+vector scoring
- Prefer `project_summary` at session start over reading all `recent` entries manually
- Use `stats` to gauge how much memory is stored before deciding to search vs. summarize
- Keep `content` concise — mem-lite is for recall hints, not full documentation
