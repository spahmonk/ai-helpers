# Installation Guide for ctx-lite

ctx-lite is a high-performance context extractor for large codebases with security boundaries. This guide covers installation on Linux, macOS, and Windows.

## Prerequisites

### From Source
- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Cargo**: Bundled with Rust
- **Git**: For cloning the repository

### Pre-built Binaries
- No additional dependencies required
- Glibc 2.31+ on Linux (most distributions from 2020+)

## Installation Methods

### Option 1: Pre-built Binaries (Recommended)

Pre-built binaries are available for:
- **Linux** (x86_64): `ctx-lite-0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- **macOS** (x86_64): `ctx-lite-0.1.0-x86_64-apple-darwin.tar.gz`
- **macOS** (ARM64/M1+): `ctx-lite-0.1.0-aarch64-apple-darwin.tar.gz`
- **Windows** (x86_64): `ctx-lite-0.1.0-x86_64-pc-windows-msvc.zip`

#### Linux & macOS

```bash
# Download the appropriate release
version="0.1.0"
platform="x86_64-unknown-linux-gnu"  # or x86_64-apple-darwin, aarch64-apple-darwin

wget "https://github.com/spahmonk/ai-helpers/releases/download/v${version}/ctx-lite-${version}-${platform}.tar.gz"

# Extract
tar xzf "ctx-lite-${version}-${platform}.tar.gz"

# Install globally (optional)
sudo mv ctx-lite /usr/local/bin/
sudo chmod +x /usr/local/bin/ctx-lite

# Verify
ctx-lite --help
```

#### Windows

```powershell
# Download from releases page (or use PowerShell)
$version = "0.1.0"
$url = "https://github.com/spahmonk/ai-helpers/releases/download/v${version}/ctx-lite-${version}-x86_64-pc-windows-msvc.zip"
Invoke-WebRequest -Uri $url -OutFile ctx-lite.zip

# Extract
Expand-Archive -Path ctx-lite.zip -DestinationPath .

# Install globally (optional, requires admin PowerShell)
Move-Item -Path ctx-lite.exe -Destination $env:ProgramFiles\ctx-lite\
$env:Path += ";$env:ProgramFiles\ctx-lite\"

# Verify
ctx-lite --help
```

### Option 2: Build from Source

Clone and build ctx-lite from the source repository:

#### All Platforms

```bash
git clone https://github.com/spahmonk/ai-helpers.git
cd ai-helpers/.worktrees/ctx-lite-mcp

# Build in release mode
cargo build --release

# Binary location: target/release/ctx-lite (Linux/macOS) or target/release/ctx-lite.exe (Windows)

# Install to PATH
cargo install --path modules/ctx-lite

# Or copy manually
cp target/release/ctx-lite /usr/local/bin/  # Linux/macOS
# Or move to Program Files on Windows

# Verify
ctx-lite --help
```

#### Linux Specific

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt-get install -y build-essential

# Build
cargo build --release

# Install
sudo cp target/release/ctx-lite /usr/local/bin/
sudo chmod +x /usr/local/bin/ctx-lite
```

#### macOS Specific

```bash
# Install Xcode Command Line Tools (if not already installed)
xcode-select --install

# Build
cargo build --release

# Install
cp target/release/ctx-lite /usr/local/bin/
```

#### Windows Specific

```powershell
# Install Visual Studio Build Tools for C++ (if not already installed)
# Or use: https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Build (in PowerShell or cmd)
cargo build --release

# Install
Copy-Item -Path target/release/ctx-lite.exe -Destination $env:ProgramFiles\ctx-lite\

# Add to PATH environment variable if desired
```

## Verifying Installation

After installation, verify ctx-lite is working:

```bash
# Display version and help
ctx-lite --help

# Check version
ctx-lite --version
```

## Configuration

### Environment Variables

- `CTX_CACHE_DIR`: Override default cache directory (optional)
- `CTX_VERBOSE`: Enable verbose logging (optional)

### Environment Setup

#### Linux/macOS

If not installed globally, add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
export PATH="$PATH:/path/to/ctx-lite/directory"
```

#### Windows

Add `ctx-lite` installation directory to system PATH via Environment Variables settings, or use in PowerShell:

```powershell
$env:Path += ";C:\path\to\ctx-lite"
```

## Upgrading

### From Pre-built Binary

Download the new version and follow the same extraction steps, overwriting the previous binary.

### From Source

```bash
cd ai-helpers/.worktrees/ctx-lite-mcp
git pull origin feature/ctx-lite-mcp
cargo install --path modules/ctx-lite --force
```

## Troubleshooting

### "ctx-lite: command not found"

- **Linux/macOS**: Verify installation path is in `$PATH`: `echo $PATH`
- **Windows**: Verify installation directory is in system PATH and restart terminal after changes
- Run full path: `/usr/local/bin/ctx-lite --help` (Linux/macOS) or `C:\Program Files\ctx-lite\ctx-lite.exe --help` (Windows)

### Permission Denied on Linux/macOS

```bash
chmod +x /usr/local/bin/ctx-lite
```

### Windows: "Cannot be loaded because running scripts is disabled"

Run PowerShell as Administrator and execute:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Build Failures

- Update Rust: `rustup update`
- Clean build directory: `cargo clean && cargo build --release`
- Check Rust version: `rustc --version` (should be 1.70+)
- Verify dependencies: `cargo tree`

## Next Steps

- Read the [README](../README.md) for usage examples
- Check [ARCHITECTURE.md](../docs/implementation-orchestration.md) for design details
- Review source code: `modules/ctx-lite/src/`

## Support

Report issues at: https://github.com/spahmonk/ai-helpers/issues

---

**Last Updated**: April 2024
**Version**: 0.1.0
