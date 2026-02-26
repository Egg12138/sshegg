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
