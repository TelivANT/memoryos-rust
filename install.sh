#!/bin/bash
# MemoryOS-Rust One-Click Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/install.sh | bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPO="TelivANT/memoryos-rust"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.memoryos}"
BINARY_NAME="memoryos-gateway"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  MemoryOS-Rust Installation Script${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

case "$OS" in
    linux)
        PLATFORM="unknown-linux-gnu"
        ;;
    darwin)
        PLATFORM="apple-darwin"
        ;;
    *)
        echo -e "${RED}Error: Unsupported OS: $OS${NC}"
        exit 1
        ;;
esac

TARGET="${ARCH}-${PLATFORM}"
echo -e "${YELLOW}Detected platform: $TARGET${NC}"

# Check if Docker is installed
if command -v docker &> /dev/null; then
    echo -e "${GREEN}✓ Docker detected${NC}"
    USE_DOCKER=true
else
    echo -e "${YELLOW}⚠ Docker not found. Will install binary directly.${NC}"
    USE_DOCKER=false
fi

# Function: Install via Docker
install_docker() {
    echo -e "${GREEN}Installing MemoryOS-Rust via Docker...${NC}"
    
    # Create installation directory
    mkdir -p "$INSTALL_DIR"
    cd "$INSTALL_DIR"
    
    # Download docker-compose.yml
    echo "Downloading docker-compose.yml..."
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/docker-compose.yml" -o docker-compose.yml
    
    # Download example config
    echo "Downloading example configuration..."
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/config.example.toml" -o config.toml
    
    echo -e "${GREEN}✓ Docker setup complete${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Edit configuration: nano $INSTALL_DIR/config.toml"
    echo "2. Start services: cd $INSTALL_DIR && docker-compose up -d"
    echo "3. Check status: docker-compose ps"
    echo "4. View logs: docker-compose logs -f"
}

# Function: Install binary
install_binary() {
    echo -e "${GREEN}Installing MemoryOS-Rust binary...${NC}"
    
    # Get latest release
    echo "Fetching latest release..."
    LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$LATEST_RELEASE" ]; then
        echo -e "${RED}Error: Could not fetch latest release${NC}"
        exit 1
    fi
    
    echo -e "${YELLOW}Latest version: $LATEST_RELEASE${NC}"
    
    # Download binary
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_RELEASE/${BINARY_NAME}-${TARGET}.tar.gz"
    echo "Downloading from: $DOWNLOAD_URL"
    
    mkdir -p "$INSTALL_DIR/bin"
    cd "$INSTALL_DIR"
    
    if ! curl -fsSL "$DOWNLOAD_URL" -o "${BINARY_NAME}.tar.gz"; then
        echo -e "${RED}Error: Failed to download binary${NC}"
        echo -e "${YELLOW}Note: Pre-built binaries may not be available yet.${NC}"
        echo -e "${YELLOW}Please build from source: https://github.com/$REPO${NC}"
        exit 1
    fi
    
    # Extract binary
    echo "Extracting binary..."
    tar -xzf "${BINARY_NAME}.tar.gz" -C bin/
    chmod +x "bin/$BINARY_NAME"
    rm "${BINARY_NAME}.tar.gz"
    
    # Download example config
    echo "Downloading example configuration..."
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/config.example.toml" -o config.toml
    
    # Add to PATH
    SHELL_RC="$HOME/.bashrc"
    if [ -f "$HOME/.zshrc" ]; then
        SHELL_RC="$HOME/.zshrc"
    fi
    
    if ! grep -q "MEMORYOS_HOME" "$SHELL_RC"; then
        echo "" >> "$SHELL_RC"
        echo "# MemoryOS-Rust" >> "$SHELL_RC"
        echo "export MEMORYOS_HOME=\"$INSTALL_DIR\"" >> "$SHELL_RC"
        echo "export PATH=\"\$MEMORYOS_HOME/bin:\$PATH\"" >> "$SHELL_RC"
        echo -e "${GREEN}✓ Added to PATH in $SHELL_RC${NC}"
    fi
    
    echo -e "${GREEN}✓ Binary installation complete${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Reload shell: source $SHELL_RC"
    echo "2. Edit configuration: nano $INSTALL_DIR/config.toml"
    echo "3. Start MemoryOS: $BINARY_NAME"
}

# Main installation logic
if [ "$USE_DOCKER" = true ]; then
    read -p "Install via Docker? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        install_docker
    else
        install_binary
    fi
else
    install_binary
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Installation Complete! 🎉${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${YELLOW}Documentation: https://github.com/$REPO${NC}"
echo -e "${YELLOW}Issues: https://github.com/$REPO/issues${NC}"
echo ""
