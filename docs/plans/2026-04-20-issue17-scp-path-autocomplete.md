# ISSUE17 SCP Path Autocomplete Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add clear autocomplete support for SCP local and remote path entry in the TUI for both send and receive flows.

**Architecture:** Extend `ScpForm` with suggestion state for the active path field, reuse local filesystem scans for local completions, and add a bounded remote path listing helper that shells out over the existing SSH connection to enumerate matching remote entries. Keep the interaction simple: update suggestions while typing, allow tab completion, and show a short candidate list in the SCP panel.

**Tech Stack:** Rust, ratatui, crossterm, ssh2, anyhow

### Task 1: Lock the autocomplete behavior with tests

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/mod.rs`
- Test: `src/ui/state.rs`
- Test: `src/ui/mod.rs`

**Step 1: Write the failing state tests**
- Add tests for `ScpForm` suggestion management so the active field can hold autocomplete candidates, apply a selected suggestion, and keep local and remote suggestion sets separate.
- Add tests for display helpers so the SCP panel includes the relevant suggestion hints when candidates exist.

**Step 2: Run the targeted tests and verify they fail**
- Run: `cargo test scp_form -- --nocapture`
- Expected: FAIL because `ScpForm` does not yet track any autocomplete state or suggestion rendering.

### Task 2: Implement local path autocomplete

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/mod.rs`

**Step 1: Add local suggestion state**
- Extend `ScpForm` with per-field suggestions, selected suggestion index, and helpers to refresh/apply candidates for the active path field.

**Step 2: Add local filesystem completion**
- Implement a helper in `src/ui/mod.rs` that expands `~`, scans the relevant directory, and returns sorted path suggestions for the current local input prefix.

**Step 3: Wire key handling**
- Update SCP key handling so typing/backspace refreshes local suggestions when the local field is active and `Tab` applies the highlighted suggestion before moving to the next field.

### Task 3: Implement remote path autocomplete

**Files:**
- Modify: `src/ssh.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/state.rs`

**Step 1: Add a bounded SSH helper**
- Add a helper that connects with the existing authentication rules, lists remote directory entries for a prefix, and returns normalized remote path suggestions.

**Step 2: Refresh remote suggestions from the SCP form**
- Reuse the same SCP field helpers so remote typing/backspace updates remote candidates and remote `Tab` completion applies the current match instead of advancing immediately.

**Step 3: Keep failure behavior safe**
- If remote suggestion lookup fails, keep the typed value, clear stale candidates, and surface the error through status text instead of blocking the user from manual entry.

### Task 4: Verify and release hygiene

**Files:**
- Modify: `Cargo.toml`
- Modify: `issues.md`
- Modify: `AGENTS.md`
- Optionally modify: `README.md`

**Step 1: Run targeted tests**
- Run the new SCP state/UI tests first, then the broader relevant cargo test targets.

**Step 2: Bump version**
- Apply a semantic version bump in `Cargo.toml` for the user-visible TUI improvement.

**Step 3: Update issue tracking**
- Remove issue 17 from `issues.md` if complete and update `AGENTS.md` progress/goals sections if the new capability changes those stable summaries.

**Step 4: Verify completion**
- Confirm local and remote autocomplete both work in send/receive flows and that the tests covering the new behavior pass.
