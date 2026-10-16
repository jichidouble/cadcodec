# On-Demand Test Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add short Cargo aliases for focused FFI tests while retaining explicit full-suite release gates.

**Architecture:** Keep all existing test files and release settings unchanged. Add aliases that select the library target, the monolithic ABI target, the setter inventory target, or the complete suite; Cargo's normal name filter remains available after each alias.

**Tech Stack:** Cargo aliases, Rust integration tests, MinGW release target.

**Spec:** `docs/superpowers/specs/2026-08-21-test-workflow-on-demand-design.md`

## Global Constraints

- Work directly in the current workspace; do not create a worktree.
- Do not change `profile.release`, test semantics, dependencies, or existing test placement.
- Keep `.superpowers/` untracked and out of commits.
- Preserve the existing `rb` and `rt` aliases.

---

### Task 1: Add focused Cargo aliases

**Files:**
- Modify: `.cargo/config.toml`

**Interfaces:**
- Produces `cargo tffi-lib`, `cargo tffi-abi`, `cargo tffi-inventory`, `cargo tffi-all`, and `cargo tall`.

- [ ] **Step 1: Add the aliases**

  Extend the existing `[alias]` table with:

  ```toml
  tffi-lib = ["test", "--release", "--features", "ffi", "--lib"]
  tffi-abi = ["test", "--release", "--features", "ffi", "--test", "abi_test"]
  tffi-inventory = ["test", "--release", "--features", "ffi", "--test", "ffi_setter_inventory"]
  tffi-all = ["test", "--release", "--features", "ffi"]
  tall = ["test", "--all-features"]
  ```

- [ ] **Step 2: Verify focused target selection**

  Run:

  ```powershell
  cargo tffi-abi phase4a_ -- --nocapture
  cargo tffi-lib buffer::tests -- --nocapture
  cargo tffi-inventory abi_manifest_pairs_every_writable_getter -- --exact --nocapture
  ```

  Expected: each command runs only its selected target and exits successfully.

- [ ] **Step 3: Commit the workflow change**

  ```powershell
  git diff --check
  git add .cargo/config.toml
  git commit -m "chore(test): add focused cargo test aliases"
  ```

### Task 2: Verify the full-suite aliases remain explicit gates

**Files:**
- Modify: none

**Interfaces:**
- Consumes the aliases from Task 1 and the existing complete test targets.

- [ ] **Step 1: Verify alias expansion without running the full suite**

  Run:

  ```powershell
  cargo tffi-abi -- --list
  cargo tffi-lib -- --list
  cargo tffi-inventory -- --list
  ```

  Expected: the ABI, library, and inventory test names are listed from their respective targets.

- [ ] **Step 2: Record the release-gate commands**

  Use `cargo tffi-all` before FFI handoff and `cargo tall` before all-feature release handoff. Do not replace either command with a filtered test run.

- [ ] **Step 3: Check the final diff**

  Run `git diff --check` and `git status --short`. Expected: no whitespace errors and no uncommitted product changes after Task 1's commit, except intentionally untracked `.superpowers/` notes.
