# FFI Phase 6b-1 Basic Entity Creation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add C ABI constructors for LINE, CIRCLE, ARC, and POINT that add
entities to Model Space and return document-owned object handles.

**Architecture:** Add a focused `src/ffi/entity_create.rs` module with four
`extern "C"` wrappers. Extend `FfiDocument` with one internal helper that
delegates entity insertion and handle allocation to `CadDocument::add_entity`,
then appends a stable boxed arena record and relinks the active chain. The
core document storage and ownership model remain unchanged.

**Tech Stack:** Rust 2021, existing FFI arena and error codes, cbindgen 0.29.0,
C11-compatible header consumer, PowerShell.

**Spec:** `docs/superpowers/specs/2026-09-02-ffi-phase6b1-entity-create-design.md`

## Global Constraints

- Implement exactly `dwg_entity_line_create`, `dwg_entity_circle_create`,
  `dwg_entity_arc_create`, and `dwg_entity_point_create` from the spec.
- New entities use `CadDocument::add_entity()` and are routed to Model Space.
- Returned `DwgObject*` pointers are borrowed, document-owned, and never freed
  by C callers.
- Null `data` or `out` returns `DWG_EINVAL` without adding an entity.
- Do not change `CadDocument` storage, ownership, existing iteration semantics,
  or `Cargo.lock`.
- Work directly on the current branch; stage and commit each completed task.
- Run only focused FFI/header checks; do not run the complete slow test matrix.

---

### Task 1: Add failing Rust creation contract tests

**Files:**
- Create: `tests/ffi_create.rs`

**Interfaces:**
- Consumes: `dwg_data_create`, the four planned constructor symbols, existing
  count/iteration/lookup/type/handle APIs, and `dwg_data_destroy`.
- Produces: executable Rust contract tests that fail before the four symbols
  exist and define the exact success and invalid-argument behavior.

- [x] **Step 1: Build an owned document test helper**

Create an `OwnedData(*mut DwgData)` helper. Initialize it with
`dwg_data_create`, expose a `*mut DwgData` accessor, and call
`dwg_data_destroy` exactly once from `Drop`. Do not construct or inspect
`FfiDocument` directly in this integration test.

- [x] **Step 2: Write the four constructor success cases**

Create one empty document, call each constructor with concrete values, and
assert `DWG_OK`, non-null output, nonzero handle, and the expected object type:

```rust
let mut line: *mut DwgObject = std::ptr::null_mut();
assert_eq!(
    unsafe { dwg_entity_line_create(data, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, &mut line) },
    DWG_OK
);
assert_eq!(unsafe { dwg_object_get_type(line) }, DwgObjectType::Line as i32);
assert_ne!(unsafe { dwg_object_get_handle(line) }, 0);
```

Use analogous assertions for CIRCLE, ARC, and POINT. Verify entity count
becomes four and object count grows by four from the initial empty document.

- [x] **Step 3: Verify iteration, lookup, and independent ownership**

For every returned pointer, resolve its handle with `dwg_object_by_handle` and
assert pointer identity. Walk `dwg_data_first_object`/`dwg_object_next` and
assert every returned object is reachable. Create a second document, add one
POINT there, and assert its count and pointer are independent from the first
document.

- [x] **Step 4: Cover invalid arguments and verify the red state**

For each constructor, call it once with null `data` and once with null `out`.
Initialize output to null, assert `DWG_EINVAL`, and assert the document entity
count remains unchanged after invalid calls. Run:

```powershell
cargo test --release --features ffi --test ffi_create
```

Expected result: compilation fails because the four new Rust exports do not
exist. Fix test-only errors until the failure is specifically missing planned
symbols, then commit:

```powershell
git add tests/ffi_create.rs
git commit -m "test(ffi): define phase6b entity creation contract"
```

---

### Task 2: Implement the four entity constructors

**Files:**
- Create: `src/ffi/entity_create.rs`
- Modify: `src/ffi/object.rs`
- Modify: `src/ffi/mod.rs`

**Interfaces:**
- Consumes: the existing opaque `DwgData`/`DwgObject` types, FFI error
  constants, `FfiDocument`, `EntityType`, and Rust entity constructors.
- Produces: the exact four C exports and a stable object pointer in the
  existing active object chain.

- [x] **Step 1: Add the internal arena insertion helper**

In `FfiDocument`, add a `pub(crate)` helper accepting an `EntityType` and
returning `Result<*mut DwgObject, i32>`. Delegate to
`self.doc.add_entity(entity)`, retrieve the newly stored entity by its
allocated handle, and append:

```rust
Box::new(FfiObject {
    kind: entity_kind(entity_ref),
    handle,
    entity: entity_ref as *const EntityType,
    object: std::ptr::null(),
    next: std::ptr::null(),
    alive: true,
})
```

Convert the boxed record address to `*mut DwgObject`, push it into
`self.objects`, call `relink_objects()`, and return the address. Existing
boxed records must not be moved or recreated.

- [x] **Step 2: Add one validated shared wrapper**

In `src/ffi/entity_create.rs`, implement a private helper with signature
`unsafe fn create_entity(data: *mut DwgData, out: *mut *mut DwgObject, entity: EntityType) -> i32`.
Return `DWG_EINVAL` before dereferencing null `data` or `out`. On success,
write the returned object pointer, clear the last error, and return `DWG_OK`.
Map the internal insertion error to the existing FFI error path without adding
extra validation or ownership.

- [x] **Step 3: Export the four exact constructors**

Implement these wrappers using the corresponding constructors and
`Vector3::new`:

```rust
dwg_entity_line_create(data, x1, y1, z1, x2, y2, z2, out)
    -> EntityType::Line(Line::from_coords(x1, y1, z1, x2, y2, z2))
dwg_entity_circle_create(data, x, y, z, radius, out)
    -> EntityType::Circle(Circle::from_coords(x, y, z, radius))
dwg_entity_arc_create(data, x, y, z, radius, start_angle, end_angle, out)
    -> EntityType::Arc(Arc::from_coords(x, y, z, radius, start_angle, end_angle))
dwg_entity_point_create(data, x, y, z, out)
    -> EntityType::Point(Point::from_coords(x, y, z))
```

Mark each export `#[no_mangle] pub unsafe extern "C" fn`, re-export the new
module from `src/ffi/mod.rs`, and keep all signatures exactly as specified.

- [x] **Step 4: Run the focused green test and commit**

Run:

```powershell
rustfmt --edition 2021 src/ffi/entity_create.rs src/ffi/object.rs tests/ffi_create.rs
cargo test --release --features ffi --test ffi_create -- --nocapture
git diff --check
```

Expected result: all creation, count, lookup, iteration, independence, and
invalid-argument assertions pass. Commit:

```powershell
git add src/ffi/entity_create.rs src/ffi/object.rs src/ffi/mod.rs
git commit -m "feat(ffi): add basic entity creation APIs"
```

---

### Task 3: Regenerate the public header and extend the C consumer

**Files:**
- Modify: `tests/ffi_header.rs`
- Modify: `tests/ffi/c_header.c`
- Regenerate: `include/dwg_api.h`

**Interfaces:**
- Consumes: the four new exports through the generated public header only.
- Produces: header drift coverage and a C11 smoke test for LINE creation,
  counts, handle lookup, and borrowed-pointer use.

- [x] **Step 1: Add required-symbol assertions**

Add all four constructor names to the existing `required` list in
`tests/ffi_header.rs`. Preserve the current opaque-type, callback, and
internal-type leakage assertions.

- [x] **Step 2: Extend the C header smoke consumer**

In `tests/ffi/c_header.c`, keep the callback registration checks, then create
a LINE using only `dwg_api.h`:

```c
DwgObject *line = NULL;
if (dwg_entity_line_create(data, 0, 1, 2, 3, 4, 5, &line) != DWG_OK) return 4;
if (line == NULL) return 5;
```

Assert entity count is one, object count is positive, the returned handle is
nonzero, `dwg_object_by_handle` returns the same pointer, and
`UINT64_MAX` returns NULL with `DWG_ENOTFOUND`. Do not manually declare any
FFI function in the C file.

- [x] **Step 3: Regenerate and run focused ABI checks**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/generate_ffi_header.ps1
powershell -ExecutionPolicy Bypass -File tools/generate_ffi_header.ps1 -Check
cargo test --release --features ffi --test ffi_header
cargo build --release --features ffi
powershell -ExecutionPolicy Bypass -File tools/run_ffi_header_smoke.ps1 -Compiler gcc `
  -Dll target/x86_64-pc-windows-gnu/release/acadrust.dll `
  -ImportLibrary target/x86_64-pc-windows-gnu/release/libacadrust.dll.a
```

- [x] **Step 4: Commit the ABI consumer changes**

Run `git diff --check`, confirm no `Cargo.lock` change, and commit:

```powershell
git add include/dwg_api.h tests/ffi_header.rs tests/ffi/c_header.c
git commit -m "test(ffi): cover phase6b creation header consumer"
```

---

### Task 4: Document and finalize P6b-1

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-09-02-ffi-phase6b1-entity-create.md`

**Interfaces:**
- Consumes: the four stable public C constructors and their ownership rules.
- Produces: contributor-facing usage documentation and a completed plan with
  exact verification evidence.

- [x] **Step 1: Document creation and ownership**

Extend the README C FFI section with the four constructor names, Model Space
routing, automatic handles, and the rule that returned objects are borrowed
and must not be freed. State that null document/output arguments return
`DWG_EINVAL`.

- [x] **Step 2: Run final scoped verification**

Run all of the following after the documentation edit:

```powershell
git status --short
git diff --check
powershell -ExecutionPolicy Bypass -File tools/generate_ffi_header.ps1 -Check
rustfmt --edition 2021 --check src/ffi/entity_create.rs src/ffi/object.rs tests/ffi_create.rs tests/ffi_header.rs
cargo test --release --features ffi --test ffi_create -- --nocapture
cargo test --release --features ffi --test ffi_header
powershell -ExecutionPolicy Bypass -File tools/run_ffi_header_smoke.ps1 -Compiler gcc `
  -Dll target/x86_64-pc-windows-gnu/release/acadrust.dll `
  -ImportLibrary target/x86_64-pc-windows-gnu/release/libacadrust.dll.a
```

Record the observed test counts and any existing warnings. Do not claim full
suite or local MSVC verification.

- [x] **Step 3: Mark the plan complete and commit documentation**

Mark completed checkboxes and add an execution record listing commit hashes,
focused verification results, the existing repository-wide formatting
limitation, and the MSVC CI-only boundary. Confirm the final worktree has no
unrelated changes, then commit:

```powershell
git add README.md docs/superpowers/plans/2026-09-02-ffi-phase6b1-entity-create.md
git commit -m "docs(ffi): document phase6b1 entity creation"
```

## Verification Checklist

- [x] All four constructors appear in `include/dwg_api.h`.
- [x] Each constructor adds exactly one Model Space entity with an allocated
  handle and stable document-owned object pointer.
- [x] New objects participate in counts, iteration, and handle lookup.
- [x] Null document/output arguments return `DWG_EINVAL` without mutation.
- [x] Rust creation/header tests and GNU C smoke pass.
- [x] No `Cargo.lock` or unrelated source changes exist.
- [x] MSVC remains explicitly CI-only.

## Execution Record

- Task 1 contract tests: `b330b39` (`test(ffi): define phase6b entity
  creation contract`).
- Task 2 implementation: `9e4cc06` (`feat(ffi): add basic entity creation
  APIs`).
- Task 3 generated header and C consumer: `ccac5c2` (`test(ffi): cover
  phase6b creation header consumer`).
- `git status --short` initially reported only `README.md` modified;
  subsequent inspection confirmed only the two Task 4 owned tracked files
  changed and no `Cargo.lock` or unrelated source change exists.
- `git diff --check` exited successfully and reported the existing README
  LF-to-CRLF working-copy conversion warning.
- `tools/generate_ffi_header.ps1 -Check` passed and reported
  `include/dwg_api.h is up to date.`
- Scoped `rustfmt --check` passed for `src/ffi/entity_create.rs`,
  `src/ffi/object.rs`, `tests/ffi_create.rs`, and `tests/ffi_header.rs`.
- `ffi_create`: **1 passed, 0 failed, 0 ignored, 0 measured, 0 filtered
  out**. The initial 120-second invocation timed out during compilation; the
  identical command completed successfully when rerun with a longer harness
  timeout.
- `ffi_header`: **1 passed, 0 failed, 0 ignored, 0 measured, 0 filtered
  out**.
- GNU C header smoke: passed with exit code 0 and no output.
- Existing warnings observed: unreachable pattern at
  `src/io/dxf/reader/section_reader.rs:7830`, GNU linker message
  `corrupt .drectve at end of def file`, and an unnecessary nested `unsafe`
  block at `tests/ffi_create.rs:132`.
- Repository-wide `cargo fmt --check` remains limited by pre-existing format
  drift outside this phase; verification therefore used the required scoped
  `rustfmt --check` without rewriting unrelated files.
- The complete slow suite and local MSVC verification were not run or
  claimed. MSVC validation remains CI-only.
