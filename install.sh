#!/usr/bin/env bash
set -e

# Neuro Compiler Installer

echo "========================================="
echo "        🧠 Installing NEURO               "
echo "========================================="

# Determine OS and Arch
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "Error: Currently only Linux is supported."
    exit 1
fi

if [ "$ARCH" != "x86_64" ]; then
    echo "Error: Currently only x86_64 architecture is supported."
    exit 1
fi

INSTALL_DIR="$HOME/.neuro"
BIN_DIR="$INSTALL_DIR/bin"
LIB_DIR="$INSTALL_DIR/lib"

echo "-> Setting up installation directory at $INSTALL_DIR"
mkdir -p "$BIN_DIR"
mkdir -p "$LIB_DIR"

echo "-> Fetching latest release from GitHub..."
REPO="pd241008/Neuro"
LATEST_RELEASE_URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep "browser_download_url.*neuro-linux-x64.tar.gz" | cut -d '"' -f 4)

if [ -z "$LATEST_RELEASE_URL" ]; then
    echo "Error: Could not find the latest release for Linux x64."
    exit 1
fi

echo "-> Downloading $LATEST_RELEASE_URL"
curl -sSL "$LATEST_RELEASE_URL" -o /tmp/neuro-linux-x64.tar.gz

echo "-> Extracting binaries..."
tar -xzf /tmp/neuro-linux-x64.tar.gz -C "$INSTALL_DIR"

chmod +x "$BIN_DIR/neuro"
chmod +x "$BIN_DIR/neuro-frontend"
chmod +x "$BIN_DIR/neuro-backend"

rm /tmp/neuro-linux-x64.tar.gz

echo "========================================="
echo "✅ NEURO installed successfully!"
echo ""
echo "Please add the following to your ~/.bashrc or ~/.zshrc:"
echo "  export PATH=\"\$HOME/.neuro/bin:\$PATH\""
echo ""
echo "Or run it directly using: ~/.neuro/bin/neuro"
echo "========================================="
