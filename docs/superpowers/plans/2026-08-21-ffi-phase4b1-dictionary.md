# FFI Phase 4b1 Dictionary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add stable C ABI access to dictionary handles and key/value entries, including mutation and DWG round-trip coverage.

**Architecture:** Add a focused `src/ffi/dictionary.rs` module over `FfiDocument::doc.objects`. Dictionary identity is a fixed-width `uint64_t` handle; the named-objects root handle is exposed through a query so C callers do not depend on implementation-assigned handle values. Keys use caller-owned string buffers, and mutations update existing `Dictionary` entries without creating arbitrary object types.

**Tech Stack:** Rust 2021, `extern "C"`, existing `DwgData`/`FfiDocument`, `Dictionary`, `Handle`, release MinGW C smoke tests.

**Spec:** `docs/superpowers/specs/2026-08-21-ffi-phase4b-objects-design.md`

## Global Constraints

- Work directly in the current workspace; do not create a worktree.
- Export only the P4b1 dictionary functions listed in this plan; XRECORD, EED/XData, callbacks, ACIS, and cbindgen remain out of scope.
- No Rust collection address or temporary pointer crosses the ABI.
- Invalid arguments return `DWG_EINVAL`; missing handles, keys, or entries return `DWG_ENOTFOUND`; non-dictionary handles return `DWG_ETYPE`; success clears TLS and failure preserves the document.
- String output follows `buf`/`cap`/`needed`: `needed` includes the terminating NUL, `buf == NULL` is a size query, and insufficient capacity returns `DWG_EBUFFER` without mutation.
- Keep `.superpowers/` untracked and commit each completed task separately.

## Public ABI

```c
int dwg_data_named_objects_dictionary(const DwgData *data, uint64_t *handle);
int dwg_dictionary_count(const DwgData *data, uint64_t dictionary, size_t *count);
int dwg_dictionary_get(const DwgData *data, uint64_t dictionary, const char *key, uint64_t *value);
int dwg_dictionary_entry(const DwgData *data, uint64_t dictionary, size_t index,
                         char *key, size_t cap, size_t *needed, uint64_t *value);
int dwg_dictionary_set(DwgData *data, uint64_t dictionary, const char *key, uint64_t value);
int dwg_dictionary_remove(DwgData *data, uint64_t dictionary, const char *key, uint64_t *value);
```

`dwg_dictionary_set` replaces a case-insensitive existing key or appends a new
entry. It stores the supplied handle as a reference and does not manufacture a
new object. `dwg_dictionary_remove` writes the removed handle before returning
success.

---

### Task 1: Add failing Rust ABI coverage

**Files:**
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Tests the exact six functions in the Public ABI block.
- Uses the existing `DwgData` lifecycle and reads `FfiDocument.doc.header.named_objects_dict_handle` only inside Rust test setup to discover the root handle.

- [ ] **Step 1: Add the test fixture and invalid-argument cases**

  Add a helper that creates `DwgData`, obtains the named-objects dictionary handle, and destroys the document with a guard. Add tests asserting `DWG_EINVAL` for null `data`, null output pointers, null key, and a non-null key buffer with zero capacity. Add tests asserting `DWG_ENOTFOUND` for an unknown dictionary handle and missing key, and `DWG_ETYPE` when the dictionary handle points to a non-dictionary object.

- [ ] **Step 2: Add lookup, enumeration, and mutation assertions**

  Use the existing `ACAD_GROUP` entry as a known value. Assert `dwg_dictionary_count` is positive, `dwg_dictionary_get` returns its handle, and `dwg_dictionary_entry` returns the key and value at an index. Call `dwg_dictionary_set` with `P4B1_TEST` and the known handle, read it back, replace it with the root handle, then remove it and assert the removed value. Verify the document remains unchanged after a failed removal.

- [ ] **Step 3: Add buffer and round-trip assertions**

  Query an entry with `key == NULL`, assert `needed` includes the NUL, call with a too-small buffer and assert `DWG_EBUFFER`, then call with enough capacity and compare the bytes. Write the mutated document with `dwg_write_bytes`, read it into a second document, and assert `P4B1_TEST` is present with the expected handle. Assert successful calls clear `dwg_last_error_code`.

- [ ] **Step 4: Run the focused tests and confirm failure**

  Run:

  ```powershell
  cargo tffi-abi phase4b1_ -- --nocapture --test-threads=1
  ```

  Expected: compilation or test failure because the six dictionary functions are not exported yet.

### Task 2: Implement dictionary accessors and validation

**Files:**
- Create: `src/ffi/dictionary.rs`
- Modify: `src/ffi/mod.rs`

**Interfaces:**
- Consumes `FfiDocument`, `CadDocument.objects`, `ObjectType::Dictionary`, `Dictionary`, `Handle`, and existing TLS helpers.
- Produces the six `extern "C"` functions from the Public ABI block.

- [ ] **Step 1: Add private helpers for document and dictionary lookup**

  Implement helpers that reject null pointers before casting `DwgData` to `FfiDocument`, convert the `u64` handle with `Handle::new`, and return `&Dictionary` or `&mut Dictionary` only when the object map contains `ObjectType::Dictionary`. Map absent handles to `DWG_ENOTFOUND` and other object variants to `DWG_ETYPE` through the existing TLS error helper.

- [ ] **Step 2: Implement the named-objects root query and count/get**

  Return `ffi.doc.header.named_objects_dict_handle.value()` from `dwg_data_named_objects_dictionary`. Implement `dwg_dictionary_count` and `dwg_dictionary_get` with full pointer/key validation, case-insensitive lookup through `Dictionary::get`, TLS updates, and no mutation on failure.

- [ ] **Step 3: Implement indexed entry output**

  Implement `dwg_dictionary_entry` using `Dictionary.entries[index]`. Set `*needed = key.len() + 1` before the capacity check, support `key == NULL` as a size query, return `DWG_EBUFFER` when capacity is insufficient, and copy the UTF-8 key plus NUL with `copy_nonoverlapping`. Do not expose an internal string pointer.

- [ ] **Step 4: Implement set/remove mutations**

  Validate all pointers and the dictionary kind before mutation. For `set`, find an existing key with case-insensitive comparison and update its handle, or call `Dictionary::add_entry` for a new key. For `remove`, find the matching index, remove exactly one entry, write the old handle, and return `DWG_ENOTFOUND` when absent. Clear TLS only after successful mutation.

- [ ] **Step 5: Run focused Rust tests and verify success**

  Run:

  ```powershell
  cargo tffi-abi phase4b1_ -- --nocapture --test-threads=1
  cargo tffi-lib ffi::object::tests -- --nocapture
  ```

  Expected: all P4b1 Rust tests pass and the existing object-arena tests remain green.

- [ ] **Step 6: Commit the Rust dictionary ABI**

  ```powershell
  git diff --check
  git add src/ffi/dictionary.rs src/ffi/mod.rs tests/abi_test.rs
  git commit -m "feat(ffi): add dictionary access API"
  ```

### Task 3: Add C dictionary smoke coverage

**Files:**
- Create: `tests/ffi/c_dictionary.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes the six dictionary functions, `dwg_data_create`, `dwg_data_destroy`, `dwg_last_error_code`, `dwg_write_bytes`, `dwg_read_bytes`, and `dwg_free_buffer`.
- Produces a real MinGW compile/link/run check for the handle, string-buffer, mutation, and round-trip ABI.

- [ ] **Step 1: Write the C smoke program**

  Declare the opaque `DwgData` type and exact prototypes from the Public ABI block. Create a document, query the named-objects dictionary handle, get `ACAD_GROUP`, set `P4B1_TEST`, enumerate it into a caller-owned buffer, write bytes, read into a second document, verify the entry, remove it, and destroy both documents. Return distinct nonzero codes for each failed assertion.

- [ ] **Step 2: Add the Rust harness test**

  Add `c_dictionary_compiles_links_and_runs` to `tests/abi_test.rs`. Reuse `ffi_dll`, `c_runtime_command`, the `$CC`/`gcc` lookup, and the existing temporary executable cleanup pattern. Compile `tests/ffi/c_dictionary.c` against the selected DLL and fail with the compiler output or process exit code.

- [ ] **Step 3: Run the C smoke test**

  Run:

  ```powershell
  cargo tffi-abi c_dictionary_compiles_links_and_runs -- --exact --nocapture
  ```

  Expected: C compilation, symbol resolution, process execution, and dictionary round-trip all pass.

- [ ] **Step 4: Commit the C coverage**

  ```powershell
  git diff --check
  git add tests/ffi/c_dictionary.c tests/abi_test.rs
  git commit -m "test(ffi): add dictionary C smoke test"
  ```

### Task 4: P4b1 verification and handoff

**Files:**
- Modify: none unless a P4b1 test exposes an implementation defect

**Interfaces:**
- Consumes the committed aliases and dictionary ABI.
- Produces focused P4b1 evidence and a clean product diff.

- [ ] **Step 1: Run the focused P4b1 suite**

  Run `cargo tffi-abi phase4b1_ -- --nocapture --test-threads=1` and confirm all P4b1 tests pass.

- [ ] **Step 2: Run the complete FFI release gate**

  Run `cargo tffi-all --quiet`. Expected: all FFI-enabled library, integration, and C smoke tests pass.

- [ ] **Step 3: Run the all-features gate when handing off P4b1**

  Run `cargo tall --quiet`. Expected: all feature combinations pass.

- [ ] **Step 4: Check the final state**

  Run `git diff --check`, `git status --short`, and `git log --oneline -n 6`. Product changes must be committed; `.superpowers/` remains intentionally untracked.
