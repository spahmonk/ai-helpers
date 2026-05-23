# AI Helpers Monorepo README and mem-lite Layout Design

## Problem

The current repository layout is understandable to someone who already knows the release mechanics, but it is confusing to a normal contributor or user:

- the root `README.md` reads like a `ctx-lite` product README rather than a repository README
- `mem-lite` appears to live in two places: `modules/mem-lite` and `packages/mem-lite`
- the npm wrapper for `mem-lite` is technically valid, but its current placement makes the product feel duplicated
- the documentation hierarchy does not clearly distinguish product code, packaging adapters, and repository-level navigation

The goal of this cycle is to make the repository easier to understand without removing the existing npm delivery model for `mem-lite`.

## Current state

### Repository-level entrypoint

The root `README.md` is currently product-first for `ctx-lite`:

- install instructions are for `ctx-lite`
- feature list is for `ctx-lite`
- documentation links are mostly `ctx-lite` links

This makes the repository look like a single-product repo even though it already contains at least:

- `modules/ctx-lite`
- `modules/mem-lite`

### mem-lite structure

`mem-lite` is currently split across:

- `modules/mem-lite` — Rust crate, binary, tests, source of truth for the product
- `packages/mem-lite` — npm wrapper that downloads and launches the released binary
- `scripts/install-mem-lite.sh` and `scripts/install-mem-lite.ps1` — direct installer scripts
- `.github/workflows/mem-lite-release.yml` — release pipeline using `packages/mem-lite`

This split is not intrinsically wrong, but the current physical layout makes `packages/mem-lite` look like a second product home rather than a packaging adapter.

## Design goals

1. Make the root `README.md` a true repository README for `ai-helpers`
2. Make `modules/mem-lite` the only obvious home of the `mem-lite` product
3. Preserve the existing `mem-lite` npm distribution model
4. Keep packaging concerns visibly subordinate to product code
5. Avoid a large repo-wide restructuring in this cycle

## Non-goals

- Do not redesign the Rust structure of `modules/mem-lite`
- Do not change `mem-lite` runtime behavior
- Do not remove npm delivery for `mem-lite`
- Do not normalize `ctx-lite` packaging layout in this same cycle
- Do not rewrite the whole release system

## Chosen approach

### 1. Root README becomes a monorepo README

The root `README.md` should become the repository entrypoint for `ai-helpers`, not the product entrypoint for `ctx-lite`.

It should answer:

- what this repository contains
- which tools are available
- which README to open next depending on the user's goal

The root README should stay concise and act as a hub. It should link users to product-specific documentation rather than trying to fully document each tool inline.

### 2. modules/mem-lite becomes the visible product home

The `mem-lite` product should be understood as living under:

`modules/mem-lite`

That directory should be the human-facing home for:

- product overview
- installation options
- usage
- development notes

### 3. The npm wrapper moves under modules/mem-lite/npm

The current contents of `packages/mem-lite` should move to:

`modules/mem-lite/npm`

Specifically:

- `packages/mem-lite/package.json` -> `modules/mem-lite/npm/package.json`
- `packages/mem-lite/bin/index.js` -> `modules/mem-lite/npm/bin/index.js`
- `packages/mem-lite/bin/download-binary.js` -> `modules/mem-lite/npm/bin/download-binary.js`
- `packages/mem-lite/bin/release-assets.js` -> `modules/mem-lite/npm/bin/release-assets.js`

This preserves the wrapper but makes it physically subordinate to the product instead of looking like a second top-level product location.

### 4. packages/ stops being part of the user mental model

After the move, `packages/mem-lite` should be removed.

If `packages/` becomes empty, it should be removed as well.

The important design rule is:

- users and contributors should think in terms of products under `modules/`
- packaging adapters should be treated as implementation details unless someone is explicitly working on packaging

## Documentation strategy

### Root README

The root README should include:

- short repository description
- short catalog of available tools
- short pointer for each tool
- links to product READMEs

For this cycle, that means at minimum:

- `ctx-lite` -> `modules/ctx-lite/README.md`
- `mem-lite` -> `modules/mem-lite/README.md`

### modules/mem-lite/README.md

This file should become the primary `mem-lite` user-facing document.

It should cover:

- what `mem-lite` is
- install options
- CLI usage
- MCP usage
- where npm install fits
- where direct installer scripts fit

### modules/mem-lite/npm

This directory is a technical packaging layer.

It does not need a full product README.

If a README is needed there at all, it should be short and explicitly describe the directory as the npm publish wrapper for `mem-lite`.

## Release and tooling impact

The following references must move from `packages/mem-lite` to `modules/mem-lite/npm`:

- `.github/workflows/mem-lite-release.yml`
- npm package metadata such as `repository.directory`
- any script, doc, or release reference that points at `packages/mem-lite`

The release model itself does not change:

- Rust binaries are still built from `modules/mem-lite`
- release assets are still published to GitHub Releases
- npm package still acts as a wrapper that downloads the correct released binary

Only the repository location of the wrapper changes.

## Tradeoff

This cycle intentionally creates a temporary asymmetry:

- `mem-lite` packaging moves under `modules/mem-lite/npm`
- `ctx-lite` packaging remains in its current older structure

This is acceptable because it solves the immediate clarity problem for `mem-lite` without expanding the scope into a full repository normalization project.

If the result works well, a later cleanup cycle can apply the same pattern to `ctx-lite`.

## Implementation guidance

The implementation should proceed in this order:

1. move the `mem-lite` npm wrapper under `modules/mem-lite/npm`
2. update workflow and metadata references
3. remove obsolete `packages/mem-lite` path
4. rewrite the root README as a repository hub
5. ensure `modules/mem-lite/README.md` exists and serves as the product entrypoint
6. verify install/release/docs references still resolve correctly

## Acceptance criteria

This design is complete when all of the following are true:

- the root `README.md` reads as a repository README for `ai-helpers`
- `mem-lite` no longer appears to have two separate top-level homes
- the npm wrapper lives under `modules/mem-lite/npm`
- release workflow references the new npm wrapper path
- `modules/mem-lite/README.md` is the obvious primary documentation home for `mem-lite`
- `packages/mem-lite` is gone
- existing `mem-lite` install and release paths remain functional after the move
