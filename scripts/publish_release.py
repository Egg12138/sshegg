#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from _devops import repo_root_from, run_release


def main() -> int:
    parser = argparse.ArgumentParser(description="Publish a GitHub release for the current version.")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--repo-root", type=Path, default=None)
    args = parser.parse_args()
    return run_release(repo_root_from(args.repo_root), args.dry_run)


if __name__ == "__main__":
    raise SystemExit(main())
