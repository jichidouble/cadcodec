# FFI Phase 3c4 完整性与跨格式验证实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用自动化接口盘点、错误矩阵、DWG/DXF round-trip 和全量质量门禁证明 P3c setter 覆盖完整且可持久化。

**Architecture:** 先用显式 ABI 清单生成 C 翻译单元并真实编译、链接全部 getter/setter 导出，再通过 FFI 入口执行 read → mutate → write → read。版本矩阵只选各版本均支持的基础实体；高级实体使用现有支持版本的专用 fixture。最终验证不引入头文件生成或 P4 API。

**Tech Stack:** Rust integration tests、C ABI、DWG/DXF codecs、Cargo fmt/clippy/test。

## Global Constraints

- 必须先完成 Phase 3c1、3c2、3c3。
- 84 个原有 entity getter 中，11 个只读 getter必须进入显式白名单，其余 73 个必须存在按 `_get_` → `_set_` 命名的配对 setter。
- 只读白名单仅含 9 个集合 count getter，以及 Dimension subtype/measurement。
- round-trip 比较 getter 可观察值，不比较内部指针；每次重新打开文件并重新查找对象。
- 测试临时文件使用现有 fixture helper 创建并在成功/失败路径清理。

## File Structure

- Create: `tests/ffi_setter_inventory.rs` — ABI 清单配对与 C 链接测试。
- Modify: `tests/abi_test.rs` — 错误矩阵与 DWG/DXF round-trip。
- Modify: `docs/superpowers/specs/2026-08-17-ffi-phase3c-entity-setters-design.md` — 仅在实现暴露规范偏差时同步最终接口表；无偏差则不修改。

---

### Task 1: Getter/Setter ABI 清单与链接盘点

**Files:**
- Create: `tests/ffi_setter_inventory.rs`

**Interfaces:**
- Consumes: 84 项 getter ABI 清单、74 项 setter ABI 清单、GNU C linker。
- Produces: 73 个原有可写 getter 的一一配对门禁，以及新增 Mesh face setter 的导出验证。

- [ ] **Step 1: 写显式 ABI 清单及配对测试**

在 `tests/ffi_setter_inventory.rs` 定义固定长度清单 `const GETTERS: [&str; 84]`、`const SETTERS: [&str; 74]`。GETTERS 逐项列出 P3c 前已有的 84 个 `dwg_entity_*_get_*`；SETTERS 列出其 73 个可写配对以及新增的 `dwg_entity_mesh_set_face_vertex`。只读集合固定为：

```rust
const READ_ONLY: &[&str] = &[
    "dwg_entity_spline_get_control_point_count",
    "dwg_entity_lwpolyline_get_vertex_count",
    "dwg_entity_polyline2d_get_vertex_count",
    "dwg_entity_polyline3d_get_vertex_count",
    "dwg_entity_polyface_mesh_get_vertex_count",
    "dwg_entity_leader_get_vertex_count",
    "dwg_entity_mesh_get_vertex_count",
    "dwg_entity_mesh_get_face_count",
    "dwg_entity_mesh_get_face_vertex_count",
    "dwg_entity_dimension_get_subtype",
    "dwg_entity_dimension_get_measurement",
];
```

测试 `abi_manifest_pairs_every_writable_getter` 排除上述 11 项后，将每个 getter 名字的 `_get_` 替换为 `_set_` 并断言 SETTERS 包含结果；另断言 SETTERS 包含 `dwg_entity_mesh_set_face_vertex`。固定长度数组在编译期锁定 84/74，白名单固定长度为 11。

Run: `cargo test --release --features ffi --test ffi_setter_inventory`

Expected: 若清单缺失配对则测试列出 getter/setter 名；补齐后通过。

- [ ] **Step 2: 编译并链接全部 ABI 清单符号**

测试 `abi_manifest_symbols_export_from_cdylib` 从 GETTERS、SETTERS 以及新增 `dwg_entity_mesh_get_face_vertex` 生成临时 C 文件。每个符号使用 `extern void symbol(void);` 声明并将地址放入函数指针数组；程序不调用这些占位原型，只返回数组首项是否为 NULL。复用现有 GNU DLL 定位和链接参数编译、链接并运行，要求链接器解析 84 个原 getter、1 个新增 getter和 74 个 setter。真实链接结果是权威证据，源码注释或测试字符串不能伪造导出。

另在 Rust 配对测试中断言清单不存在 `dimension_set_subtype`、`dimension_set_measurement` 或任何 `set_*_count`；append/remove/clear 不参与一一配对，由各阶段的 C smoke 覆盖。

- [ ] **Step 3: 运行并提交**

Run: `cargo test --release --features ffi --test ffi_setter_inventory`

Expected: 全绿，报告 84/11/73 三个固定计数。

```powershell
git add tests/ffi_setter_inventory.rs
git commit -m "test(ffi): enforce entity setter inventory"
```

---

### Task 2: 统一错误语义矩阵

**Files:**
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: 各 setter 家族代表入口、`dwg_last_error_code`、`dwg_last_error`。
- Produces: NULL、foreign、wrong type、invalid value/index 的稳定错误码测试。

- [ ] **Step 1: 写表驱动失败测试**

新增 `phase3c_setter_error_matrix`，分别调用 common、scalar、vector、string、collection、Mesh face 六类代表 setter，覆盖：

| Case | Expected |
|---|---|
| NULL `DwgData*` / NULL `DwgObject*` / NULL required value | `DWG_EINVAL` |
| object 属于另一 `DwgData` | `DWG_ENOTFOUND` |
| object 类型与 setter 不符 | `DWG_ETYPE` |
| NaN/Inf、非正尺寸、零方向、负数或越界 index | `DWG_EINVAL` |
| 成功调用后的 last error | `DWG_OK` 且空消息 |

每个失败调用后读取 TLS last error，断言 code 与返回值一致且消息非空；随后调用合法 setter，断言错误状态被清除。失败调用前后用 getter 比较字段，证明无部分写入。

- [ ] **Step 2: 运行确认并修正实现**

Run: `cargo test --release --features ffi phase3c_setter_error_matrix`

Expected: 首次运行暴露任何未统一设置错误状态的路径；只在 `entity_set.rs`/`object.rs` 修正对应入口，不改变错误码表。

- [ ] **Step 3: 验证并提交**

Run: `cargo test --release --features ffi phase3c_setter_error_matrix`

Expected: 全绿。

```powershell
git add src/ffi/object.rs src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "test(ffi): standardize setter error semantics"
```

---

### Task 3: DWG 多版本持久化 round-trip

**Files:**
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: 现有 DWG fixture writer、`dwg_read_file`、`dwg_write_file`、基础 setter/getter。
- Produces: R13–R2018 八版本写后重读验证。

- [ ] **Step 1: 写版本矩阵测试**

新增 `phase3c_dwg_setters_roundtrip_all_versions`，遍历 `AC1012, AC1014, AC1015, AC1018, AC1021, AC1024, AC1027, AC1032`。每个版本创建至少包含 LINE、CIRCLE、TEXT 的文档，经 FFI 读取后写入：layer=`P3C_RT`、color=`0x112233`、Line start=`(11,12,13)`、Circle radius=`4.5`、Text value=`P3C DWG`。

调用 `dwg_write_file` 写到新路径，销毁原 `DwgData`，重新 `dwg_read_file` 并查找对应类型，用 getter 断言五项值。错误信息中必须包含版本名和临时路径，便于定位 codec 限制。

- [ ] **Step 2: 运行矩阵确认失败或通过**

Run: `cargo test --release --features ffi phase3c_dwg_setters_roundtrip_all_versions -- --nocapture`

Expected: 8 个版本全部写入、重读并保持 getter 可观察值；若命中已知 R13/R14 codec 限制，仅在有现有测试/文档证据时记录精确跳过，不扩大跳过范围。

- [ ] **Step 3: 增加现代 DWG 集合 round-trip**

新增 `phase3c_dwg_collection_setters_roundtrip`，使用 AC1032 fixture 对 LwPolyline append、Spline set control point、Mesh append vertex/face 执行写后重读；验证 count 和每个新增 item/face index。

Run: `cargo test --release --features ffi phase3c_dwg_collection_setters_roundtrip -- --nocapture`

Expected: 全部集合值持久化。

- [ ] **Step 4: 提交**

```powershell
git add tests/abi_test.rs
git commit -m "test(ffi): verify setter DWG roundtrips"
```

---

### Task 4: DXF round-trip 与最终质量门禁

**Files:**
- Modify: `tests/abi_test.rs`
- Modify: `docs/superpowers/specs/2026-08-17-ffi-phase3c-entity-setters-design.md` only if required by verified implementation differences

**Interfaces:**
- Consumes: 现有 DXF FFI read/write、前三项测试。
- Produces: DXF 持久化证据和 P3c 完成门禁。

- [ ] **Step 1: 写 DXF round-trip 测试**

新增 `phase3c_dxf_setters_roundtrip`，创建包含 LINE、TEXT、LWPOLYLINE 的 DXF fixture，调用 common、Line、Text 和 LwPolyline append setter，写出、销毁、重读，再用 getter 验证 layer、坐标、字符串和 vertex count/item。

Run: `cargo test --release --features ffi phase3c_dxf_setters_roundtrip -- --nocapture`

Expected: 全绿。

- [ ] **Step 2: 运行 P3c 定向测试**

Run: `cargo test --release --features ffi phase3c_ -- --nocapture`

Expected: 所有 Rust ABI 与 round-trip 测试通过。

Run: `cargo test --release --features ffi c_entity_set_ -- --nocapture`

Expected: basic、fixed、collections 三个 C smoke 全部通过。

- [ ] **Step 3: 运行仓库质量门禁**

Run: `cargo fmt --check`

Expected: exit 0；若失败，运行 `cargo fmt` 后复查。

Run: `cargo clippy --all-targets --features ffi -- -D warnings`

Expected: exit 0。

Run: `cargo test --release --features ffi`

Expected: 全绿。

Run: `cargo test --all-features`

Expected: 全绿。

- [ ] **Step 4: 核对规范并提交**

逐项比较设计规范的 ABI、校验规则、只读例外和非目标。只有已验证实现与规范确需一致化时才修改 spec，并在提交说明中列出差异；否则保持 spec 不变。

```powershell
git add tests/abi_test.rs docs/superpowers/specs/2026-08-17-ffi-phase3c-entity-setters-design.md
git commit -m "test(ffi): complete phase 3c verification"
```

---

## Self-Review

- Completeness: 自动门禁固定检查原有 84 getter、11 个只读例外和 73 个 getter/setter 配对。
- Persistence: DWG 八版本、现代集合 DWG 和 DXF 都执行真实 write/reopen/getter 验证。
- Error semantics: 六类 setter 覆盖返回码、TLS 消息、成功清除和失败不写入。
- Scope: 不生成公共头文件，不增加实体生命周期 API，不实现 P4。

## Execution Handoff

四份计划全部完成后，使用 `superpowers:verification-before-completion` 复核最终命令输出，再使用 `superpowers:requesting-code-review` 做阶段审查。
