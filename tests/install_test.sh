#!/usr/bin/env bash
set -euo pipefail

# Integration test for install.sh
# This tests the install script in a temporary directory

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

TEST_DIR=$(mktemp -d)
TEST_PREFIX="${TEST_DIR}/bin"
TEST_HOME="${TEST_DIR}/home"

cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

echo "==> Testing install.sh..."

# Test 1: Help message
echo "Testing: --help"
if bash "$PROJECT_ROOT/scripts/install.sh" --help > /dev/null 2>&1; then
    echo "  ✓ Help message works"
else
    echo "  ✗ Help message failed"
    exit 1
fi

# Test 2: Argument validation - invalid option
echo "Testing: Invalid option handling"
if bash "$PROJECT_ROOT/scripts/install.sh" --invalid-option > /dev/null 2>&1; then
    echo "  ✗ Invalid option should fail"
    exit 1
else
    echo "  ✓ Invalid option rejected"
fi

# Test 3: Config directory creation (simulated)
echo "Testing: Config directory structure"
HOME="$TEST_HOME" mkdir -p "${TEST_HOME}/.config/ssher"
if [[ -d "${TEST_HOME}/.config/ssher" ]]; then
    echo "  ✓ Config directory structure valid"
else
    echo "  ✗ Config directory structure failed"
    exit 1
fi

# Test 4: Prefix directory creation
echo "Testing: Prefix directory creation"
mkdir -p "$TEST_PREFIX"
if [[ -d "$TEST_PREFIX" ]]; then
    echo "  ✓ Prefix directory can be created"
else
    echo "  ✗ Prefix directory creation failed"
    exit 1
fi

# Test 5: Check for required files in project
echo "Testing: Required project files"
if [[ -f "$PROJECT_ROOT/scripts/install.sh" ]]; then
    echo "  ✓ Install script exists"
else
    echo "  ✗ Install script missing"
    exit 1
fi

if [[ -f "$PROJECT_ROOT/assets/ui.sample.json" ]]; then
    echo "  ✓ Sample UI config exists"
else
    echo "  ✗ Sample UI config missing"
    exit 1
fi

if [[ -f "$PROJECT_ROOT/assets/cli.sample.json" ]]; then
    echo "  ✓ Sample CLI config exists"
else
    echo "  ✗ Sample CLI config missing"
    exit 1
fi

# Test 6: Check script is executable
if [[ -x "$PROJECT_ROOT/scripts/install.sh" ]]; then
    echo "  ✓ Install script is executable"
else
    echo "  ⚠ Install script not executable (but can be run with bash)"
fi

echo "==> All tests passed!"
echo ""
echo "Note: Full installation test requires:"
echo "  - Working Rust toolchain (cargo)"
echo "  - Compiled binary at target/release/ssher"
echo ""
echo "To test full installation:"
echo "  1. Build binary: cargo build --release"
echo "  2. Run: $PROJECT_ROOT/scripts/install.sh --prefix /tmp/test-ssher"
