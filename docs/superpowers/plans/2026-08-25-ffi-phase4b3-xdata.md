# FFI Phase 4b3 XData/EED Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add stable C ABI access to entity XData records and values with safe mutations and DXF/DWG persistence.

**Architecture:** Keep XData in the existing `EntityCommon::extended_data` model and address entities by document-owned handles. Add focused public accessors to `ExtendedData`, then implement conversion, validation, TLS, APPID registration, and raw-EED invalidation in a new `src/ffi/xdata.rs` module. Test the ABI through Rust calls first, then a real MinGW C smoke program and cross-format reopen checks.

**Tech Stack:** Rust 2021, `repr(C)`, `extern "C"`, existing `DwgData`/`FfiDocument`, `ExtendedData`/`XDataValue`, DXF/DWG codecs, release MinGW C smoke tests, and the existing on-demand Cargo aliases.

**Spec:** `docs/superpowers/specs/2026-08-25-ffi-phase4b3-xdata-design.md`

## Global Constraints

- Work directly in the current workspace; do not create a worktree.
- Export exactly the nine P4b3 functions and the `DwgXdataValue` tags defined in the spec; do not add raw-EED, ACIS, callbacks, cbindgen, or unrelated object APIs.
- Use document-owned `uint64_t` entity handles; never retain a Rust `Vec` address, temporary object pointer, or Rust-owned allocation across the ABI.
- Mutations validate all arguments before changing the entity, preserve the entity on failure, set TLS on failure, and clear TLS on success.
- `record_append` registers the APPID when needed and rejects duplicate application names; record order remains stable.
- Before any record/value mutation, remove matching application handles from `raw_dwg_eed` so structured values are re-encoded instead of being shadowed by stale raw EED.
- Keep `.superpowers/` untracked and commit each completed task with a focused Conventional Commit.

## File Map

- `src/xdata/mod.rs` — add narrow indexed/mutable record accessors required by the FFI layer; preserve the existing `records` storage and order.
- `src/ffi/xdata.rs` — define the C layout/tags, entity lookup, value conversion, APPID registration, raw-EED invalidation, and all nine exports.
- `src/ffi/mod.rs` — register and re-export the XData FFI module.
- `tests/abi_test.rs` — Rust ABI fixtures, RED/Green assertions, and DWG/DXF reopen tests.
- `tests/ffi/c_xdata.c` — opaque C declarations, layout assertions, caller-buffer checks, and runtime smoke coverage.
- `docs/superpowers/plans/2026-08-25-ffi-phase4b3-xdata.md` — execution checkboxes and verification ledger.

---

## Current Status (2026-08-27)

- P4b3 implementation and verification are complete in the direct workspace.
- Tasks 1–5 are pending execution in the current workspace.
- Task 1 Rust ABI coverage: committed through 4abe1ae; Task 2 indexed
  accessors: 83a7fb8; Task 3 typed FFI: 9ff47ca.
- The pending wording in the original preflight note is superseded by the
  completed task checkboxes and verification results below.
- Task 4 C ABI smoke coverage: 88ef53c; DXF XData displacement mapping:
  f17fb94.
- Focused P4b3 tests: 4 passed. C smoke test: 1 passed. The FFI release
  gate passed 1291 tests plus 62 ABI tests; all-features passed 1335 tests
  plus 62 ABI tests. Existing ignored tests and warnings remain unchanged.
- cargo fmt --check remains red because of repository-wide pre-existing
  formatting drift; no unrelated formatter rewrite was applied.

### Task 1: Add the failing Rust XData ABI coverage

**Files:**
- Modify: `tests/abi_test.rs`
- Test target: `cargo tffi-abi phase4b3_ -- --nocapture --test-threads=1`

**Interfaces:**
- Consumes the public signatures and tags from the P4b3 spec plus `DwgData` lifecycle, existing iteration helpers, `CadDocument::get_entity_mut`, `ExtendedDataRecord`, and every `XDataValue` variant.
- Produces RED tests named `phase4b3_xdata_argument_and_type_errors`, `phase4b3_xdata_record_and_value_mutations`, `phase4b3_xdata_buffers_and_foreign_handle`, and `phase4b3_xdata_dwg_dxf_roundtrip`.

- [x] **Step 1: Build a fixture with a real entity handle and a full XData record.**

Create a temporary DWG fixture containing a `Line`. Add an application record
named `P4B3_APP` with `String`, `ControlString`, `LayerName`, `BinaryData`,
`Handle`, `Point3D`, `Position3D`, `Displacement3D`, `Direction3D`, `Real`,
`Distance`, `ScaleFactor`, `Integer16`, and `Integer32` values. Keep the entity
handle, a second same-document object handle, and a foreign-document handle in
the fixture state so validation tests do not rely on hard-coded handles.

- [x] **Step 2: Add argument and type RED assertions.**

Assert `DWG_EINVAL` for null documents, missing output pointers, null
application names, empty names, malformed UTF-8, unknown tags, nonzero
`reserved`, null data with nonzero length, zero-sized output buffers, invalid
indices, non-finite numeric values, and invalid control-string payloads. Assert
`DWG_ETYPE` for a non-entity handle and incompatible output shapes; assert
`DWG_ENOTFOUND` for a foreign entity and foreign handle payload. After every
failed mutation, compare record count, value count, and all fixture values.

- [x] **Step 3: Add record and value mutation assertions.**

Cover record count/find/entry, duplicate application rejection, empty-record
append, record removal, value lookup, append, indexed replacement, and
removal. Verify application order, shifted indices, returned removed values,
same-document handle acceptance, and `dwg_last_error_code() == DWG_OK` after
successful calls. Verify removing the last value leaves the empty record.

- [x] **Step 4: Add buffer and round-trip assertions.**

For strings and binary data, query required sizes, reject insufficient capacity
without mutation, then copy into caller-owned buffers and verify NUL/binary
contents. Write and reopen the fixture as AC1018 and AC1032 DWG plus DXF; locate
the entity again by handle/type and compare application order and every
surviving structured value. Include a mutation after reading a DWG with raw
EED and assert the changed application is re-encoded rather than restored from
the stale raw block.

- [x] **Step 5: Run the focused tests and confirm RED.**

Run:

```powershell
cargo tffi-abi phase4b3_ -- --nocapture --test-threads=1
```

Expected: compilation fails because the nine XData symbols, `DwgXdataValue`,
and any required `ExtendedData` indexed accessors are not implemented.

- [x] **Step 6: Commit the failing coverage.**

```powershell
git diff --check
git add tests/abi_test.rs
git commit -m "test(ffi): add xdata Rust ABI coverage"
```

### Task 2: Expose narrow indexed `ExtendedData` access

**Files:**
- Modify: `src/xdata/mod.rs`
- Test: existing XData unit tests in `src/xdata/mod.rs`

**Interfaces:**
- Consumes the private `ExtendedData::records` vector and existing `add_record`, `records`, `get_record`, and `clear` behavior.
- Produces `record(index)`, `record_mut(index)`, and `remove_record(index)` accessors with zero-based indexing, plus no new storage or reordering behavior for the FFI layer.

- [x] **Step 1: Add failing unit tests for indexed access.**

Create two records with distinct names and assert `record(0)`,
`record_mut(1)`, and `remove_record(0)` return the expected record while
preserving the remaining order. Assert out-of-range indices return `None` and
that existing `records()`/`get_record()` behavior is unchanged.

- [x] **Step 2: Implement the minimal public accessors.**

Add documented methods with these signatures:

```rust
pub fn record(&self, index: usize) -> Option<&ExtendedDataRecord>;
pub fn record_mut(&mut self, index: usize) -> Option<&mut ExtendedDataRecord>;
pub fn remove_record(&mut self, index: usize) -> Option<ExtendedDataRecord>;
```

Implement them as direct indexed access to `records`; do not expose the
vector, add duplicate filtering, or alter application-name comparison.

- [x] **Step 3: Run the focused XData unit tests.**

```powershell
cargo test --release --features ffi xdata::tests -- --nocapture
```

Expected: all XData unit tests pass.

- [x] **Step 4: Commit the core accessors.**

```powershell
git diff --check
git add src/xdata/mod.rs
git commit -m "feat(xdata): expose indexed record access"
```

### Task 3: Implement the typed XData C ABI

**Files:**
- Create: `src/ffi/xdata.rs`
- Modify: `src/ffi/mod.rs`
- Test: `tests/abi_test.rs` from Task 1

**Interfaces:**
- Consumes `FfiDocument`, `CadDocument::get_entity`, `get_entity_mut`, `allocate_handle`, `app_ids`, `AppId`, the indexed `ExtendedData` accessors, `XDataValue`, and existing TLS/error helpers.
- Produces the exact nine `dwg_entity_xdata_*` exports and fourteen `DWG_XDATA_*` tag constants from the spec.

- [x] **Step 1: Add the public C layout and conversion helpers.**

Define `#[repr(C)] DwgXdataValue` with the same fixed-width slots as the spec
and constants 1–14. Add private helpers to resolve an entity handle to an
`EntityType`, reject non-entities as `DWG_ETYPE`, map missing handles to
`DWG_ENOTFOUND`, validate UTF-8/buffers/finiteness, convert every tag to and
from `XDataValue`, and preserve caller-owned `data` pointers during output.

- [x] **Step 2: Implement record count/find/entry and APPID registration.**

Implement record enumeration in stored order. `record_find` compares
application names case-sensitively to the structured model and counts
occurrences from zero. `record_entry` writes the NUL-inclusive application
name, required length, and value count. `record_append` validates a non-empty
UTF-8 name, rejects duplicates, allocates an `AppId` handle with the document
allocator when missing, and appends an empty `ExtendedDataRecord`.

- [x] **Step 3: Implement record removal and value operations.**

Implement `record_remove` with complete output validation before removal. Add
value lookup, append, replacement, and removal using indexed accessors. For
all mutations, validate the complete input before borrowing mutable entity
state, preserve order, and clear TLS only after the mutation succeeds.

- [x] **Step 4: Invalidate stale raw EED before mutations.**

Resolve the application name to its APPID handle and call
`raw_dwg_eed.retain(|(handle, _)| *handle != app_handle)` on the target
entity before changing its structured record. For record removal, invalidate
before deleting the record. Ensure failed validation occurs before this step,
so failed calls cannot discard raw data.

- [x] **Step 5: Run the focused Rust suite and existing object tests.**

```powershell
cargo tffi-abi phase4b3_ -- --nocapture --test-threads=1
cargo tffi-lib ffi::object::tests -- --nocapture
cargo test --release --features ffi xdata::tests -- --nocapture
```

Expected: all four P4b3 ABI tests, object-arena tests, and XData unit tests
pass; existing warnings may remain unchanged.

- [x] **Step 6: Commit the ABI implementation.**

```powershell
git diff --check
git add src/ffi/xdata.rs src/ffi/mod.rs
git commit -m "feat(ffi): add entity xdata access API"
```

### Task 4: Add the C XData smoke test

**Files:**
- Create: `tests/ffi/c_xdata.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes the nine XData functions, the exact tagged layout, existing object iteration/lifecycle APIs, and a Rust-generated fixture path.
- Produces `c_xdata_compiles_links_and_runs`, a real MinGW compile/link/run check for all tags, record/value mutations, buffers, and failure preservation.

- [x] **Step 1: Write the C smoke program.**

Declare opaque `DwgData`, `DwgXdataValue`, all fourteen tags, error codes, and
exact prototypes. Add `_Static_assert` checks for the x64 `sizeof`/`offsetof`
layout. Enumerate `P4B3_APP`, copy string/binary values into caller buffers,
append/set/remove values, create/remove an empty record, and verify a missing
entity handle returns `DWG_ENOTFOUND` without changing counts or values.

- [x] **Step 2: Add the Rust C harness and fixture.**

Create a fixture helper that writes a line with the `P4B3_APP` record and a
second independent fixture. Compile the C source against the release DLL using
the existing `ffi_dll` and `c_runtime_command` helpers, pass both fixture paths
and the Rust-generated source entity/object handles, capture compiler/runtime
stdout and stderr, and remove the temporary executable.

- [x] **Step 3: Run the C smoke test.**

```powershell
cargo tffi-abi c_xdata_compiles_links_and_runs -- --exact --nocapture
```

Expected: the C source compiles, links all nine symbols, and exits zero after
the mutation and missing-handle checks.

- [x] **Step 4: Commit the C coverage.**

```powershell
git diff --check
git add tests/ffi/c_xdata.c tests/abi_test.rs
git commit -m "test(ffi): add xdata C smoke test"
```

### Task 5: P4b3 verification and handoff

**Files:**
- Modify: `docs/superpowers/plans/2026-08-25-ffi-phase4b3-xdata.md` to record verified checkboxes and totals.
- Modify: none in product code unless a verification test exposes a defect.

**Interfaces:**
- Consumes all P4b3 Rust/C tests and the existing on-demand FFI aliases.
- Produces focused evidence, complete FFI/all-features gates, and a clean committed product diff.

- [x] **Step 1: Run focused P4b3 and C tests.**

```powershell
cargo tffi-abi phase4b3_ -- --nocapture --test-threads=1
cargo tffi-abi c_xdata_compiles_links_and_runs -- --exact --nocapture
```

- [x] **Step 2: Run the complete FFI release gate.**

```powershell
cargo tffi-all --quiet
```

Record the passing test count and distinguish existing warnings/ignored tests
from new failures.

- [x] **Step 3: Run the all-features regression gate.**

```powershell
cargo tall --quiet
```

- [x] **Step 4: Check formatting and final state.**

```powershell
cargo fmt --check
git diff --check
git status --short
git log --oneline -n 12
```

If `cargo fmt --check` remains red from repository-wide baseline drift, record
that fact without applying an unrelated formatter rewrite. Confirm only the
focused P4b3 commits and intended documentation changes are present.

- [x] **Step 5: Review the final ABI and persistence boundary.**

Inspect `DwgXdataValue` layout, entity-handle validation, APPID allocation,
raw-EED invalidation ordering, caller-buffer ownership, and AC1018/AC1032 plus
DXF reopen assertions. Then update this plan's Current Status section and
commit the verification ledger:

```powershell
git add docs/superpowers/plans/2026-08-25-ffi-phase4b3-xdata.md
git commit -m "docs(ffi): record phase4b3 verification"
```

## Self-Review Checklist

- Spec coverage: Tasks 1–3 cover the complete tag set, nine ABI operations,
  validation/TLS behavior, APPID registration, and raw-EED invalidation;
  Task 4 covers C layout/linkage and Task 5 covers all required gates.
- Placeholder scan: this plan contains no TODO/TBD or unspecified edge-case
  steps.
- Type consistency: all signatures use `uint64_t entity`, `size_t` indices and
  counts, `DwgXdataValue *` payloads, and the same tag numbering as the spec.
- Scope: no raw EED API, object-pointer lifetime API, ACIS, callbacks, or
  unrelated entity behavior is included.
