# FFI Phase 3c 实体 Setter 设计规格

## 1. 目标与范围

P3c 为现有 C ABI 实体读取面补齐写能力，使 C 调用者可以修改当前 84 个
`dwg_entity_*_get_*` getter 所暴露的全部可写数据，并将修改结果可靠写回 DWG/DXF。

本阶段不新增或删除文档实体，不改变实体类型，不实现 P4 的 Dictionary、XRECORD、
EED、ACIS、内存字节流或通知回调。实体 handle、对象类型、集合 count、Dimension
subtype 和 Dimension measurement 是身份、结构或派生元数据，保持只读；集合通过元素
更新及 append/remove/clear 写入。

## 2. 当前基线

- `src/ffi/entity.rs` 提供 84 个实体 getter，没有实体 setter。
- `FfiDocument` 拥有 `CadDocument` 和稳定的 `Vec<Box<FfiObject>>` arena。
- `FfiObject.entity` 是指向 `EntityType` 的原始只读指针。
- `CadDocument::get_entity_mut()` 使用 `Arc::make_mut()`；实体被共享时底层地址可能改变。
- 当前唯一表写接口是 `dwg_layer_set_color`。

因此 setter 不得直接写 `FfiObject.entity` 指向的内存。每次写入必须由所属
`FfiDocument` 路由，并在写入后刷新 arena 中对应的实体指针。

## 3. ABI 约定

所有实体 setter 使用以下形状：

```c
int dwg_entity_line_set_start(
    DwgData *data,
    const DwgObject *obj,
    double x,
    double y,
    double z);
```

- `data` 必须可写，`obj` 必须是该文档 arena 中仍有效的对象引用。
- setter 返回 `DWG_OK`、`DWG_EINVAL`、`DWG_ENOTFOUND` 或 `DWG_ETYPE`。
- 空指针、非法 UTF-8、非有限浮点数、非法索引和违反实体约束的值返回
  `DWG_EINVAL`。
- 外来或失效的 `DwgObject*` 返回 `DWG_ENOTFOUND`；类型不匹配返回
  `DWG_ETYPE`。
- 字符串输入统一为 NUL 结尾的 `const char *`，由调用方持有。
- 成功清空线程本地错误槽；失败写入错误码和可诊断消息。
- setter 不改变 `DwgObject*` 地址，现有遍历链继续有效。

## 4. 写入架构

在 `FfiDocument` 上增加唯一的实体变更入口，职责如下：

1. 先按指针地址在 `objects` arena 中查找 `obj`，不得先解引用未知指针。
2. 取得匹配对象的 handle，并确认其为期望实体类型。
3. 通过 `doc.get_entity_mut(handle)` 获得规范的可变实体。
4. 执行已经完成参数校验的变更闭包。
5. 变更结束后重新调用 `doc.get_entity(handle)`，刷新对应 `FfiObject.entity`。
6. 不重建整个 arena，不改变 `next` 链或其他 `DwgObject*`。

该入口放在 `src/ffi/object.rs`。setter 实现放在新的
`src/ffi/entity_set.rs`，避免继续扩大已经接近 900 行的 getter 模块。公共 C 字符串、
浮点数和索引校验放在 `src/ffi/util.rs`。

## 5. Setter 覆盖面

### 5.1 通用字段

新增 `dwg_entity_set_layer`、`dwg_entity_set_color`、
`dwg_entity_set_linetype` 和 `dwg_entity_set_linetype_scale`。handle、owner、对象类型和
内部 DWG round-trip 字段不在 P3c 暴露。

### 5.2 固定字段

每个现有标量、字符串、二维向量、三维向量或固定复合 getter 都提供对应 setter，
命名规则为把 `_get_` 替换为 `_set_`。覆盖以下实体族：

- Line、Circle、Arc、Point、Ellipse、Ray、XLine、Solid、Face3D；
- Text、MText、AttributeDefinition、Tolerance、MLine、Shape；
- Insert、Viewport、Hatch、Helix、Dimension；
- RasterImage、Underlay、Light、Wipeout。

复合字段保持一次调用原子更新，例如 `dwg_entity_insert_set_array` 同时接收列数、
行数及两个间距，`dwg_entity_raster_image_set_size` 同时接收宽高。corner 参数继续使用
现有的 `1..=4` 约定。Dimension subtype 表示 Rust 枚举变体，measurement 由各子类型的
几何点实时计算，二者均保持只读。

### 5.3 集合字段

集合 count getter 保持只读，不增加 `set_count`。为完整写能力增加：

- Spline：`set_control_point`、`append_control_point`、
  `remove_control_point`、`clear_control_points`；degree 使用普通 setter。
- LwPolyline、Polyline2D、Polyline3D、PolyfaceMesh、Leader：对应的
  `set_vertex`、`append_vertex`、`remove_vertex`、`clear_vertices`。
- Mesh：`set_vertex`、顶点增删清空，以及 face 的读取、替换、追加、删除和清空。
  P3c 同时补充缺失的 `dwg_entity_mesh_get_face_vertex`，使 face 写入可以验证。

删除操作使用零基索引；越界返回 `DWG_EINVAL`。可能破坏引用完整性的操作必须在变更前
验证，例如删除 Mesh 顶点时，若仍有 face 引用该顶点则拒绝操作。

## 6. 数值与领域校验

- 所有 `double` 输入必须是有限值。
- radius、text height、shape size、viewport width/height 和图像宽高必须大于零。
- Insert scale 不在当前 getter 面中，本阶段不新增其 setter。
- Spline degree 必须满足核心模型支持范围，并与当前控制点/节点数据相容。
- 方向向量和法向量不得为零向量；沿用核心实体已有归一化或校验方法。
- 字符串是否允许为空沿用 Rust 字段语义；layer 和 block name 不允许为空。
- setter 不自动创建缺失的 layer、block 或 style 表项，只写引用名称。

## 7. 测试与验收

按 TDD 分批实现，每批先增加失败测试。最低验收矩阵：

1. `object.rs` 单元测试强制持有实体 `Arc` 克隆，触发 `Arc::make_mut()` 地址变化，验证
   setter 后旧 `DwgObject*` 仍可通过 getter 读取新值。
2. 每个 setter 至少覆盖成功、NULL、外来对象、错误实体类型、非法值或越界中的适用项。
3. 每个实体族执行 setter 后立即调用对应 getter 验证值。
4. 字符串、向量、集合及复合字段分别有 C 编译、链接和运行测试。
5. 代表性实体执行 C setter → DWG/DXF 写出 → 重新读入 → getter 验证。
6. DWG 验证覆盖 R13、R14、R2000、R2004、R2007、R2010、R2013、R2018；仅断言该版本
   支持并可持久化的字段。
7. 最终运行 `cargo test --release --features ffi`、`cargo test --all-features`、
   `cargo clippy --all-targets --features ffi` 和 `cargo fmt --check`。

## 8. 文件边界

- Modify: `src/ffi/object.rs` — 文档归属校验、规范可变访问、指针刷新。
- Create: `src/ffi/entity_set.rs` — 所有实体 setter 和 setter 宏。
- Modify: `src/ffi/util.rs` — 输入字符串、浮点数、索引校验。
- Modify: `src/ffi/mod.rs` — 导出 setter 模块。
- Modify: `tests/abi_test.rs` — Rust ABI、失效指针和 round-trip 测试。
- Create: `tests/ffi/c_entity_set_*.c` — 分实体族的 C smoke tests。
- Modify: FFI 头文件生成输入或公开头文件（若 P5 生成流程尚未落地，则先维护测试声明）。

## 9. 非目标与后续阶段

P3c 不提供实体创建、删除、类型替换或 handle 修改，不修改 `CadDocument` 的 handle 存储
模型，也不实现非实体对象 setter。P3c 完成并通过版本矩阵后，P3 才视为完成；随后再进入
P4 高级对象与 bytes/callback API。
