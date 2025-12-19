#!/bin/bash

# Rust FTP TUI Client - Installation Script
# This script installs the FTP client from the .deb package or builds from source

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}"
echo "========================================"
echo "  Rust FTP TUI Client - Installer"
echo "========================================"
echo -e "${NC}"

print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }

# Check if running as root for .deb installation
install_from_deb() {
    DEB_PATH="target/debian/rust-ftp-tui_0.1.0-1_amd64.deb"
    
    if [ ! -f "$DEB_PATH" ]; then
        print_warning ".deb package not found. Building..."
        
        if ! command -v cargo &> /dev/null; then
            print_error "Cargo not found. Please install Rust first."
            echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            exit 1
        fi
        
        if ! command -v cargo-deb &> /dev/null; then
            print_info "Installing cargo-deb..."
            cargo install cargo-deb
        fi
        
        print_info "Building .deb package..."
        cargo deb
    fi
    
    print_info "Installing .deb package..."
    sudo dpkg -i "$DEB_PATH"
    print_success "Installation complete!"
}

install_from_source() {
    print_info "Installing from source..."
    
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first."
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    print_info "Building release version..."
    cargo build --release
    
    print_info "Installing to /usr/local/bin..."
    sudo cp target/release/rust-ftp-tui /usr/local/bin/
    sudo chmod +x /usr/local/bin/rust-ftp-tui
    
    print_success "Installation complete!"
}

# Main menu
echo "Choose installation method:"
echo "  1) Install from .deb package (recommended)"
echo "  2) Install from source"
echo "  3) Build only (no system install)"
echo "  4) Cancel"
echo

read -p "Enter choice [1-4]: " choice

case $choice in
    1)
        install_from_deb
        ;;
    2)
        install_from_source
        ;;
    3)
        print_info "Building release version..."
        source $HOME/.cargo/env 2>/dev/null || true
        cargo build --release
        print_success "Build complete! Binary at: target/release/rust-ftp-tui"
        ;;
    4)
        print_info "Cancelled."
        exit 0
        ;;
    *)
        print_error "Invalid choice."
        exit 1
        ;;
esac

echo
echo -e "${GREEN}========================================"
echo "  Installation successful!"
echo "========================================"
echo -e "${NC}"
echo "Usage:"
echo "  rust-ftp-tui              # Interactive mode"
echo "  rust-ftp-tui --help       # Show help"
echo
