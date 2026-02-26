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
