# FFI Phase 5: Public C Header and Toolchain Validation

## Context

The FFI implementation now exposes the Phase 1–4 lifecycle, I/O, entity,
table, mutation, object, XRECORD, XDATA, callback, and ACIS APIs. Consumers
still have to reproduce declarations manually in C tests because the
repository does not publish a generated header or validate the ABI on both
Windows toolchains. Phase 5 makes the existing ABI consumable and testable;
it does not add new CAD behavior or change ownership semantics.

## Goals

- Publish a checked-in `include/dwg_api.h` generated from the Rust FFI API.
- Expose only the supported C ABI: opaque handles, C-compatible value types,
  constants, callback declarations, and `dwg_*`/`dxf_*` functions.
- Prevent internal Rust implementation types such as `FfiDocument` and
  `FfiObject` from appearing in the public header.
- Detect header drift in local verification and CI.
- Compile and link a representative C consumer with both MinGW/GNU and MSVC.
- Document build, header usage, and the targeted verification commands.

## Non-Goals

- No new FFI functions or setter coverage beyond the completed P4 scope.
- No change to `CadDocument` storage, stable object pointers, or callback
  ownership/lifetime rules.
- No mandatory execution of the full slow test corpus in the normal FFI job.
- No C++ wrapper, package-manager artifact, or cross-platform ABI promise in
  this phase; the supported CI targets are Windows GNU and Windows MSVC.

## Design

### Public header boundary

The FFI root remains the source of truth for exported Rust symbols. cbindgen
runs directly against `src/ffi/mod.rs`, which is the module boundary for the
FFI declarations and avoids traversing the unrelated public library root.
`cbindgen.toml` defines the output policy without a symbol-by-symbol exclusion
list.
The generated header must use standard C headers (`stdint.h`, `stddef.h`, and
`stdbool.h` only when required), `extern "C"` guards, opaque declarations for
`DwgData` and `DwgObject`, and stable integer-width types. Rust-only arena
records and document internals are excluded rather than made part of the ABI.

`include/dwg_api.h` is checked in so downstream C consumers can include it
without installing Rust. A documented generator command is the only supported
way to update it.

### Drift and compatibility checks

The repository adds a repeatable header-check command that regenerates the
header in a temporary location and fails when the checked-in file differs.
A C smoke test includes the checked-in header and exercises representative
lifecycle, version, iteration, mutation, and P4 APIs. The existing Rust ABI
and exported-symbol inventory remains complementary: the header check catches
declaration drift, while the linker inventory catches missing DLL symbols.

### Toolchain validation

GitHub Actions uses two Windows jobs. The GNU job builds the release `ffi`
cdylib with MinGW and runs the header smoke test. The MSVC job builds the same
feature set with the default MSVC target and compiles/links the same C test.
Both jobs run formatting and targeted FFI checks; the full suite remains an
explicit local or release-validation command because it is slow.

### Documentation

README documentation will show the generated-header location, feature flag,
platform/library naming, a minimal `DwgData` lifecycle example, and the
targeted commands for regenerating and validating the header.

## Acceptance Criteria

1. `include/dwg_api.h` is generated from the current FFI declarations and
   contains no internal Rust arena/document types.
2. Regeneration produces no diff in a clean checkout.
3. A C source including only `dwg_api.h` compiles, links to the release
   library, and runs successfully on MinGW and MSVC.
4. CI validates both Windows toolchains with the `ffi` feature.
5. Existing targeted Rust ABI, C smoke, and inventory tests continue to pass.
6. README instructions are sufficient for a consumer to build and call the
   lifecycle API without copying declarations from test files.

## Risks and Decisions

- `cbindgen` is not currently installed locally, so the plan must pin the
  generator version and make CI installation reproducible.
- The existing FFI modules expose some Rust implementation items publicly;
  the header allowlist is therefore mandatory, and a generated-header scan is
  required to prevent accidental leakage.
- Windows GNU and MSVC must use fixed-width C signatures and opaque pointers;
  platform-dependent `long`, Rust layout-dependent structs, and implicit
  ownership transfers are out of scope.
