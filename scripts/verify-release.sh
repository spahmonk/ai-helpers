#!/bin/bash
# Release verification script for ctx-lite
# Runs smoke tests and verifies the release binary works correctly

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== ctx-lite Release Verification ===${NC}\n"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found. Please run from the ctx-lite root directory.${NC}"
    exit 1
fi

# Step 1: Build release binary
echo -e "${YELLOW}Step 1: Building release binary...${NC}"
cargo build --release --quiet
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Release binary built successfully${NC}\n"

# Step 2: Test binary exists and is executable
echo -e "${YELLOW}Step 2: Verifying binary is executable...${NC}"
BINARY="./target/release/ctx-lite"
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Release binary not found at $BINARY${NC}"
    exit 1
fi
if [ ! -x "$BINARY" ]; then
    echo -e "${RED}Error: Binary is not executable${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Binary exists and is executable${NC}\n"

# Step 3: Test basic commands
echo -e "${YELLOW}Step 3: Testing basic CLI commands...${NC}"

# Test --help
if ! $BINARY --help > /dev/null 2>&1; then
    echo -e "${RED}✗ --help command failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ --help works${NC}"

# Test --version
if ! $BINARY --version > /dev/null 2>&1; then
    echo -e "${RED}✗ --version command failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ --version works${NC}"

# Test doctor
if ! $BINARY doctor > /dev/null 2>&1; then
    echo -e "${RED}✗ doctor command failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ doctor works${NC}\n"

# Step 4: Test file operations
echo -e "${YELLOW}Step 4: Testing file operations...${NC}"

# Test read
if ! $BINARY read Cargo.toml > /dev/null 2>&1; then
    echo -e "${RED}✗ read command failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ read works${NC}"

# Test tree
if ! $BINARY tree docs > /dev/null 2>&1; then
    echo -e "${RED}✗ tree command failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ tree works${NC}\n"

# Step 5: Run smoke tests
echo -e "${YELLOW}Step 5: Running smoke tests...${NC}"
if cargo test --test smoke_tests --quiet; then
    SMOKE_TESTS=$(cargo test --test smoke_tests --quiet 2>&1 | grep "test result:" | tail -1)
    echo -e "${GREEN}✓ Smoke tests passed${NC}"
    echo "  $SMOKE_TESTS"
else
    echo -e "${RED}✗ Smoke tests failed${NC}"
    exit 1
fi
echo

# Step 6: Run full test suite
echo -e "${YELLOW}Step 6: Running full test suite...${NC}"
if cargo test --quiet; then
    echo -e "${GREEN}✓ All tests passed${NC}"
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
echo

# Step 7: Show binary info
echo -e "${YELLOW}Step 7: Binary information...${NC}"
echo "Location: $(readlink -f $BINARY)"
echo "Size: $(du -h $BINARY | cut -f1)"
FILE_INFO=$($BINARY --version 2>&1)
echo "Version: $FILE_INFO"
echo

# Success
echo -e "${GREEN}=== Release Verification Complete ===${NC}"
echo -e "${GREEN}✓ All checks passed - ready for release!${NC}"
