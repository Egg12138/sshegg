#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT

WORKTREE="${TEST_DIR}/repo"
mkdir -p "$WORKTREE/scripts"

cp "${PROJECT_ROOT}/Cargo.toml" "${WORKTREE}/Cargo.toml"
cp "${PROJECT_ROOT}/Cargo.lock" "${WORKTREE}/Cargo.lock"
cp "${PROJECT_ROOT}/AGENTS.md" "${WORKTREE}/AGENTS.md"
cp "${PROJECT_ROOT}/issues.md" "${WORKTREE}/issues.md"
cp "${PROJECT_ROOT}/pyproject.toml" "${WORKTREE}/pyproject.toml"
cp "${PROJECT_ROOT}/scripts/git_lint.py" "${WORKTREE}/scripts/git_lint.py"
mkdir -p "${WORKTREE}/.githooks"

echo "==> Testing devops scripts..."

echo "Test 0: uv project metadata declares dev tooling"
grep -q 'requires-python = ">=' "${WORKTREE}/pyproject.toml"
grep -q 'ruff' "${WORKTREE}/pyproject.toml"
grep -q '\[tool.ruff\]' "${WORKTREE}/pyproject.toml"
echo "  ✓ pyproject configures uv and ruff"

echo "Test 1: show-plans prints parseable sections"
if python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" show-plans > "${TEST_DIR}/plans.out"; then
    grep -q '^Goals:' "${TEST_DIR}/plans.out"
    grep -q '^Progress:' "${TEST_DIR}/plans.out"
    grep -q '^Current Severe Defects:' "${TEST_DIR}/plans.out"
    grep -q '^Updated Requirements:' "${TEST_DIR}/plans.out"
    echo "  ✓ show-plans output includes all plan sections"
else
    echo "  ✗ show-plans failed"
    exit 1
fi

echo "Test 2: show-issues prints tracked issues"
if python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" show-issues > "${TEST_DIR}/issues.out"; then
    grep -q '#16' "${TEST_DIR}/issues.out"
    grep -q '#19' "${TEST_DIR}/issues.out"
    echo "  ✓ show-issues output includes urgent issues"
else
    echo "  ✗ show-issues failed"
    exit 1
fi

echo "Test 3: bump patch updates Cargo.toml and Cargo.lock root package"
python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" bump patch > "${TEST_DIR}/bump.out"
grep -q '0.5.2 -> 0.5.3' "${TEST_DIR}/bump.out"
grep -q '^version = "0.5.3"$' "${WORKTREE}/Cargo.toml"
python3 - <<'PY' "${WORKTREE}/Cargo.lock"
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text()
if 'name = "ssher"\nversion = "0.5.3"' not in text:
    raise SystemExit(1)
PY
echo "  ✓ patch bump updated version files"

echo "Test 4: bump minor increments minor and resets patch"
python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" bump minor > "${TEST_DIR}/bump-minor.out"
grep -q '0.5.3 -> 0.6.0' "${TEST_DIR}/bump-minor.out"
grep -q '^version = "0.6.0"$' "${WORKTREE}/Cargo.toml"
echo "  ✓ minor bump updated semantic version"

echo "Test 5: release dry-run emits gh release command"
if python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" release --dry-run > "${TEST_DIR}/release.out"; then
    grep -q 'gh release create v0.6.0' "${TEST_DIR}/release.out"
    echo "  ✓ release dry-run shows expected gh command"
else
    echo "  ✗ release dry-run failed"
    exit 1
fi

echo "Test 6: git-lint accepts conventional commit messages"
if python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" git-lint --commit-message 'feat: add devops automation' > /dev/null; then
    echo "  ✓ git-lint accepts conventional commits"
else
    echo "  ✗ git-lint rejected valid conventional commit"
    exit 1
fi

echo "Test 7: git-lint rejects invalid commit messages"
if python3 "${PROJECT_ROOT}/scripts/dev.py" --repo-root "$WORKTREE" git-lint --commit-message 'bad message' > /dev/null 2>&1; then
    echo "  ✗ git-lint accepted invalid commit message"
    exit 1
else
    echo "  ✓ git-lint rejects invalid commit messages"
fi

echo "Test 8: commit-msg hook enforces the same rule"
cp "${PROJECT_ROOT}/.githooks/commit-msg" "${WORKTREE}/.githooks/commit-msg"
chmod +x "${WORKTREE}/.githooks/commit-msg"
printf 'feat: hook validation\n' > "${TEST_DIR}/good-commit.txt"
printf 'oops not valid\n' > "${TEST_DIR}/bad-commit.txt"
if "${WORKTREE}/.githooks/commit-msg" "${TEST_DIR}/good-commit.txt" > /dev/null 2>&1; then
    echo "  ✓ commit-msg hook accepts valid messages"
else
    echo "  ✗ commit-msg hook rejected valid message"
    exit 1
fi
if "${WORKTREE}/.githooks/commit-msg" "${TEST_DIR}/bad-commit.txt" > /dev/null 2>&1; then
    echo "  ✗ commit-msg hook accepted invalid message"
    exit 1
else
    echo "  ✓ commit-msg hook rejects invalid messages"
fi

echo ""
echo "==> All devops tests passed!"
