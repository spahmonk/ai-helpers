# Capability Policy Design for ctx-lite

## Problem

`ctx-lite` currently exposes shell access through a low-level allowlist of command patterns. That works for implementation, but it is awkward as a user-facing policy model:

- users have to think in command strings instead of capabilities;
- MCP clients need a convenient way to narrow or expand shell behavior per server instance;
- documentation, MCP instructions, setup flows, and actual behavior can drift apart;
- there is no clean way to express "safe by default, but broader when explicitly trusted."

The goal is to introduce a capability-policy layer that remains security-conscious, is convenient to configure, and stays consistent across CLI, MCP setup, docs, skills, and runtime instructions.

## Goals

1. Provide a user-facing configuration model based on named capabilities instead of raw shell strings.
2. Support both persistent policy configuration and per-instance MCP overrides.
3. Keep a clear distinction between safe, balanced, and dangerous shell behavior.
4. Ensure runtime instructions, documentation, setup flows, and diagnostics describe the effective policy accurately.

## Non-Goals

1. Replace the low-level allowlist engine; raw allowlist patterns remain the execution boundary.
2. Auto-detect arbitrary commands from project files.
3. Introduce unconstrained shell execution as the default.

## Recommended Approach

Use a hybrid model:

- **config file** for the base policy;
- **CLI/MCP args** for per-server overrides;
- **setup-mcp** as the ergonomic front door that writes those args for supported clients.

This keeps the core model stable while making MCP usage practical.

## Alternatives Considered

### 1. CLI/MCP args only

Pros:
- easiest to implement initially;
- no new persisted config format.

Cons:
- poor UX for real users;
- verbose MCP configs;
- hard to inspect and document.

### 2. Config file only

Pros:
- clean mental model;
- central source of truth.

Cons:
- weak for multiple MCP clients with different trust levels;
- awkward for quick overrides.

### 3. Hybrid model (recommended)

Pros:
- supports both stable defaults and per-client trust tuning;
- works naturally with MCP configs;
- scales to setup wizard, docs, and diagnostics.

Cons:
- slightly more surface area to document.

## High-Level Model

The user configures **capability IDs**, not raw commands.

Examples:

- `git.inspect`
- `docker.inspect`
- `docker.logs`
- `docker.compose.ps`
- `docker.compose.logs`
- `npm.test`
- `npm.build`
- `npm.lint`
- `npm.typecheck`
- `cargo.test`
- `cargo.build`
- `cargo.check`
- `cargo.fmt.check`
- `cargo.clippy`
- `python.pytest`
- `python3.pytest`
- `ruby.version`
- `ruby.rspec`

Internally, each capability maps to one or more existing allowlist patterns such as:

- `docker logs ...`
- `cargo test ...`
- `python -m pytest ...`

The low-level allowlist remains authoritative for execution. Capability IDs are the stable user-facing layer.

## Policy Layers

Policy is resolved in this order:

1. built-in defaults from the selected profile;
2. config file allow/deny capability changes;
3. CLI or MCP arg overrides;
4. optional raw custom allowlist entries.

Later layers override earlier ones.

## Profiles

### `safe`

Read/inspect/log/test behavior only.

Examples:
- git inspection;
- docker inspection/logs/compose ps;
- test runners;
- diagnostics.

### `balanced`

`safe` plus local build/lint/typecheck workflows.

Examples:
- `npm run build`
- `cargo build`
- `cargo check`
- `cargo clippy --all-targets --all-features`

### `dangerous`

Includes side-effectful operations that materially change local state or runtime behavior.

Examples:
- `docker run`
- `docker build`
- `docker compose up`
- `docker exec`
- `npm install`
- `cargo run`

`dangerous` must never be the implicit default.

## Config File

Recommended shape:

```json
{
  "shell": {
    "enabled": true,
    "profile": "balanced",
    "allowCapabilities": [
      "docker.logs",
      "cargo.test",
      "python.pytest"
    ],
    "denyCapabilities": [
      "docker.compose.logs"
    ],
    "customAllowlist": [
      "docker compose logs ..."
    ]
  }
}
```

### Semantics

- `profile` selects the base built-in set.
- `allowCapabilities` adds named capabilities on top of that base.
- `denyCapabilities` removes named capabilities from the effective set.
- `customAllowlist` is the escape hatch for advanced users who need raw patterns.

## CLI / MCP Arguments

Recommended args:

- `--shell-profile <safe|balanced|dangerous>`
- `--allow-capability <csv>`
- `--deny-capability <csv>`
- `--allow-command <pattern>` (advanced escape hatch)

Example:

```bash
ctx-lite --mcp \
  --shell-profile balanced \
  --allow-capability docker.logs,cargo.test \
  --deny-capability docker.compose.logs
```

These args should apply only to the current process and override the config file.

## setup-mcp Behavior

`setup-mcp` should become the easiest way to provision policy for supported MCP clients.

Recommended flow:

1. choose profile: `safe`, `balanced`, or `dangerous`;
2. optionally toggle specific capability IDs;
3. write the resulting args into the generated MCP config;
4. preserve existing user config entries that are unrelated to `ctx-lite`.

This avoids inventing per-client bespoke config formats and keeps behavior explicit at the launch boundary.

## Runtime Instructions

MCP instructions must describe the **effective** active capabilities, not a generic superset.

Examples:

- if `docker.logs` is disabled, instructions must not recommend `docker logs`;
- if `dangerous` is not enabled, instructions must not suggest `docker run`, `npm install`, or equivalent operations;
- instructions should encourage `search/read/tree` first, then shell only when needed.

## Doctor Output

`doctor` should report:

- whether shell is enabled;
- active profile;
- effective enabled capability IDs;
- effective denied capability IDs;
- custom raw allowlist patterns.

This gives users a single place to verify what the agent can actually execute.

## Documentation Changes Required

### README / MCP docs

Add a capability matrix:

| Capability | Commands enabled | Profile |
| --- | --- | --- |
| `docker.logs` | `docker logs ...`, `docker compose logs ...` | safe |
| `cargo.test` | `cargo test ...` | safe |
| `npm.build` | `npm run build` | balanced |

The docs must avoid vague phrasing like "supports npm/cargo/python/etc" without stating the exact enabled shapes.

### setup docs

Explain:

- how profiles work;
- how to disable one capability from a broader profile;
- how to add a custom raw pattern;
- why `dangerous` is opt-in.

### Skills and instructions

The guidance used by agents should reflect the capability model directly:

- prefer read/search/tree first;
- use shell only within active capabilities;
- side-effectful capabilities are allowed only when the effective policy includes them.

## Error Handling

When a command is denied, errors should explain the reason in capability terms when possible.

Examples:

- "command is disabled by capability policy: docker.compose.logs"
- "command requires dangerous shell profile"

If the denial comes from a raw pattern mismatch, include the existing whitelist-style reason as a fallback.

## Testing Strategy

1. unit tests for capability-to-allowlist mapping;
2. unit tests for profile composition;
3. unit tests for allow/deny override resolution;
4. tests for CLI arg parsing precedence over config;
5. tests for `setup-mcp` config generation with profile/capability args;
6. tests ensuring runtime instructions reflect effective capabilities;
7. docs examples validated against the actual supported policy.

## Rollout Strategy

1. introduce capability IDs and profile resolver under the current raw allowlist engine;
2. wire CLI/MCP args and config loading;
3. update `setup-mcp`;
4. update `doctor`;
5. update README, MCP docs, and agent instructions together.

## Why This Design

This design keeps the current security boundary intact while giving users a much better way to control agent autonomy.

It is:

- easier to understand than raw shell strings;
- safer than a single broad "enable shell" switch;
- more practical than per-client manual allowlists;
- flexible enough for both conservative and power-user workflows.
