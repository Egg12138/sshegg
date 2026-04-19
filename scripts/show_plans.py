#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from _devops import print_plan_sections, repo_root_from


def main() -> int:
    parser = argparse.ArgumentParser(description="Show parseable plan sections from AGENTS.md.")
    parser.add_argument("--repo-root", type=Path, default=None)
    args = parser.parse_args()
    print_plan_sections(repo_root_from(args.repo_root))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
