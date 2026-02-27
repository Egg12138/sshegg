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
FORCE_INSTALL=false
REPO="Egg12138/sshegg"
VERSION="${VERSION:-latest}"
BINARY_NAME_SHORT="se"

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
  --version VERSION       Install specific version (default: latest)
  --force                 Force reinstall even if already up to date

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
        --version)
            VERSION="$2"
            shift 2
            ;;
        --no-completions)
            INSTALL_COMPLETIONS=false
            shift
            ;;
        --force)
            FORCE_INSTALL=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Detect platform and return appropriate binary name
get_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$arch" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            echo "unsupported architecture: $arch" >&2
            return 1
            ;;
    esac

    case "$os" in
        linux)
            # Try musl first for static binary, fallback to gnu
            if command -v ldd &> /dev/null; then
                echo "ssher-${arch}-unknown-linux-musl"
            else
                echo "ssher-${arch}-unknown-linux-gnu"
            fi
            ;;
        darwin)
            echo "ssher-${arch}-apple-darwin"
            ;;
        msys*|mingw*|windows*)
            echo "ssher-${arch}-pc-windows-msvc"
            ;;
        *)
            echo "unsupported os: $os" >&2
            return 1
            ;;
    esac
}

download_binary() {
    print_header "Downloading pre-built binary"

    # Determine platform
    LOCAL_BINARY_NAME=$(get_platform) || {
        print_error "Unsupported platform"
        print_step "Falling back to building from source..."
        return 1
    }

    print_step "Detected platform: ${LOCAL_BINARY_NAME}"

    # Get latest version from GitHub API
    LATEST_VERSION=""
    if [[ "$VERSION" == "latest" ]]; then
        LATEST_VERSION=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
        if [[ -z "$LATEST_VERSION" ]]; then
            LATEST_VERSION="unknown"
        fi
        DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${LOCAL_BINARY_NAME}"
    else
        LATEST_VERSION="$VERSION"
        DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${LOCAL_BINARY_NAME}"
    fi

    print_step "Latest version: ${LATEST_VERSION}"

    # Check if se is already installed
    if [[ -x "${PREFIX}/se" && "$FORCE_INSTALL" != "true" ]]; then
        # Get version and commit hash from installed binary
        INSTALLED_INFO=$("${PREFIX}/se" --version 2>/dev/null || echo "unknown")
        INSTALLED_VERSION=$(echo "$INSTALLED_INFO" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
        INSTALLED_HASH=$(echo "$INSTALLED_INFO" | grep -oE '\([a-f0-9]{8}\)' | tr -d '()' || echo "unknown")

        if [[ -z "$INSTALLED_VERSION" ]]; then
            INSTALLED_VERSION="unknown"
        fi
        if [[ -z "$INSTALLED_HASH" ]]; then
            INSTALLED_HASH="unknown"
        fi

        print_step "Installed: ${INSTALLED_VERSION} (${INSTALLED_HASH})"

        # Compare versions (only if both are known and we're installing latest)
        if [[ "$VERSION" == "latest" && "$INSTALLED_VERSION" != "unknown" && "$LATEST_VERSION" != "unknown" ]]; then
            # Strip 'v' prefix if present
            INSTALLED_CLEAN="${INSTALLED_VERSION#v}"
            LATEST_CLEAN="${LATEST_VERSION#v}"

            if [[ "$INSTALLED_CLEAN" == "$LATEST_CLEAN" && "$INSTALLED_HASH" != "unknown" ]]; then
                # For pre-built binaries, we can't know the commit hash without downloading
                # So if semantic versions match, we assume it's up to date
                print_success "Already up to date (v${INSTALLED_CLEAN})"
                echo ""
                read -p "  Reinstall anyway? (y/N) " -n 1 -r
                echo
                if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                    print_header "Installation skipped"
                    return 2
                fi
                print_step "Forcing reinstall..."
            elif [[ "$(printf '%s\n' "$INSTALLED_CLEAN" "$LATEST_CLEAN" | sort -V | head -n1)" == "$LATEST_CLEAN" && "$INSTALLED_CLEAN" != "$LATEST_CLEAN" ]]; then
                print_step "Newer version available: ${LATEST_CLEAN} (currently: ${INSTALLED_CLEAN})"
            elif [[ "$INSTALLED_CLEAN" != "$LATEST_CLEAN" && "$(printf '%s\n' "$INSTALLED_CLEAN" "$LATEST_CLEAN" | sort -V | head -n1)" == "$INSTALLED_CLEAN" ]]; then
                print_warning "Installed version (${INSTALLED_CLEAN}) is newer than latest release (${LATEST_CLEAN})"
            fi
        fi
    fi

    print_step "Downloading from: ${DOWNLOAD_URL}"

    # Create temp directory
    TEMP_DIR=$(mktemp -d)
    CLEANUP_NEEDED=true

    # Download binary
    if [[ "$LOCAL_BINARY_NAME" == *"-windows-"* ]]; then
        BINARY_FILE="se.exe"
        ARCHIVE="${TEMP_DIR}/se.zip"
        if command -v curl &> /dev/null; then
            curl -fsSL "$DOWNLOAD_URL.zip" -o "$ARCHIVE"
        elif command -v wget &> /dev/null; then
            wget -q "$DOWNLOAD_URL.zip" -O "$ARCHIVE"
        else
            print_error "Neither curl nor wget available"
            rm -rf "$TEMP_DIR"
            return 1
        fi
        unzip -q "$ARCHIVE" -d "$TEMP_DIR"
    else
        BINARY_FILE="se"
        ARCHIVE="${TEMP_DIR}/se.tar.gz"
        if command -v curl &> /dev/null; then
            curl -fsSL "$DOWNLOAD_URL.tar.gz" -o "$ARCHIVE"
        elif command -v wget &> /dev/null; then
            wget -q "$DOWNLOAD_URL.tar.gz" -O "$ARCHIVE"
        else
            print_error "Neither curl nor wget available"
            rm -rf "$TEMP_DIR"
            return 1
        fi
        tar -xzf "$ARCHIVE" -C "$TEMP_DIR"
    fi

    if [[ ! -f "${TEMP_DIR}/${BINARY_FILE}" ]]; then
        print_error "Downloaded binary not found"
        rm -rf "$TEMP_DIR"
        return 1
    fi

    # Move to target/release for consistency with install_binary
    mkdir -p target/release
    cp "${TEMP_DIR}/${BINARY_FILE}" target/release/se
    chmod +x target/release/se

    rm -rf "$TEMP_DIR"

    print_success "Binary downloaded successfully"
    return 0
}

check_dependencies() {
    print_header "Checking dependencies"

    # Check for rust/cargo
    if command -v cargo &> /dev/null; then
        CARGO_VERSION=$(cargo --version 2>/dev/null | cut -d' ' -f2)
        if [[ -n "$CARGO_VERSION" ]]; then
            print_success "Rust toolchain found ($CARGO_VERSION)"
        else
            print_success "Rust toolchain found (version unavailable)"
        fi
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
    BINARY_PATH="${PREFIX}/se"
    if cp target/release/se "$BINARY_PATH" && chmod +x "$BINARY_PATH"; then
        print_success "Binary installed to ${BINARY_PATH}"
    else
        print_error "Failed to install binary"
        exit 1
    fi
}

setup_config() {
    print_header "Setting up configuration"

    CONFIG_DIR="${HOME}/.config/ssher"

    # Create config directory
    if [[ ! -d "$CONFIG_DIR" ]]; then
        print_step "Creating config directory: ${CONFIG_DIR}"
        mkdir -p "$CONFIG_DIR"
        print_success "Config directory created"
    else
        print_step "Config directory exists: ${CONFIG_DIR}"
    fi

    # Copy sample configs if they don't exist
    if [[ -f "assets/ui.sample.json" ]] && [[ ! -f "${CONFIG_DIR}/ui.json" ]]; then
        print_step "Installing sample UI config..."
        cp assets/ui.sample.json "${CONFIG_DIR}/ui.json"
        print_success "UI config installed"
    fi

    if [[ -f "assets/cli.sample.json" ]] && [[ ! -f "${CONFIG_DIR}/cli.json" ]]; then
        print_step "Installing sample CLI config..."
        cp assets/cli.sample.json "${CONFIG_DIR}/cli.json"
        print_success "CLI config installed"
    fi

    echo ""
    echo "  ${BLUE}Config location:${NC} ${CONFIG_DIR}"
    echo "  ${BLUE}Edit configs to customize ssher behavior${NC}"
}

install_completions() {
    if [[ "$INSTALL_COMPLETIONS" != "true" ]]; then
        return
    fi

    print_header "Installing shell completions"

    # Detect current shell
    CURRENT_SHELL=$(basename "$SHELL")
    INSTALLED_SOMETHING=false

    # Bash completions
    if [[ -d "${HOME}/.local/share/bash-completion/completions" ]] || command -v bash &> /dev/null; then
        COMPLETION_DIR="${HOME}/.local/share/bash-completion/completions"
        mkdir -p "$COMPLETION_DIR"
        if [[ -f "scripts/completions/ssher.bash" ]]; then
            cp scripts/completions/ssher.bash "${COMPLETION_DIR}/ssher"
            print_success "bash completions installed"
            INSTALLED_SOMETHING=true
        fi
    fi

    # Zsh completions
    if command -v zsh &> /dev/null || [[ "$CURRENT_SHELL" == "zsh" ]]; then
        ZSH_FUNCTIONS="${ZDOTDIR:-${HOME}}/.zfunc"
        mkdir -p "$ZSH_FUNCTIONS"
        if [[ -f "scripts/completions/ssher.zsh" ]]; then
            cp scripts/completions/ssher.zsh "${ZSH_FUNCTIONS}/_ssher"
            print_success "zsh completions installed to ${ZSH_FUNCTIONS}/_ssher"
            echo "    Add to ~/.zshrc: fpath=(\"${ZSH_FUNCTIONS}\" \$fpath)"
            INSTALLED_SOMETHING=true
        fi
    fi

    # Fish completions
    if command -v fish &> /dev/null || [[ "$CURRENT_SHELL" == "fish" ]]; then
        FISH_COMPLETIONS="${HOME}/.config/fish/completions"
        mkdir -p "$FISH_COMPLETIONS"
        if [[ -f "scripts/completions/ssher.fish" ]]; then
            cp scripts/completions/ssher.fish "${FISH_COMPLETIONS}/"
            print_success "fish completions installed"
            INSTALLED_SOMETHING=true
        fi
    fi

    if [[ "$INSTALLED_SOMETHING" != "true" ]]; then
        print_warning "No recognized shell found for completions"
    fi
}

print_summary() {
    echo ""
    echo -e "${GREEN}==> Installation complete!${NC}"
    echo ""
    echo "  ${BLUE}Binary:${NC}        ${PREFIX}/se"
    echo "  ${BLUE}Config:${NC}        ${HOME}/.config/ssher/"
    echo ""
    echo "  ${BLUE}Next steps:${NC}"
    echo "    1. Make sure ${PREFIX} is in your PATH"
    if [[ ":$PATH:" != *":${PREFIX}:"* ]]; then
        echo "       ${YELLOW}Currently NOT in PATH${NC}"
        echo "       Add to ~/.bashrc or ~/.zshrc:"
        echo "         export PATH=\"${PREFIX}:\$PATH\""
    fi
    echo "    2. Run: se --help"
    echo "    3. Launch TUI: se tui"
    echo ""
}

# Main execution
main() {
    echo -e "${GREEN}==> Installing ssher${NC}"
    echo ""

    # Try to download pre-built binary first
    if [[ "$FORCE_SOURCE" != "true" ]]; then
        set +e  # Temporarily disable errexit to handle return codes
        if download_binary; then
            DOWNLOAD_STATUS=0
        else
            DOWNLOAD_STATUS=$?
        fi
        set -e  # Re-enable errexit

        if [[ $DOWNLOAD_STATUS -eq 2 ]]; then
            # Already up to date, user chose not to reinstall
            exit 0
        elif [[ $DOWNLOAD_STATUS -ne 0 ]]; then
            print_warning "Download failed, building from source..."
            echo ""
            check_dependencies
            echo ""
            build_binary
            echo ""
        else
            print_success "Using pre-built binary"
            echo ""
        fi
    else
        check_dependencies
        echo ""
        build_binary
        echo ""
    fi

    install_binary
    echo ""

    setup_config
    echo ""

    install_completions
    echo ""

    print_summary
}

main "$@"
