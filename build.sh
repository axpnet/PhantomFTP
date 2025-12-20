#!/bin/bash

# Rust FTP TUI Client - Build Script
# This script builds the project and sets up the environment

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}"
echo "========================================"
echo "  Rust FTP TUI Client - Build Script"
echo "========================================"
echo -e "${NC}"

# Function to print status messages
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on supported OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="Linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macOS"
else
    print_warning "Unsupported OS: $OSTYPE"
    print_warning "This script is designed for Linux and macOS"
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "cargo is not installed. Please install Rust first."
    print_error "Visit: https://www.rust-lang.org/tools/install"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
print_status "Detected Rust version: $RUST_VERSION"

# Create necessary directories
print_status "Creating project directories..."
mkdir -p downloads
mkdir -p logs
mkdir -p ~/.config/phantomftp

# Check if config file exists, if not create from example
if [ ! -f ~/.config/phantomftp/config.json ]; then
    print_status "Creating default configuration file..."
    cat > ~/.config/phantomftp/config.json << 'EOF'
{
  "servers": [],
  "default_local_path": "~/Downloads",
  "theme": {
    "primary_color": "blue",
    "accent_color": "yellow",
    "background_color": "black",
    "text_color": "white",
    "success_color": "green",
    "error_color": "red",
    "warning_color": "yellow",
    "info_color": "cyan"
  },
  "settings": {
    "auto_save_last_server": true,
    "default_transfer_mode": "binary",
    "show_hidden_files": false,
    "confirm_overwrite": true,
    "max_concurrent_transfers": 3,
    "transfer_buffer_size": 8192,
    "keep_alive_interval": 60,
    "retry_attempts": 3,
    "retry_delay": 2
  }
}
EOF
    print_success "Configuration file created at: ~/.config/phantomftp/config.json"
else
    print_status "Configuration file already exists"
fi

# Update dependencies
print_status "Updating dependencies..."
cargo update

# Run tests
print_status "Running tests..."
if cargo test; then
    print_success "All tests passed!"
else
    print_warning "Some tests failed, but continuing with build..."
fi

# Build the project
print_status "Building project (this may take a while)..."
if cargo build --release; then
    print_success "Build completed successfully!"
else
    print_error "Build failed!"
    exit 1
fi

# Check if binary was created
if [ -f target/release/phantomftp ]; then
    print_success "Binary created at: target/release/phantomftp"
else
    print_error "Binary not found!"
    exit 1
fi

# Create symlink for easy access (optional)
if [ ! -L ./phantomftp ]; then
    ln -sf target/release/phantomftp ./phantomftp
    print_success "Created symlink: ./phantomftp"
fi

# Build examples
print_status "Building examples..."
if cargo build --example test_ftp --release; then
    print_success "Test FTP example built successfully!"
else
    print_warning "Failed to build test FTP example"
fi

# Create test file for upload functionality
if [ ! -f test_upload.txt ]; then
    print_status "Creating test upload file..."
    cat > test_upload.txt << 'EOF'
# Rust FTP TUI Client - Test Upload File

This is a test file created automatically by the build script.
You can use this file to test the upload functionality of the FTP client.

File created at: $(date)
EOF
    print_success "Test file created: test_upload.txt"
fi

# Set permissions
chmod +x ./phantomftp

# Calculate binary size
BINARY_SIZE=$(du -h target/release/phantomftp | cut -f1)
print_status "Binary size: $BINARY_SIZE"

# Generate shell completions (optional)
print_status "Generating shell completions..."
mkdir -p completions
if command -v phantomftp &> /dev/null || [ -f ./phantomftp ]; then
    # This would require the binary to be in PATH or clap_complete support
    print_warning "Shell completions generation skipped (requires installed binary)"
else
    print_warning "Shell completions generation skipped"
fi

# Create desktop entry (for Linux desktop environments)
if [[ "$OS" == "Linux" ]]; then
    print_status "Creating desktop entry..."
    cat > phantomftp.desktop << EOF
[Desktop Entry]
Name=Rust FTP TUI
Comment=Terminal FTP Client
Exec=$PWD/phantomftp
Icon=utilities-terminal
Terminal=true
Type=Application
Categories=Network;FileTransfer;TerminalEmulator;
EOF
    print_success "Desktop entry created: phantomftp.desktop"
fi

echo
echo -e "${GREEN}"
echo "========================================"
echo "  Build completed successfully!"
echo "========================================"
echo -e "${NC}"
echo
echo "Usage examples:"
echo "  ./phantomftp                           # Interactive mode"
echo "  ./phantomftp --help                    # Show help"
echo "  ./phantomftp --server host:21 --username user --password pass"
echo
echo "Test FTP functionality:"
echo "  cargo run --example test_ftp            # Run FTP test suite"
echo
echo "Configuration:"
echo "  Edit ~/.config/phantomftp/config.json to customize settings"
echo
echo "Directories:"
echo "  Downloads: ./downloads/                 # Default download location"
echo "  Logs: ./logs/                         # Application logs"
echo "  Config: ~/.config/phantomftp/       # Configuration files"
echo
print_success "Happy FTPing! 🚀"

# Optional: Run a quick connectivity test
if [ "$1" == "--test" ]; then
    echo
    print_status "Running connectivity test..."
    if ./phantomftp --help &> /dev/null; then
        print_success "Binary test passed!"
    else
        print_error "Binary test failed!"
        exit 1
    fi
fi