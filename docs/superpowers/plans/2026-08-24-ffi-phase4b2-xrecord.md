# FFI Phase 4b2 XRECORD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add stable typed C ABI access to XRECORD entries, including lookup, enumeration, insertion, replacement, removal, and DWG/DXF persistence coverage.

**Architecture:** Add a focused `src/ffi/xrecord.rs` module over the existing `FfiDocument::doc.objects` map and `XRecord` core model. The ABI uses a fixed `repr(C)` tagged value with caller-owned string/chunk storage; XRECORDs are addressed by document-owned handles obtained through the existing Dictionary API.

**Tech Stack:** Rust 2021, `repr(C)`, `extern "C"`, existing `DwgData`/`FfiDocument`, `XRecord`/`XRecordEntry`/`XRecordValue`, release MinGW C smoke tests, DWG byte and DXF file round-trips.

**Spec:** `docs/superpowers/specs/2026-08-24-ffi-phase4b2-xrecord-design.md`

## Current Status (2026-08-25)

- **Task 1 — complete.** Rust ABI coverage is committed through `466734a`.
- **Task 2 — complete.** Typed ABI, DXF metadata round-trip, and reference
  synchronization fixes are committed in `d03abd9`, `38d4e6f`, `732c23f`, and
  `45d00f9`.
- Focused verification passes: P4b2 ABI **4/4** and XRECORD-related Rust unit
  tests **19/19**. `git diff --check` passes.
- `cargo fmt --check` remains blocked by pre-existing repository-wide format
  drift; no formatter rewrite was applied.
- **Task 3 — complete.** C smoke coverage is committed as `0a567c8`.
- Focused C verification passes: `c_xrecord_compiles_links_and_runs` **1/1**.
- **Task 4 — complete.** Focused Rust/C, full FFI, and all-features gates pass;
  final review verdict is **READY**.
- Verification totals include P4b2 Rust **4/4**, C smoke **1/1**, FFI release
  suites **1289/1289** plus ABI **57/57**, and all-features **1333/1333**.
- Three non-blocking review minors are recorded in the ledger for future test
  hardening; no product blocker remains.

## Global Constraints

- Work directly in the current workspace; do not create a worktree.
- Export only the six P4b2 XRECORD functions listed below; XData/EED, ACIS, callbacks, cbindgen, and unrelated objects remain out of scope.
- No Rust `Vec` address, temporary object pointer, or Rust-owned allocation crosses the ABI.
- `DwgXrecordValue` uses fixed-width scalar slots, three `double` point slots, a caller-owned `void *data`, and `size_t data_len`.
- Tags are fixed: `DWG_XRECORD_STRING=1`, `DOUBLE=2`, `INT16=3`, `INT32=4`, `INT64=5`, `BYTE=6`, `BOOL=7`, `HANDLE=8`, `POINT3D=9`, `CHUNK=10`.
- Unknown tags return `DWG_EINVAL`; known tag/group-code mismatches return `DWG_ETYPE`.
- Null pointers, invalid capacities, malformed input UTF-8, and non-finite input numeric values return `DWG_EINVAL`; missing XRECORD handles/indexes return `DWG_ENOTFOUND`; non-XRECORD handles return `DWG_ETYPE`; insufficient output storage returns `DWG_EBUFFER`.
- Failed calls preserve the document and set TLS; successful calls clear TLS.
- Handle/ObjectId values supplied to mutations must exist in the same document.
- Mutations set `entries_complete`, call `synchronize_object_references()` for codes 330–369, and disable preservation of the decoded object-reference stream.
- Keep `.superpowers/` untracked and commit each completed task separately.

## Public ABI

```c
typedef struct DwgXrecordValue {
    uint32_t tag;
    uint32_t reserved;
    int64_t integer;
    double real;
    double point[3];
    uint64_t handle;
    void *data;
    size_t data_len;
} DwgXrecordValue;

int dwg_xrecord_count(const DwgData *data, uint64_t xrecord, size_t *count);
int dwg_xrecord_find(const DwgData *data, uint64_t xrecord, int32_t code,
                     size_t occurrence, size_t *index);
int dwg_xrecord_entry(const DwgData *data, uint64_t xrecord, size_t index,
                      int32_t *code, DwgXrecordValue *value);
int dwg_xrecord_append(DwgData *data, uint64_t xrecord, int32_t code,
                       const DwgXrecordValue *value);
int dwg_xrecord_set(DwgData *data, uint64_t xrecord, size_t index,
                    int32_t code, const DwgXrecordValue *value);
int dwg_xrecord_remove(DwgData *data, uint64_t xrecord, size_t index,
                       int32_t *code, DwgXrecordValue *value);
```

For output, `entry` and `remove` overwrite `tag` and the selected payload;
for String/Chunk, `data_len` is capacity on entry and required/written length
on return. Numeric payloads use `integer`, `real`, `point`, or `handle` based
on `tag`. `reserved` must be zero for mutation inputs.

---

### Task 1: Add failing Rust ABI coverage

**Files:**
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes the P4b2 public ABI block and the existing `DwgData` lifecycle.
- Produces RED tests named `phase4b2_xrecord_argument_and_type_errors`, `phase4b2_xrecord_lookup_and_mutation`, `phase4b2_xrecord_buffers_and_roundtrip`, and `phase4b2_xrecord_foreign_handle_preserves_document`.

- [x] **Step 1: Add the XRECORD fixture**

Create `Phase4b2XrecordFixture` that creates a document, obtains an existing
object handle through the iteration ABI, casts only inside Rust test setup to
`FfiDocument`, calls `CadDocument::ensure_xrecord(owner, "P4B2_TEST")`, and
seeds entries with group codes 1, 40, 90, 10, and 310. Store the document,
XRECORD handle, and owner handle; destroy the document in `Drop`.

- [x] **Step 2: Add invalid-argument and typed-value tests**

Assert `DWG_EINVAL` for null documents, null outputs, nonzero `reserved`, null
data with nonzero data length on input, malformed UTF-8, and non-finite input
numeric values. Assert `DWG_EINVAL` for tag 99, `DWG_ETYPE` for a valid tag
paired with the wrong group code, `DWG_ENOTFOUND` for a foreign XRECORD and an
out-of-range index, and `DWG_ETYPE` for a document object that is not an
XRECORD. After each failed mutation, compare count and all seeded entries.

- [x] **Step 3: Add lookup, enumeration, and mutation assertions**

Assert count, `find` occurrence behavior, and `entry` decoding for String,
Double, Int32, Point3D, and Chunk. Append an Int64 and Bool entry, replace the
Double entry with a new finite value, then remove one entry and verify the
returned code/value and the shifted count. Assert successful calls clear TLS.

- [x] **Step 4: Add buffer and round-trip assertions**

Use `data == NULL` to query String and Chunk sizes, assert NUL-inclusive
string length, assert `DWG_EBUFFER` leaves the document unchanged, and read
the values into caller-owned buffers. Write the mutated document through
`dwg_write_bytes`, read it into a second document, resolve the same XRECORD
through its named dictionary entry, and verify all surviving entries and
object-reference values. Repeat the persistence check through the DXF file
ABI and reopen the file with the Rust API.

- [x] **Step 5: Run the focused tests and confirm RED**

```powershell
cargo tffi-abi phase4b2_ -- --nocapture --test-threads=1
```

Expected: compilation fails because the six XRECORD symbols and
`DwgXrecordValue` are not implemented yet.

- [x] **Step 6: Commit the failing Rust coverage**

```powershell
git diff --check
git add tests/abi_test.rs
git commit -m "test(ffi): add xrecord Rust ABI coverage"
```

### Task 2: Implement the typed XRECORD ABI

**Files:**
- Create: `src/ffi/xrecord.rs`
- Modify: `src/ffi/mod.rs`

**Interfaces:**
- Consumes `FfiDocument`, `CadDocument::objects`, `ObjectType::XRecord`, `XRecord`, `XRecordEntry`, `XRecordValue`, `XRecordValueType`, `Handle`, and existing TLS/string helpers.
- Produces the six exact functions in the Public ABI block and the public `DwgXrecordValue` plus ten tag constants.

- [x] **Step 1: Add the public C layout and private lookup/conversion helpers**

Define the exact `#[repr(C)]` structure and constants from the Public ABI
block. Add helpers that reject null `DwgData`, resolve only `ObjectType::XRecord`,
map missing handles to `DWG_ENOTFOUND`, map other object types to `DWG_ETYPE`,
and convert between the C value and `XRecordValue` without retaining a C
pointer. Validate group-code/tag compatibility with `XRecordValueType::from_code`.

- [x] **Step 2: Implement count, find, and entry output**

Implement `dwg_xrecord_count`, `dwg_xrecord_find`, and
`dwg_xrecord_entry`. `find` counts matching codes from zero and returns the
physical entry index. `entry` reads the stored code/value, writes the tag and
fixed-width payload, and handles String/Chunk size queries and NUL-terminated
copying entirely through caller storage.

- [x] **Step 3: Implement append and indexed replacement**

Implement `dwg_xrecord_append` and `dwg_xrecord_set`. Validate document-owned
handle references, tag/code compatibility, reserved bits, UTF-8, finite
numbers, and all caller buffers before borrowing the mutable XRECORD. Append
or replace exactly one entry, set `entries_complete = true`, and call
`synchronize_object_references()` when the affected code is 330–369.

- [x] **Step 4: Implement indexed removal**

Implement `dwg_xrecord_remove` with complete output validation before removal.
Remove exactly one entry, return its original code/value through caller-owned
storage, update the reference stream for 330–369, set `entries_complete`, and
clear TLS only after the removal succeeds.

- [x] **Step 5: Run the focused Rust suite and format check**

```powershell
cargo fmt --check
cargo tffi-abi phase4b2_ -- --nocapture --test-threads=1
cargo tffi-lib ffi::object::tests -- --nocapture
```

Expected: all P4b2 Rust tests and existing object tests pass.

- [x] **Step 6: Commit the ABI implementation**

```powershell
git diff --check
git add src/ffi/xrecord.rs src/ffi/mod.rs
git commit -m "feat(ffi): add xrecord typed access API"
```

### Task 3: Add C XRECORD smoke coverage

**Files:**
- Create: `tests/ffi/c_xrecord.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes the six XRECORD functions, the existing file/byte lifecycle API,
  and a Rust-generated DWG fixture path passed as `argv[1]`.
- Produces `c_xrecord_compiles_links_and_runs`, a real MinGW compile/link/run
  check for the typed value layout, caller buffers, mutations, and round-trip.

- [x] **Step 1: Write the C smoke program**

Declare the opaque `DwgData`, exact `DwgXrecordValue`, tag constants, and all
prototypes from the Public ABI block. Read the fixture path from `argv[1]`,
resolve the named dictionary and `P4B2_TEST`, enumerate seeded entries, append
an Int32, replace a Double, query/copy a String and Chunk, write bytes, read a
second document, verify the mutated entries, remove one entry, and destroy
both documents. Return distinct nonzero codes for every failed assertion.

- [x] **Step 2: Add the Rust C harness**

Add a `write_xrecord_smoke_dwg` helper that creates the seeded fixture used by
the C program. Add `c_xrecord_compiles_links_and_runs` beside the existing C
smoke tests, reusing `ffi_dll`, `c_runtime_command`, `$CC`/`gcc` lookup, and
temporary executable cleanup. Pass the fixture path as the C program's first
argument and include compiler stdout/stderr in failure messages.

- [x] **Step 3: Run the C smoke test**

```powershell
cargo tffi-abi c_xrecord_compiles_links_and_runs -- --exact --nocapture
```

Expected: the current release DLL links all six symbols and the executable
returns zero after the XRECORD round-trip.

- [x] **Step 4: Commit the C coverage**

```powershell
git diff --check
git add tests/ffi/c_xrecord.c tests/abi_test.rs
git commit -m "test(ffi): add xrecord C smoke test"
```

### Task 4: P4b2 verification and handoff

**Files:**
- Modify: none unless a P4b2 test exposes an implementation defect.

**Interfaces:**
- Consumes the committed P4b1 Dictionary API and P4b2 XRECORD ABI.
- Produces focused P4b2 evidence, complete FFI/all-features gates, and a clean
  product diff.

- [x] **Step 1: Run focused P4b2 and C tests**

```powershell
cargo tffi-abi phase4b2_ -- --nocapture --test-threads=1
cargo tffi-abi c_xrecord_compiles_links_and_runs -- --exact --nocapture
```

- [x] **Step 2: Run the complete FFI release gate**

```powershell
cargo tffi-all --quiet
```

- [x] **Step 3: Run the all-features gate**

```powershell
cargo tall --quiet
```

- [x] **Step 4: Check final state and review handoff**

```powershell
git diff --check
git status --short
git log --oneline -n 10
```

Product changes must be committed, `.superpowers/` must remain untracked,
and the final branch review must explicitly inspect the tagged value layout,
buffer ownership, reference synchronization, and cross-format persistence.
