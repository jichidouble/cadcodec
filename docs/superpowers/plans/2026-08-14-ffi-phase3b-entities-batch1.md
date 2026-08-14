# FFI Phase 3b 实体家族 getter（第一批）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为全部实体的公共字段加 getter，并为几何/曲线/文本家族（Ellipse/Ray/XLine/Solid/Face3D/Spline/MText/LwPolyline）加字段 getter。

**Architecture:** 复用 P2 的 `as_entity`/宏模式与 util.rs 的 `copy_c_string`/`color_to_c`。实体通过 `FfiObject.entity` 指针 match `EntityType` 取字段。

**Tech Stack:** Rust 2021、现有 ffi feature、MinGW gcc。

## Global Constraints

- 工具链：MSVC/GNU；`cargo test --features ffi` 全绿。
- 前缀 `dwg_`；错误码 int + 线程本地错误；caller-owned buffer；空指针 `DWG_EINVAL`。
- 颜色编码沿用 util.rs（RGB>=0，ACI 负值）。
- 提交：本地，不推送；controller 直接 TDD；进度记入 `phase3b-progress.md`。

---

## File Structure

- Modify: `src/ffi/entity.rs` — 追加 getter
- Modify: `tests/abi_test.rs`
- Create: `tests/ffi/c_entity_batch1.c`

## Interfaces

```c
/* common */
int dwg_entity_get_layer(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
int dwg_entity_get_color(const DwgObject *obj, int32_t *color);
int dwg_entity_get_linetype(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
int dwg_entity_get_linetype_scale(const DwgObject *obj, double *scale);

/* ellipse */
int dwg_entity_ellipse_get_center(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_ellipse_get_major_axis(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_ellipse_get_minor_axis_ratio(const DwgObject *obj, double *ratio);
/* ray / xline */
int dwg_entity_ray_get_base_point(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_ray_get_direction(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_xline_get_base_point(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_xline_get_direction(const DwgObject *obj, double *x, double *y, double *z);
/* solid / face3d */
int dwg_entity_solid_get_corner(const DwgObject *obj, int corner, double *x, double *y, double *z);
int dwg_entity_face3d_get_corner(const DwgObject *obj, int corner, double *x, double *y, double *z);
/* spline */
int dwg_entity_spline_get_degree(const DwgObject *obj, int *degree);
int dwg_entity_spline_get_control_point_count(const DwgObject *obj, int *count);
int dwg_entity_spline_get_control_point(const DwgObject *obj, int index, double *x, double *y, double *z);
/* mtext */
int dwg_entity_mtext_get_value(const DwgObject *obj, char *buf, size_t cap, size_t *needed);
int dwg_entity_mtext_get_insertion_point(const DwgObject *obj, double *x, double *y, double *z);
int dwg_entity_mtext_get_height(const DwgObject *obj, double *height);
/* lwpolyline */
int dwg_entity_lwpolyline_get_vertex_count(const DwgObject *obj, int *count);
int dwg_entity_lwpolyline_get_vertex(const DwgObject *obj, int index, double *x, double *y);
```

---

### Task 1: 通用实体 getter

- 写失败测试：自造 Line 文档读回，断言 layer/color/linetype/linetype_scale。
- 实现：`as_entity` 已有；`dwg_entity_get_layer` 用 `entity.common()` 的 layer；color 用 `color_to_c`。
  `EntityType::common()` 返回 `&EntityCommon`（mod.rs:612）。
- 提交：`feat(ffi): common entity getters (layer/color/linetype)`

### Task 2: 几何家族（Ellipse/Ray/XLine/Solid/Face3D）

- `as_ellipse`/`as_ray`/`as_xline`/`as_solid`/`as_face3d` 辅助 + 宏 getter。
- corner 参数校验 `1..=4`，越界 `DWG_EINVAL`。
- 提交：`feat(ffi): ellipse/ray/xline/solid/face3d getters`

### Task 3: Spline/MText/LwPolyline

- Spline degree/count/control_point；MText value/insertion_point/height（仿 Text）；
  LwPolyline vertex_count/get_vertex（Vector2 双 double）。
- 提交：`feat(ffi): spline/mtext/lwpolyline getters`

### Task 4: C 集成测试

- `c_entity_batch1.c`：写读自造含 Ellipse/MText/Spline/LwPolyline 的文档，
  遍历断言类型与关键字段（ellipse minor_axis_ratio、mtext value、spline degree、lwpolyline 顶点数）。
- abi_test 编译运行；提交：`test(ffi): C entity batch1 smoke`

---

## Self-Review

- 覆盖：common + 8 个实体族；宏模式与 P2 一致。
- 无占位符：Task 1-3 有明确实现路径，Task 4 有 C 测试结构。
- 类型一致：`int32_t` 颜色、`double*` 向量、corner 1..=4 约定一致。
