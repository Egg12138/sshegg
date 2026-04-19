#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

from _devops import (
    bump_semver,
    current_version,
    print_issue_sections,
    print_plan_sections,
    repo_root_from,
    run_release,
    write_version,
)

SCRIPT_DIR = Path(__file__).resolve().parent


def main() -> int:
    parser = argparse.ArgumentParser(description="Unified devops entrypoint.")
    parser.add_argument("--repo-root", type=Path, default=None)
    subparsers = parser.add_subparsers(dest="command", required=True)

    bump_parser = subparsers.add_parser("bump", help="Bump semantic version.")
    bump_parser.add_argument("level", choices=("patch", "minor", "major"))

    release_parser = subparsers.add_parser("release", help="Publish the current GitHub release.")
    release_parser.add_argument("--dry-run", action="store_true")

    subparsers.add_parser("show-plans", help="Show plans parsed from AGENTS.md.")
    subparsers.add_parser("show-issues", help="Show urgent issues parsed from issues.md.")
    git_lint_parser = subparsers.add_parser("git-lint", help="Lint git commit messages.")
    git_lint_group = git_lint_parser.add_mutually_exclusive_group(required=True)
    git_lint_group.add_argument("--commit-message")
    git_lint_group.add_argument("--commit-message-file", type=Path)

    args = parser.parse_args()
    repo_root = repo_root_from(args.repo_root)

    if args.command == "bump":
        old_version = current_version(repo_root)
        new_version = bump_semver(old_version, args.level)
        write_version(repo_root, new_version)
        print(f"{old_version} -> {new_version}")
        return 0
    if args.command == "release":
        return run_release(repo_root, args.dry_run)
    if args.command == "show-plans":
        print_plan_sections(repo_root)
        return 0
    if args.command == "show-issues":
        print_issue_sections(repo_root)
        return 0
    if args.command == "git-lint":
        cmd = [str(SCRIPT_DIR / "git_lint.py")]
        if args.commit_message is not None:
            cmd.extend(["--commit-message", args.commit_message])
        else:
            cmd.extend(["--commit-message-file", str(args.commit_message_file)])
        return subprocess.run(cmd, cwd=repo_root, check=False).returncode
    raise SystemExit(f"Unknown command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
