# Release Checklist for ctx-lite

This document provides a repeatable process for releasing new versions of ctx-lite.

## Release Process Overview

- **Branch**: Always release from `feature/ctx-lite-mcp` (or main if production-ready)
- **Versioning**: Follow semantic versioning (v0.1.0, v0.2.0, v1.0.0, etc.)
- **Artifacts**: Binaries for Linux, macOS (x86_64 and ARM64), and Windows
- **Distribution**: GitHub Releases with downloadable binaries

## Pre-Release Verification

### 1. Code Readiness

- [ ] All feature branches merged into release branch
- [ ] Code review completed for all changes
- [ ] No outstanding TODOs or FIXME comments in code
- [ ] Documentation updated (INSTALL.md, README.md, etc.)

```bash
# Review commit log since last release
git log v0.1.0..HEAD --oneline

# Check for TODOs
grep -r "TODO\|FIXME" modules/ctx-lite/src/ --include="*.rs" || echo "No TODOs found"
```

### 2. Testing & Quality Checks

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Documentation tests pass
- [ ] Code compiles with no warnings (Release mode)
- [ ] No security vulnerabilities flagged

```bash
# Run comprehensive test suite
cargo test --all

# Check for warnings in release build
cargo build --release 2>&1 | grep -i warning || echo "No warnings"

# Run doc tests
cargo test --doc

# Check clippy for code quality
cargo clippy --all -- -D warnings
```

### 3. Dependency Audit

- [ ] Dependencies are up to date where appropriate
- [ ] No critical security vulnerabilities in dependencies
- [ ] Cargo.lock is committed and consistent

```bash
# Check for outdated dependencies
cargo outdated

# Audit for security vulnerabilities
cargo audit || true

# Verify Cargo.lock is committed
git status Cargo.lock
```

### 4. Binary Functionality & Smoke Tests

- [ ] Binary builds successfully
- [ ] Binary help output is complete: `--help`
- [ ] Binary version flag works: `--version`
- [ ] Smoke tests pass
- [ ] All CLI commands work correctly

```bash
# Build release binary
cargo build --release

# Test help
./target/release/ctx-lite --help

# Test version
./target/release/ctx-lite --version

# Run comprehensive smoke tests
cargo test --test smoke_tests --quiet

# Use automated verification script
./scripts/verify-release.sh

# Or manually test key commands
./target/release/ctx-lite read Cargo.toml
./target/release/ctx-lite tree .
./target/release/ctx-lite doctor
```

### Smoke Tests

The `tests/smoke_tests.rs` file contains 30+ end-to-end tests that verify:

- **CLI Commands**: help, version, and argument parsing
- **File Operations**: read, tree directory listing
- **Search**: text pattern matching
- **Shell Commands**: execution of whitelisted commands
- **Diagnostics**: doctor command functionality
- **Error Handling**: proper error messages for invalid input
- **Performance**: baseline performance checks
- **Cross-Platform**: path handling on different systems

Run smoke tests with:

```bash
# Run all smoke tests
cargo test --test smoke_tests

# Run specific smoke test
cargo test --test smoke_tests smoke_read_readme

# Run with output
cargo test --test smoke_tests -- --nocapture
```

Expected output: All 30+ tests should pass before release.

## Version Bump

### 1. Update Version in Cargo.toml

Check current version and determine next version:

```bash
# Current version
grep "^version" modules/ctx-lite/Cargo.toml

# Decide on next version based on changes:
# - Patch: 0.1.0 -> 0.1.1 (bug fixes only)
# - Minor: 0.1.0 -> 0.2.0 (new features, backward compatible)
# - Major: 0.1.0 -> 1.0.0 (breaking changes)
```

Update `modules/ctx-lite/Cargo.toml`:

```toml
[package]
name = "ctx-lite"
version = "0.2.0"  # Update version here
```

### 2. Update CHANGELOG.md

Create or update `CHANGELOG.md` at the workspace root:

```markdown
# Changelog

All notable changes to ctx-lite are documented here.

## [0.2.0] - YYYY-MM-DD

### Added
- Feature description

### Changed
- Change description

### Fixed
- Bug fix description

### Breaking Changes
- Breaking change description (if applicable)

## [0.1.0] - YYYY-MM-DD

### Added
- Initial release
```

Verify format with:

```bash
ls -la CHANGELOG.md
head -30 CHANGELOG.md
```

## Commit & Tag

### 1. Commit Changes

```bash
# Verify what changed
git status

# Stage version and changelog
git add modules/ctx-lite/Cargo.toml Cargo.lock CHANGELOG.md

# Commit with descriptive message
git commit -m "chore: release v0.2.0

- Update version in Cargo.toml
- Add CHANGELOG entries for v0.2.0

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"

# Verify commit
git log -1 --stat
```

### 2. Create Annotated Git Tag

```bash
# Create annotated tag (not lightweight)
git tag -a v0.2.0 -m "Release v0.2.0

This release includes:
- Feature 1
- Feature 2
- Bug fix 1

See CHANGELOG.md for details."

# Verify tag
git tag -v v0.2.0
git log --oneline -1 v0.2.0
```

### 3. Push to Repository

```bash
# Push commits and tags
git push origin feature/ctx-lite-mcp
git push origin v0.2.0

# Verify push
git ls-remote origin | grep "v0.2.0"
```

## Build Release Binaries

The GitHub Actions workflow (`.github/workflows/release.yml`) automatically builds binaries when a tag is pushed. Alternatively, build manually:

### Manual Cross-Platform Build

Install cross-compilation tools:

```bash
# Install cross for simplified cross-compilation
cargo install cross

# Or use platform-specific build instructions
```

**Linux x86_64** (native or cross):

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

**macOS x86_64**:

```bash
# On macOS with Apple Silicon, install x86_64 support
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
```

**macOS ARM64**:

```bash
# On Apple Silicon (native) or cross-compile
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

**Windows x86_64**:

```bash
# On Windows with MSVC toolchain
cargo build --release --target x86_64-pc-windows-msvc
```

### Package Binaries

```bash
# Linux
tar czf ctx-lite-0.2.0-x86_64-unknown-linux-gnu.tar.gz \
  -C target/x86_64-unknown-linux-gnu/release ctx-lite

# macOS x86_64
tar czf ctx-lite-0.2.0-x86_64-apple-darwin.tar.gz \
  -C target/x86_64-apple-darwin/release ctx-lite

# macOS ARM64
tar czf ctx-lite-0.2.0-aarch64-apple-darwin.tar.gz \
  -C target/aarch64-apple-darwin/release ctx-lite

# Windows
cd target/x86_64-pc-windows-msvc/release && \
  zip -r ctx-lite-0.2.0-x86_64-pc-windows-msvc.zip ctx-lite.exe && \
  cd -
```

## Create GitHub Release

### 1. Via GitHub Web UI

1. Go to https://github.com/spahmonk/ai-helpers/releases
2. Click "Draft a new release"
3. Select tag: `v0.2.0`
4. Set title: `Release v0.2.0`
5. Add description from CHANGELOG.md
6. Upload binary artifacts (`.tar.gz` and `.zip` files)
7. Check "This is a pre-release" if not production-ready
8. Click "Publish release"

### 2. Via GitHub CLI

```bash
# Create release and attach artifacts
gh release create v0.2.0 \
  --title "Release v0.2.0" \
  --notes-file CHANGELOG.md \
  ctx-lite-0.2.0-x86_64-unknown-linux-gnu.tar.gz \
  ctx-lite-0.2.0-x86_64-apple-darwin.tar.gz \
  ctx-lite-0.2.0-aarch64-apple-darwin.tar.gz \
  ctx-lite-0.2.0-x86_64-pc-windows-msvc.zip

# Verify
gh release view v0.2.0
```

## Post-Release Verification

### 1. Verify Release Assets

- [ ] All platform binaries are attached to the release
- [ ] File checksums are correct
- [ ] Download links work from the GitHub release page

```bash
# Download and verify binary
version="0.2.0"
curl -L "https://github.com/spahmonk/ai-helpers/releases/download/v${version}/ctx-lite-${version}-x86_64-unknown-linux-gnu.tar.gz" \
  -o test-binary.tar.gz

tar tzf test-binary.tar.gz
```

### 2. Test Installation from Release

Follow steps in [INSTALL.md](INSTALL.md) to verify installation works from released binaries.

### 3. Verify Documentation

- [ ] INSTALL.md version references are correct
- [ ] README.md links point to correct release tag
- [ ] Changelog is complete and accurate

```bash
grep -n "0.2.0\|v0.2.0" docs/INSTALL.md README.md CHANGELOG.md
```

## Automated CI/CD Release Workflow

When you push a tag (e.g., `git push origin v0.2.0`), GitHub Actions automatically:

1. Builds binaries for Linux, macOS (x86_64 & ARM64), and Windows
2. Creates release artifacts (`.tar.gz` and `.zip`)
3. Attaches artifacts to GitHub Release

**Status**: Check workflow runs at:
https://github.com/spahmonk/ai-helpers/actions

## Rollback

If a critical issue is discovered after release:

```bash
# Delete tag locally and remotely (DANGER - affects public release)
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0

# Or create a new patch release:
# - Update Cargo.toml to v0.2.1
# - Commit: "chore: release v0.2.1 (patch)"
# - Tag: v0.2.1
# - Push and repeat release steps
```

## Version History

Track all releases in the GitHub releases page:
https://github.com/spahmonk/ai-helpers/releases

---

**Last Updated**: April 2024
**Release Process Version**: 1.0
