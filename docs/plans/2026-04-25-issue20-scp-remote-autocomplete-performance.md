# ISSUE20 SCP Remote Autocomplete Performance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate severe TUI lag during SCP remote path input by reducing remote autocomplete calls and bounding each lookup cost.

**Architecture:** Keep the existing SCP UX, but change remote autocomplete to fetch remote directory candidates once per directory and then filter locally while the user types. Bound remote enumeration output at the SSH helper layer so large directories cannot flood the UI loop. Add regression tests for caching/filter helpers and state cache behavior to lock performance-sensitive logic.

**Tech Stack:** Rust, ratatui, crossterm, ssh2, anyhow

### Task 1: Lock helper/state behavior with failing tests

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/mod.rs`
- Test: `src/ui/state.rs`
- Test: `src/ui/mod.rs`

**Step 1: Write failing `ScpForm` cache state tests**
- Add tests that verify:
- `ScpForm` can store remote directory cache (`directory` + raw candidate list).
- `ScpForm` can clear the remote cache.

**Step 2: Write failing remote helper tests in UI module**
- Add tests that verify:
- remote input parsing splits input into `(directory, prefix)` correctly.
- prefix matching against cached directory candidates keeps only expected suggestions.
- relative-path short-prefix gate prevents expensive lookup trigger decisions.

**Step 3: Run targeted tests and confirm RED**
- Run: `cargo test scp_form -- --nocapture`
- Run: `cargo test remote_autocomplete -- --nocapture`
- Expected: FAIL due to missing cache fields/helper functions.

### Task 2: Implement remote autocomplete directory-cache flow

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/mod.rs`

**Step 1: Add remote cache state to `ScpForm`**
- Add fields for:
- cached remote directory string.
- cached remote directory candidates.
- Add accessors/mutators:
- set cache.
- read cache.
- clear cache.

**Step 2: Add remote input helper functions in `src/ui/mod.rs`**
- Implement helpers to:
- parse remote input into directory/prefix.
- build normalized prefix for matching candidates.
- filter cached candidates by current prefix.
- decide if relative short prefix should skip network refresh.

**Step 3: Rework `update_scp_autocomplete` remote branch**
- On empty remote input: clear visible suggestions and cache.
- On remote input:
- reuse cache if input directory matches cached directory.
- otherwise query remote suggestions for `<directory>/` once, store in cache, and filter locally.
- preserve existing failure behavior: keep user text, clear suggestions, set status.

**Step 4: Keep key handling unchanged**
- Retain current key flow (`Tab`, typing, backspace), but ensure it now benefits from cache-based filtering.

### Task 3: Bound remote SSH completion cost

**Files:**
- Modify: `src/ssh.rs`
- Test: `src/ssh.rs`

**Step 1: Add a maximum remote completion result cap**
- Introduce a small constant for max entries returned per lookup (for example 256).
- Update remote `find` command pipeline to cap output.

**Step 2: Add/adjust test coverage for bounded behavior**
- Add test for output parsing/behavior where practical.
- If shell command itself is hard to unit-test, test helper behavior around deterministic entry lists and document cap in code.

**Step 3: Run targeted SSH tests**
- Run: `cargo test remote_completion -- --nocapture`
- Expected: PASS after implementation.

### Task 4: Verify, bump version, and hygiene

**Files:**
- Modify: `Cargo.toml`
- Optionally modify: `README.md`
- Optionally modify: `issues.md`
- Optionally modify: `AGENTS.md`

**Step 1: Run relevant tests**
- Run:
- `cargo test scp_form -- --nocapture`
- `cargo test remote_autocomplete -- --nocapture`
- `cargo test remote_completion -- --nocapture`
- `cargo test`

**Step 2: Bump semver**
- Bump `Cargo.toml` patch version for user-visible performance fix.

**Step 3: Track issue hygiene**
- If issue #20 is fully resolved and verified, update `issues.md` and related progress notes in `AGENTS.md` as needed.

**Step 4: Prepare release follow-up**
- After merge-ready state, publish matching GitHub release for bumped version per repository policy.
