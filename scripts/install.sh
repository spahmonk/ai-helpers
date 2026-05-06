#!/bin/bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO="spahmonk/ai-helpers"

# Auto-detect latest release version (override with CTX_LITE_VERSION env var)
detect_version() {
    local v="${CTX_LITE_VERSION:-}"
    # Strip leading 'v' if user supplied it (e.g. v1.0.6 → 1.0.6)
    v="${v#v}"
    if [ -n "$v" ]; then
        echo "$v"
        return
    fi
    v=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | grep -o '"tag_name": *"v[^"]*"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')
    if [ -z "$v" ]; then
        echo -e "${RED}✗ Could not detect latest version. Set CTX_LITE_VERSION to override.${NC}" >&2
        exit 1
    fi
    echo "$v"
}

# Install directory: env override → default /usr/local/bin
# NOTE: When using pipe syntax (curl | bash), env vars must be passed to bash, not curl:
#   curl -fsSL .../install.sh | CTX_LITE_INSTALL_DIR=$HOME/.local/bin bash
# Or export first:
#   export CTX_LITE_INSTALL_DIR=$HOME/.local/bin && curl -fsSL .../install.sh | bash
INSTALL_DIR="${CTX_LITE_INSTALL_DIR:-/usr/local/bin}"

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
    echo -e "${YELLOW}ctx-lite installer v$VERSION${NC}"
    echo ""

    PLATFORM=$(detect_platform)
    echo -e "Platform: ${GREEN}$PLATFORM${NC}"

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/v$VERSION/ctx-lite-$VERSION-$PLATFORM.tar.gz"
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT

    # Download
    echo -e "${YELLOW}Downloading ctx-lite $VERSION...${NC}"
    echo -e "  $DOWNLOAD_URL" 
    if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/ctx-lite.tar.gz"; then
        echo -e "${RED}✗ Failed to download from: $DOWNLOAD_URL${NC}"
        echo "Make sure version $VERSION exists on GitHub."
        exit 1
    fi
    echo -e "${GREEN}✓ Downloaded${NC}"

    # Extract
    echo -e "${YELLOW}Extracting...${NC}"
    tar -xzf "$TEMP_DIR/ctx-lite.tar.gz" -C "$TEMP_DIR"

    # Find binary (archive always has ctx-lite at root, check bin/ as fallback)
    BINARY_PATH=""
    if [ -f "$TEMP_DIR/ctx-lite" ]; then
        BINARY_PATH="$TEMP_DIR/ctx-lite"
    elif [ -f "$TEMP_DIR/bin/ctx-lite" ]; then
        BINARY_PATH="$TEMP_DIR/bin/ctx-lite"
    else
        echo -e "${RED}✗ Binary not found in downloaded archive${NC}"
        exit 1
    fi

    # Install (create directory if needed, use sudo only when required)
    if [ ! -w "$(dirname "$INSTALL_DIR")" ] && [ ! -d "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}Installing to $INSTALL_DIR (requires sudo)...${NC}"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "$BINARY_PATH" "$INSTALL_DIR/ctx-lite"
        sudo chmod +x "$INSTALL_DIR/ctx-lite"
    elif [ ! -w "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}Installing to $INSTALL_DIR (requires sudo)...${NC}"
        sudo mkdir -p "$INSTALL_DIR"
        sudo cp "$BINARY_PATH" "$INSTALL_DIR/ctx-lite"
        sudo chmod +x "$INSTALL_DIR/ctx-lite"
    else
        mkdir -p "$INSTALL_DIR"
        cp "$BINARY_PATH" "$INSTALL_DIR/ctx-lite"
        chmod +x "$INSTALL_DIR/ctx-lite"
    fi
    echo -e "${GREEN}✓ Installed${NC}"

    # Verify — test the actual installed binary directly
    echo -e "${YELLOW}Verifying...${NC}"
    if VERSION_OUTPUT=$("$INSTALL_DIR/ctx-lite" --version 2>/dev/null); then
        echo -e "${GREEN}✓ Successfully installed!${NC}"
        echo -e "  Location : $INSTALL_DIR/ctx-lite"
        echo -e "  Version  : $VERSION_OUTPUT"
        echo ""
        # Warn if install dir is not in PATH (but don't fail)
        if ! command -v ctx-lite &>/dev/null; then
            echo -e "${YELLOW}Note: $INSTALL_DIR is not in your PATH.${NC}"
            echo -e "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo -e "  export PATH=\"\$PATH:$INSTALL_DIR\""
        fi
        echo -e "${GREEN}Try it out:${NC}"
        echo -e "  ${YELLOW}ctx-lite --help${NC}"
        echo -e "  ${YELLOW}ctx-lite tree .${NC}"
    else
        echo -e "${RED}✗ Verification failed — binary did not run${NC}"
        exit 1
    fi
}

main "$@"
