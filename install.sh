#!/bin/sh
set -e

REPO="kkd16/evict"
INSTALL_DIR="/usr/local/bin"

OS=$(uname -s)
ARCH=$(uname -m)

if [ "$OS" = "Darwin" ] && [ "$ARCH" = "arm64" ]; then
    ARTIFACT="evict-macos-arm"
elif [ "$OS" = "Linux" ] && [ "$ARCH" = "x86_64" ]; then
    ARTIFACT="evict-linux-amd64"
else
    echo "Unsupported platform: $OS $ARCH"
    exit 1
fi

VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
    echo "Failed to fetch latest version"
    exit 1
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"

echo "Installing evict ${VERSION} (${ARTIFACT})..."

TMP=$(mktemp)
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP" "$INSTALL_DIR/evict"
else
    sudo mv "$TMP" "$INSTALL_DIR/evict"
fi

echo "Installed evict to ${INSTALL_DIR}/evict"
