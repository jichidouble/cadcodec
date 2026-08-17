# FFI Phase 3c2 固定字段实体 Setter 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为文本、注释、引用、显示和尺寸实体族补齐所有现有可写固定字段 getter 的 setter。

**Architecture:** 复用 Phase 3c1 的 `FfiDocument::mutate_entity`、输入校验和 setter 宏。简单字段由字符串/标量/向量宏生成，嵌套或复合字段使用手写函数，所有写入后仍由变更入口刷新 arena 指针。

**Tech Stack:** Rust 2021、C ABI、现有 `ffi` feature、MinGW C smoke harness。

## Global Constraints

- 必须先完成 `2026-08-17-ffi-phase3c1-mutation-common-geometry.md`。
- 所有 setter 使用 `DwgData *data, const DwgObject *obj`，不直接解引用未知 object。
- 字符串输入是 caller-owned NUL 字符串；非法 UTF-8 返回 `DWG_EINVAL`。
- 所有浮点输入必须有限；正尺寸字段必须严格大于零；法向量不得为零。
- Dimension subtype 与 measurement 保持只读，不生成 setter。
- RasterImage `get_size` 读取像素尺寸 `size`，对应 setter 必须写 `size`，不得调用修改世界尺寸向量的 `RasterImage::set_size()`。
- 不新增依赖，不修改 `Cargo.lock`，不实现集合增删或 P4 API。

---

## File Structure

- Modify: `src/ffi/entity_set.rs` — 固定字段 setter 与宏调用。
- Modify: `tests/abi_test.rs` — 各实体族 setter/getter 测试。
- Create: `tests/ffi/c_entity_set_fixed.c` — 字符串、复合字段和错误路径 C smoke。

## Interfaces

本计划新增以下 ABI：

```c
int dwg_entity_text_set_value(DwgData*, const DwgObject*, const char*);
int dwg_entity_text_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_text_set_height(DwgData*, const DwgObject*, double);
int dwg_entity_mtext_set_value(DwgData*, const DwgObject*, const char*);
int dwg_entity_mtext_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_mtext_set_height(DwgData*, const DwgObject*, double);

int dwg_entity_attrdef_set_tag(DwgData*, const DwgObject*, const char*);
int dwg_entity_attrdef_set_prompt(DwgData*, const DwgObject*, const char*);
int dwg_entity_attrdef_set_default_value(DwgData*, const DwgObject*, const char*);
int dwg_entity_attrdef_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_attrdef_set_height(DwgData*, const DwgObject*, double);
int dwg_entity_tolerance_set_text(DwgData*, const DwgObject*, const char*);
int dwg_entity_tolerance_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_mline_set_style_name(DwgData*, const DwgObject*, const char*);
int dwg_entity_mline_set_start_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_mline_set_scale_factor(DwgData*, const DwgObject*, double);
int dwg_entity_shape_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_shape_set_size(DwgData*, const DwgObject*, double);
int dwg_entity_shape_set_rotation(DwgData*, const DwgObject*, double);

int dwg_entity_insert_set_block_name(DwgData*, const DwgObject*, const char*);
int dwg_entity_insert_set_insert_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_insert_set_rotation(DwgData*, const DwgObject*, double);
int dwg_entity_insert_set_normal(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_insert_set_array(DwgData*, const DwgObject*, uint16_t, uint16_t, double, double);
int dwg_entity_viewport_set_center(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_viewport_set_width(DwgData*, const DwgObject*, double);
int dwg_entity_viewport_set_height(DwgData*, const DwgObject*, double);
int dwg_entity_hatch_set_elevation(DwgData*, const DwgObject*, double);
int dwg_entity_hatch_set_pattern_name(DwgData*, const DwgObject*, const char*);
int dwg_entity_hatch_set_pattern_angle(DwgData*, const DwgObject*, double);
int dwg_entity_hatch_set_pattern_scale(DwgData*, const DwgObject*, double);
int dwg_entity_helix_set_axis_base_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_helix_set_radius(DwgData*, const DwgObject*, double);
int dwg_entity_helix_set_turns(DwgData*, const DwgObject*, double);
int dwg_entity_helix_set_turn_height(DwgData*, const DwgObject*, double);

int dwg_entity_dimension_set_style_name(DwgData*, const DwgObject*, const char*);
int dwg_entity_dimension_set_definition_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_dimension_set_text_middle_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_raster_image_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_raster_image_set_size(DwgData*, const DwgObject*, double, double);
int dwg_entity_underlay_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_light_set_position(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_wipeout_set_insertion_point(DwgData*, const DwgObject*, double, double, double);
```

---

### Task 1: Text 与 MText setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Phase 3c1 的字符串、向量和正数 setter 模式。
- Produces: 6 个 Text/MText setter。

- [ ] **Step 1: 写失败测试**

在现有 Text/MText fixtures 上分别写入：

```rust
let text_value = CString::new("P3C TEXT").unwrap();
assert_eq!(unsafe { dwg_entity_text_set_value(data, text, text_value.as_ptr()) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_text_set_insertion_point(data, text, 1.0, 2.0, 3.0) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_text_set_height(data, text, 4.0) }, DWG_OK);

let mtext_value = CString::new("P3C\\PMTEXT").unwrap();
assert_eq!(unsafe { dwg_entity_mtext_set_value(data, mtext, mtext_value.as_ptr()) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_mtext_set_insertion_point(data, mtext, 5.0, 6.0, 7.0) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_mtext_set_height(data, mtext, 2.25) }, DWG_OK);
```

立即调用六个对应 getter，断言字符串、坐标和高度完全一致。另断言 NULL 字符串、非法
UTF-8、height 为 0/NaN 和类型错误返回对应错误码。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi phase3c_text_setters`

Expected: 编译失败，setter 未定义。

- [ ] **Step 3: 实现 setter**

在 `entity_set.rs` 增加 `string_setter!`，将 `read_c_string` 产生的 owned `String`
移动到匹配实体字段。向量和高度复用 Phase 3c1 宏，调用清单：

```rust
string_setter!(dwg_entity_text_set_value, Text, Text, value, allow_empty);
vec3_setter!(dwg_entity_text_set_insertion_point, Text, Text, insertion_point);
f64_setter!(dwg_entity_text_set_height, Text, Text, height, positive);
string_setter!(dwg_entity_mtext_set_value, MText, MText, value, allow_empty);
vec3_setter!(dwg_entity_mtext_set_insertion_point, MText, MText, insertion_point);
f64_setter!(dwg_entity_mtext_set_height, MText, MText, height, positive);
```

- [ ] **Step 4: 运行验证**

Run: `cargo test --release --features ffi phase3c_text_setters`

Expected: 成功与错误路径全绿。

- [ ] **Step 5: 提交**

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add text entity setters"
```

---

### Task 2: 注释与 Shape setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1 的字符串宏与 Phase 3c1 标量/向量宏。
- Produces: AttributeDefinition、Tolerance、MLine、Shape 共 13 个 setter。

- [ ] **Step 1: 写失败测试**

使用 batch3 fixture，按以下字段表写入并由对应 getter读回：

| Entity | String values | Vector values | Scalar values |
|---|---|---|---|
| AttributeDefinition | tag=`P3C_TAG`, prompt=`P3C_PROMPT`, default=`P3C_DEFAULT` | insertion=`(10,11,12)` | height=`3.5` |
| Tolerance | text=`P3C_TOL` | insertion=`(20,21,22)` | — |
| MLine | style=`P3C_STYLE` | start=`(30,31,32)` | scale=`1.75` |
| Shape | — | insertion=`(40,41,42)` | size=`2.5`, rotation=`0.5` |

错误断言：AttributeDefinition/Shape height 或 size 为零、MLine scale 非有限、将 Shape
setter 用于 Tolerance、NULL 字符串。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi phase3c_annotation_setters`

Expected: 编译失败，新 setter 未定义。

- [ ] **Step 3: 实现宏调用**

精确调用清单：

```rust
string_setter!(dwg_entity_attrdef_set_tag, AttributeDefinition, AttributeDefinition, tag, allow_empty);
string_setter!(dwg_entity_attrdef_set_prompt, AttributeDefinition, AttributeDefinition, prompt, allow_empty);
string_setter!(dwg_entity_attrdef_set_default_value, AttributeDefinition, AttributeDefinition, default_value, allow_empty);
vec3_setter!(dwg_entity_attrdef_set_insertion_point, AttributeDefinition, AttributeDefinition, insertion_point);
f64_setter!(dwg_entity_attrdef_set_height, AttributeDefinition, AttributeDefinition, height, positive);
string_setter!(dwg_entity_tolerance_set_text, Tolerance, Tolerance, text, allow_empty);
vec3_setter!(dwg_entity_tolerance_set_insertion_point, Tolerance, Tolerance, insertion_point);
string_setter!(dwg_entity_mline_set_style_name, MLine, MLine, style_name, allow_empty);
vec3_setter!(dwg_entity_mline_set_start_point, MLine, MLine, start_point);
f64_setter!(dwg_entity_mline_set_scale_factor, MLine, MLine, scale_factor, positive);
vec3_setter!(dwg_entity_shape_set_insertion_point, Shape, Shape, insertion_point);
f64_setter!(dwg_entity_shape_set_size, Shape, Shape, size, positive);
f64_setter!(dwg_entity_shape_set_rotation, Shape, Shape, rotation, finite_only);
```

- [ ] **Step 4: 运行验证并提交**

Run: `cargo test --release --features ffi phase3c_annotation_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add annotation and shape setters"
```

---

### Task 3: Insert、Viewport 与 Hatch setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: 已有 setter 宏和 `Insert::mark_as_minsert()`。
- Produces: Insert 5 个、Viewport 3 个、Hatch 4 个 setter。

- [ ] **Step 1: 写失败测试**

构造/复用含 Insert、Viewport、Hatch 的 fixture，写入并立即读回：

```rust
let block = CString::new("P3C_BLOCK").unwrap();
assert_eq!(unsafe { dwg_entity_insert_set_block_name(data, insert, block.as_ptr()) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_insert_set_insert_point(data, insert, 1.0, 2.0, 3.0) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_insert_set_rotation(data, insert, 0.75) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_insert_set_normal(data, insert, 0.0, 0.0, 1.0) }, DWG_OK);
assert_eq!(unsafe { dwg_entity_insert_set_array(data, insert, 3, 2, 10.0, 20.0) }, DWG_OK);
```

Viewport 使用 center `(4,5,6)`、width `200`、height `100`；Hatch 使用 elevation `7`、
pattern name `ANSI31`、angle `0.25`、scale `2.0`。对应 getter 必须返回可见字段；
`insert_get_array` 断言 3×2。另断言空 block、0 列/行、零 normal、非正 viewport 尺寸和
非正 hatch scale 返回 `DWG_EINVAL`。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi phase3c_insert_viewport_hatch_setters`

Expected: 编译失败。

- [ ] **Step 3: 实现简单字段和三个复合 setter**

简单字段使用既有宏。手写 `insert_set_normal` 使用 `nonzero3`；
`insert_set_array` 要求 `columns >= 1`、`rows >= 1`、spacing 有限，写四个字段并在
`columns > 1 || rows > 1` 时调用 `mark_as_minsert()`；`hatch_set_pattern_name` 写
`hatch.pattern.name`。其余精确字段为：

```rust
insert.insert_point = Vector3::new(x, y, z);
insert.rotation = value;
viewport.center = Vector3::new(x, y, z);
viewport.width = value;
viewport.height = value;
hatch.elevation = value;
hatch.pattern_angle = value;
hatch.pattern_scale = value;
```

- [ ] **Step 4: 运行验证并提交**

Run: `cargo test --release --features ffi phase3c_insert_viewport_hatch_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add insert viewport and hatch setters"
```

---

### Task 4: Helix、Dimension 与图像/引用实体 setter

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: batch4 getter fixtures、`Dimension::base_mut()`。
- Produces: 本计划剩余 12 个 setter；不产生 Dimension subtype/measurement setter。

- [ ] **Step 1: 写失败测试**

使用 batch4 fixture，固定写入：

- Helix: axis base `(1,2,3)`、radius `5`、turns `4`、turn height `1.5`。
- Dimension: style `P3C_DIM`、definition `(10,11,12)`、text middle `(13,14,15)`。
- RasterImage: insertion `(20,21,22)`、像素 size `(640,480)`。
- Underlay: insertion `(30,31,32)`；Light: position `(40,41,42)`；Wipeout: insertion `(50,51,52)`。

每项立即由 getter 读回。增加编译期保证：测试只调用
`dwg_entity_dimension_get_measurement`，不存在 `dimension_set_measurement` 调用。
错误断言覆盖 Helix 非正 radius/turns/turn height、RasterImage 非正尺寸和错误实体类型。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --release --features ffi phase3c_reference_setters`

Expected: 编译失败。

- [ ] **Step 3: 实现 setter**

Helix 使用普通字段宏，四个数值中的 radius/turns/turn_height 使用正数校验。
Dimension 三个手写 closure 通过 `let EntityType::Dimension(dimension)` 后写：

```rust
dimension.base_mut().style_name = value;
dimension.base_mut().definition_point = Vector3::new(x, y, z);
dimension.base_mut().text_middle_point = Vector3::new(x, y, z);
```

RasterImage size 必须直接写：

```rust
raster.size = Vector2::new(width, height);
```

不得调用 `RasterImage::set_size()`。Underlay/Light/Wipeout 分别写 `insertion_point`、
`position`、`insertion_point`。

- [ ] **Step 4: 运行验证并提交**

Run: `cargo test --release --features ffi phase3c_reference_setters`

Expected: 全绿。

Run: `cargo test --release --features ffi`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add dimension and reference entity setters"
```

---

### Task 5: 固定字段 C smoke test

**Files:**
- Create: `tests/ffi/c_entity_set_fixed.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1-4 的 ABI。
- Produces: 字符串、复合字段与只读 Dimension 行为的 C 验证。

- [ ] **Step 1: 写 C 程序**

C 程序读取包含 TEXT、INSERT、DIMENSION、RASTERIMAGE 的 fixture，分别调用：

```c
dwg_entity_text_set_value(data, text, "C P3C TEXT");
dwg_entity_insert_set_array(data, insert, 2, 3, 4.0, 5.0);
dwg_entity_dimension_set_style_name(data, dimension, "C_P3C_DIM");
dwg_entity_raster_image_set_size(data, image, 800.0, 600.0);
```

由 getter 验证四项结果；对 TEXT 调 RasterImage setter 必须返回 `DWG_ETYPE`。文件只声明
实际调用的完整原型，不声明不存在的 Dimension measurement setter。

- [ ] **Step 2: 扩展 Rust C harness**

新增 `c_entity_set_fixed_compiles_links_and_runs`，使用 batch fixtures 合并出的专用文档，
按现有 `ffi_dll()` + gcc 模式编译、链接和运行，断言退出码 0。

- [ ] **Step 3: 运行验证**

Run: `cargo test --release --features ffi c_entity_set_fixed_compiles_links_and_runs -- --nocapture`

Expected: 1 passed。

Run: `cargo fmt --check`

Expected: exit 0。

Run: `cargo clippy --all-targets --features ffi -- -D warnings`

Expected: exit 0。

- [ ] **Step 4: 提交**

```powershell
git add tests/ffi/c_entity_set_fixed.c tests/abi_test.rs
git commit -m "test(ffi): add fixed entity setter C smoke test"
```

---

## Self-Review

- Spec coverage: 43 个固定字段 setter 均在 Interfaces 和任务中明确列出。
- Read-only consistency: Dimension subtype、measurement 和所有 handle 未提供 setter。
- Nested-field consistency: Hatch pattern、Dimension base 和 RasterImage pixel size 均写 getter 实际读取的字段。
- Boundary: 集合元素、append/remove/clear 留给 Phase 3c3。

## Execution Handoff

本计划完成后继续执行 `2026-08-17-ffi-phase3c3-collection-setters.md`。
