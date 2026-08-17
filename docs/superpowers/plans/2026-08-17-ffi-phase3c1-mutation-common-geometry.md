# FFI Phase 3c1 写入基础、通用字段与几何 Setter 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立安全的 FFI 实体变更入口，并实现通用字段和基础几何实体的 setter。

**Architecture:** 所有 setter 同时接收 `DwgData*` 与 `const DwgObject*`，先在所属 `FfiDocument` 的 arena 中按地址验证对象，再通过 handle 调用 `CadDocument::get_entity_mut()`。写入后刷新对应 `FfiObject.entity`，保证 `Arc::make_mut()` 移动实体时既有 `DwgObject*` 仍有效。

**Tech Stack:** Rust 2021、C ABI、现有 `ffi` feature、MinGW C smoke harness。

## Global Constraints

- 规格来源：`docs/superpowers/specs/2026-08-17-ffi-phase3c-entity-setters-design.md`。
- 所有实体 setter 签名以 `DwgData *data, const DwgObject *obj` 开头。
- 成功返回 `DWG_OK` 并清空线程本地错误；失败返回现有 `DWG_E*` 并记录消息。
- 不直接写 `FfiObject.entity` 指向的内存；不重建 arena；不改变 `DwgObject*`、handle 或实体类型。
- 所有 `double` 输入必须有限；半径、高度和尺寸必须大于零；方向向量不得为零。
- C 字符串由调用方持有；无依赖变更，不修改 `Cargo.lock`。
- 默认验证使用 release + GNU 工具链：`cargo test --release --features ffi`。

---

## File Structure

- Modify: `src/ffi/object.rs` — 文档归属验证、规范实体写入、arena 指针刷新。
- Modify: `src/ffi/util.rs` — C 输入和数值校验。
- Create: `src/ffi/entity_set.rs` — setter 调度、setter 宏与本批接口。
- Modify: `src/ffi/mod.rs` — 挂载并导出 `entity_set`。
- Modify: `tests/abi_test.rs` — Rust ABI setter 测试。
- Create: `tests/ffi/c_entity_set_basic.c` — C 调用、链接和运行测试。

## Interfaces

内部接口：

```rust
pub(crate) type MutationError = (i32, &'static str);

impl FfiDocument {
    pub(crate) fn mutate_entity<F>(
        &mut self,
        obj: *const DwgObject,
        expected: Option<DwgObjectType>,
        mutate: F,
    ) -> i32
    where
        F: FnOnce(&mut EntityType) -> Result<(), MutationError>;
}
```

C ABI：

```c
int dwg_entity_set_layer(DwgData*, const DwgObject*, const char*);
int dwg_entity_set_color(DwgData*, const DwgObject*, int32_t);
int dwg_entity_set_linetype(DwgData*, const DwgObject*, const char*);
int dwg_entity_set_linetype_scale(DwgData*, const DwgObject*, double);

int dwg_entity_line_set_start(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_line_set_end(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_circle_set_center(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_circle_set_radius(DwgData*, const DwgObject*, double);
int dwg_entity_arc_set_center(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_arc_set_radius(DwgData*, const DwgObject*, double);
int dwg_entity_arc_set_start_angle(DwgData*, const DwgObject*, double);
int dwg_entity_arc_set_end_angle(DwgData*, const DwgObject*, double);
int dwg_entity_point_set_location(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_ellipse_set_center(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_ellipse_set_major_axis(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_ellipse_set_minor_axis_ratio(DwgData*, const DwgObject*, double);
int dwg_entity_ray_set_base_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_ray_set_direction(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_xline_set_base_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_xline_set_direction(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_solid_set_corner(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_face3d_set_corner(DwgData*, const DwgObject*, int32_t, double, double, double);
```

---

### Task 1: 安全实体变更入口与指针刷新

**Files:**
- Modify: `src/ffi/object.rs:55-142`
- Test: `src/ffi/object.rs` 的现有测试模块

**Interfaces:**
- Consumes: `FfiDocument`, `FfiObject`, `CadDocument::get_entity_mut`, `DwgObjectType`。
- Produces: `MutationError` 和 `FfiDocument::mutate_entity`。

- [ ] **Step 1: 写会触发 `Arc::make_mut()` 的失败测试**

在 `object.rs` 测试模块加入：

```rust
#[test]
fn mutation_refreshes_entity_pointer_without_moving_object_handle() {
    let mut ffi = FfiDocument::new(sample_doc());
    let handle = ffi.objects[0].handle;
    let obj = ffi.objects[0].as_ref() as *const FfiObject as *const DwgObject;
    let old_entity = ffi.objects[0].entity;
    let held_old_arc = ffi.doc.get_entity_arc(handle).unwrap();

    let rc = ffi.mutate_entity(obj, Some(DwgObjectType::Line), |entity| {
        let EntityType::Line(line) = entity else {
            return Err((DWG_ETYPE, "expected LINE"));
        };
        line.start = Vector3::new(9.0, 8.0, 7.0);
        Ok(())
    });

    assert_eq!(rc, DWG_OK);
    assert_eq!(obj, ffi.objects[0].as_ref() as *const FfiObject as *const DwgObject);
    assert_ne!(old_entity, ffi.objects[0].entity);
    assert_eq!(held_old_arc.as_ref().common().handle, handle);
    let EntityType::Line(line) = unsafe { &*ffi.objects[0].entity } else { panic!() };
    assert_eq!(line.start, Vector3::new(9.0, 8.0, 7.0));
}

#[test]
fn mutation_rejects_foreign_and_wrong_type_objects() {
    let mut left = FfiDocument::new(sample_doc());
    let right = FfiDocument::new(sample_doc());
    let foreign = right.objects[0].as_ref() as *const FfiObject as *const DwgObject;
    assert_eq!(left.mutate_entity(foreign, None, |_| Ok(())), DWG_ENOTFOUND);

    let local = left.objects[0].as_ref() as *const FfiObject as *const DwgObject;
    assert_eq!(
        left.mutate_entity(local, Some(DwgObjectType::Circle), |_| Ok(())),
        DWG_ETYPE
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --release --features ffi object::tests::mutation_`

Expected: 编译失败，提示 `mutate_entity` 不存在。

- [ ] **Step 3: 实现 `mutate_entity`**

在 `object.rs` 增加 `DwgObject`/错误槽导入，并实现：

```rust
pub(crate) type MutationError = (i32, &'static str);

impl FfiDocument {
    pub(crate) fn mutate_entity<F>(
        &mut self,
        obj: *const DwgObject,
        expected: Option<DwgObjectType>,
        mutate: F,
    ) -> i32
    where
        F: FnOnce(&mut EntityType) -> Result<(), MutationError>,
    {
        if obj.is_null() {
            set_last_error(DWG_EINVAL, "object pointer is null");
            return DWG_EINVAL;
        }
        let Some(index) = self.objects.iter().position(|candidate| {
            let candidate = candidate.as_ref() as *const FfiObject as *const DwgObject;
            std::ptr::eq(candidate, obj)
        }) else {
            set_last_error(DWG_ENOTFOUND, "object does not belong to document");
            return DWG_ENOTFOUND;
        };
        if self.objects[index].entity.is_null() {
            set_last_error(DWG_ETYPE, "object is not an entity");
            return DWG_ETYPE;
        }
        if expected.is_some_and(|kind| self.objects[index].kind != kind) {
            set_last_error(DWG_ETYPE, "entity type does not match setter");
            return DWG_ETYPE;
        }

        let handle = self.objects[index].handle;
        let result = match self.doc.get_entity_mut(handle) {
            Some(entity) => mutate(entity),
            None => Err((DWG_ENOTFOUND, "entity handle is missing from document")),
        };
        let Some(entity) = self.doc.get_entity(handle) else {
            set_last_error(DWG_ENOTFOUND, "entity disappeared during mutation");
            return DWG_ENOTFOUND;
        };
        self.objects[index].entity = entity as *const EntityType;

        match result {
            Ok(()) => {
                clear_last_error();
                DWG_OK
            }
            Err((code, message)) => {
                set_last_error(code, message);
                code
            }
        }
    }
}
```

- [ ] **Step 4: 运行变更入口测试**

Run: `cargo test --release --features ffi object::tests::mutation_`

Expected: 2 passed。

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/object.rs
git commit -m "feat(ffi): add safe entity mutation gateway"
```

---

### Task 2: 输入校验与 setter 模块骨架

**Files:**
- Modify: `src/ffi/util.rs`
- Create: `src/ffi/entity_set.rs`
- Modify: `src/ffi/mod.rs`
- Test: `src/ffi/util.rs` 内部测试

**Interfaces:**
- Consumes: Task 1 的 `mutate_entity`。
- Produces: `read_c_string`, `finite`, `finite2`, `finite3`, `nonzero3` 和 setter 模块。

- [ ] **Step 1: 写输入校验失败测试**

```rust
#[test]
fn setter_input_helpers_reject_invalid_values() {
    assert!(!finite(f64::NAN));
    assert!(!finite(f64::INFINITY));
    assert!(finite3(1.0, 2.0, 3.0));
    assert!(!nonzero3(0.0, 0.0, 0.0));
    assert_eq!(unsafe { read_c_string(std::ptr::null()) }, Err(DWG_EINVAL));
    let invalid = [0xff_u8, 0];
    assert_eq!(
        unsafe { read_c_string(invalid.as_ptr() as *const i8) },
        Err(DWG_EINVAL)
    );
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi setter_input_helpers_reject_invalid_values`

Expected: 编译失败，输入 helper 未定义。

- [ ] **Step 3: 实现 helper 和模块骨架**

在 `util.rs` 增加：

```rust
pub(crate) unsafe fn read_c_string(value: *const i8) -> Result<String, i32> {
    if value.is_null() {
        return Err(DWG_EINVAL);
    }
    std::ffi::CStr::from_ptr(value)
        .to_str()
        .map(str::to_owned)
        .map_err(|_| DWG_EINVAL)
}

pub(crate) fn finite(value: f64) -> bool { value.is_finite() }
pub(crate) fn finite2(x: f64, y: f64) -> bool { x.is_finite() && y.is_finite() }
pub(crate) fn finite3(x: f64, y: f64, z: f64) -> bool {
    x.is_finite() && y.is_finite() && z.is_finite()
}
pub(crate) fn nonzero3(x: f64, y: f64, z: f64) -> bool {
    finite3(x, y, z) && (x != 0.0 || y != 0.0 || z != 0.0)
}
```

创建 `entity_set.rs`，导入 `EntityType`、`Vector2`、`Vector3`、错误槽、
`FfiDocument`、`DwgObjectType`、opaque 类型和 util helper。`mod.rs` 增加：

```rust
pub mod entity_set;
pub use entity_set::*;
```

- [ ] **Step 4: 运行 helper 与全量 FFI 测试**

Run: `cargo test --release --features ffi setter_input_helpers_reject_invalid_values`

Expected: 1 passed。

Run: `cargo test --release --features ffi`

Expected: 全绿。

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/util.rs src/ffi/entity_set.rs src/ffi/mod.rs
git commit -m "feat(ffi): add setter input validation module"
```

---

### Task 3: 通用实体 setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: `mutate_entity(obj, None, closure)`、`read_c_string`、`color_from_c`。
- Produces: `dwg_entity_set_layer/color/linetype/linetype_scale`。

- [ ] **Step 1: 写通用字段失败测试**

在 `abi_test.rs` 使用 `open_sample_document()` 和 `find_object_of_type()`：

```rust
#[test]
fn common_entity_setters_roundtrip_through_getters() {
    let (data, path) = open_sample_document();
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let layer = CString::new("P3C_LAYER").unwrap();
    let linetype = CString::new("DASHED").unwrap();

    assert_eq!(unsafe { dwg_entity_set_layer(data, line, layer.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_color(data, line, 0x00112233) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype(data, line, linetype.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype_scale(data, line, 2.5) }, DWG_OK);

    let mut needed = 0;
    assert_eq!(unsafe { dwg_entity_get_layer(line, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    let mut buf = vec![0_i8; needed];
    assert_eq!(unsafe { dwg_entity_get_layer(line, buf.as_mut_ptr(), buf.len(), &mut needed) }, DWG_OK);
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_str().unwrap(), "P3C_LAYER");
    let mut color = 0;
    let mut scale = 0.0;
    assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_get_linetype_scale(line, &mut scale) }, DWG_OK);
    assert_eq!(color, 0x00112233);
    assert_eq!(scale, 2.5);

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(path);
}
```

另加断言：NULL data/object/value 返回 `DWG_EINVAL`；另一个文档的 object 返回
`DWG_ENOTFOUND`；空 layer 和非有限 scale 返回 `DWG_EINVAL`。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi common_entity_setters_roundtrip_through_getters`

Expected: 编译失败，四个 setter 未定义。

- [ ] **Step 3: 实现四个 setter**

共同入口形状：

```rust
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_layer(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "layer is not valid UTF-8");
        return DWG_EINVAL;
    };
    if value.is_empty() {
        set_last_error(DWG_EINVAL, "layer must not be empty");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().layer = value;
        Ok(())
    })
}
```

`set_color` 写 `color_from_c(value)`；`set_linetype` 允许空字符串；
`set_linetype_scale` 要求 `finite(value) && value > 0.0`。四个函数均只修改
`entity.common_mut()` 中对应字段。

- [ ] **Step 4: 运行通用字段和全量测试**

Run: `cargo test --release --features ffi common_entity_setters`

Expected: 成功、错误路径测试全绿。

Run: `cargo test --release --features ffi`

Expected: 全绿。

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add common entity setters"
```

---

### Task 4: 基础几何 setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1-3 的写入入口、验证 helper 和错误语义。
- Produces: 本计划 Interfaces 中列出的 18 个几何 setter。

- [ ] **Step 1: 写几何 setter 失败测试**

扩展现有 batch1 fixture，逐类调用 setter 后立即调用对应 getter。测试值固定为：

```rust
let p1 = (11.0, 12.0, 13.0);
let p2 = (21.0, 22.0, 23.0);
let radius = 4.5;
let start_angle = 0.25;
let end_angle = 2.75;
let ratio = 0.625;
```

必须断言：Line start/end、Circle center/radius、Arc center/radius/angles、Point location、
Ellipse center/major axis/ratio、Ray/XLine base/direction、Solid/Face3D corners 1 和 4。
另断言 radius/ratio 为零、NaN、零 direction、corner 0/5、对 Circle 调 Line setter
分别返回 `DWG_EINVAL` 或 `DWG_ETYPE`。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi phase3c_basic_geometry_setters`

Expected: 编译失败，几何 setter 未定义。

- [ ] **Step 3: 实现标量和向量 setter 宏**

在 `entity_set.rs` 增加 `vec3_setter!` 和 `f64_setter!`。宏必须生成完整
`#[no_mangle] pub unsafe extern "C" fn`，检查 data 和有限值，通过
`DwgObjectType` 与 `EntityType` 双重匹配后写字段。调用清单固定为：

```rust
vec3_setter!(dwg_entity_line_set_start, Line, Line, start);
vec3_setter!(dwg_entity_line_set_end, Line, Line, end);
vec3_setter!(dwg_entity_circle_set_center, Circle, Circle, center);
f64_setter!(dwg_entity_circle_set_radius, Circle, Circle, radius, positive);
vec3_setter!(dwg_entity_arc_set_center, Arc, Arc, center);
f64_setter!(dwg_entity_arc_set_radius, Arc, Arc, radius, positive);
f64_setter!(dwg_entity_arc_set_start_angle, Arc, Arc, start_angle, finite_only);
f64_setter!(dwg_entity_arc_set_end_angle, Arc, Arc, end_angle, finite_only);
vec3_setter!(dwg_entity_point_set_location, Point, Point, location);
vec3_setter!(dwg_entity_ellipse_set_center, Ellipse, Ellipse, center);
vec3_setter!(dwg_entity_ellipse_set_major_axis, Ellipse, Ellipse, major_axis);
f64_setter!(dwg_entity_ellipse_set_minor_axis_ratio, Ellipse, Ellipse, minor_axis_ratio, positive);
vec3_setter!(dwg_entity_ray_set_base_point, Ray, Ray, base_point);
vec3_setter!(dwg_entity_xline_set_base_point, XLine, XLine, base_point);
```

`minor_axis_ratio` 的显式校验为 `value > 0.0 && value <= 1.0`。Ray/XLine direction
使用 `nonzero3` 的手写函数。Solid/Face3D corner 使用 `match corner { 1..=4 }`，在进入
`mutate_entity` 前拒绝其他值。

- [ ] **Step 4: 运行几何、格式和 lint 验证**

Run: `cargo test --release --features ffi phase3c_basic_geometry_setters`

Expected: 全绿。

Run: `cargo fmt --check`

Expected: exit 0；若失败，先运行 `cargo fmt` 再复查。

Run: `cargo clippy --all-targets --features ffi -- -D warnings`

Expected: exit 0。

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add basic geometry entity setters"
```

---

### Task 5: C 基础 setter smoke test

**Files:**
- Create: `tests/ffi/c_entity_set_basic.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 3-4 的 C ABI。
- Produces: GNU C 编译、链接、运行覆盖。

- [ ] **Step 1: 写 C 测试**

`c_entity_set_basic.c` 接收 fixture 路径，创建/读取 `DwgData`，遍历到 LINE，调用：

```c
if (dwg_entity_set_layer(data, line, "P3C_C") != 0) return 10;
if (dwg_entity_line_set_start(data, line, 31.0, 32.0, 33.0) != 0) return 11;
double x = 0.0, y = 0.0, z = 0.0;
if (dwg_entity_line_get_start(line, &x, &y, &z) != 0) return 12;
if (x != 31.0 || y != 32.0 || z != 33.0) return 13;
if (dwg_entity_circle_set_radius(data, line, 2.0) != DWG_ETYPE) return 14;
```

文件中声明本计划实际调用的 opaque 类型、错误码和函数原型；销毁 data 后返回 0。

- [ ] **Step 2: 在 Rust harness 编译并运行 C 测试**

在 `abi_test.rs` 新增 `c_entity_set_basic_compiles_links_and_runs`，复用 `ffi_dll()`、
`write_sample_dwg()` 和既有 gcc 参数；运行 exe 时传 fixture 路径并断言退出码 0。

- [ ] **Step 3: 运行 C smoke 和全量 FFI 测试**

Run: `cargo test --release --features ffi c_entity_set_basic_compiles_links_and_runs -- --nocapture`

Expected: 1 passed，C 程序退出 0。

Run: `cargo test --release --features ffi`

Expected: 全绿。

- [ ] **Step 4: 提交**

```powershell
git add tests/ffi/c_entity_set_basic.c tests/abi_test.rs
git commit -m "test(ffi): add basic entity setter C smoke test"
```

---

## Self-Review

- Spec coverage: 写入入口、归属验证、指针刷新、错误槽、通用字段和 18 个几何 setter 均有任务。
- Type consistency: 所有 setter 使用 `DwgData* + const DwgObject*`；向量为三个 `double`；corner 为 `int32_t`。
- Boundary: 本计划不实现文本/注释、集合、实体创建删除或 P4 API。
- Verification: Rust ABI、强制 `Arc::make_mut()`、错误路径、C smoke、fmt 与 clippy 均有明确命令。

## Execution Handoff

本计划完成后继续执行 `2026-08-17-ffi-phase3c2-fixed-field-setters.md`。
