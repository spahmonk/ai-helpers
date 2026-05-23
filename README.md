# ai-helpers

`ai-helpers` is a repository of local-first tools for AI-assisted development.
The root README is a hub: use it to find the right module, then jump into that module's README for installation, usage, and release details.

## Tool catalog

| Tool | What it does | Docs |
| --- | --- | --- |
| `ctx-lite` | Fast context extraction and compression for AI coding workflows and MCP setups. | [modules/ctx-lite/README.md](modules/ctx-lite/README.md) |
| `mem-lite` | Local, project-scoped memory storage and recall with CLI and MCP support. | [modules/mem-lite/README.md](modules/mem-lite/README.md) |

## Where to go next

- **ctx-lite** — If you want code-context extraction, shell-policy-aware MCP setup, or the CLI install paths, start with [modules/ctx-lite/README.md](modules/ctx-lite/README.md).
- **mem-lite** — If you want local project memory, search/recall workflows, or MCP usage, start with [modules/mem-lite/README.md](modules/mem-lite/README.md).

## Repository notes

- The workspace includes `ctx-lite` and `mem-lite` under `modules/`.
- For the AI-agent entrypoint, see [AGENT.md](AGENT.md).
- Module-local READMEs are the canonical user-facing docs for each tool.
- Source build and test from the repository root:

```bash
cargo build --workspace --release
cargo test --workspace
```

## Releases and support

- Releases: <https://github.com/spahmonk/ai-helpers/releases>
- Issues: <https://github.com/spahmonk/ai-helpers/issues>
- License: MIT
