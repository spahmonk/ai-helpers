# mem-lite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform `mem-lite` module that provides per-project persistent memory for AI agents via CLI + MCP, with explicit and automatic writes, hybrid retrieval, and agent-facing setup/usage skills.

**Architecture:** `mem-lite` will be a sibling Rust crate to `ctx-lite` under `modules/mem-lite/`, following the same library/bin split. Storage will use one SQLite database per project, with FTS5 tables for lexical lookup, `sqlite-vec` for vector similarity, and a small service layer that powers both CLI commands and MCP tools. Agent integration will mirror the existing `agent/modules/ctx-lite/` contract with a machine-readable manifest and two required skills.

**Tech Stack:** Rust, rusqlite, sqlite-vec, fastembed, serde/serde_json, rmcp, tokio, SQLite FTS5, Node/npm wrapper docs, GitHub Actions, Markdown skills.

---

## File Structure

### New files
- `modules/mem-lite/Cargo.toml` — crate manifest for the new module
- `modules/mem-lite/src/lib.rs` — library entry point
- `modules/mem-lite/src/main.rs` — CLI + MCP binary entry point
- `modules/mem-lite/src/app/mod.rs` — app layer exports
- `modules/mem-lite/src/app/cli.rs` — command parsing and CLI rendering
- `modules/mem-lite/src/app/contracts.rs` — request normalization and service traits
- `modules/mem-lite/src/app/mcp.rs` — MCP adapter for memory tools
- `modules/mem-lite/src/core/mod.rs` — core layer exports
- `modules/mem-lite/src/core/config.rs` — default config and project scope resolution
- `modules/mem-lite/src/core/project.rs` — canonical project identity + fingerprinting
- `modules/mem-lite/src/core/schema.rs` — DB bootstrap and migrations
- `modules/mem-lite/src/core/store.rs` — durable memory CRUD, stats, and recent queries
- `modules/mem-lite/src/core/embed.rs` — embedding provider abstraction and fastembed integration
- `modules/mem-lite/src/core/retrieval.rs` — hybrid retrieval (metadata + FTS + vector)
- `modules/mem-lite/src/core/capture.rs` — auto-capture and explicit write orchestration
- `modules/mem-lite/src/core/summary.rs` — project summary and lightweight consolidation helpers
- `modules/mem-lite/tests/project_scope.rs` — project isolation integration tests
- `modules/mem-lite/tests/store_flow.rs` — CRUD, retrieval, stats tests
- `modules/mem-lite/tests/cli_flow.rs` — CLI smoke/integration tests
- `agent/modules/mem-lite/manifest.json` — agent installation/config metadata
- `agent/modules/mem-lite/skills/mem-lite-setup.md` — rigid setup skill
- `agent/modules/mem-lite/skills/mem-lite-usage.md` — usage skill with orchestration guidance

### Modified files
- `Cargo.toml` — add `modules/mem-lite` to workspace members
- `AGENT.md` — add `mem-lite` to the available modules table and quick reference
- `.github/workflows/release.yml` — extend or factor release packaging for `mem-lite`
- `release-please-config.json` — add `mem-lite` version-tracked files if release automation covers it now

---

### Task 1: Scaffold the workspace and module skeleton

**Files:**
- Create: `modules/mem-lite/Cargo.toml`
- Create: `modules/mem-lite/src/lib.rs`
- Create: `modules/mem-lite/src/main.rs`
- Create: `modules/mem-lite/src/app/mod.rs`
- Create: `modules/mem-lite/src/core/mod.rs`
- Modify: `Cargo.toml`
- Test: `cargo test -p mem-lite --no-run`

- [ ] **Step 1: Write the failing workspace assertion**

```rust
// modules/mem-lite/src/lib.rs
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_exposed() {
        assert!(!version().is_empty());
    }
}
```

- [ ] **Step 2: Run test/build to verify the workspace does not know `mem-lite` yet**

Run: `cargo test -p mem-lite --no-run`  
Expected: FAIL with `package ID specification 'mem-lite' did not match any packages`

- [ ] **Step 3: Add the minimal workspace + crate skeleton**

```toml
# Cargo.toml
[workspace]
members = ["modules/ctx-lite", "modules/mem-lite"]
resolver = "2"
```

```toml
# modules/mem-lite/Cargo.toml
[package]
name = "mem-lite"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "mem_lite"
path = "src/lib.rs"

[[bin]]
name = "mem-lite"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
// modules/mem-lite/src/main.rs
fn main() {
    println!("mem-lite {}", mem_lite::version());
}
```

- [ ] **Step 4: Run the crate test/build to verify the skeleton passes**

Run: `cargo test -p mem-lite --no-run`  
Expected: PASS with one discovered package and compiled test targets

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml modules/mem-lite
git commit -m "feat: scaffold mem-lite crate"
```

---

### Task 2: Implement project scope resolution and per-project DB identity

**Files:**
- Create: `modules/mem-lite/src/core/config.rs`
- Create: `modules/mem-lite/src/core/project.rs`
- Modify: `modules/mem-lite/src/core/mod.rs`
- Modify: `modules/mem-lite/src/lib.rs`
- Test: `modules/mem-lite/tests/project_scope.rs`

- [ ] **Step 1: Write the failing project isolation test**

```rust
#[test]
fn different_workspace_roots_get_different_project_fingerprints() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    let left_scope = ProjectScope::from_workspace_root(left.path()).unwrap();
    let right_scope = ProjectScope::from_workspace_root(right.path()).unwrap();

    assert_ne!(left_scope.project_id, right_scope.project_id);
    assert_ne!(left_scope.database_path, right_scope.database_path);
}
```

- [ ] **Step 2: Run the test to verify `ProjectScope` does not exist yet**

Run: `cargo test -p mem-lite project_scope -- --nocapture`  
Expected: FAIL with unresolved imports / missing type `ProjectScope`

- [ ] **Step 3: Add minimal project identity implementation**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectScope {
    pub workspace_root: PathBuf,
    pub project_id: String,
    pub database_path: PathBuf,
}

impl ProjectScope {
    pub fn from_workspace_root(root: &Path) -> Result<Self, ProjectError> {
        let canonical = root.canonicalize().map_err(ProjectError::io)?;
        let mut hasher = Sha256::new();
        hasher.update(canonical.to_string_lossy().as_bytes());
        let digest = hex::encode(hasher.finalize());
        let project_id = digest[..16].to_string();
        let database_path = default_mem_lite_home()
            .join("projects")
            .join(&project_id)
            .join("memory.sqlite");

        Ok(Self {
            workspace_root: canonical,
            project_id,
            database_path,
        })
    }
}
```

- [ ] **Step 4: Add companion tests for stability and relative-path rejection**

```rust
#[test]
fn same_workspace_root_produces_stable_project_identity() {
    let temp = tempdir().unwrap();
    let first = ProjectScope::from_workspace_root(temp.path()).unwrap();
    let second = ProjectScope::from_workspace_root(temp.path()).unwrap();

    assert_eq!(first.project_id, second.project_id);
    assert_eq!(first.database_path, second.database_path);
}
```

- [ ] **Step 5: Run the scope tests**

Run: `cargo test -p mem-lite --test project_scope`  
Expected: PASS with stable IDs and different DB paths for different workspaces

- [ ] **Step 6: Commit**

```bash
git add modules/mem-lite/src/core modules/mem-lite/tests/project_scope.rs
git commit -m "feat: add mem-lite project scope resolution"
```

---

### Task 3: Add SQLite schema bootstrap and durable store primitives

**Files:**
- Create: `modules/mem-lite/src/core/schema.rs`
- Create: `modules/mem-lite/src/core/store.rs`
- Modify: `modules/mem-lite/src/core/mod.rs`
- Test: `modules/mem-lite/tests/store_flow.rs`

- [ ] **Step 1: Write the failing explicit remember / recent / stats tests**

```rust
#[test]
fn remember_persists_semantic_and_procedural_entries() {
    let fixture = MemoryFixture::new();

    fixture.store().remember(RememberInput {
        level: MemoryLevel::Semantic,
        title: "Path jail rule".into(),
        content: "Absolute paths are allowed when inside project root".into(),
        tags: vec!["security".into()],
        source: MemorySource::Explicit,
    }).unwrap();

    let stats = fixture.store().stats().unwrap();
    assert_eq!(stats.semantic_count, 1);
}
```

- [ ] **Step 2: Run the store tests to verify schema/store do not exist**

Run: `cargo test -p mem-lite --test store_flow`  
Expected: FAIL with missing `MemoryStore`, `RememberInput`, and schema bootstrap

- [ ] **Step 3: Implement schema bootstrap and minimal store API**

```rust
pub fn bootstrap(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS semantic_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            source TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS episodic_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            content TEXT NOT NULL,
            event_kind TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS procedural_memories (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
}
```

```rust
pub struct MemoryStore {
    scope: ProjectScope,
    conn: Connection,
}
```

- [ ] **Step 4: Implement `remember`, `recent`, and `stats` minimally**

```rust
pub fn stats(&self) -> Result<MemoryStats, StoreError> {
    Ok(MemoryStats {
        episodic_count: query_count(&self.conn, "episodic_memories")?,
        semantic_count: query_count(&self.conn, "semantic_memories")?,
        procedural_count: query_count(&self.conn, "procedural_memories")?,
    })
}
```

- [ ] **Step 5: Run the store flow tests**

Run: `cargo test -p mem-lite --test store_flow`  
Expected: PASS for explicit remember, recent ordering, and per-level stats

- [ ] **Step 6: Commit**

```bash
git add modules/mem-lite/src/core/schema.rs modules/mem-lite/src/core/store.rs modules/mem-lite/tests/store_flow.rs
git commit -m "feat: add mem-lite durable store and schema"
```

---

### Task 4: Integrate embeddings and hybrid retrieval

**Files:**
- Create: `modules/mem-lite/src/core/embed.rs`
- Create: `modules/mem-lite/src/core/retrieval.rs`
- Modify: `modules/mem-lite/src/core/schema.rs`
- Modify: `modules/mem-lite/src/core/store.rs`
- Test: `modules/mem-lite/tests/store_flow.rs`

- [ ] **Step 1: Write the failing retrieval ranking test**

```rust
#[test]
fn search_prefers_same_project_fact_and_recent_keyword_match() {
    let fixture = MemoryFixture::new();

    fixture.store().remember(explicit_semantic(
        "Windows root hint",
        "Drive-relative paths like \\.aws should be explained clearly",
    )).unwrap();

    let hits = fixture.store().search(SearchInput {
        query: "drive-relative path on windows".into(),
        limit: 5,
        level: None,
        tags: vec![],
    }).unwrap();

    assert_eq!(hits[0].title.as_deref(), Some("Windows root hint"));
}
```

- [ ] **Step 2: Run the retrieval test to verify `search` is missing or lexical-only**

Run: `cargo test -p mem-lite --test store_flow search_prefers_same_project_fact_and_recent_keyword_match`  
Expected: FAIL due to missing `search` or incorrect empty results

- [ ] **Step 3: Extend schema with FTS + vector storage hooks**

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_fts USING fts5(
    memory_id UNINDEXED,
    title,
    content,
    tags
);
```

```rust
pub trait Embedder: Send + Sync {
    fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError>;
}
```

- [ ] **Step 4: Implement hybrid retrieval with deterministic fallback**

```rust
// score = lexical_score * 1000 + vector_score * 100 + recency_boost
// if embedding provider is unavailable in tests, use lexical-only fallback and keep API stable
```

- [ ] **Step 5: Add a test embedder for deterministic tests and run the suite**

Run: `cargo test -p mem-lite --test store_flow`  
Expected: PASS with stable, test-friendly ranking

- [ ] **Step 6: Commit**

```bash
git add modules/mem-lite/src/core/embed.rs modules/mem-lite/src/core/retrieval.rs modules/mem-lite/src/core/schema.rs modules/mem-lite/src/core/store.rs modules/mem-lite/tests/store_flow.rs
git commit -m "feat: add mem-lite hybrid retrieval"
```

---

### Task 5: Build CLI commands and MCP tool surface

**Files:**
- Create: `modules/mem-lite/src/app/cli.rs`
- Create: `modules/mem-lite/src/app/contracts.rs`
- Create: `modules/mem-lite/src/app/mcp.rs`
- Modify: `modules/mem-lite/src/app/mod.rs`
- Modify: `modules/mem-lite/src/main.rs`
- Test: `modules/mem-lite/tests/cli_flow.rs`

- [ ] **Step 1: Write the failing CLI smoke tests**

```rust
#[test]
fn remember_and_search_round_trip_via_cli() {
    let fixture = CliFixture::new();

    fixture.run_ok(["remember", "--level", "semantic", "--title", "Path jail", "--content", "Keep paths inside the project"]);
    let output = fixture.run_ok(["search", "project paths"]);

    assert!(output.contains("Path jail"));
}
```

- [ ] **Step 2: Run the CLI tests to verify command parsing is missing**

Run: `cargo test -p mem-lite --test cli_flow`  
Expected: FAIL with missing CLI adapter / unsupported commands

- [ ] **Step 3: Implement CLI command parsing with a small surface**

```rust
mem-lite init
mem-lite remember --level semantic --title "..." --content "..."
mem-lite search <query>
mem-lite recent
mem-lite stats
mem-lite project-summary
mem-lite capture-batch <json-file>
mem-lite --mcp
```

- [ ] **Step 4: Implement service traits and MCP tool mapping**

```rust
// tools
memory_search
memory_recent
memory_get
memory_remember
memory_update
memory_forget
memory_project_summary
memory_capture_batch
memory_procedure_save
memory_procedure_list
memory_stats
```

- [ ] **Step 5: Run CLI tests and targeted MCP startup smoke**

Run: `cargo test -p mem-lite --test cli_flow && cargo run -p mem-lite -- --help | head`  
Expected: PASS plus a help screen listing the supported commands

- [ ] **Step 6: Commit**

```bash
git add modules/mem-lite/src/app modules/mem-lite/src/main.rs modules/mem-lite/tests/cli_flow.rs
git commit -m "feat: add mem-lite cli and mcp adapters"
```

---

### Task 6: Implement auto-capture and project summaries

**Files:**
- Create: `modules/mem-lite/src/core/capture.rs`
- Create: `modules/mem-lite/src/core/summary.rs`
- Modify: `modules/mem-lite/src/core/store.rs`
- Modify: `modules/mem-lite/tests/store_flow.rs`

- [ ] **Step 1: Write the failing capture-batch and summary tests**

```rust
#[test]
fn capture_batch_writes_episodic_entries_and_summary_surfaces_top_facts() {
    let fixture = MemoryFixture::new();

    fixture.store().capture_batch(vec![
        CaptureEvent::tool_result("cargo test", "193 passed"),
        CaptureEvent::decision("Use per-project DBs for memory isolation"),
    ]).unwrap();

    let summary = fixture.store().project_summary().unwrap();
    assert!(summary.contains("per-project DBs"));
}
```

- [ ] **Step 2: Run the tests to verify auto-capture/summary are missing**

Run: `cargo test -p mem-lite --test store_flow capture_batch_writes_episodic_entries_and_summary_surfaces_top_facts`  
Expected: FAIL with missing `capture_batch` / `project_summary`

- [ ] **Step 3: Implement capture input normalization and episodic inserts**

```rust
pub enum CaptureEventKind {
    ToolResult,
    Decision,
    Constraint,
    Milestone,
}
```

- [ ] **Step 4: Implement a deterministic summary heuristic**

```rust
// summary ordering:
// 1. latest semantic decisions
// 2. latest procedural entries
// 3. latest episodic milestones
// truncate to a small, agent-friendly byte budget
```

- [ ] **Step 5: Run the store tests**

Run: `cargo test -p mem-lite --test store_flow`  
Expected: PASS for explicit + automatic write flows and project summary behavior

- [ ] **Step 6: Commit**

```bash
git add modules/mem-lite/src/core/capture.rs modules/mem-lite/src/core/summary.rs modules/mem-lite/src/core/store.rs modules/mem-lite/tests/store_flow.rs
git commit -m "feat: add mem-lite capture and project summary"
```

---

### Task 7: Add agent-facing manifest and skills

**Files:**
- Create: `agent/modules/mem-lite/manifest.json`
- Create: `agent/modules/mem-lite/skills/mem-lite-setup.md`
- Create: `agent/modules/mem-lite/skills/mem-lite-usage.md`
- Modify: `AGENT.md`

- [ ] **Step 1: Write the failing agent integration checklist as doc assertions**

```markdown
- [ ] manifest includes install / verify / mcp_config / config_paths / skills
- [ ] setup skill asks for consent before config writes
- [ ] usage skill teaches project-scoped retrieval and memory-first-but-narrow behavior
```

- [ ] **Step 2: Validate the current repo has no `mem-lite` agent module**

Run: `test -f agent/modules/mem-lite/manifest.json`  
Expected: exit code 1 / missing file

- [ ] **Step 3: Create the manifest with project-scoped config placeholders**

```json
{
  "name": "mem-lite",
  "version": "0.1.0",
  "verify": "mem-lite --version",
  "mcp_config": {
    "copilot_cli": {
      "command": "mem-lite",
      "args": ["--mcp", "--workspace-root", "{{workspace_root}}"]
    }
  }
}
```

- [ ] **Step 4: Write the setup and usage skills**

```markdown
# mem-lite Setup
1. Detect OS and agent platform
2. Verify whether `mem-lite` is installed
3. Derive workspace root and show it to the user
4. Ask for consent before writing MCP config
5. Register `mem-lite-usage`
```

```markdown
# mem-lite Usage
- query current project memory first
- keep retrieval narrow
- capture only durable facts
- batch related writes
- consolidate at task boundaries
```

- [ ] **Step 5: Update `AGENT.md` and validate links**

Run: `rg "mem-lite" AGENT.md agent/modules/mem-lite -n`  
Expected: PASS with manifest + both skill files referenced

- [ ] **Step 6: Commit**

```bash
git add AGENT.md agent/modules/mem-lite
git commit -m "feat: add mem-lite agent integration docs"
```

---

### Task 8: Wire packaging/tests and verify end-to-end quality

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `release-please-config.json`
- Modify: `package.json` or add a dedicated npm package if release layout requires it
- Test: repo-wide verification commands

- [ ] **Step 1: Write the failing packaging gap checklist**

```markdown
- [ ] mem-lite binary is built in CI
- [ ] release pipeline knows how to archive/publish mem-lite
- [ ] docs/install metadata stays version-synced
```

- [ ] **Step 2: Verify the current release pipeline only knows `ctx-lite`**

Run: `rg "ctx-lite|mem-lite" .github/workflows/release.yml package.json release-please-config.json -n`  
Expected: `ctx-lite` only, no `mem-lite` packaging

- [ ] **Step 3: Implement the smallest production-safe packaging change**

```yaml
# either:
# A. add mem-lite to the existing release matrix and publish path
# or
# B. create a dedicated release workflow / package metadata for mem-lite
```

- [ ] **Step 4: Run verification commands**

Run:

```bash
cargo fmt --check
cargo test -p mem-lite
cargo test -p ctx-lite
npm test
```

Expected: PASS across the new module and no regressions in existing packaging/tests

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release.yml release-please-config.json package.json
git commit -m "feat: wire mem-lite packaging and verification"
```

---

## Self-Review

### Spec coverage
- Per-project isolation: Task 2
- 4 memory levels and durable storage: Task 3 + Task 6
- hybrid retrieval: Task 4
- CLI + MCP: Task 5
- auto + explicit writes: Task 3 + Task 6
- AI-facing manifest/skills: Task 7
- packaging + cross-platform readiness: Task 8

### Placeholder scan
- No `TODO`/`TBD` placeholders remain.
- Every task names exact files and at least one concrete test/command.
- The only deliberate implementation choice left open is the exact release wiring shape in Task 8, because that depends on the existing pipeline layout discovered during execution; the step still constrains the acceptable outcome to “smallest production-safe packaging change”.

### Type consistency
- Project identity is always `ProjectScope`.
- Durable store entry path uses `MemoryStore`.
- Retrieval API uses `SearchInput`.
- Auto-capture API uses `CaptureEvent`.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-10-mem-lite-implementation.md`.

The user already requested orchestrated execution with delegated subtasks, so proceed with **Subagent-Driven (recommended)** using `superpowers:subagent-driven-development`, with TDD discipline per task and review between batches.
