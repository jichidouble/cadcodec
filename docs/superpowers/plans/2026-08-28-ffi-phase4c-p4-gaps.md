# FFI Phase 4c P4 Gap Implementation Plan

**Goal:** Finish the remaining P4 FFI gaps: object removal, notification
callbacks, and raw ACIS access.

**Architecture:** Extend `FfiDocument` and its stable boxed arena. Keep the
Rust `CadDocument` as the ownership root; never expose Rust collection or
allocator addresses to C. Implement each slice with focused Rust ABI coverage
followed by a C compile/link/run smoke test.

**Spec:** `docs/superpowers/specs/2026-08-28-ffi-phase4c-p4-gaps-design.md`

## Current Status (2026-08-28)

- **P4c1 complete** in `1797ba5`: stable object-removal tokens, entity/core
  index cleanup, Rust ABI coverage 2/2, and C smoke coverage 1/1.
- **P4c2 complete** in `059077d`: per-document notification callback,
  read-time dispatch, TLS validation, Rust coverage 1/1, and C smoke coverage
  1/1.
- **P4c3 complete** in `c5e3563`: SAT/SAB access for Solid3D, Region, Body,
  and Surface, Rust round-trip coverage 1/1, and C smoke coverage 1/1.
- **Final P4c verification complete**: release FFI cdylib build passed;
  combined Rust P4c coverage is 4/4 and C smoke coverage is 3/3. Export
  inspection confirms all new removal, callback, and ACIS symbols.
- Full FFI/all-features suites remain deferred to the final handoff because
  the repository's release test build is intentionally run on demand.

## Constraints

- Work in the current workspace; do not create a worktree.
- Preserve existing public signatures and handle semantics.
- Do not modify `Cargo.lock`.
- Use on-demand release tests; run the full suite only at final handoff.
- Keep deleted FFI arena records allocated until document destruction.
- Do not expose parsed ACIS Rust structures through C.

## Task 1: P4c1 object removal

**Files:** modify `src/ffi/object.rs`, `src/ffi/iterate.rs` or a new
`src/ffi/remove.rs`; add focused cases to `tests/abi_test.rs` and a C smoke
fixture if the existing harness supports it.

- Add failing tests for entity removal, ordinary object removal, null/foreign/
  dead pointers, iteration after removal, and failed-removal preservation.
- Add an arena `alive` state and relink helper. Keep object addresses stable,
  clear entity/object references after removal, and reject dead pointers in
  getters/mutators.
- Remove entities through the core removal API, clean block membership, and
  remove associated solid history where the entity type carries it.
- Remove ordinary objects by handle and preserve unrelated dictionary entries.
- Export `dwg_object_remove(data, obj) -> i32`; clear TLS only on success.
- Run the P4c1-filtered ABI tests and existing object/unit tests, then commit.

## Task 2: P4c2 notification callback

**Files:** modify `src/ffi/data.rs`, `src/ffi/io.rs`, `src/ffi/mod.rs`, and
`src/ffi/types.rs`; add `tests/abi_test.rs` coverage and a C callback smoke
test.

- Add the callback typedef-compatible Rust function pointer and per-document
  callback/user-data fields.
- Preserve the callback when a read replaces the document contents.
- Dispatch notifications after successful reads/writes, copying no caller
  memory into the document; clear the callback with `NULL`.
- Test callback type/message/user data, clear behavior, read replacement, and
  no callback on invalid arguments.
- Run focused callback tests and commit.

## Task 3: P4c3 ACIS access

**Files:** create `src/ffi/acis.rs`, modify `src/ffi/mod.rs`; add Rust ABI,
C smoke, and round-trip coverage.

- Add type checks for `Solid3D`, `Region`, `Body`, and `Surface`.
- Export raw SAT/SAB query, getter, and setter functions using the existing
  caller-owned buffer protocol and explicit binary/text operations.
- Validate UTF-8, finite sizes, null pointers, and output capacities before
  mutation; refresh the arena entity pointer after mutation.
- Verify SAT/SAB read-back and DWG/DXF persistence without changing handles or
  ownership.
- Run focused ACIS tests and commit.

## Task 4: Final verification and documentation

- Update the P4 progress record and API inventory with exact symbols.
- Run `git diff --check` and focused P4c tests.
- Run the release FFI ABI suite and all-features suite only for final handoff.
- Record any pre-existing formatting drift without applying a repository-wide
  formatter rewrite.
