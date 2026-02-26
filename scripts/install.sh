#!/usr/bin/env bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Icons
CHECK_MARK="✓"
X_MARK="✗"
INFO="➜"

# Default values
PREFIX="${HOME}/.local/bin"
INSTALL_COMPLETIONS=true
FORCE_SOURCE=false

# Print functions
print_header() {
    echo -e "${BLUE}==>${NC} $1"
}

print_step() {
    echo -e "  ${BLUE}${INFO}${NC} $1"
}

print_success() {
    echo -e "  ${GREEN}${CHECK_MARK}${NC} $1"
}

print_error() {
    echo -e "  ${RED}${X_MARK}${NC} $1"
}

print_warning() {
    echo -e "  ${YELLOW}!${NC} $1"
}

# Show help message
show_help() {
    cat << EOF
${GREEN}ssher Installation Script${NC}

${BLUE}Usage:${NC}
  ./scripts/install.sh [OPTIONS]

${BLUE}Options:${NC}
  -h, --help              Show this help message
  --prefix PATH           Install to custom location (default: ~/.local/bin)
  --from-source           Force build from source
  --no-completions        Skip shell completion installation

${BLUE}Examples:${NC}
  ./scripts/install.sh                    # Install to default location
  ./scripts/install.sh --prefix ~/bin     # Install to custom location
  ./scripts/install.sh --no-completions   # Skip completions

${BLUE}After installation:${NC}
  Make sure ~/.local/bin (or your custom prefix) is in your PATH
  Config will be created at ~/.config/ssher/

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --from-source)
            FORCE_SOURCE=true
            shift
            ;;
        --no-completions)
            INSTALL_COMPLETIONS=false
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

check_dependencies() {
    print_header "Checking dependencies"

    # Check for rust/cargo
    if command -v cargo &> /dev/null; then
        print_success "Rust toolchain found ($(cargo --version | cut -d' ' -f2))"
    else
        print_error "Rust toolchain not found"
        echo ""
        echo "${YELLOW}Install Rust via:${NC}"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        exit 1
    fi

    # Check for ssh
    if command -v ssh &> /dev/null; then
        print_success "ssh command available"
    else
        print_warning "ssh command not found"
        echo ""
        echo "${YELLOW}Install OpenSSH client:${NC}"
        if command -v apt-get &> /dev/null; then
            echo "  sudo apt-get install openssh-client"
        elif command -v yum &> /dev/null; then
            echo "  sudo yum install openssh-clients"
        elif command -v pacman &> /dev/null; then
            echo "  sudo pacman -S openssh"
        elif command -v brew &> /dev/null; then
            echo "  brew install openssh"
        else
            echo "  Please install openssh-client for your distribution"
        fi
        echo ""
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
}

build_binary() {
    print_header "Building binary"
    print_step "Building release binary..."

    if cargo build --release 2>&1 | while IFS= read -r line; do
        echo "    $line"
    done; then
        print_success "Build complete"
    else
        print_error "Build failed"
        echo "Please check the error output above"
        exit 1
    fi
}

install_binary() {
    print_header "Installing to ${PREFIX}"

    # Create prefix directory if it doesn't exist
    if [[ ! -d "$PREFIX" ]]; then
        print_step "Creating directory: ${PREFIX}"
        if mkdir -p "$PREFIX"; then
            print_success "Directory created"
        else
            print_error "Failed to create directory: ${PREFIX}"
            echo "Try using --prefix to specify a writable location"
            exit 1
        fi
    fi

    # Check if directory is writable
    if [[ ! -w "$PREFIX" ]]; then
        print_error "No write permission for: ${PREFIX}"
        echo "Try using --prefix to specify a user-writable location"
        echo "  Example: ./scripts/install.sh --prefix ~/.local/bin"
        exit 1
    fi

    # Copy binary
    print_step "Installing binary..."
    BINARY_PATH="${PREFIX}/ssher"
    if cp target/release/ssher "$BINARY_PATH" && chmod +x "$BINARY_PATH"; then
        print_success "Binary installed to ${BINARY_PATH}"
    else
        print_error "Failed to install binary"
        exit 1
    fi
}
