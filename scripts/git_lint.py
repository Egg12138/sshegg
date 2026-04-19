#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from pathlib import Path


COMMIT_MESSAGE_RE = re.compile(
    r"^(feat|fix|chore|docs|refactor|test|build|ci|perf|style)(\([a-z0-9._/-]+\))?!?: .+"
)


def lint_commit_message(message: str) -> int:
    first_line = message.strip().splitlines()[0] if message.strip() else ""
    if not COMMIT_MESSAGE_RE.fullmatch(first_line):
        print(
            "Invalid commit message. Expected Conventional Commits, for example: "
            "'feat: add devops automation'"
        )
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Lint git commit messages.")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--commit-message")
    group.add_argument("--commit-message-file", type=Path)
    args = parser.parse_args()

    if args.commit_message_file is not None:
        message = args.commit_message_file.read_text()
    else:
        message = args.commit_message or ""

    return lint_commit_message(message)


if __name__ == "__main__":
    raise SystemExit(main())
