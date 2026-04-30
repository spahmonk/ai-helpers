# Changelog

All notable changes to ctx-lite are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-04-30

### Added

- **Initial Release**: Full-featured context extractor for large codebases
- **Security-First Design**: 
  - Path jail mechanism to prevent directory traversal attacks
  - Symlink attack prevention during file traversal
  - Runtime root escape detection and prevention
- **High-Performance File Reading**:
  - Byte-limited reading with UTF-8 boundary protection
  - Prevents invalid UTF-8 sequences in output
  - Configurable byte budget for extraction limits
- **Tree Building & Traversal**:
  - Efficient directory traversal with hard link detection
  - Configurable entry limits and byte budgets
  - Automatic response byte capping to respect budgets
- **MCP (Model Context Protocol) Adapter**:
  - Full MCP v0.1.0 integration for AI model interaction
  - Standardized resource and tool interfaces
  - Comprehensive error handling with MCP-compliant error codes
- **Command-Line Interface**:
  - Extract context from any directory
  - Configurable extraction options
  - Help and version information
- **Comprehensive Test Suite**:
  - 12 unit tests covering security boundaries
  - Path jail security tests
  - UTF-8 boundary tests
  - Symlink attack prevention tests
  - 1 doctest for API documentation
  - 100% critical path coverage
- **Documentation**:
  - Installation guide for Linux, macOS, Windows
  - Release checklist and procedures
  - This changelog

### Security

- Path traversal attack mitigation
- Symlink attack prevention
- Runtime root escape detection
- UTF-8 validation to prevent encoding attacks
- Byte budget enforcement to prevent DoS

### Known Limitations

- Package managers (apt, brew, etc.) not yet supported - defer to later release
- Binary signing and notarization not yet implemented - defer to later release
- Docker/container support not yet implemented - defer to later release

---

**Current Version**: 0.1.0
**Release Date**: 2024-04-30
**Status**: Stable Release
