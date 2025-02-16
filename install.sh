#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Installing Forge...${NC}"

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ARCH="x86_64"
        ;;
    aarch64)
        ARCH="aarch64"
        ;;
    *)
        echo -e "${RED}Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS" != "linux" ]; then
    echo -e "${RED}This script is for Linux only. For other platforms, please see:${NC}"
    echo -e "${BLUE}https://github.com/antinomyhq/forge#installation${NC}"
    exit 1
fi

# Detect libc
LIBC_INFO=$(ldd --version 2>&1 | head -n 1)
if echo "$LIBC_INFO" | grep -iF "musl"; then
    LIBC_SUFFIX="-musl"
else
    LIBC_SUFFIX="-gnu"
fi

# Allow optional version argument, defaulting to "latest"
VERSION="${1:-latest}"

# Construct download URL
DOWNLOAD_URL="https://release-download.tailcall.workers.dev/download/$VERSION/forge-$ARCH-unknown-linux$LIBC_SUFFIX"

# Create temp directory
TMP_DIR=$(mktemp -d)

# Download Forge
echo -e "${BLUE}Downloading Forge from $DOWNLOAD_URL...${NC}"
curl -L "$DOWNLOAD_URL" -o "$TMP_DIR/forge"

# Install
echo -e "${BLUE}Installing to /usr/local/bin...${NC}"
sudo mv "$TMP_DIR/forge" "/usr/local/bin/"
sudo chmod +x "/usr/local/bin/forge"
rm -rf "$TMP_DIR"

# Verify installation
if command -v forge >/dev/null 2>&1; then
    echo -e "${GREEN}Forge has been successfully installed!${NC}"
    echo -e "${BLUE}You can now run 'forge' to get started.${NC}"
else
    echo -e "${RED}Installation failed. Please try again or install manually.${NC}"
    exit 1
fi
