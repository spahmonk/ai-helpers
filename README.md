# ctx-lite 🚀

**Fast context extractor for AI coding with cross-platform support**

> Extract code context with **87% compression** for Claude, Copilot, and other AI models. Perfect for MCP integration.

---

## ⚡ Quick Install (30 seconds)

### Linux / macOS
```bash
curl -fsSL https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.sh | bash
```

### Windows (PowerShell)
```powershell
powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))"
```

### npm (Node.js 18+)
```bash
npm install -g @spahmonk/ctx-lite
```

### Verify
```bash
ctx-lite --version  # Should print: ctx-lite 0.1.0
```

**👉 [Full installation guide →](QUICK_START.md)**

---

## 💡 Usage Examples

```bash
# Read and compress a file
ctx-lite read src/main.rs

# Show directory tree with compression info
ctx-lite tree ./src

# Search for text or regex patterns
ctx-lite search "function_name"

# Run diagnostics
ctx-lite doctor

# Configure MCP with explicit shell policy
ctx-lite setup-mcp --shell-profile balanced --deny-capability docker.compose.logs
```

---

## 📦 Features

- ✅ **87% Compression** - High-efficiency context reduction
- ✅ **Cross-Platform** - Linux, macOS, Windows
- ✅ **MCP Compatible** - Use as Model Context Protocol server
- ✅ **Capability Policy** - `safe`, `balanced`, `dangerous` profiles + per-capability overrides
- ✅ **Fast** - <1s per session
- ✅ **Secure** - Path jail, audit logging
- ✅ **ML Optimized** - Adaptive compression modes

---

## 🛡️ Shell Capability Policy

ctx-lite keeps the raw allowlist as the execution boundary, but exposes a higher-level capability model for MCP and CLI configuration.

### Profiles

- **safe** - inspect/log/test workflows only
- **balanced** - `safe` plus build/lint/typecheck workflows
- **dangerous** - side-effectful local state/runtime changes such as `docker run`, `npm install`, `cargo run`

### Process-level args

```bash
ctx-lite --mcp \
  --shell-profile balanced \
  --allow-capability docker.logs,cargo.test \
  --deny-capability docker.compose.logs \
  --allow-command "git show --stat"
```

### Common capability IDs

| Capability | Commands enabled | Profile |
| --- | --- | --- |
| `git.inspect` | `git rev-parse --show-toplevel`, `git status --short`, `git diff --stat`, `git log --oneline -n 20` | safe |
| `docker.logs` | `docker logs ...` | safe |
| `docker.compose.logs` | `docker compose logs ...` | safe |
| `npm.test` | `npm test` | safe |
| `cargo.test` | `cargo test ...` | safe |
| `npm.build` | `npm run build` | balanced |
| `cargo.build` | `cargo build` | balanced |
| `cargo.clippy` | `cargo clippy --all-targets --all-features` | balanced |
| `docker.run` | `docker run ...` | dangerous |
| `npm.install` | `npm install ...` | dangerous |
| `cargo.run` | `cargo run ...` | dangerous |

Use `ctx-lite doctor` to inspect the effective policy seen by the runtime.

---

## 📚 Documentation

- **[Quick Start](QUICK_START.md)** - Get started in 3 minutes
- **[MCP Integration](MCP_INTEGRATION.md)** - Profiles, capability IDs, setup examples, and exact shell support
- **[Module Details](modules/ctx-lite/README.md)** - ctx-lite specifics

---

## 🏗️ Development

### Build from source
```bash
git clone https://github.com/spahmonk/ai-helpers.git
cd ai-helpers
cargo build --release
```

### Run tests
```bash
cargo test --workspace
```

### Code quality
```bash
cargo fmt --all --check
```

---

## 📊 Benchmarks

| Metric | Result |
|--------|--------|
| Session Compression | 87% |
| Token Reduction | 66-75% |
| Processing Time | <1s per session |
| Cache Hit Rate | 94%+ |

---

## 🔐 Security

- Path jail prevents directory traversal
- Git config injection prevention
- Audit log with redaction
- SHA256 content hashing

---

## 📦 Release Information

- **Version**: 0.1.0
- **Status**: Production Ready
- **Tests**: 258/258 passing ✓
- **Platforms**: Linux, macOS, Windows
- **License**: MIT (see LICENSE)

**[All releases →](https://github.com/spahmonk/ai-helpers/releases)**

---

## 🤝 Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests first (TDD)
4. Ensure all tests pass: `cargo test --workspace`
5. Submit a pull request

---

## ❓ Support

- **Issues**: [GitHub Issues](https://github.com/spahmonk/ai-helpers/issues)
- **Discussions**: [GitHub Discussions](https://github.com/spahmonk/ai-helpers/discussions)
- **Documentation**: [Wiki](https://github.com/spahmonk/ai-helpers/wiki)

---

## 📄 License

MIT License - see [LICENSE](LICENSE) file

---

**Made with ❤️ for AI-assisted development**
