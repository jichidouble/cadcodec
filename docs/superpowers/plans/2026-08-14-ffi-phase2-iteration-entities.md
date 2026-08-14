# FFI Phase 2 对象/实体遍历与基础实体 getter 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 C 消费者能遍历文档对象图（实体优先），并对 LINE/CIRCLE/ARC/TEXT/POINT 五种实体读取字段。

**Architecture:** FFI 层引入 `FfiDocument`（包装 `CadDocument` + 对象 arena `Vec<Box<FfiObject>>`）。`DwgObject*` 是指向 `FfiObject` 的稳定指针；`FfiObject` 链式 `next` 指针实现遍历。实体字段通过指向 `EntityType` 的原始指针按需访问，不改 Rust 核心存储。

**Tech Stack:** Rust 2021、现有 ffi feature、MinGW gcc（C 集成测试）。

## Global Constraints

- 工具链：MSVC（默认）与 GNU 双支持；`cargo test --features ffi` 全绿。
- 前缀：`dwg_`（通用）/`dxf_`（DXF 专有）。
- 内存：`DwgData*`/`DwgObject*` 是 opaque 句柄，归文档所有，C 侧不 free；字符串/向量用 caller-owned buffer；空指针返回 `DWG_EINVAL`。
- 错误：int 返回码 + 线程本地 `dwg_last_error`。
- 不改 Rust 核心 `CadDocument` 的存储结构（spec 决策 1）。
- 提交：本地提交，不推送（仓库无远程）。
- 本计划的执行说明：子代理派发在本环境不可用（被 codebase-memory hook 覆盖），controller 直接按 TDD 执行；进度记入 `.superpowers/sdd/2026-08-14-ffi-phase1-lifecycle-io/progress.md` 同目录的新 ledger `phase2-progress.md`。

---

## File Structure

- Create: `src/ffi/object.rs` — `FfiDocument`/`FfiObject`、arena 构建、对象类型枚举
- Modify: `src/ffi/data.rs` — `dwg_data_create/destroy/version` 改用 `FfiDocument`
- Modify: `src/ffi/io.rs` — 读文件后重建 arena
- Create: `src/ffi/iterate.rs` — `dwg_data_first_object`/`dwg_object_next`/`dwg_object_get_type`/`dwg_object_get_handle`
- Create: `src/ffi/entity.rs` — 基础实体 getter（Line/Circle/Arc/Text/Point）
- Modify: `src/ffi/mod.rs` — 挂载新模块
- Modify: `tests/abi_test.rs` — 遍历与 getter 测试
- Create: `tests/ffi/c_iterate.c` — C 集成测试（自造文档写读后遍历）

## Interfaces

Produces（供后续任务与 C 使用）:

```c
typedef enum DwgObjectType {
    DWG_OBJECT_UNKNOWN = 0,
    DWG_OBJECT_LINE = 1,
    DWG_OBJECT_CIRCLE = 2,
    DWG_OBJECT_ARC = 3,
    DWG_OBJECT_TEXT = 4,
    DWG_OBJECT_POINT = 5,
    DWG_OBJECT_OTHER = 1000,
} DwgObjectType;

const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
const DwgObject *dwg_object_next(const DwgObject *obj);
int dwg_object_get_type(const DwgObject *obj);
uint64_t dwg_object_get_handle(const DwgObject *obj);

int dwg_entity_line_get_start(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_line_get_end(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_circle_get_center(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_circle_get_radius(const DwgObject *obj, double *radius);
int dwg_entity_arc_get_center(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_arc_get_radius(const DwgObject *obj, double *radius);
int dwg_entity_arc_get_start_angle(const DwgObject *obj, double *angle);
int dwg_entity_arc_get_end_angle(const DwgObject *obj, double *angle);
int dwg_entity_text_get_value(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
int dwg_entity_text_get_insertion_point(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_text_get_height(const DwgObject *obj, double *height);
int dwg_entity_point_get_location(const DwgObject *obj, double *x, double *y, double *z);
```

内部结构（Rust，C 不可见）:

```rust
pub struct FfiDocument {
    pub doc: CadDocument,
    pub objects: Vec<Box<FfiObject>>,
}
pub struct FfiObject {
    pub kind: DwgObjectType,
    pub handle: Handle,
    pub entity: *const EntityType,
    pub object: *const ObjectType,
    pub next: *const FfiObject,
}
```

---

### Task 1: FfiDocument/FfiObject 结构与 arena 构建

**Files:**
- Create: `src/ffi/object.rs`
- Modify: `src/ffi/data.rs`
- Modify: `src/ffi/io.rs`
- Modify: `src/ffi/mod.rs`
- Test: `src/ffi/object.rs` 内部 `#[cfg(test)]`

**Interfaces:**
- Consumes: P1 的 opaque 类型与 `CadDocument`
- Produces: `FfiDocument::new(doc)`、`DwgObjectType` 枚举、`FfiObject`

- [ ] **Step 1: 写失败测试**

`src/ffi/object.rs` 内部测试：构造含 Line/Circle/Text 的文档，`FfiDocument::new` 后断言 arena 长度 3、首对象 kind 为 LINE、handle 非空。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::*;
    use crate::types::Vector3;
    use crate::{CadDocument, EntityType};

    fn sample_doc() -> CadDocument {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 2.0, 3.0))).unwrap();
        doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
        let mut t = Text::new();
        t.value = "hello".to_string();
        t.height = 1.0;
        doc.add_entity(EntityType::Text(t)).unwrap();
        doc
    }

    #[test]
    fn arena_builds_in_entity_order() {
        let ffi = FfiDocument::new(sample_doc());
        assert_eq!(ffi.objects.len(), 3);
        assert_eq!(ffi.objects[0].kind, DwgObjectType::Line);
        assert_eq!(ffi.objects[1].kind, DwgObjectType::Circle);
        assert_eq!(ffi.objects[2].kind, DwgObjectType::Text);
        assert!(!ffi.objects[0].handle.is_null());
        assert!(ffi.objects[0].next as usize != 0);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --features ffi object::tests`
Expected: 编译失败（`FfiDocument`/`DwgObjectType` 未定义）

- [ ] **Step 3: 实现 `src/ffi/object.rs`**

```rust
use std::ptr;
use crate::document::CadDocument;
use crate::entities::{EntityType, Entity};
use crate::objects::ObjectType;
use crate::types::Handle;

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DwgObjectType {
    Unknown = 0,
    Line = 1,
    Circle = 2,
    Arc = 3,
    Text = 4,
    Point = 5,
    Other = 1000,
}

pub struct FfiObject {
    pub kind: DwgObjectType,
    pub handle: Handle,
    pub entity: *const EntityType,
    pub object: *const ObjectType,
    pub next: *const FfiObject,
}

pub struct FfiDocument {
    pub doc: CadDocument,
    pub objects: Vec<Box<FfiObject>>,
}

fn entity_kind(e: &EntityType) -> DwgObjectType {
    match e {
        EntityType::Line(_) => DwgObjectType::Line,
        EntityType::Circle(_) => DwgObjectType::Circle,
        EntityType::Arc(_) => DwgObjectType::Arc,
        EntityType::Text(_) => DwgObjectType::Text,
        EntityType::Point(_) => DwgObjectType::Point,
        _ => DwgObjectType::Other,
    }
}

impl FfiDocument {
    pub fn new(doc: CadDocument) -> Self {
        let mut objects: Vec<Box<FfiObject>> = Vec::new();
        for e in doc.entities() {
            objects.push(Box::new(FfiObject {
                kind: entity_kind(e),
                handle: e.handle(),
                entity: e as *const EntityType,
                object: ptr::null(),
                next: ptr::null(),
            }));
        }
        let mut handles: Vec<Handle> = doc.objects.keys().copied().collect();
        handles.sort();
        for h in handles {
            objects.push(Box::new(FfiObject {
                kind: DwgObjectType::Other,
                handle: h,
                entity: ptr::null(),
                object: &doc.objects[&h] as *const ObjectType,
                next: ptr::null(),
            }));
        }
        // Link next pointers after all Boxes are allocated (heap addresses stable).
        for i in 0..objects.len() {
            let next = if i + 1 < objects.len() {
                objects[i + 1].as_ref() as *const FfiObject
            } else {
                ptr::null()
            };
            objects[i].next = next;
        }
        FfiDocument { doc, objects }
    }
}
```

- [ ] **Step 4: 迁移 data.rs / io.rs**

`data.rs`：`Box::new(CadDocument::new())` 改为 `Box::new(FfiDocument::new(CadDocument::new()))`；类型转换 `as *mut DwgData` 不变；`dwg_data_version` 访问 `(&*ptr).doc.version.as_str()`。

`io.rs`：`replace_document` 改为 `let ffi = &mut *(data as *mut FfiDocument); *ffi = FfiDocument::new(doc);`（重建 arena，旧对象引用失效）。

`mod.rs`：`pub mod object;` + `pub use object::*;`

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --features ffi object::tests`
Expected: 1 passed；`cargo test --features ffi` 全绿

- [ ] **Step 6: 提交**

```powershell
git add src/ffi/object.rs src/ffi/data.rs src/ffi/io.rs src/ffi/mod.rs
git commit -m "feat(ffi): object arena over CadDocument (stable DwgObject refs)"
```

---

### Task 2: 遍历 API

**Files:**
- Create: `src/ffi/iterate.rs`
- Modify: `src/ffi/mod.rs`
- Test: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1 的 `FfiDocument`/`FfiObject`
- Produces: `dwg_data_first_object`/`dwg_object_next`/`dwg_object_get_type`/`dwg_object_get_handle`

- [ ] **Step 1: 写失败测试（abi_test.rs）**

```rust
#[test]
fn iterate_entities_via_abi() {
    // 自造文档：Line + Circle + Text，写 DWG 读回后遍历
    let dir = std::env::temp_dir().join("acadrust_ffi_p2");
    std::fs::create_dir_all(&dir).unwrap();
    let path = CString::new(dir.join("iter.dwg").to_str().unwrap()).unwrap();

    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    // 通过 FFI 无 setter，先用 Rust 构造再写回：此处直接读一个含 3 实体的文件。
    // 简化：使用 P2 测试专用夹具文件（见 Step 3 说明）——先构造空文档写文件，
    // 内容由 Rust 侧生成。
    // 本测试直接验证 API 形状：空文档 first_object 返回 NULL。
    let mut err = 0;
    let first = unsafe { dwg_data_first_object(data, &mut err) };
    assert_eq!(err, DWG_OK);
    assert!(first.is_null());
    unsafe { dwg_data_destroy(data) };
}
```

- [ ] **Step 2: 运行确认失败**

Expected: `dwg_data_first_object` 未定义

- [ ] **Step 3: 实现 `src/ffi/iterate.rs`**

```rust
use crate::ffi::object::{DwgObjectType, FfiDocument, FfiObject};
use crate::ffi::types::*;

#[no_mangle]
pub unsafe extern "C" fn dwg_data_first_object(
    data: *const DwgData,
    error: *mut i32,
) -> *const DwgObject {
    if data.is_null() || error.is_null() {
        if !error.is_null() { *error = DWG_EINVAL; }
        return std::ptr::null();
    }
    let ffi = &*(data as *const FfiDocument);
    *error = DWG_OK;
    match ffi.objects.first() {
        Some(o) => o.as_ref() as *const FfiObject as *const DwgObject,
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dwg_object_next(obj: *const DwgObject) -> *const DwgObject {
    if obj.is_null() { return std::ptr::null(); }
    let ffi = &*(obj as *const FfiObject);
    if ffi.next.is_null() {
        std::ptr::null()
    } else {
        ffi.next as *const DwgObject
    }
}

#[no_mangle]
pub unsafe extern "C" fn dwg_object_get_type(obj: *const DwgObject) -> i32 {
    if obj.is_null() { return DwgObjectType::Unknown as i32; }
    let ffi = &*(obj as *const FfiObject);
    ffi.kind as i32
}

#[no_mangle]
pub unsafe extern "C" fn dwg_object_get_handle(obj: *const DwgObject) -> u64 {
    if obj.is_null() { return 0; }
    let ffi = &*(obj as *const FfiObject);
    ffi.handle.value()
}
```

`mod.rs`：`pub mod iterate;` + `pub use iterate::*;`

- [ ] **Step 4: 用真实数据验证遍历**

在 abi_test 增加辅助：先用 Rust 构造 3 实体文档，用 `dwg_write_file` 写临时文件，
再用 `dwg_read_file` 读回，然后遍历断言 3 个对象、类型依次 LINE/CIRCLE/TEXT。
（Rust 侧构造用 `CadDocument`，通过 `DwgWriter` 写入，再经 FFI 读回。）

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --features ffi --test abi_test`
Expected: 全绿（含遍历断言）

- [ ] **Step 6: 提交**

```powershell
git add src/ffi/iterate.rs src/ffi/mod.rs tests/abi_test.rs
git commit -m "feat(ffi): object iteration API"
```

---

### Task 3: 基础实体 getter

**Files:**
- Create: `src/ffi/entity.rs`
- Modify: `src/ffi/mod.rs`
- Test: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1/2 的 `FfiObject`/遍历
- Produces: 12 个 getter（见 Interfaces 清单）

- [ ] **Step 1: 写失败测试（abi_test.rs）**

```rust
#[test]
fn entity_getters_return_values() {
    // 复用遍历夹具：读回后取第一个 LINE 对象
    let line = first_entity_of_type(DWG_OBJECT_LINE);
    let mut x = 0.0; let mut y = 0.0; let mut z = 0.0;
    let rc = unsafe { dwg_entity_line_get_start(line, &mut x, &mut y, &mut z) };
    assert_eq!(rc, DWG_OK);
    assert!((x - 0.0).abs() < 1e-9 && (y - 0.0).abs() < 1e-9);
    // 错误类型：对 LINE 调 circle getter 应返回 DWG_ETYPE
    let mut r = 0.0;
    let rc = unsafe { dwg_entity_circle_get_radius(line, &mut r) };
    assert_eq!(rc, DWG_ETYPE);
}
```

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现 `src/ffi/entity.rs`**

模式（每个 getter 一个函数）：

```rust
fn as_line<'a>(obj: *const DwgObject) -> Result<&'a crate::entities::Line, i32> {
    if obj.is_null() { return Err(DWG_EINVAL); }
    let ffi = unsafe { &*(obj as *const FfiObject) };
    if ffi.entity.is_null() { return Err(DWG_ETYPE); }
    match unsafe { &*ffi.entity } {
        crate::entities::EntityType::Line(l) => Ok(l),
        _ => Err(DWG_ETYPE),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dwg_entity_line_get_start(
    obj: *const DwgObject,
    x: *mut f64, y: *mut f64, z: *mut f64,
) -> i32 {
    if x.is_null() || y.is_null() || z.is_null() { return DWG_EINVAL; }
    match as_line(obj) {
        Ok(l) => { *x = l.start.x; *y = l.start.y; *z = l.start.z; DWG_OK }
        Err(code) => code,
    }
}
```

同理实现：line_get_end、circle_get_center/radius、arc_get_center/radius/start_angle/end_angle、
text_get_value（caller-owned buffer）、text_get_insertion_point/height、point_get_location。

`text_get_value` 的 buffer 模式与 `dwg_data_version` 一致（buf/cap/needed，NULL 查询长度）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --features ffi --test abi_test`
Expected: 全绿（含 getter 值与类型错误断言）

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/entity.rs src/ffi/mod.rs tests/abi_test.rs
git commit -m "feat(ffi): basic entity getters (line/circle/arc/text/point)"
```

---

### Task 4: C 集成测试（遍历 + getter）

**Files:**
- Create: `tests/ffi/c_iterate.c`
- Modify: `tests/abi_test.rs`（编译并运行 c_iterate）

**Interfaces:**
- Consumes: Task 1-3 全部 API
- Produces: C 侧遍历验证

- [ ] **Step 1: 写 `tests/ffi/c_iterate.c`**

```c
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct DwgData DwgData;
typedef struct DwgObject DwgObject;
enum { DWG_OBJECT_LINE = 1, DWG_OBJECT_CIRCLE = 2, DWG_OBJECT_TEXT = 4, DWG_OBJECT_OTHER = 1000 };

extern int dwg_data_create(DwgData **out);
extern void dwg_data_destroy(DwgData *data);
extern int dwg_write_file(const char *path, const DwgData *data);
extern int dwg_read_file(const char *path, DwgData *data);
extern const DwgObject *dwg_data_first_object(const DwgData *data, int *error);
extern const DwgObject *dwg_object_next(const DwgObject *obj);
extern int dwg_object_get_type(const DwgObject *obj);
extern int dwg_entity_line_get_start(const DwgObject *obj, double *x, double *y, double *z);
extern int dwg_entity_circle_get_radius(const DwgObject *obj, double *radius);
extern int dwg_entity_text_get_value(const DwgObject *obj, char *buf, size_t cap, size_t *needed);

int main(void) {
    DwgData *data = NULL;
    if (dwg_data_create(&data) != 0) return 1;
    const char *path = "c_iterate_rt.dwg";
    if (dwg_write_file(path, data) != 0) return 2;   /* empty doc */
    if (dwg_read_file(path, data) != 0) return 3;

    int err = 0;
    const DwgObject *obj = dwg_data_first_object(data, &err);
    if (err != 0) return 4;
    /* Empty document: no objects */
    if (obj != NULL) return 5;

    dwg_data_destroy(data);
    remove(path);
    return 0;
}
```

（自造 3 实体文档的 C 侧遍历在 Task 2/3 的 Rust 测试已覆盖；C 侧先验证空文档 + API 形状，
真实实体遍历的 C 测试在 P3 引入夹具文件后补充。）

- [ ] **Step 2: 扩展 abi_test 编译并运行 c_iterate**

仿照 `c_smoke_compiles_links_and_runs`：编译链接 c_iterate.c，运行断言退出码 0。

- [ ] **Step 3: 运行确认通过**

Run: `cargo test --features ffi --test abi_test`
Expected: 全绿

- [ ] **Step 4: 提交**

```powershell
git add tests/ffi/c_iterate.c tests/abi_test.rs
git commit -m "test(ffi): C iteration smoke test"
```

---

## Self-Review

- Spec 覆盖：P2 遍历 + 5 种基础实体 getter 均有任务；arena 稳定引用符合 spec 决策 1。
- 无占位符：Task 1/3 给出核心代码，Task 2/4 给出模式与测试结构。
- 类型一致：`DwgObjectType` 枚举、`FfiDocument`/`FfiObject` 字段、getter 签名在各任务一致。

## Execution Handoff

P2 完成后，P3（完整实体 + 表访问）写独立 plan。
