#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO="spahmonk/ai-helpers"
VERSION="${CTX_LITE_VERSION:-1.0.0}"
INSTALL_DIR="${CTX_LITE_INSTALL_DIR:-/usr/local/bin}"

# Detect OS and architecture
detect_platform() {
    local os kernel arch
    os=$(uname -s)
    kernel=$(uname -m)
    
    case "$os" in
        Linux)
            case "$kernel" in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *) echo "Unsupported architecture: $kernel" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$kernel" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64) echo "aarch64-apple-darwin" ;;
                *) echo "Unsupported architecture: $kernel" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            exit 1
            ;;
    esac
}

main() {
    echo -e "${YELLOW}⚙️  ctx-lite installer v$VERSION${NC}"
    echo ""
    
    # Detect platform
    PLATFORM=$(detect_platform)
    echo -e "Detected platform: ${GREEN}$PLATFORM${NC}"
    
    # Download URL
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/v$VERSION/ctx-lite-$VERSION-$PLATFORM.tar.gz"
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # Download
    echo -e "${YELLOW}Downloading ctx-lite $VERSION...${NC}"
    if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/ctx-lite.tar.gz"; then
        echo -e "${RED}✗ Failed to download from: $DOWNLOAD_URL${NC}"
        echo "Make sure version $VERSION is released on GitHub."
        exit 1
    fi
    
    # Extract
    echo -e "${YELLOW}Extracting...${NC}"
    tar -xzf "$TEMP_DIR/ctx-lite.tar.gz" -C "$TEMP_DIR"
    
    # Find the binary (it should be at TEMP_DIR/ctx-lite or TEMP_DIR/ctx-lite/bin/ctx-lite)
    local BINARY_PATH
    if [ -f "$TEMP_DIR/ctx-lite" ]; then
        BINARY_PATH="$TEMP_DIR/ctx-lite"
    elif [ -f "$TEMP_DIR/bin/ctx-lite" ]; then
        BINARY_PATH="$TEMP_DIR/bin/ctx-lite"
    else
        echo -e "${RED}✗ Binary not found in downloaded archive${NC}"
        exit 1
    fi
    
    # Check if we need sudo
    if [ ! -w "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}Installing to $INSTALL_DIR requires sudo...${NC}"
        sudo cp "$BINARY_PATH" "$INSTALL_DIR/ctx-lite"
        sudo chmod +x "$INSTALL_DIR/ctx-lite"
    else
        cp "$BINARY_PATH" "$INSTALL_DIR/ctx-lite"
        chmod +x "$INSTALL_DIR/ctx-lite"
    fi
    
    # Verify installation
    echo -e "${YELLOW}Verifying installation...${NC}"
    if command -v ctx-lite &> /dev/null; then
        VERSION_OUTPUT=$("$INSTALL_DIR/ctx-lite" --version 2>/dev/null || echo "unknown")
        echo -e "${GREEN}✓ Successfully installed!${NC}"
        echo -e "  Location: $(command -v ctx-lite)"
        echo -e "  Version: $VERSION_OUTPUT"
        echo ""
        echo -e "${GREEN}You're all set! Try:${NC}"
        echo -e "  ${YELLOW}ctx-lite --help${NC}"
    else
        echo -e "${RED}✗ Installation verification failed${NC}"
        echo "ctx-lite command not found. Make sure $INSTALL_DIR is in your PATH."
        echo ""
        echo "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        exit 1
    fi
}

main "$@"
