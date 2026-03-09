#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# shellcheck source=../scripts/install.sh
source "$PROJECT_ROOT/scripts/install.sh"

issue_output=$(cat <<'EOF'
Updating mirror index
error: failed to select a version for the requirement `anyhow = "^1.0.86"` (locked to 1.0.102)
candidate versions found which didn't match: 1.0.100, 1.0.99, 1.0.98, ...
location searched: mirror index (which is replacing registry `crates-io`)
required by package `ssher v0.3.2 (/tmp/ssher)`
EOF
)

normal_error_output=$(cat <<'EOF'
Compiling ssher v0.3.2 (/tmp/ssher)
error[E0425]: cannot find value `missing_symbol` in this scope
EOF
)

echo "Test 1: mirror-backed resolution failures retry against crates.io"
if should_retry_with_crates_io "$issue_output"; then
    echo "  ✓ Retry requested for mirror dependency resolution failure"
else
    echo "  ✗ Expected retry for mirror dependency resolution failure"
    exit 1
fi

echo "Test 2: normal compile failures do not trigger registry retry"
if should_retry_with_crates_io "$normal_error_output"; then
    echo "  ✗ Unexpected retry for ordinary compile failure"
    exit 1
else
    echo "  ✓ No retry requested for ordinary compile failure"
fi

echo ""
echo "==> Retry detection tests passed!"
