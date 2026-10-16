# FFI Phase 4a Byte API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe in-memory DWG read/write functions to the C ABI with explicit ownership and allocator rules.

**Architecture:** Reuse `DwgReader::from_stream(Cursor<...>)` and `DwgWriter::write_to_vec` so the core codecs remain unchanged. The FFI layer owns a private header-plus-payload allocation and returns only the payload pointer; `dwg_free_buffer` reconstructs the allocation from its private header. A successful byte read replaces the document only after parsing completes.

**Tech Stack:** Rust 2021, `extern "C"` ABI, existing `ffi` feature, Rust integration tests, MinGW C smoke tests.

**Spec:** `docs/superpowers/specs/2026-08-21-ffi-phase4a-bytes-design.md`

## Global Constraints

- Export exactly `dwg_read_bytes`, `dwg_write_bytes`, and `dwg_free_buffer` in P4a.
- `dwg_read_bytes` accepts only a non-NULL input with `input_len > 0`; malformed DWG returns `DWG_EREAD` and leaves the existing document unchanged.
- `dwg_write_bytes` requires non-NULL `data`, `output`, and `output_len`; it clears both output values before attempting the write.
- The caller releases successful output only with `dwg_free_buffer`; the buffer is never released with C `free`.
- Preserve the existing TLS error contract: success clears the last error, failure stores the matching code and a diagnostic message.
- Do not add DXF byte APIs, Dictionary/XRECORD/EED APIs, callbacks, cbindgen, or unrelated codec changes.
- Work directly in the current workspace; keep each completed task in a focused Conventional Commit.

---

### Task 1: Private FFI buffer allocator

**Files:**
- Create: `src/ffi/buffer.rs`
- Modify: `src/ffi/mod.rs`
- Test: `src/ffi/buffer.rs` unit tests

**Interfaces:**
- Consumes: `Vec<u8>` produced by the DWG writer.
- Produces: `pub(crate) fn into_ffi_buffer(bytes: Vec<u8>) -> Result<(*mut u8, usize), i32>` and `pub(crate) unsafe fn free_ffi_buffer(ptr: *mut c_void)` for `io.rs`.

- [ ] **Step 1: Write the failing allocator tests**

  Add tests for a non-empty payload and an empty payload. The tests must read the
  returned bytes before freeing them, assert the returned length, call
  `free_ffi_buffer` exactly once, and verify that `free_ffi_buffer(NULL)` is a
  no-op. Add an allocation-overflow helper test if the chosen `Layout` calculation
  exposes a deterministic overflow path.

- [ ] **Step 2: Run the focused tests and confirm failure**

  Run: `cargo test --release --features ffi buffer::tests -- --nocapture`

  Expected: FAIL because the allocator functions do not exist yet.

- [ ] **Step 3: Implement the header-plus-payload allocation**

  Define a private `#[repr(C)] struct BufferHeader { magic: u64, len: usize }` and a
  private magic constant. Compute one `Layout` for the header followed by `len`
  bytes, allocate with `std::alloc::alloc`, copy the vector payload after the
  header, and return `(payload_ptr, len)`. Return `DWG_ENOMEM` for layout overflow
  or a null allocation. In `free_ffi_buffer`, accept NULL, recover the header by
  the same payload offset, validate the magic, and deallocate with the original
  layout. Keep the header and magic private to this module.

- [ ] **Step 4: Run the focused tests and verify success**

  Run: `cargo test --release --features ffi buffer::tests -- --nocapture`

  Expected: all allocator tests pass, including empty payload and NULL release.

- [ ] **Step 5: Commit the allocator boundary**

  ```powershell
  git add src/ffi/buffer.rs src/ffi/mod.rs
  git commit -m "feat(ffi): add owned byte buffer allocator"
  ```

### Task 2: DWG byte read/write ABI

**Files:**
- Modify: `src/ffi/io.rs`
- Test: `src/ffi/io.rs` unit tests and `tests/abi_test.rs`

**Interfaces:**
- Consumes: `FfiDocument`, `DwgReader::from_stream`, `DwgWriter::write_to_vec`, the Task 1 allocator, and the existing TLS error helpers.
- Produces: `dwg_read_bytes(const uint8_t*, size_t, DwgData*) -> i32`,
  `dwg_write_bytes(const DwgData*, uint8_t**, size_t*) -> i32`, and
  `dwg_free_buffer(void*)` with `extern "C"` ABI.

- [ ] **Step 1: Add failing ABI tests for argument and state rules**

  In `tests/abi_test.rs`, add tests that:

  - reject NULL input, zero input length, NULL data, NULL output, and NULL output
    length with `DWG_EINVAL`;
  - write a valid in-memory DWG and assert non-NULL output plus positive length;
  - read that output into a document and assert the AC1032 version;
  - feed malformed bytes and assert `DWG_EREAD`, a non-empty TLS message, and that
    the document version remains unchanged;
  - verify a successful read or write clears the TLS error.

  In the `io.rs` test module, construct an `FfiDocument` with
  `DxfVersion::Unknown`, call `dwg_write_bytes`, and assert `DWG_EWRITE`, a NULL
  output, length zero, and a matching TLS error. This gives a deterministic write
  failure without changing production codec behavior.

- [ ] **Step 2: Run the focused tests and confirm failure**

  Run: `cargo test --release --features ffi phase4a_ -- --nocapture`

  Expected: FAIL because the three ABI functions are not exported yet.

- [ ] **Step 3: Implement `dwg_read_bytes`**

  Add a `read_bytes_impl` helper in `src/ffi/io.rs`. Validate `input`, length, and
  `data` before constructing `Cursor::new(slice)`. Call
  `crate::io::dwg::DwgReader::from_stream(cursor).read()`. Only on `Ok(doc)` call
  the existing `replace_document`; on `Err(e)`, set `DWG_EREAD` with `format!("{e}")`
  and leave the existing `FfiDocument` untouched. Clear TLS on success.

- [ ] **Step 4: Implement `dwg_write_bytes` and `dwg_free_buffer`**

  Validate all three output-related pointers. Set `*output = NULL` and
  `*output_len = 0` before accessing the document. Call
  `DwgWriter::write_to_vec(&ffi.doc)`, map codec errors to `DWG_EWRITE`, then pass
  the successful vector to `into_ffi_buffer`. Map allocator failure to
  `DWG_ENOMEM`. On success store the payload pointer and length and clear TLS.
  Export `dwg_free_buffer` as a thin unsafe wrapper around `free_ffi_buffer`.

- [ ] **Step 5: Run focused Rust tests and verify success**

  Run: `cargo test --release --features ffi phase4a_ -- --nocapture`

  Expected: all byte ABI argument, round-trip, failure-preservation, and
  allocator-error tests pass.

- [ ] **Step 6: Commit the Rust ABI implementation**

  ```powershell
  git add src/ffi/io.rs tests/abi_test.rs
  git commit -m "feat(ffi): add DWG byte read write API"
  ```

### Task 3: C byte API smoke test

**Files:**
- Create: `tests/ffi/c_bytes.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: the three exported byte functions, `dwg_data_create`,
  `dwg_data_destroy`, and `dwg_data_version`.
- Produces: a real C compile/link/run check for pointer ownership and the ABI
  signatures.

- [ ] **Step 1: Write the C smoke test**

  Declare the opaque `DwgData` type and these exact prototypes:

  ```c
  extern int dwg_data_create(DwgData **out);
  extern void dwg_data_destroy(DwgData *data);
  extern int dwg_data_version(const DwgData *data, char *buf, size_t cap, size_t *needed);
  extern int dwg_read_bytes(const uint8_t *input, size_t input_len, DwgData *data);
  extern int dwg_write_bytes(const DwgData *data, uint8_t **output, size_t *output_len);
  extern void dwg_free_buffer(void *buffer);
  ```

  The program creates a document, writes bytes, checks that the pointer and length
  are valid, creates a second document, reads the bytes, checks version `AC1032`,
  frees the output through `dwg_free_buffer`, calls the NULL-free no-op, and
  destroys both documents. Return distinct nonzero values for each failed check.

- [ ] **Step 2: Add the Rust C harness test**

  Add `c_bytes_compiles_links_and_runs` to `tests/abi_test.rs`. Follow the existing
  C smoke harness: locate the release FFI DLL, compile `tests/ffi/c_bytes.c` with
  the repository's MinGW C compiler and `-L target/release -lacadrust`, run the
  executable through `c_runtime_command`, and clean the temporary executable and
  source on both success and failure paths.

- [ ] **Step 3: Run the C smoke test**

  Run: `cargo test --release --features ffi c_bytes_compiles_links_and_runs -- --nocapture`

  Expected: C compilation, linker symbol resolution, process execution, and
  allocator release all pass.

- [ ] **Step 4: Commit the C coverage**

  ```powershell
  git add tests/ffi/c_bytes.c tests/abi_test.rs
  git commit -m "test(ffi): add byte API C smoke test"
  ```

### Task 4: P4a verification and handoff

**Files:**
- Modify: none unless a test exposes an implementation defect
- Test: existing FFI and full repository suites

**Interfaces:**
- Consumes: the committed P4a byte API and C smoke coverage.
- Produces: verified P4a completion evidence and a clean product diff.

- [ ] **Step 1: Run focused byte and C tests**

  ```powershell
  cargo test --release --features ffi phase4a_ -- --nocapture
  cargo test --release --features ffi c_bytes_compiles_links_and_runs -- --nocapture
  ```

  Expected: all P4a Rust and C checks pass.

- [ ] **Step 2: Run the release FFI suite**

  Run: `cargo test --release --features ffi`

  Expected: the full FFI-enabled test suite passes.

- [ ] **Step 3: Run the all-features regression suite**

  Run: `cargo test --all-features`

  Expected: all existing feature combinations and C harnesses pass, including the
  DLL path handling used by prior smoke tests.

- [ ] **Step 4: Check the final diff and commit state**

  Run:

  ```powershell
  git diff --check
  git status --short
  git log --oneline -n 5
  ```

  Expected: no whitespace errors and no uncommitted product changes. Existing
  `.superpowers/` process notes remain intentionally untracked. Do not run a
  repository-wide formatter that would rewrite unrelated pre-existing drift.

