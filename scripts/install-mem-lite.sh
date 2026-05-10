#!/bin/bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO="spahmonk/ai-helpers"

# Auto-detect latest mem-lite release version (override with MEM_LITE_VERSION env var)
detect_version() {
    local v="${MEM_LITE_VERSION:-}"
    # Strip leading 'mem-lite-v' or 'v' if user supplied it
    v="${v#mem-lite-v}"
    v="${v#v}"
    if [ -n "$v" ]; then
        echo "$v"
        return
    fi
    # Fetch latest mem-lite release tag
    v=$(curl -fsSL "https://api.github.com/repos/$REPO/releases" 2>/dev/null \
        | grep -o '"tag_name": *"mem-lite-v[^"]*"' | head -1 | sed 's/.*"mem-lite-v\([^"]*\)".*/\1/')
    if [ -z "$v" ]; then
        echo -e "${RED}✗ Could not detect latest mem-lite version. Set MEM_LITE_VERSION to override.${NC}" >&2
        exit 1
    fi
    echo "$v"
}

# Install directory: env override → default /usr/local/bin
# NOTE: When using pipe syntax (curl | bash), env vars must be passed to bash, not curl:
#   curl -fsSL .../install-mem-lite.sh | MEM_LITE_INSTALL_DIR=$HOME/.local/bin bash
INSTALL_DIR="${MEM_LITE_INSTALL_DIR:-/usr/local/bin}"

detect_platform() {
    local os kernel
    os=$(uname -s)
    kernel=$(uname -m)
    case "$os" in
        Linux)
            case "$kernel" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *) echo -e "${RED}✗ Unsupported Linux architecture: $kernel${NC}" >&2; exit 1 ;;
            esac ;;
        Darwin)
            case "$kernel" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64)  echo "aarch64-apple-darwin" ;;
                *) echo -e "${RED}✗ Unsupported macOS architecture: $kernel${NC}" >&2; exit 1 ;;
            esac ;;
        *)
            echo -e "${RED}✗ Unsupported OS: $os${NC}" >&2
            exit 1 ;;
    esac
}

main() {
    VERSION=$(detect_version)
    echo -e "${YELLOW}mem-lite installer v$VERSION${NC}"
    echo ""

    PLATFORM=$(detect_platform)
    echo -e "Platform: ${GREEN}$PLATFORM${NC}"

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/mem-lite-v$VERSION/mem-lite-$VERSION-$PLATFORM.tar.gz"
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT

    # Download
    echo -e "${YELLOW}Downloading mem-lite $VERSION...${NC}"
    echo -e "  $DOWNLOAD_URL"
    if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/mem-lite.tar.gz"; then
        echo -e "${RED}✗ Failed to download from: $DOWNLOAD_URL${NC}"
        echo "Make sure version $VERSION exists on GitHub."
        exit 1
    fi
    echo -e "${GREEN}✓ Downloaded${NC}"

    # Extract
    echo -e "${YELLOW}Extracting...${NC}"
    tar -xzf "$TEMP_DIR/mem-lite.tar.gz" -C "$TEMP_DIR"

    # Find binary
    BINARY_PATH=""
    if [ -f "$TEMP_DIR/mem-lite" ]; then
        BINARY_PATH="$TEMP_DIR/mem-lite"
    elif [ -f "$TEMP_DIR/bin/mem-lite" ]; then
        BINARY_PATH="$TEMP_DIR/bin/mem-lite"
    else
        echo -e "${RED}✗ Binary not found in downloaded archive${NC}"
        exit 1
    fi

    # Install (use sudo only when required)
    if [ ! -w "$(dirname "$INSTALL_DIR")" ] && [ ! -d "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}Installing to $INSTALL_DIR (requires sudo)...${NC}"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "$BINARY_PATH" "$INSTALL_DIR/mem-lite"
        sudo chmod +x "$INSTALL_DIR/mem-lite"
    elif [ ! -w "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}Installing to $INSTALL_DIR (requires sudo)...${NC}"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "$BINARY_PATH" "$INSTALL_DIR/mem-lite"
        sudo chmod +x "$INSTALL_DIR/mem-lite"
    else
        mkdir -p "$INSTALL_DIR"
        cp "$BINARY_PATH" "$INSTALL_DIR/mem-lite"
        chmod +x "$INSTALL_DIR/mem-lite"
    fi
    echo -e "${GREEN}✓ Installed${NC}"

    # Verify
    echo -e "${YELLOW}Verifying...${NC}"
    if VERSION_OUTPUT=$("$INSTALL_DIR/mem-lite" --version 2>/dev/null); then
        echo -e "${GREEN}✓ Successfully installed!${NC}"
        echo -e "  Location : $INSTALL_DIR/mem-lite"
        echo -e "  Version  : $VERSION_OUTPUT"
        echo ""
        if ! command -v mem-lite &>/dev/null; then
            echo -e "${YELLOW}Note: $INSTALL_DIR is not in your PATH.${NC}"
            echo -e "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo -e "  export PATH=\"\$PATH:$INSTALL_DIR\""
        fi
        echo -e "${GREEN}Try it out:${NC}"
        echo -e "  ${YELLOW}mem-lite --help${NC}"
        echo -e "  ${YELLOW}mem-lite init${NC}"
        echo -e "  ${YELLOW}mem-lite --mcp   # start MCP server${NC}"
    else
        echo -e "${RED}✗ Verification failed — binary did not run${NC}"
        exit 1
    fi
}

main "$@"
