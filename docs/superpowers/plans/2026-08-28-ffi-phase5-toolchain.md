# FFI Phase 5 Toolchain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish a cbindgen-generated C header for the existing FFI surface and validate it with targeted Windows GNU/MinGW and MSVC builds.

**Architecture:** Keep `src/ffi` as the Rust ABI source of truth. Use a cbindgen allowlist to generate a checked-in `include/dwg_api.h`, excluding Rust arena/document internals. PowerShell tools provide reproducible generation, drift checking, and compiler-specific C smoke execution; GitHub Actions invokes the same checks for both Windows toolchains.

**Tech Stack:** Rust 2021, cbindgen 0.29.0, C11-compatible C compiler, PowerShell, GitHub Actions, MinGW GCC, MSVC `cl.exe`.

**Spec:** `docs/superpowers/specs/2026-08-28-ffi-phase5-toolchain-design.md`

## Global Constraints

- The public C ABI remains limited to existing `dwg_*`/`dxf_*` exports and C-compatible FFI value types.
- `FfiDocument`, `FfiObject`, `CadDocument`, and Rust entity/object internals must not appear in `include/dwg_api.h`.
- Do not add FFI behavior or change document/object ownership semantics.
- Pin the cbindgen executable to `0.29.0`; do not add it to runtime dependencies or modify `Cargo.lock` for this tool-only change.
- Normal verification uses targeted FFI/header checks; the full slow test suite remains explicit.
- Work directly on the current branch; stage and commit each completed task.

---

### Task 1: Define the cbindgen boundary and generator command

**Files:**
- Create: `cbindgen.toml`
- Create: `tools/generate_ffi_header.ps1`
- Create: `include/` and generated `include/dwg_api.h`

**Interfaces:**
- Consumes: the FFI module boundary at `src/ffi/mod.rs`.
- Produces: `include/dwg_api.h` and a `-Check` mode that returns nonzero when regeneration differs.

- [x] **Step 1: Write the cbindgen configuration**

Use C output, C++ compatibility guards, `stddef.h`/`stdint.h`,
`usize_is_size_t = true`. Run cbindgen against `src/ffi/mod.rs`, not the
library root, so unrelated public Rust APIs are outside the parsed boundary.
Use these cbindgen settings:

```toml
language = "C"
cpp_compat = true
pragma_once = true
include_guard = "ACADRUST_DWG_API_H"
documentation = true
usize_is_size_t = true
sys_includes = ["stddef.h", "stdint.h"]

```

The generated header must still include opaque handles, both payload structs, both
FFI enums, the callback typedef, all FFI constants, and every exported
function.

- [x] **Step 2: Implement reproducible generation**

Implement `tools/generate_ffi_header.ps1` with:

```powershell
param([switch]$Check)
```

Resolve the repository root from `$PSScriptRoot`, require cbindgen on `PATH`,
and print `cargo install cbindgen --version 0.29.0 --locked` when it is
missing. Normal mode must run:

```powershell
cbindgen --config cbindgen.toml src/ffi/mod.rs --output include/dwg_api.h
```

`-Check` must generate to a temporary file, compare it byte-for-byte with the
checked-in header, return 1 on mismatch, and clean up in `finally`.

- [x] **Step 3: Generate and inspect the header**

Run:

```powershell
cargo install cbindgen --version 0.29.0 --locked
powershell -File tools/generate_ffi_header.ps1
rg -n "FfiDocument|FfiObject|CadDocument|EntityType|ObjectType" include/dwg_api.h
```

Expected: generation succeeds and the leakage scan finds no internal type.
If cbindgen emits an unsupported declaration, adjust only the config or the
public C-compatible declarations; do not hand-edit the generated header.

- [x] **Step 4: Verify and commit**

Run `powershell -File tools/generate_ffi_header.ps1 -Check` and `git diff --check`.
Commit with `feat(ffi): add reproducible public header generation`.

---

### Task 2: Add a C consumer smoke test

**Files:**
- Create: `tests/ffi/c_header.c`
- Create: `tools/run_ffi_header_smoke.ps1`

**Interfaces:**
- Consumes: `include/dwg_api.h`, a release DLL, and its matching import library.
- Produces: a compiler-independent exit status proving header compile, link, and runtime calls.

- [x] **Step 1: Write the C consumer without manual declarations**

Create `tests/ffi/c_header.c` with this behavior:

```c
#include "dwg_api.h"

static void callback(int code, const char *message, void *user_data) {
    (void)code; (void)message; (void)user_data;
}

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != DWG_OK || data == NULL) return 1;
    if (dwg_set_notification_callback(data, callback, NULL) != DWG_OK) return 2;
    if (dwg_set_notification_callback(data, NULL, (void *)1) != DWG_OK) return 3;
    dwg_data_destroy(data);
    return 0;
}
```

If cbindgen uses a different C spelling for the optional callback, update the
callback declaration to match the generated header; keep all function
declarations sourced exclusively from `dwg_api.h`.

- [x] **Step 2: Implement compiler-specific execution**

Create `tools/run_ffi_header_smoke.ps1` with:

```powershell
param(
  [ValidateSet("gcc", "cl")] [string]$Compiler,
  [Parameter(Mandatory = $true)] [string]$Dll,
  [Parameter(Mandatory = $true)] [string]$ImportLibrary
)
```

Compile with `-I include` and GCC DLL/import-library arguments, or `/Iinclude`
and the import library for `cl`. Use a unique temp directory, prepend the DLL
directory to `PATH`, propagate compiler/program failures, validate paths, and
clean up in `finally`.

- [x] **Step 3: Run the GNU smoke test**

Run:

```powershell
cargo build --release --features ffi
powershell -File tools/run_ffi_header_smoke.ps1 `
  -Compiler gcc `
  -Dll target/x86_64-pc-windows-gnu/release/acadrust.dll `
  -ImportLibrary target/x86_64-pc-windows-gnu/release/libacadrust.dll.a
```

Expected: compile, link, and runtime exit code 0.

- [x] **Step 4: Verify and commit**

Run `git diff --check` and commit with `test(ffi): compile C consumer through generated header`.

---

### Task 3: Add fast header contract and drift guards

**Files:**
- Create: `tests/ffi_header.rs`
- Modify: `.cargo/config.toml` only if a focused alias is useful

**Interfaces:**
- Consumes: the checked-in header and generator `-Check` mode.
- Produces: fast Rust checks for required declarations and internal-type leakage.

- [x] **Step 1: Add the public-header assertions**

Read `include/dwg_api.h` and assert required declarations and forbidden names:

```rust
assert!(header.contains("typedef struct DwgData DwgData"));
assert!(header.contains("typedef struct DwgObject DwgObject"));
assert!(header.contains("dwg_data_create"));
assert!(header.contains("dwg_object_remove"));
assert!(header.contains("dwg_set_notification_callback"));
assert!(header.contains("dwg_entity_acis_set_sat"));
assert!(header.contains("DWG_OK"));
assert!(header.contains("DWG_XDATA_STRING"));
assert!(header.contains("DWG_XRECORD_STRING"));
for forbidden in ["FfiDocument", "FfiObject", "CadDocument", "EntityType"] {
    assert!(!header.contains(forbidden), "internal type leaked: {forbidden}");
}
```

Keep assertions semantic rather than dependent on cbindgen whitespace.

- [x] **Step 2: Run the focused test**

Run `cargo test --release --features ffi --test ffi_header`. It must pass after
Task 1 and fail if a required declaration or forbidden-name guard is broken.

- [x] **Step 3: Verify and commit**

No cargo alias was needed; the focused command is run directly:

```powershell
cargo test --release --features ffi --test ffi_header
```

Run the focused test, the generator `-Check` command, and `git diff --check`.
Commit with `test(ffi): guard generated header contract`.

---

### Task 4: Add Windows GNU and MSVC CI

**Files:**
- Create: `.github/workflows/ffi.yml`

**Interfaces:**
- Consumes: Tasks 1–3 scripts/header/tests and the Rust `ffi` feature.
- Produces: CI coverage for Windows GNU/MinGW and Windows MSVC.

- [x] **Step 1: Define the GNU job**

On `windows-latest`, install target `x86_64-pc-windows-gnu` and cbindgen
`0.29.0`, then run:

```powershell
rustfmt --edition 2021 --check src/ffi/types.rs tests/ffi_header.rs
powershell -File tools/generate_ffi_header.ps1 -Check
cargo build --release --target x86_64-pc-windows-gnu --features ffi
cargo test --release --target x86_64-pc-windows-gnu --features ffi --test ffi_header
powershell -File tools/run_ffi_header_smoke.ps1 -Compiler gcc `
  -Dll target/x86_64-pc-windows-gnu/release/acadrust.dll `
  -ImportLibrary target/x86_64-pc-windows-gnu/release/libacadrust.dll.a
```

- [x] **Step 2: Define the MSVC job**

Use the default MSVC toolchain, build/test with
`--target x86_64-pc-windows-msvc`, and run the same format/drift checks. Invoke
the smoke script with `-Compiler cl`, the target DLL, and
`target/x86_64-pc-windows-msvc/release/acadrust.dll.lib` or the exact import
library name emitted by Cargo (record the observed name in the workflow).

- [x] **Step 3: Keep CI targeted and commit**

Do not run `cargo test --all-features` or the complete slow FFI matrix here.
The repository-wide `cargo fmt --check` currently reports pre-existing drift
outside this phase, so CI gates only the touched FFI Rust files with the
FFI-scoped `rustfmt --check` command. Validate workflow YAML locally if a
parser is available, run the GNU commands locally, inspect target paths, and commit with
`ci(ffi): validate header on gnu and msvc`.

---

### Task 5: Document the public C API

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: `include/dwg_api.h`, the `ffi` feature, and Tasks 1–4 commands.
- Produces: matching consumer and verification documentation.

- [x] **Step 1: Add the FFI usage section**

Document the header path, cbindgen version, `--features ffi`, opaque handle
semantics, caller-owned buffers, and callback message lifetime. Include:

```c
#include "dwg_api.h"

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != DWG_OK) return 1;
    dwg_data_destroy(data);
    return 0;
}
```

- [x] **Step 2: Document fast commands**

Include:

```powershell
cargo build --release --features ffi
powershell -File tools/generate_ffi_header.ps1
powershell -File tools/generate_ffi_header.ps1 -Check
cargo test --release --features ffi --test ffi_header
```

State that the full suite is intentionally separate because it is slow, and
link the GNU/MSVC workflow for compiler-specific linking.

- [x] **Step 3: Verify and commit**

Run `git diff --check`, confirm every documented path exists, and commit with
`docs(ffi): document public C header usage`.

---

### Task 6: Final targeted verification and handoff

**Files:**
- Modify: only Task 1–5 files if a scoped verification defect is found.

- [x] **Step 1: Check scope**

Run `git status --short` and `git diff --check`. Confirm no unrelated files or
`Cargo.lock` changes exist.

- [x] **Step 2: Run header checks**

Run:

```powershell
powershell -File tools/generate_ffi_header.ps1 -Check
cargo test --release --features ffi --test ffi_header
rustfmt --edition 2021 --check src/ffi/types.rs tests/ffi_header.rs
```

- [x] **Step 3: Run available compiler smoke tests**

Run the GNU build/smoke commands from Task 2. Run the MSVC commands locally
when `cl.exe` is available; otherwise record MSVC as CI-only and do not claim
local MSVC verification.

- [x] **Step 4: Report evidence**

Report exact targeted commands, results, existing non-blocking warnings, and
the task commit list. Do not claim the full suite passed unless it was run.

## Self-Review Checklist

- Every P5 spec requirement maps to Tasks 1–5.
- No step relies on a vague placeholder or unspecified error handling.
- The allowlist and leakage assertions protect the ABI/ownership boundary.
- GNU and MSVC use target-specific DLL/import-library paths.
- No task requires the slow full test suite.
