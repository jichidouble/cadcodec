# FFI Phase 3c3 集合字段 Setter 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为现有集合 getter 补齐逐项替换和 append/remove/clear 写接口，并为 MESH 补充面索引读取能力。

**Architecture:** 所有入口继续通过 Phase 3c1 的 `FfiDocument::mutate_entity`。索引统一为 0-based `int32_t`；无效索引、非法拓扑和不满足 Spline 次数约束均返回 `DWG_EINVAL`。Polyline 子实体使用现有构造器创建。MESH 的面变更后重算边；顶点删除仅允许未被面引用的顶点，并同步下调后续索引。

**Tech Stack:** Rust 2021、C ABI、现有 `ffi` feature、MinGW C smoke harness。

## Global Constraints

- 必须先完成 Phase 3c1 和 3c2。
- 计数 getter 保持只读；append 不返回索引，调用方用 count getter 查询。
- 所有坐标必须有限；负索引、越界索引、NULL face indices、少于 3 个顶点的面均为 `DWG_EINVAL`。
- MESH 面索引使用 0-based `uint32_t`，每个索引必须小于当前 vertex count。
- PolyfaceMesh ABI 顶点索引仍为 0-based；内部 face 引用是带符号的 1-based 索引，删除顶点时必须保留符号并重映射。
- `compute_edges()` 会重建边并清除已有 crease；本阶段面写接口接受该行为，不额外公开 edge/crease API。
- 不增加实体创建/删除、knots/weights、Polyline bulge/width、Polyface face 或 P4 API。

## File Structure

- Modify: `src/ffi/entity.rs` — 新增 MESH face vertex getter。
- Modify: `src/ffi/entity_set.rs` — 集合 setter、append/remove/clear 和拓扑校验。
- Modify: `tests/abi_test.rs` — Rust ABI 集合与错误路径测试。
- Create: `tests/ffi/c_entity_set_collections.c` — C 集合 smoke test。

## Interfaces

```c
int dwg_entity_spline_set_degree(DwgData*, const DwgObject*, int32_t);
int dwg_entity_spline_set_control_point(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_spline_append_control_point(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_spline_remove_control_point(DwgData*, const DwgObject*, int32_t);
int dwg_entity_spline_clear_control_points(DwgData*, const DwgObject*);

int dwg_entity_lwpolyline_set_vertex(DwgData*, const DwgObject*, int32_t, double, double);
int dwg_entity_lwpolyline_append_vertex(DwgData*, const DwgObject*, double, double);
int dwg_entity_lwpolyline_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_lwpolyline_clear_vertices(DwgData*, const DwgObject*);

int dwg_entity_polyline2d_set_vertex(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_polyline2d_append_vertex(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_polyline2d_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_polyline2d_clear_vertices(DwgData*, const DwgObject*);
int dwg_entity_polyline3d_set_vertex(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_polyline3d_append_vertex(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_polyline3d_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_polyline3d_clear_vertices(DwgData*, const DwgObject*);
int dwg_entity_polyface_mesh_set_vertex(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_polyface_mesh_append_vertex(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_polyface_mesh_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_polyface_mesh_clear_vertices(DwgData*, const DwgObject*);

int dwg_entity_leader_set_vertex(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_leader_append_vertex(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_leader_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_leader_clear_vertices(DwgData*, const DwgObject*);

int dwg_entity_mesh_set_vertex(DwgData*, const DwgObject*, int32_t, double, double, double);
int dwg_entity_mesh_append_vertex(DwgData*, const DwgObject*, double, double, double);
int dwg_entity_mesh_remove_vertex(DwgData*, const DwgObject*, int32_t);
int dwg_entity_mesh_clear_vertices(DwgData*, const DwgObject*);
int dwg_entity_mesh_get_face_vertex(const DwgObject*, int32_t, int32_t, uint32_t*);
int dwg_entity_mesh_set_face_vertex(DwgData*, const DwgObject*, int32_t, int32_t, uint32_t);
int dwg_entity_mesh_set_face(DwgData*, const DwgObject*, int32_t, const uint32_t*, size_t);
int dwg_entity_mesh_append_face(DwgData*, const DwgObject*, const uint32_t*, size_t);
int dwg_entity_mesh_remove_face(DwgData*, const DwgObject*, int32_t);
int dwg_entity_mesh_clear_faces(DwgData*, const DwgObject*);
```

---

### Task 1: Spline degree 与控制点集合

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: `Spline::degree`, `Spline::control_points`。
- Produces: 5 个 Spline 写接口。

- [ ] **Step 1: 写失败测试**

新增 `phase3c_spline_collection_setters`：从至少含 4 个控制点的 Spline 开始，依次 set index 1、append、remove appended point、set degree 2、clear，并用 degree/count/item getter 逐步读回。断言 degree `0/26`、负数/越界索引、NaN 坐标返回 `DWG_EINVAL`。

另写不变量断言：非空控制点集合必须满足 `degree >= 1 && degree < control_points.len()`；删除后若剩余非空点数 `<= degree` 则拒绝，显式 clear 允许得到空集合。含 fit points 的 Spline 拒绝全部控制点写操作，避免 DWG writer 选择 fit-point scenario 后静默丢弃改动。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --release --features ffi phase3c_spline_collection_setters`

Expected: 编译失败，5 个 setter 尚未定义。

- [ ] **Step 3: 实现 Spline 接口**

实现内部校验 `valid_index(index: i32, len: usize) -> Option<usize>`。`set_degree` 接受 `1..=25`，集合非空时要求 `degree < len`。所有控制点入口先拒绝非空 `fit_points`；set 只替换坐标。append/remove 再校验变更后的数量；若 `weights` 非空，append 同步追加 `1.0`、remove 同步删除对应 weight。set degree、append 和 remove 后使用 `Spline::generate_clamped_knots` 重建 knot vector；clear 同时清空 control points、knots 和 weights。由此不保留自定义 knot 分布，但保证公开写操作后的结构可被 DWG/DXF writer 安全持久化。

- [ ] **Step 4: 验证并提交**

Run: `cargo test --release --features ffi phase3c_spline_collection_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add spline collection setters"
```

---

### Task 2: Polyline 系列集合写接口

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: `LwVertex::new`, `Vertex2D::new`, `Vertex3DPolyline::new`, `PolyfaceVertex::new`。
- Produces: LwPolyline、Polyline2D、Polyline3D、PolyfaceMesh 共 16 个写接口。

- [ ] **Step 1: 写失败测试**

新增 `phase3c_polyline_collection_setters`，对四类实体均执行：记录初始 count、替换 index 0、append 固定坐标、通过 getter 验证、remove appended index、clear 后 count 为 0。LwPolyline 使用 `(11, 12)`，其余使用 `(21, 22, 23)`。每类都断言 `-1`、`count`、NaN 坐标为 `DWG_EINVAL`，并至少用一次错误类型对象断言 `DWG_ETYPE`。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --release --features ffi phase3c_polyline_collection_setters`

Expected: 编译失败，新 ABI 未定义。

- [ ] **Step 3: 实现构造与变更**

实现 `vec3_collection_setters!`，参数明确包含四个导出函数名、entity kind、enum variant、元素构造器和坐标字段。普通 Polyline 的调用清单固定为：

```rust
vec3_collection_setters!(
    dwg_entity_polyline2d_set_vertex,
    dwg_entity_polyline2d_append_vertex,
    dwg_entity_polyline2d_remove_vertex,
    dwg_entity_polyline2d_clear_vertices,
    Polyline2D,
    Polyline2D,
    Vertex2D::new,
    location
);
vec3_collection_setters!(
    dwg_entity_polyline3d_set_vertex,
    dwg_entity_polyline3d_append_vertex,
    dwg_entity_polyline3d_remove_vertex,
    dwg_entity_polyline3d_clear_vertices,
    Polyline3D,
    Polyline3D,
    Vertex3DPolyline::new,
    position
);
```

LwPolyline 手写 2D 版本并用 `LwVertex::new(Vector2::new(x, y))`。set 只改坐标字段，以保留既有 bulge/width/handle；append 使用构造器；remove 用 `Vec::remove`；clear 用 `Vec::clear`。

PolyfaceMesh 手写拓扑版本：ABI index `i` 对应 face 内部引用 `i + 1`。append 在 `vertices.len() >= i16::MAX as usize` 时拒绝。remove 使用 `i16::unsigned_abs()` 检查 `index1..index4`，绝对值等于目标引用时拒绝；成功删除后，对绝对值大于目标引用的每个非零 face index 减 1，并保留原正负号。clear 仅在 `faces.is_empty()` 时允许。增加 `entity_set.rs` 内部测试，覆盖负数 invisible-edge 引用的拒绝与符号保持重映射。

- [ ] **Step 4: 验证并提交**

Run: `cargo test --release --features ffi phase3c_polyline_collection_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add polyline collection setters"
```

---

### Task 3: Leader 与 Mesh 顶点写接口

**Files:**
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`
- Test: `src/ffi/entity_set.rs` 内部测试模块

**Interfaces:**
- Consumes: `Leader::vertices`, `Mesh::vertices/faces/edges`。
- Produces: Leader 4 个、Mesh 顶点 4 个写接口。

- [ ] **Step 1: 写失败测试**

新增 `phase3c_leader_and_mesh_vertex_setters`。Leader 按 set/append/remove/clear 顺序验证。Mesh fixture 建立 4 个顶点和面 `[0,1,2]`：set index 3 成功；append/remove 新的未引用顶点成功；删除 index 1 返回 `DWG_EINVAL` 且 getter 仍读到原数据；另建 edge 引用测试，删除 edge 端点同样返回 `DWG_EINVAL`；clear vertices 在 faces 非空时返回 `DWG_EINVAL`。

再在 `entity_set.rs` 内部测试中建立无 face Mesh，删除 index 1 后断言后续顶点前移，已有 edge 的 start/end 大于 1 时均减 1；clear 后 vertices/edges 均为空。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --release --features ffi phase3c_leader_and_mesh_vertex_setters`

Expected: 编译失败，新 setter 未定义。

- [ ] **Step 3: 实现 Leader 与 Mesh 规则**

Leader 直接修改 `Vec<Vector3>`。Mesh remove 在删除前同时检查 `faces` 的 vertex indices 和 `edges` 的 start/end，任一引用目标 index 即返回 `DWG_EINVAL`；删除成功后，将所有大于 index 的 face indices 以及 edge `start/end` 减 1。`clear_vertices` 仅在 `faces.is_empty()` 时清除 vertices 和 edges，否则拒绝。

- [ ] **Step 4: 验证并提交**

Run: `cargo test --release --features ffi phase3c_leader_and_mesh_vertex_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add leader and mesh vertex setters"
```

---

### Task 4: Mesh 面读取、替换与增删

**Files:**
- Modify: `src/ffi/entity.rs`
- Modify: `src/ffi/entity_set.rs`
- Modify: `tests/abi_test.rs`
- Test: `src/ffi/entity_set.rs` 内部测试模块

**Interfaces:**
- Consumes: `MeshFace::new`, `Mesh::compute_edges`。
- Produces: 1 个缺失 getter 与 6 个 face 写接口。

- [ ] **Step 1: 写失败测试**

新增 `phase3c_mesh_face_setters`。先用新 getter 读取 face 0 slot 1；调用 `set_face_vertex(..., 0, 1, 3)` 并读回；用 `[0, 2, 3]` 替换；append `[0, 1, 3]`；remove appended face；clear 后 face count 为 0。在 `entity_set.rs` 内部测试中另行断言每次成功 face 变更后 `edges` 与 `compute_edges()` 结果一致，clear 后 edges 为空。

断言 face/slot 负数或越界、NULL indices、`count < 3`、任一 vertex index 越界、错误对象类型均返回对应 `DWG_EINVAL`/`DWG_ETYPE`，失败后原 face 和 edges 不变。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --release --features ffi phase3c_mesh_face_setters`

Expected: 编译失败，face ABI 未定义。

- [ ] **Step 3: 实现 getter 和统一 face 校验**

在 `entity.rs` 实现 `dwg_entity_mesh_get_face_vertex`：先检查输出指针，再分别验证 face 和 slot 的非负及边界，成功时把 `usize` 经 `u32::try_from` 写出；转换失败返回 `DWG_EINVAL`。

在 `entity_set.rs` 实现：

```rust
unsafe fn read_face_indices(
    indices: *const u32,
    count: usize,
    vertex_count: usize,
) -> Result<Vec<usize>, MutationError>;
```

该 helper 要求非 NULL、`count >= 3`，并逐项验证 `< vertex_count`。所有 face 写接口先完成全部校验再改集合，成功后调用 `mesh.compute_edges()`；`set_face_vertex` 修改单个 slot 后同样重算边。

- [ ] **Step 4: 验证并提交**

Run: `cargo test --release --features ffi phase3c_mesh_face_setters`

Expected: 全绿。

```powershell
git add src/ffi/entity.rs src/ffi/entity_set.rs tests/abi_test.rs
git commit -m "feat(ffi): add mesh face accessors"
```

---

### Task 5: 集合 C smoke test

**Files:**
- Create: `tests/ffi/c_entity_set_collections.c`
- Modify: `tests/abi_test.rs`

**Interfaces:**
- Consumes: Task 1-4 的集合 ABI。
- Produces: 0-based 索引、caller-owned face array 与错误码的 C 运行验证。

- [ ] **Step 1: 写 C 程序**

C 程序读取包含 SPLINE、LWPOLYLINE、LEADER、MESH 的 fixture，执行并验证：Spline append 后 count +1；LwPolyline set index 0 后 getter 返回新坐标；Leader remove `-1` 返回 `DWG_EINVAL`；Mesh append vertex 后用本地数组 `{0, 1, new_index}` append face，并通过 face count/vertex getter 读回。

- [ ] **Step 2: 扩展 Rust harness**

新增 `c_entity_set_collections_compiles_links_and_runs`，复用现有 GNU DLL 链接流程，传入专用 fixture，断言 C 进程 exit code 0。

- [ ] **Step 3: 运行验证**

Run: `cargo test --release --features ffi c_entity_set_collections_compiles_links_and_runs -- --nocapture`

Expected: 1 passed，C 程序退出 0。

Run: `cargo test --release --features ffi`

Expected: 全绿。

- [ ] **Step 4: 提交**

```powershell
git add tests/ffi/c_entity_set_collections.c tests/abi_test.rs
git commit -m "test(ffi): add collection setter C smoke test"
```

---

## Self-Review

- Getter pairing: 7 个集合 item getter 均有同名 `set_*` 配对；9 个 count getter保持只读；Mesh 新增 face item getter。
- Index consistency: 所有 ABI 均使用现有 getter 的 0-based `int32_t`，无符号转换前先拒绝负数。
- Invariants: Spline degree、Mesh face 引用、删除重映射和 face/edge 同步都有失败测试。
- Boundary: knots、weights、bulge、width、crease 与 Polyface faces 不在本阶段范围。

## Execution Handoff

本计划完成后继续执行 `2026-08-17-ffi-phase3c4-completeness-roundtrip.md`。
