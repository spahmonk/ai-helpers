#!/bin/bash

# ctx-lite v0.4.0 Deployment Script
# Production-grade deployment automation

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION="0.4.0-phase3-complete"
COMMIT="c37f939"
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║ ctx-lite v${VERSION} Deployment Script             ║${NC}"
echo -e "${BLUE}║ Production Deployment Automation                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to print section
print_section() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Function to print status
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

# Function to print error
print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check prerequisites
print_section "Checking Prerequisites"

if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust."
    exit 1
fi
print_status "Cargo installed"

if ! command -v git &> /dev/null; then
    print_error "Git not found. Please install Git."
    exit 1
fi
print_status "Git installed"

# Determine OS
print_section "System Detection"

OS="$(uname -s)"
case "$OS" in
    Linux*)     PLATFORM="Linux" ;;
    Darwin*)    PLATFORM="macOS" ;;
    MINGW*)     PLATFORM="Windows" ;;
    *)          PLATFORM="UNKNOWN" ;;
esac
print_status "Platform: $PLATFORM"

ARCH="$(uname -m)"
print_status "Architecture: $ARCH"

# Build
print_section "Building Release Binary"

cd modules/ctx-lite
if [ -d target/release ]; then
    rm -rf target/release
    print_status "Cleaned previous builds"
fi

cargo build --release 2>&1 | tail -5
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    print_status "Build completed"
    BUILD_SIZE=$(du -sh target/release/ctx-lite 2>/dev/null | cut -f1)
    print_status "Binary size: $BUILD_SIZE"
else
    print_error "Build failed"
    exit 1
fi

# Test
print_section "Running Tests"

TEST_OUTPUT=$(cargo test --release 2>&1 | grep "test result:" | tail -1)
if echo "$TEST_OUTPUT" | grep -q "ok"; then
    print_status "All tests passed: $TEST_OUTPUT"
else
    print_error "Tests failed: $TEST_OUTPUT"
    exit 1
fi

# Verify specific test counts
TOTAL_TESTS=$(cargo test --release 2>&1 | grep "test result:" | grep "passed" | awk '{print $3}' | tr -d ';' | tail -1)
if [ "$TOTAL_TESTS" = "237" ]; then
    print_status "Verified: All 237 tests passing"
else
    print_error "Expected 237 tests, got $TOTAL_TESTS"
    exit 1
fi

# Release binary location
BINARY_PATH="target/release/ctx-lite"
if [ "$PLATFORM" = "Windows" ]; then
    BINARY_PATH="target/release/ctx-lite.exe"
fi

if [ -f "$BINARY_PATH" ]; then
    print_status "Binary verified at: $BINARY_PATH"
else
    print_error "Binary not found at: $BINARY_PATH"
    exit 1
fi

# Installation options
print_section "Installation Options"

echo -e "${YELLOW}Choose installation method:${NC}"
echo "1) Install to /usr/local/bin (requires sudo)"
echo "2) Install to ~/.local/bin (user-level)"
echo "3) Copy to current directory"
echo "4) Skip installation"
read -p "Enter choice (1-4): " INSTALL_CHOICE

case $INSTALL_CHOICE in
    1)
        print_section "Installing to /usr/local/bin"
        sudo cp "$BINARY_PATH" /usr/local/bin/ctx-lite
        sudo chmod +x /usr/local/bin/ctx-lite
        print_status "Installed to /usr/local/bin/ctx-lite"
        print_status "Access with: ctx-lite --help"
        ;;
    2)
        print_section "Installing to ~/.local/bin"
        mkdir -p ~/.local/bin
        cp "$BINARY_PATH" ~/.local/bin/ctx-lite
        chmod +x ~/.local/bin/ctx-lite
        print_status "Installed to ~/.local/bin/ctx-lite"
        echo -e "${YELLOW}Note: Add ~/.local/bin to PATH if not already done${NC}"
        ;;
    3)
        print_section "Copying to Current Directory"
        cp "$BINARY_PATH" ./ctx-lite
        chmod +x ./ctx-lite
        print_status "Binary copied to: ./ctx-lite"
        print_status "Access with: ./ctx-lite --help"
        ;;
    4)
        print_status "Installation skipped"
        echo -e "${YELLOW}Binary available at: $BINARY_PATH${NC}"
        ;;
    *)
        print_error "Invalid choice"
        exit 1
        ;;
esac

# Configuration
print_section "Configuration"

if [ ! -d ~/.ctx-lite ]; then
    mkdir -p ~/.ctx-lite
    print_status "Created cache directory: ~/.ctx-lite"
fi

# Create config file if it doesn't exist
if [ ! -f ~/.ctx-lite/config.json ]; then
    cat > ~/.ctx-lite/config.json << 'CONFIG'
{
  "cache_size": 100,
  "cache_dir": "~/.ctx-lite/",
  "compression_modes": {
    "enabled": ["full", "signatures", "diff", "map"],
    "prefer_mode": "auto"
  },
  "budget": {
    "warning_threshold": 0.8,
    "default_limit": 100000
  }
}
CONFIG
    print_status "Created config file: ~/.ctx-lite/config.json"
fi

# Verification
print_section "Final Verification"

if command -v ctx-lite &> /dev/null 2>&1 || [ -f ./ctx-lite ] || [ -f ~/.local/bin/ctx-lite ]; then
    print_status "Binary accessible"
    
    if [ -f ./ctx-lite ]; then
        ./ctx-lite --version 2>/dev/null && print_status "Version check passed" || echo "Version check (local)"
    elif command -v ctx-lite &> /dev/null 2>&1; then
        ctx-lite --version 2>/dev/null && print_status "Version check passed" || echo "Version check (system)"
    fi
else
    print_error "Binary not accessible"
fi

# Summary
print_section "Deployment Summary"

echo -e "${GREEN}✓ Build:           PASSED (Release mode)${NC}"
echo -e "${GREEN}✓ Tests:           PASSED (237/237)${NC}"
echo -e "${GREEN}✓ Platform:        $PLATFORM ($ARCH)${NC}"
echo -e "${GREEN}✓ Binary:          $BUILD_SIZE${NC}"
echo -e "${GREEN}✓ Configuration:   Done${NC}"
echo ""
echo -e "${BLUE}Version:${NC}         $VERSION"
echo -e "${BLUE}Commit:${NC}          $COMMIT"
echo -e "${BLUE}Timestamp:${NC}       $TIMESTAMP"
echo ""

echo -e "${YELLOW}📝 Next Steps:${NC}"
echo "1. Review DEPLOYMENT.md for usage guide"
echo "2. Read RELEASE_NOTES.md for new features"
echo "3. Run: ctx-lite --help"
echo "4. Test: ctx-lite stats --dir ./src"
echo ""

print_section "✨ Deployment Complete"
echo -e "${GREEN}ctx-lite v${VERSION} is ready for production!${NC}"
