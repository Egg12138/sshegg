#!/usr/bin/env bash
set -euo pipefail

# Integration test for install.sh
# This tests the install script by actually running it

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
echo "  Test directory: $TEST_DIR"
echo "  Test prefix: $TEST_PREFIX"
echo "  Test home: $TEST_HOME"
echo ""

# Test 1: Help message
echo "Test 1: --help"
if bash "$PROJECT_ROOT/scripts/install.sh" --help > /dev/null 2>&1; then
    echo "  ✓ Help message works"
else
    echo "  ✗ Help message failed"
    exit 1
fi

# Test 2: Argument validation - invalid option
echo "Test 2: Invalid option handling"
if bash "$PROJECT_ROOT/scripts/install.sh" --invalid-option > /dev/null 2>&1; then
    echo "  ✗ Invalid option should fail"
    exit 1
else
    echo "  ✓ Invalid option rejected"
fi

# Test 3: Summary output should render ANSI escapes instead of literal \033 text
echo "Test 3: Summary output ANSI formatting"
summary_output=$(
    PREFIX="$TEST_PREFIX" HOME="$TEST_HOME" bash -lc '
        set -- --help
        source "'"$PROJECT_ROOT"'/scripts/install.sh" >/dev/null 2>&1 || true
        print_summary
    '
)
if [[ "$summary_output" == *'\033['* ]]; then
    echo "  ✗ Summary output contains literal escape sequences"
    exit 1
else
    echo "  ✓ Summary output does not contain literal escape sequences"
fi

# Test 4: Build the binary
echo "Test 4: Building binary"
cd "$PROJECT_ROOT"
if cargo build --release > /dev/null 2>&1; then
    echo "  ✓ Binary built successfully"
else
    echo "  ✗ Binary build failed"
    exit 1
fi

# Test 5: Run installation with custom prefix and HOME
echo "Test 5: Running installation with --prefix --from-source"
export HOME="$TEST_HOME"
if bash "$PROJECT_ROOT/scripts/install.sh" --prefix "$TEST_PREFIX" --from-source > /dev/null 2>&1; then
    echo "  ✓ Installation script ran successfully"
else
    echo "  ✗ Installation script failed"
    exit 1
fi

# Test 6: Check binary was installed
echo "Test 6: Checking binary installation"
if [[ -f "$TEST_PREFIX/se" ]]; then
    echo "  ✓ Binary exists at $TEST_PREFIX/se"
else
    echo "  ✗ Binary not found at $TEST_PREFIX/se"
    exit 1
fi

if [[ -x "$TEST_PREFIX/se" ]]; then
    echo "  ✓ Binary is executable"
else
    echo "  ✗ Binary is not executable"
    exit 1
fi

# Test 7: Check config directory was created
echo "Test 7: Checking config directory creation"
if [[ -d "$TEST_HOME/.config/ssher" ]]; then
    echo "  ✓ Config directory created at $TEST_HOME/.config/ssher"
else
    echo "  ✗ Config directory not found at $TEST_HOME/.config/ssher"
    exit 1
fi

# Test 8: Check sample configs were installed
echo "Test 8: Checking sample config installation"
if [[ -f "$TEST_HOME/.config/ssher/ui.json" ]]; then
    echo "  ✓ UI config installed"
else
    echo "  ✗ UI config not found"
    exit 1
fi

if [[ -f "$TEST_HOME/.config/ssher/cli.json" ]]; then
    echo "  ✓ CLI config installed"
else
    echo "  ✗ CLI config not found"
    exit 1
fi

# Test 9: Run the installed binary
echo "Test 9: Running installed binary"
if "$TEST_PREFIX/se" --help > /dev/null 2>&1; then
    echo "  ✓ Installed binary runs successfully"
else
    echo "  ✗ Installed binary failed to run"
    exit 1
fi

VERSION_OUTPUT=$("$TEST_PREFIX/se" --version 2>/dev/null || true)
if [[ "$VERSION_OUTPUT" == se* ]]; then
    echo "  ✓ Installed binary identifies itself as 'se'"
else
    echo "  ✗ Installed binary version output is not 'se ...': $VERSION_OUTPUT"
    exit 1
fi

# Test 10: Check completion file naming uses 'se'
echo "Test 10: Checking completion file naming"
if [[ -f "$TEST_HOME/.local/share/bash-completion/completions/se" ]]; then
    echo "  ✓ Bash completion installed as 'se'"
else
    echo "  ✗ Bash completion not found at .../completions/se"
    exit 1
fi

if [[ ! -f "$TEST_HOME/.local/share/bash-completion/completions/ssher" ]]; then
    echo "  ✓ Legacy bash completion 'ssher' cleaned up"
else
    echo "  ✗ Legacy bash completion 'ssher' should not exist"
    exit 1
fi

# Test 11: Check legacy command compatibility binary
echo "Test 11: Checking legacy 'ssher' compatibility command"
if [[ -x "$TEST_PREFIX/ssher" ]]; then
    echo "  ✓ Legacy compatibility command exists at $TEST_PREFIX/ssher"
else
    echo "  ✗ Legacy compatibility command missing at $TEST_PREFIX/ssher"
    exit 1
fi

if "$TEST_PREFIX/ssher" --help > /dev/null 2>&1; then
    echo "  ✓ Legacy compatibility command runs successfully"
else
    echo "  ✗ Legacy compatibility command failed to run"
    exit 1
fi

echo ""
echo "==> All tests passed!"
