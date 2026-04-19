#!/usr/bin/env python3
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path
from typing import Iterable


PLAN_SECTIONS = (
    "Goals",
    "Progress",
    "Current Severe Defects",
    "Updated Requirements",
)


def repo_root_from(start: Path | None = None) -> Path:
    if start is None:
        start = Path(__file__).resolve().parent.parent
    return start.resolve()


def cargo_toml_path(repo_root: Path) -> Path:
    return repo_root / "Cargo.toml"


def cargo_lock_path(repo_root: Path) -> Path:
    return repo_root / "Cargo.lock"


def agents_path(repo_root: Path) -> Path:
    return repo_root / "AGENTS.md"


def issues_path(repo_root: Path) -> Path:
    return repo_root / "issues.md"


def current_version(repo_root: Path) -> str:
    cargo_toml = cargo_toml_path(repo_root).read_text()
    match = re.search(r'^version = "(\d+)\.(\d+)\.(\d+)"$', cargo_toml, re.MULTILINE)
    if not match:
        raise SystemExit("Could not find semantic version in Cargo.toml")
    return match.group(0).split('"')[1]


def bump_semver(version: str, level: str) -> str:
    major, minor, patch = (int(part) for part in version.split("."))
    if level == "patch":
        patch += 1
    elif level == "minor":
        minor += 1
        patch = 0
    elif level == "major":
        major += 1
        minor = 0
        patch = 0
    else:
        raise SystemExit(f"Unsupported bump level: {level}")
    return f"{major}.{minor}.{patch}"


def replace_first(pattern: str, repl: str, text: str, message: str) -> str:
    updated, count = re.subn(pattern, repl, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(message)
    return updated


def write_version(repo_root: Path, new_version: str) -> tuple[str, str]:
    old_version = current_version(repo_root)

    cargo_toml = cargo_toml_path(repo_root).read_text()
    cargo_toml = replace_first(
        r'^version = "\d+\.\d+\.\d+"$',
        f'version = "{new_version}"',
        cargo_toml,
        "Failed to update package version in Cargo.toml",
    )
    cargo_toml_path(repo_root).write_text(cargo_toml)

    cargo_lock = cargo_lock_path(repo_root).read_text()
    cargo_lock = replace_first(
        r'(\[\[package\]\]\nname = "ssher"\nversion = ")\d+\.\d+\.\d+(")',
        rf'\g<1>{new_version}\2',
        cargo_lock,
        "Failed to update root package version in Cargo.lock",
    )
    cargo_lock_path(repo_root).write_text(cargo_lock)
    return old_version, new_version


def parse_bullets(markdown: str, heading: str) -> list[str]:
    pattern = rf"^### {re.escape(heading)}\n(?P<body>.*?)(?=^### |\Z)"
    match = re.search(pattern, markdown, re.MULTILINE | re.DOTALL)
    if not match:
        raise SystemExit(f"Could not find section '{heading}' in AGENTS.md")

    items: list[str] = []
    for line in match.group("body").splitlines():
        if line.startswith("- "):
            items.append(line[2:].strip())
    return items


def iter_issue_lines(markdown: str) -> Iterable[str]:
    in_section = False
    for raw_line in markdown.splitlines():
        line = raw_line.strip()
        if line == "## Current Urgent Issues":
            in_section = True
            continue
        if in_section and line.startswith("## "):
            break
        if in_section and line.startswith("- "):
            yield line[2:].strip()


def parse_issue_entries(repo_root: Path) -> list[str]:
    content = issues_path(repo_root).read_text()
    return list(iter_issue_lines(content))


def format_command(parts: list[str]) -> str:
    return " ".join(parts)


def release_command(repo_root: Path) -> list[str]:
    version = current_version(repo_root)
    return [
        "gh",
        "release",
        "create",
        f"v{version}",
        "--generate-notes",
        "--title",
        f"v{version}",
    ]


def run_release(repo_root: Path, dry_run: bool) -> int:
    cmd = release_command(repo_root)
    if dry_run:
        print(format_command(cmd))
        return 0
    return subprocess.run(cmd, cwd=repo_root, check=False).returncode


def print_plan_sections(repo_root: Path) -> None:
    content = agents_path(repo_root).read_text()
    for section in PLAN_SECTIONS:
        print(f"{section}:")
        for item in parse_bullets(content, section):
            print(f"- {item}")
        print()


def print_issue_sections(repo_root: Path) -> None:
    print("Issues:")
    for item in parse_issue_entries(repo_root):
        print(f"- {item}")


def stderr(message: str) -> None:
    print(message, file=sys.stderr)
