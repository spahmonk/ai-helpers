# mem-lite

## What it is
`mem-lite` is the module-level home for the mem-lite product: a small, local, project-scoped memory store with CLI and MCP support.
It keeps memory in SQLite and supports semantic, episodic, and procedural entries.

## Install
Choose one install path:

- Shell installer: `scripts/install-mem-lite.sh`
- PowerShell installer: `scripts/install-mem-lite.ps1`
- npm wrapper: `npm install -g @spahmonk/mem-lite` or `npx @spahmonk/mem-lite`

## CLI usage
Common commands:

```bash
mem-lite --help
mem-lite init
mem-lite search "architecture"
```

Other useful commands include `mem-lite remember`, `mem-lite recent`, `mem-lite stats`, `mem-lite project-info`, and `mem-lite project-summary`.

## MCP usage
Start mem-lite as an MCP server with:

```bash
mem-lite --mcp
```

The MCP server uses the same local project store as the CLI.

## Storage model
mem-lite stores data locally in SQLite, scoped to a project root.
The store keeps memory entries and project metadata on disk so searches and recalls stay fast and offline.

Memory types:

- semantic
- episodic
- procedural

## Development

```bash
cargo test -p mem-lite
cargo build -p mem-lite --release
```
