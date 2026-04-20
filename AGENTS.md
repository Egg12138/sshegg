# Repository Guidelines

## publish

* publish release on github, managed by CLI `gh`
* every code change must bump the project version according to semantic versioning
* after every version bump, always publish a GitHub release for that version
* DO NOT publish to crate.io
* we alwasy wanna my project binary to be installed as `se`, just like `rg` for `ripgrep`
installation scripts should be correct for naming

## Project Structure & Module Organization
Keep the Rust CLI’s entry point under `src/main.rs`, with reusable logic moving into `src/cli/`, `src/store/`, and `src/ui/` modules. Use `src/model.rs` (or `src/model/mod.rs`) for session data definitions, and keep the TUI implementation separate from the plain CLI helpers so headless commands can run without the UI. Any assets (templates, static text, config samples) belong under `assets/` or `docs/`. For configuration such as the session store path, lean on `~/.config/ssher/sessions.json` by default and document overrides in `README.md`.

## Build, Test, and Development Commands
- `cargo run -- [command]` executes the CLI with the provided subcommand (e.g., `cargo run -- add --name office`).
- `cargo build --release` produces the optimized binary for distribution.
- `cargo test` validates unit and integration tests across the workspace.
- `cargo fmt` keeps formatting consistent; run it before every commit.
- `cargo clippy --all-targets -- -D warnings` enforces lint rules and surface issues early.
- `cargo install --path .` is useful to try the installed binary locally once `Cargo.toml` is configured.

## Coding Style & Naming Conventions
Follow Rust idioms; use `rustfmt` with the default profile and prefer idiomatic names (snake_case for functions/variables, PascalCase for structs/enums). Keep module paths short and names descriptive (e.g., `store::SessionStore`). Document any `clippy` exceptions as inline comments referencing the lint ID. Use hyphenated binary names (`ssher`) and version your crate via `Cargo.toml` once the repo grows.

## Testing Guidelines
Structure tests under `tests/` for integration suites and keep unit tests inside module files with `#[cfg(test)]` blocks. Name test functions to describe behavior, such as `fn add_session_saves_entry()`. Target the session serialization logic, CLI argument parsing, and the TUI list filtering. `cargo test` should pass locally; run `cargo test -- --nocapture` when debugging interactive flows.

## CLI Interaction & TUI Expectations
Implement subcommands like `ssher add`, `ssher list`, `ssher remove`, and `ssher tui`. Sessions must record name, host, user, port, and optional identity file and be serialized to JSON in the configured store. The TUI (e.g., using `tui`, `crossterm`, or `ratatui`) should present a searchable table of sessions and execute `ssh` with the selected entry upon confirmation. Document how the TUI navigates (arrows, search, enter) so contributors can replicate the user experience.

## Commit & Pull Request Guidelines
Adopt Conventional Commits (`feat:`, `fix:`, `chore:`) to keep history structured. Every code-bearing change must include a semantic version bump that matches the change impact (`patch` for fixes, `minor` for backward-compatible features, `major` for breaking changes). Every PR needs a concise summary, why it matters, and links to relevant issues or design notes. Mention any TUI behaviour changes in the description and add a brief demo (GIF or `asciinema`) when the UI is altered substantially. Once a version changes, a matching GitHub release should always be published.

## Security & Configuration Tips
Never commit private keys—reference `~/.ssh` identity files in session entries. Store configuration defaults in `Cargo.toml` or `config/` templates, and keep secrets in `~/.config/ssher/.env` (ignored via `.gitignore`). Run `cargo audit` periodically once dependencies exist and document any suppressed advisories with their rationale.

## Issue Tracking
Check `issues.md` first for the current urgent defect backlog and the corresponding GitHub issue links. If `Current Severe Defects` or `Updated Requirements` mention an urgent problem, treat `issues.md` as the local source of truth for which GitHub issues must be addressed first.

## Plans
The subsections in this section are intentionally stable and machine-parseable. Scripts may parse the `###` headings and the bullet lists under them directly.

### Your Hook

Hooks: after issue, requirements are fixed or finished, update AGENTS.md: Progress and Goals sections (if nessacary), update issues.md if an issues is fixed; commit changes, bump version, and push release

### Goals
- Vim-style TUI operation aligned with `~/source/todaycli`.
- Dynamic identity file validation and autocomplete while entering paths in TUI mode.
- Full test coverage with CI automation.
- TUI and CLI style/theme customization support.
- ASCII logo title in TUI.
- Highly customizable TUI layout style (panels, sizing, visibility).
- Bottom help + status bar in TUI.
- Session deletion warning panel with name confirmation.
- Optional session monitor showing SSH host details (active PIDs, smart last-login).
- Session tags for grouping/filtering.
- Fast SCP workflows (send/receive) to a host.
- Password authentication support with secure keyring storage and auto-detect fallback.
- SCP password entry must use a clear dedicated prompt/modal instead of the current confusing inline experience.
- SCP local/remote path selection must support interactive autocomplete.
- SCP path input must support wildcard matching / glob-style patterns.

### Progress
- Base CLI/TUI flow implemented (add/list/remove/tui, default to TUI, empty-store interactive add).
- JSON session store and filter logic implemented with unit tests.
- Vim-style TUI navigation, ASCII logo, theme/layout config, help/status bars.
- TUI add-session form with identity file validation + suggestions; delete confirmation modal.
- Tags support, SCP CLI subcommand, last-connected timestamps.
- Optional session monitor (active PIDs + smart last-connected display).
- CLI + TUI theme customization, CLI integration tests, and CI workflow.
- Documentation updated for usage and config.
- Password authentication infrastructure (Phase 1): keyring integration, model updates.
- Password authentication CLI support (Phase 2): ssh2 backend, --password/--no-password flags, remove-password command.
- TUI SCP local/remote path autocomplete with inline candidate lists and tab completion.
- Remote terminal sessions now resync PTY size during interactive shell loops so full-screen apps track local terminal resizes.

### Current Severe Defects
- `se` SCP password input UX is currently confusing and unreliable; this is a severe usability defect and needs a clear, well-bounded password entry UI.
- `se` SCP path handling currently needs wildcard matching support so users can target multiple files naturally.

### Updated Requirements
- Add an explicit, clean password-entry interface for SCP flows, with clear visual boundaries and predictable focus/submit behavior.
- Support wildcard matching in SCP source/target paths.
