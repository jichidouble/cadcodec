# FFI Phase 6a Object Lookup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add document entity/object counts and direct active-object lookup by handle to the existing opaque C ABI.

**Architecture:** Keep `CadDocument` and its ownership model unchanged. Expose counts through the existing `FfiDocument` wrapper and resolve handles by scanning its stable boxed arena, returning the existing borrowed `DwgObject*` token. Regenerate the public header from `src/ffi/mod.rs` and validate the ABI with focused Rust and C consumers.

**Tech Stack:** Rust 2021, existing FFI module, cbindgen 0.29.0, C11-compatible header consumer, PowerShell.

**Spec:** `docs/superpowers/specs/2026-08-28-ffi-phase6a-object-lookup-design.md`

## Global Constraints

- Do not change `CadDocument` storage, ownership, pointer allocation, or existing iteration semantics.
- `dwg_data_entity_count` reports `CadDocument::entity_count()`.
- `dwg_data_object_count` reports active entries in the existing first/next object chain, including entities and non-entity objects.
- `dwg_object_by_handle` returns only active document-owned objects; C never frees the returned pointer.
- Use `DWG_EINVAL` for null document/error/output arguments and `DWG_ENOTFOUND` for handle zero, missing handles, and removed handles.
- Work directly on the current branch; stage and commit each completed task. Do not create a worktree or modify `Cargo.lock`.
- Run only targeted FFI/header checks; do not run the complete slow test matrix.

---

### Task 1: Add failing Rust lookup and count tests

**Files:**
- Create: `tests/ffi_lookup.rs`

**Interfaces:**
- Consumes the existing public FFI functions, `FfiDocument`, and `CadDocument` test construction.
- Produces executable contract tests that initially fail because the three new symbols do not exist.

- [x] **Step 1: Build a minimal owned test document helper**

Construct a `CadDocument` with two entities, wrap it in `FfiDocument::new`,
and cast a boxed wrapper to `*mut DwgData`. Add a cleanup helper that calls
`dwg_data_destroy` exactly once.

- [x] **Step 2: Write the count contract**

Assert that entity count is two, active object count equals the number reached
through `dwg_data_first_object`/`dwg_object_next`, and counts decrease after
removing the first object/entity. Also cover an empty document's zero entity
count and independent documents.

- [x] **Step 3: Write the lookup/error contract**

Resolve an iterated handle and assert pointer identity and `DWG_OK`. Assert
`DWG_ENOTFOUND` for zero, an absent handle, and a removed handle; assert
`DWG_EINVAL` for null data and null error output. Verify the other document is
unchanged after removal from the first.

- [x] **Step 4: Run the focused test and commit the red test**

Run:

```powershell
cargo test --release --features ffi --test ffi_lookup
```

The test must fail to compile or link because the new API is not implemented.
Commit with `test(ffi): define phase6a lookup contract`.

---

### Task 2: Implement document counts and handle lookup

**Files:**
- Modify: `src/ffi/data.rs`
- Modify: `src/ffi/iterate.rs`

**Interfaces:**
- Produces the exact declarations from the spec:
  `dwg_data_entity_count`, `dwg_data_object_count`, and `dwg_object_by_handle`.
- Uses the existing `FfiDocument` arena and `CadDocument::entity_count()` only.

- [x] **Step 1: Implement entity and active-object counts**

Validate pointers, write the current entity count or active arena count to the
caller output, and return `DWG_OK`. Keep the existing `i32` count convention
used by the FFI table APIs.

- [x] **Step 2: Implement active handle lookup**

Validate `data` and `error`, scan only `alive` arena entries, compare the
64-bit handle, set `*error` to `DWG_OK` on success, and return the stable boxed
arena address. Return NULL with `DWG_ENOTFOUND` for zero, missing, or removed
handles.

- [x] **Step 3: Run the focused Rust tests**

Run the new test with `--nocapture` and confirm all count, pointer identity,
removal, invalid-argument, and independent-document assertions pass.

- [x] **Step 4: Commit the implementation**

Run `git diff --check` and commit with
`feat(ffi): add object lookup and document counts`.

---

### Task 3: Regenerate the header and extend the C smoke consumer

**Files:**
- Modify: `tests/ffi/c_header.c`
- Modify: `tests/ffi_header.rs`
- Regenerate: `include/dwg_api.h`

**Interfaces:**
- Consumes the three new Rust exports and the existing PowerShell smoke runner.
- Produces a generated header contract and a C consumer that uses no manual declarations.

- [x] **Step 1: Add generated-header contract assertions**

Require all three new function names in `tests/ffi_header.rs`, preserving the
existing required-symbol and forbidden-internal-type checks.

- [x] **Step 2: Extend the C consumer**

Use only `dwg_api.h` to query entity/object counts, obtain the first object,
resolve its handle through `dwg_object_by_handle`, and verify missing-handle
failure. Keep the existing callback registration and lifecycle checks.

- [x] **Step 3: Regenerate and run focused ABI checks**

Run:

```powershell
powershell -File tools/generate_ffi_header.ps1
powershell -File tools/generate_ffi_header.ps1 -Check
cargo test --release --features ffi --test ffi_header
cargo build --release --features ffi
powershell -File tools/run_ffi_header_smoke.ps1 -Compiler gcc `
  -Dll target/x86_64-pc-windows-gnu/release/acadrust.dll `
  -ImportLibrary target/x86_64-pc-windows-gnu/release/libacadrust.dll.a
```

- [x] **Step 4: Commit the generated ABI consumer changes**

Run `git diff --check` and commit with
`test(ffi): cover phase6a header lookup consumer`.

---

### Task 4: Document and finalize P6a

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-08-28-ffi-phase6a-object-lookup.md`

**Interfaces:**
- Documents the new count and lookup calls without changing the ownership contract.
- Records exact targeted verification and the CI-only boundary for MSVC.

- [x] **Step 1: Document the new calls**

Explain that counts cover current active state, lookup returns a borrowed
document-owned pointer, and removed/missing handles return `DWG_ENOTFOUND`.
Include the header and focused command references already used by P5.

- [x] **Step 2: Run final scoped verification**

Run `git status --short`, `git diff --check`, the generator `-Check`, the
focused Rust lookup/header tests, FFI-scoped rustfmt, and the GNU smoke test.
Do not claim local MSVC verification or full-suite success.

- [x] **Step 3: Mark the plan and commit the documentation**

Mark completed steps, verify no unrelated files or `Cargo.lock` changes exist,
and commit with `docs(ffi): document phase6a object lookup`.

## Verification Checklist

- [x] Entity count matches `CadDocument::entity_count()`.
- [x] Active object count matches the existing iteration chain.
- [x] Lookup returns the same stable pointer as iteration.
- [x] Zero, missing, removed, null-data, and null-error cases return the specified result.
- [x] Generated header contains all three declarations without internal-type leakage.
- [x] Rust contract, header contract, GNU build, and GNU C smoke pass.
- [x] MSVC remains explicitly CI-only.

## Execution Record

- Commits: `57739d5`, `1276c1a`, `98b53e5`.
- Verification: `ffi_lookup` 2/2, `ffi_header` 1/1, generator `-Check`, GNU
  release build, and GNU C smoke passed.
- Repository-wide formatting is pre-existing and outside this FFI slice;
  FFI-scoped rustfmt passed. MSVC remains CI-only.
