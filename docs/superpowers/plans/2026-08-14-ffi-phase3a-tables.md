# FFI Phase 3a 符号表访问 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 C 消费者按索引/名称访问 Layer、Block、Style、DimStyle 符号表（count/name/color/handle + Layer 颜色写入）。

**Architecture:** 表 API 走 `DwgData*` opaque 句柄 → `FfiDocument.doc` 的 `Table<T>`；索引是 `iter()` 顺序（不稳定，文档注明）；名称是稳定键。颜色按 spec 用 `int32_t` ARGB。

**Tech Stack:** Rust 2021、现有 ffi feature、MinGW gcc。

## Global Constraints

- 工具链：MSVC/GNU 双支持；`cargo test --features ffi` 全绿。
- 前缀：`dwg_`/`dxf_`；错误码 int + 线程本地 `dwg_last_error`。
- 内存：caller-owned buffer；空指针返回 `DWG_EINVAL`；表条目归文档所有，不单独 free。
- 颜色：`int32_t` ARGB（`Color::to_true_color_value`/`from_true_color_value`）。
- 索引：表迭代顺序，不稳定；名称是稳定键（API 文档注明）。
- 提交：本地，不推送。
- 执行：子代理派发不可用，controller 直接 TDD；进度记入 phase2-progress.md 同目录 `phase3a-progress.md`。

---

## File Structure

- Create: `src/ffi/table.rs` — 表访问（Layer + 通用 count/get_name）
- Modify: `src/ffi/mod.rs`
- Modify: `tests/abi_test.rs` — Layer/Block/Style/DimStyle 表测试
- Create: `tests/ffi/c_table.c` — C 集成测试

## Interfaces

```c
int dwg_layer_table_count(const DwgData *data, int *count);
int dwg_layer_get_name(const DwgData *data, int index, char *buf, size_t cap, size_t *needed);
int dwg_layer_get_color(const DwgData *data, int index, int32_t *color);
int dwg_layer_get_handle(const DwgData *data, int index, uint64_t *handle);
int dwg_layer_set_color(const DwgData *data, int index, int32_t color);
int dwg_layer_index_by_name(const DwgData *data, const char *name, int *index);

int dwg_block_table_count(const DwgData *data, int *count);
int dwg_style_table_count(const DwgData *data, int *count);
int dwg_dimstyle_table_count(const DwgData *data, int *count);
int dwg_table_entry_get_name(const DwgData *data, int table, int index, char *buf, size_t cap, size_t *needed);
```

`dwg_table_entry_get_name` 的 `table` 参数：`DWG_TABLE_LAYER=0, DWG_TABLE_BLOCK=1, DWG_TABLE_STYLE=2, DWG_TABLE_DIMSTYLE=3`。

---

### Task 1: Layer 表只读访问

**Files:**
- Create: `src/ffi/table.rs`
- Modify: `src/ffi/mod.rs`
- Test: `src/ffi/table.rs` 内部测试

**Interfaces:**
- Consumes: `FfiDocument`、`Table<Layer>`
- Produces: `dwg_layer_table_count`/`dwg_layer_get_name`/`dwg_layer_get_color`/`dwg_layer_get_handle`

- [ ] **Step 1: 写失败测试（table.rs 内部）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::CadDocument;

    #[test]
    fn layer_table_read_access() {
        let ffi = FfiDocument::new(CadDocument::new());
        let mut count = 0;
        let rc = layer_table_count(&ffi, &mut count);
        assert_eq!(rc, DWG_OK);
        assert!(count >= 1, "default document has at least layer 0");

        let mut name = [0u8; 64];
        let mut needed = 0;
        let rc = layer_get_name(&ffi, 0, name.as_mut_ptr() as *mut i8, name.len(), &mut needed);
        assert_eq!(rc, DWG_OK);
        assert_eq!(&name[..needed - 1], b"0");
    }
}
```

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现 `src/ffi/table.rs`**

```rust
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::tables::Layer;

fn doc_layers<'a>(data: *const DwgData) -> Result<&'a crate::tables::Table<Layer>, i32> {
    if data.is_null() { return Err(DWG_EINVAL); }
    let ffi = unsafe { &*(data as *const FfiDocument) };
    Ok(&ffi.doc.layers)
}

pub(crate) fn layer_table_count(ffi: &FfiDocument, count: &mut i32) -> i32 {
    *count = ffi.doc.layers.len() as i32;
    DWG_OK
}

pub(crate) fn layer_get_name(ffi: &FfiDocument, index: i32, buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) { return DWG_EINVAL; }
    let Some(layer) = ffi.doc.layers.iter().nth(index as usize) else {
        return DWG_ENOTFOUND;
    };
    copy_c_string(&layer.name, buf, cap, needed)
}

pub(crate) fn layer_get_color(ffi: &FfiDocument, index: i32, color: *mut i32) -> i32 {
    if color.is_null() { return DWG_EINVAL; }
    let Some(layer) = ffi.doc.layers.iter().nth(index as usize) else {
        return DWG_ENOTFOUND;
    };
    unsafe { *color = layer.color.to_true_color_value(); }
    DWG_OK
}

pub(crate) fn layer_get_handle(ffi: &FfiDocument, index: i32, handle: *mut u64) -> i32 {
    if handle.is_null() { return DWG_EINVAL; }
    let Some(layer) = ffi.doc.layers.iter().nth(index as usize) else {
        return DWG_ENOTFOUND;
    };
    unsafe { *handle = layer.handle().value(); }
    DWG_OK
}
```

`layer.handle()` 来自 `TableEntry` trait（`Table<T: TableEntry>`）。确认 `TableEntry` 有 `handle()`（`tables/mod.rs`）。

对外函数加 `#[no_mangle] pub unsafe extern "C"` 包装（NULL data 检查走 `doc_layers` 或直接内联）。
`copy_c_string` 从 `crate::ffi::entity` 提取为共享工具（`src/ffi/util.rs`），两处复用。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --features ffi table::tests`

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/table.rs src/ffi/util.rs src/ffi/mod.rs
git commit -m "feat(ffi): layer table read access"
```

---

### Task 2: Layer 颜色写入 + 名称查找 + 通用表 count/name

**Files:**
- Modify: `src/ffi/table.rs`
- Test: `tests/abi_test.rs`

**Interfaces:**
- Produces: `dwg_layer_set_color`/`dwg_layer_index_by_name`/`dwg_*_table_count`/`dwg_table_entry_get_name`

- [ ] **Step 1: 写失败测试（abi_test.rs）**

```rust
#[test]
fn layer_color_roundtrip_via_abi() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let mut count = 0;
    assert_eq!(unsafe { dwg_layer_table_count(data, &mut count) }, DWG_OK);
    assert!(count >= 1);
    // 层 0 默认颜色写回再读
    let color: i32 = 0x00FF0000; // 红
    assert_eq!(unsafe { dwg_layer_set_color(data, 0, color) }, DWG_OK);
    let mut read = 0;
    assert_eq!(unsafe { dwg_layer_get_color(data, 0, &mut read) }, DWG_OK);
    assert_eq!(read, color);
    // 按名称查找
    let name = CString::new("0").unwrap();
    let mut idx = -1;
    assert_eq!(unsafe { dwg_layer_index_by_name(data, name.as_ptr(), &mut idx) }, DWG_OK);
    assert_eq!(idx, 0);
    unsafe { dwg_data_destroy(data) };
}
```

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现**

`dwg_layer_set_color`：`doc.layers.get_mut(&layer.name)` 需要可变访问。但 `data: *const DwgData` 是 const——颜色写入需要 `*mut DwgData`。签名改为 `dwg_layer_set_color(DwgData *data, ...)`（非 const）。

实现：
```rust
pub unsafe extern "C" fn dwg_layer_set_color(data: *mut DwgData, index: i32, color: i32) -> i32 {
    if data.is_null() { return DWG_EINVAL; }
    let ffi = &mut *(data as *mut FfiDocument);
    let Some(layer) = ffi.doc.layers.iter_mut().nth(index as usize) else {
        return DWG_ENOTFOUND;
    };
    layer.color = Color::from_true_color_value(color);
    clear_last_error();
    DWG_OK
}
```

`dwg_layer_index_by_name`：`layers.iter().position(|l| l.name == name)`，未找到返回 `DWG_ENOTFOUND`。

通用表：内部按 `DwgTableKind` 分发到 `doc.layers/block_records/text_styles/dim_styles`，
count 返回 len，get_name 走 `iter().nth(index)` 取 `name`（BlockRecord 用 `name` 字段，
TextStyle/DimStyle 用 `name` 字段）。TableEntry trait 若无统一 `name()`，用 match 分派。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --features ffi --test abi_test`

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/table.rs tests/abi_test.rs
git commit -m "feat(ffi): layer set_color, by-name lookup, generic table counts"
```

---

### Task 3: C 集成测试（表访问）

**Files:**
- Create: `tests/ffi/c_table.c`
- Modify: `tests/abi_test.rs`

- [ ] **Step 1: 写 `tests/ffi/c_table.c`**

```c
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct DwgData DwgData;
extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_layer_table_count(const DwgData *data, int *count);
extern int dwg_layer_get_name(const DwgData *data, int index, char *buf, size_t cap, size_t *needed);
extern int dwg_layer_get_color(const DwgData *data, int index, int32_t *color);
extern int dwg_layer_set_color(DwgData *data, int index, int32_t color);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;
    int count = 0;
    if (dwg_layer_table_count(data, &count) != 0) return 2;
    if (count < 1) return 3;
    char name[64]; size_t needed = 0;
    if (dwg_layer_get_name(data, 0, name, sizeof name, &needed) != 0) return 4;
    if (strcmp(name, "0") != 0) return 5;
    int32_t color = 0x0000FF00;
    if (dwg_layer_set_color(data, 0, color) != 0) return 6;
    int32_t read = 0;
    if (dwg_layer_get_color(data, 0, &read) != 0) return 7;
    if (read != color) return 8;
    dwg_data_destroy(data);
    return 0;
}
```

- [ ] **Step 2: abi_test 编译运行 c_table**（仿 `c_iterate` 测试）
- [ ] **Step 3: 运行确认通过**

Run: `cargo test --features ffi --test abi_test`

- [ ] **Step 4: 提交**

```powershell
git add tests/ffi/c_table.c tests/abi_test.rs
git commit -m "test(ffi): C table access smoke test"
```

---

## Self-Review

- Spec 覆盖：Layer/Block/Style/DimStyle 表 count/name + Layer color/handle/by-name 均有任务。
- 无占位符：Task 1/3 有完整代码；Task 2 给签名与核心实现。
- 类型一致：`int32_t` 颜色、`uint64_t` handle、`DwgTableKind` 枚举在任务间一致。
- 注意：`dwg_layer_set_color` 需要 `*mut DwgData`（写操作），与只读 API 的 `*const` 区分。

## Execution Handoff

P3a 完成后，P3b（完整实体家族 getter）写独立 plan。
