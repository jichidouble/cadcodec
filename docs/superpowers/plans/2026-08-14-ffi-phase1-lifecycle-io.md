# FFI Phase 1 生命周期与读写 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 打通 C 消费者创建文档、读 DWG/DXF、写 DWG/DXF、查询错误的最小闭环。

**Architecture:** opaque `DwgData*` 句柄指向 `Box<CadDocument>`；FFI 层维护线程本地错误槽；字符串/字节用 caller-owned buffer；错误码 int 返回。

**Tech Stack:** Rust 2021、cc crate（C 集成测试）、cbindgen（P5 才引入）。

## Global Constraints

- 工具链：同时支持 x86_64-pc-windows-msvc（默认）与 x86_64-pc-windows-gnu。
- 前缀：DWG/DXF 通用用 `dwg_`，DXF 专有用 `dxf_`；替换现有 `acadrust_*` 导出。
- 错误处理：int 返回码 + 线程本地 `dwg_last_error()`。
- 内存：caller-owned buffer 优先；跨堆字节数组用 `dwg_free_buffer`；`DwgData*` 是唯一所有权根。
- 空指针防御：所有 FFI 函数对 NULL 句柄/输出参数返回 `DWG_EINVAL`。
- 提交：本地提交，不推送（仓库已无远程）。

---

## File Structure

- Create: `src/ffi/mod.rs` — 模块入口，`pub use` 各子模块
- Create: `src/ffi/types.rs` — opaque 类型、错误码常量、`DwgObjectType` 枚举
- Create: `src/ffi/error.rs` — 线程本地错误槽
- Create: `src/ffi/data.rs` — `dwg_data_create/destroy`、`dwg_data_version`
- Create: `src/ffi/io.rs` — `dwg_read_file/dwg_write_file/dxf_read_file/dxf_write_file`
- Modify: `src/lib.rs` — `pub mod ffi` 改为引用 `ffi/mod.rs`；删除旧 `src/ffi.rs`
- Modify: `Cargo.toml` — 保留 `ffi` feature
- Test: `tests/ffi/abi_test.rs` — Rust 直接调用 extern "C" 符号
- Test: `tests/ffi/c_smoke.c` — C 集成测试（cc crate 编译执行）

## Interfaces

Produces（后续任务依赖）:

```c
typedef struct DwgData DwgData;   /* opaque */
typedef struct DwgObject DwgObject; /* opaque（P2 使用） */

int dwg_data_create(DwgData **out);
void dwg_data_destroy(DwgData *data);
int dwg_read_file(const char *path, DwgData *data);
int dwg_write_file(const char *path, const DwgData *data);
int dxf_read_file(const char *path, DwgData *data);
int dxf_write_file(const char *path, const DwgData *data);
int dwg_last_error(char *buf, size_t cap, size_t *needed);
int dwg_last_error_code(void);
int dwg_data_version(const DwgData *data, char *buf, size_t cap, size_t *needed);
```

错误码（`src/ffi/types.rs` 与 C 头文件一致）:

```c
#define DWG_OK         0
#define DWG_EINVAL     1
#define DWG_ENOMEM     2
#define DWG_EREAD      3
#define DWG_EWRITE     4
#define DWG_EVERSION   5
#define DWG_ENOTFOUND  6
#define DWG_EBUFFER    7
#define DWG_ETYPE      8
```

---

### Task 1: 基础类型、错误码、错误槽

**Files:**
- Create: `src/ffi/mod.rs`
- Create: `src/ffi/types.rs`
- Create: `src/ffi/error.rs`
- Modify: `src/lib.rs`
- Delete: `src/ffi.rs`
- Test: `tests/ffi/abi_test.rs`

**Interfaces:**
- Consumes: 无
- Produces: `DwgData`/`DwgObject` opaque 声明、`DWG_*` 常量、`set_last_error`/`take_last_error` 内部函数

- [ ] **Step 1: 写失败测试**

创建 `tests/ffi/abi_test.rs`：

```rust
use std::ffi::CString;
use acadrust::ffi::*;

#[test]
fn last_error_is_empty_by_default() {
    let mut buf = [0u8; 64];
    let mut needed = 0usize;
    let rc = unsafe { dwg_last_error(buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, 1);
    assert_eq!(buf[0], 0);
}

#[test]
fn error_slot_roundtrip() {
    unsafe { set_last_error(DWG_EREAD, "bad read") };
    let mut buf = [0u8; 32];
    let mut needed = 0usize;
    let code = unsafe { dwg_last_error_code() };
    let rc = unsafe { dwg_last_error(buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(code, DWG_EREAD);
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"bad read");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --features ffi --test ffi_abi_test`
Expected: 编译失败（`acadrust::ffi::*` 不存在或 `DWG_OK` 未定义）

- [ ] **Step 3: 实现基础类型与错误槽**

`src/ffi/types.rs`：

```rust
#[repr(C)] pub struct DwgData { _priv: [u8; 0] }
#[repr(C)] pub struct DwgObject { _priv: [u8; 0] }

pub const DWG_OK: i32 = 0;
pub const DWG_EINVAL: i32 = 1;
pub const DWG_ENOMEM: i32 = 2;
pub const DWG_EREAD: i32 = 3;
pub const DWG_EWRITE: i32 = 4;
pub const DWG_EVERSION: i32 = 5;
pub const DWG_ENOTFOUND: i32 = 6;
pub const DWG_EBUFFER: i32 = 7;
pub const DWG_ETYPE: i32 = 8;
```

`src/ffi/error.rs`：

```rust
use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<Option<(i32, String)>> = const { RefCell::new(None) };
}

pub(crate) fn set_last_error(code: i32, msg: &str) {
    LAST_ERROR.with(|s| *s.borrow_mut() = Some((code, msg.to_string())));
}

pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|s| *s.borrow_mut() = None);
}

#[no_mangle]
pub extern "C" fn dwg_last_error(buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let (code, msg) = LAST_ERROR.with(|s| s.borrow().clone()).unwrap_or((0, String::new()));
    let bytes = msg.as_bytes();
    let total = bytes.len() + 1;
    unsafe { *needed = total; }
    if buf.is_null() { return DWG_OK; }
    if cap < total { return DWG_EBUFFER; }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
    code
}

#[no_mangle]
pub extern "C" fn dwg_last_error_code() -> i32 {
    LAST_ERROR.with(|s| s.borrow().as_ref().map(|(c, _)| *c).unwrap_or(DWG_OK))
}
```

`src/ffi/mod.rs`：

```rust
pub mod types;
pub mod error;
pub use types::*;
```

`src/lib.rs` 修改：删除 `#[cfg(feature = "ffi")] pub mod ffi;` 保留并指向新模块；删除旧 `src/ffi.rs`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --features ffi --test ffi_abi_test`
Expected: 2 passed

- [ ] **Step 5: 提交**

```powershell
git add src/ffi src/lib.rs tests/ffi/abi_test.rs Cargo.toml
git rm src/ffi.rs
git commit -m "feat(ffi): opaque types, error codes, thread-local error slot"
```

---

### Task 2: 文档生命周期与版本查询

**Files:**
- Create: `src/ffi/data.rs`
- Modify: `src/ffi/mod.rs`
- Test: `tests/ffi/abi_test.rs`

**Interfaces:**
- Consumes: Task 1 的 opaque 类型与错误槽
- Produces: `dwg_data_create`/`dwg_data_destroy`/`dwg_data_version`

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn data_create_destroy_and_version() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    let rc = unsafe { dwg_data_create(&mut data) };
    assert_eq!(rc, DWG_OK);
    assert!(!data.is_null());
    let mut buf = [0u8; 16];
    let mut needed = 0usize;
    let rc = unsafe { dwg_data_version(data, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"UNKNOWN");
    unsafe { dwg_data_destroy(data) };
}
```

- [ ] **Step 2: 运行测试确认失败**

Expected: `dwg_data_create` 未定义

- [ ] **Step 3: 实现**

`src/ffi/data.rs`：

```rust
use crate::document::CadDocument;
use super::types::*;
use super::error::*;
use std::ptr;

#[no_mangle]
pub extern "C" fn dwg_data_create(out: *mut *mut DwgData) -> i32 {
    if out.is_null() { return DWG_EINVAL; }
    let data = Box::new(CadDocument::new());
    unsafe { *out = Box::into_raw(data) as *mut DwgData; }
    clear_last_error();
    DWG_OK
}

#[no_mangle]
pub unsafe extern "C" fn dwg_data_destroy(data: *mut DwgData) {
    if data.is_null() { return; }
    drop(Box::from_raw(data as *mut CadDocument));
}

#[no_mangle]
pub unsafe extern "C" fn dwg_data_version(
    data: *const DwgData,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if data.is_null() || needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let doc = &*(data as *const CadDocument);
    let ver = doc.version.as_str();
    let bytes = ver.as_bytes();
    let total = bytes.len() + 1;
    *needed = total;
    if buf.is_null() { return DWG_OK; }
    if cap < total { return DWG_EBUFFER; }
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
    *buf.add(bytes.len()) = 0;
    DWG_OK
}
```

- [ ] **Step 4: 运行测试确认通过**

Expected: 通过

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/data.rs src/ffi/mod.rs tests/ffi/abi_test.rs
git commit -m "feat(ffi): data lifecycle and version query"
```

---

### Task 3: DWG/DXF 文件读写

**Files:**
- Create: `src/ffi/io.rs`
- Modify: `src/ffi/mod.rs`
- Test: `tests/ffi/abi_test.rs`、`tests/ffi/c_smoke.c`

**Interfaces:**
- Consumes: Task 1/2
- Produces: `dwg_read_file`/`dwg_write_file`/`dxf_read_file`/`dxf_write_file`

- [ ] **Step 1: 写失败测试**

`abi_test.rs`：

```rust
use std::ffi::CString;

#[test]
fn dwg_roundtrip_file() {
    let dir = std::env::temp_dir().join("acadrust_ffi_p1");
    std::fs::create_dir_all(&dir).unwrap();
    let src = CString::new(dir.join("src.dwg").to_str().unwrap()).unwrap();
    let dst = CString::new(dir.join("dst.dwg").to_str().unwrap()).unwrap();
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let rc = unsafe { dwg_read_file(src.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "read must succeed");
    let rc = unsafe { dwg_write_file(dst.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "write must succeed");
    unsafe { dwg_data_destroy(data) };
}
```

- [ ] **Step 2: 运行测试确认失败**

Expected: `dwg_read_file` 未定义

- [ ] **Step 3: 实现**

`src/ffi/io.rs`：用 `CStr` 转路径，`DwgReader::from_file(path).read()`，
失败时 `set_last_error(DWG_EREAD, &format!("{e}"))` 并返回错误码。写同理。
错误码映射：读失败 `DWG_EREAD`、写失败 `DWG_EWRITE`。

- [ ] **Step 4: C 集成测试**

`tests/ffi/c_smoke.c`：

```c
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct DwgData DwgData;
extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_last_error(char *buf, size_t cap, size_t *needed);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;
    char buf[256]; size_t n = 0;
    int rc = dwg_last_error(buf, sizeof buf, &n);
    dwg_data_destroy(data);
    return rc == 0 ? 0 : 2;
}
```

`build.rs` 或 `tests/ffi/abi_test.rs` 中用 `cc` crate 编译 `c_smoke.c` 并在 Rust
测试中运行（链接 cdylib 产物）。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --features ffi --test ffi_abi_test`
Expected: 全部通过；`c_smoke` 返回 0

- [ ] **Step 6: 提交**

```powershell
git add src/ffi/io.rs src/ffi/mod.rs tests/ffi/abi_test.rs tests/ffi/c_smoke.c
git commit -m "feat(ffi): DWG/DXF file read/write C API"
```

---

### Task 4: 移除旧 acadrust_* 导出并统一前缀

**Files:**
- Modify: `src/ffi/mod.rs`（迁移旧 `acadrust_version/parse_version` 到 `dwg_version`/`dwg_parse_version`）
- Test: `tests/ffi/abi_test.rs`

**Interfaces:**
- Consumes: Task 1-3
- Produces: `dwg_version()`、`dwg_parse_version()`

- [ ] **Step 1: 写失败测试**（断言 `dwg_version()` 返回 `acadrust-<ver>`，`dwg_parse_version("AC1032") == 8`）
- [ ] **Step 2: 运行确认失败**
- [ ] **Step 3: 实现**（迁移旧导出，删除 `acadrust_*` 符号）
- [ ] **Step 4: 运行确认通过**
- [ ] **Step 5: 提交**

---

## Self-Review

- Spec 覆盖：P1 生命周期/读写/错误码/线程本地错误/统一前缀 均有任务覆盖。
- 无占位符：所有步骤有具体代码与断言。
- 类型一致：`DwgData`/`DWG_*` 在 Task 1 定义，后续任务引用一致。

## Execution Handoff

P1 完成后，P2（遍历与基础实体）按同一模板写独立 plan。
