#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from _devops import bump_semver, current_version, repo_root_from, write_version


def main() -> int:
    parser = argparse.ArgumentParser(description="Bump semantic version across repository files.")
    parser.add_argument("level", choices=("patch", "minor", "major"))
    parser.add_argument("--repo-root", type=Path, default=None)
    args = parser.parse_args()

    repo_root = repo_root_from(args.repo_root)
    old_version = current_version(repo_root)
    new_version = bump_semver(old_version, args.level)
    write_version(repo_root, new_version)
    print(f"{old_version} -> {new_version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
