# FFI Phase 6b-2a: Fixed Geometry Entity Creation

## Context

The FFI can create LINE, CIRCLE, ARC, and POINT entities through the
document-owned object arena. The next low-risk creation slice covers entities
whose geometry is represented by a fixed number of numeric values: ELLIPSE,
3DFACE, and SOLID.

## Goals

- Add C constructors for ELLIPSE, 3DFACE, and SOLID.
- Add each entity directly to Model Space.
- Reuse `FfiDocument::add_entity` and `CadDocument::add_entity` for handle
  allocation, ownership, and stable `DwgObject *` storage.
- Preserve existing iteration, lookup, removal, and document-destruction
  semantics.
- Validate the ABI with focused Rust tests, the generated header, and a GNU C
  consumer.

## Proposed API

```c
int32_t dwg_entity_ellipse_create(
    DwgData *data,
    double center_x, double center_y, double center_z,
    double major_axis_x, double major_axis_y, double major_axis_z,
    double minor_axis_ratio,
    DwgObject **out);

int32_t dwg_entity_face3d_create(
    DwgData *data,
    double x1, double y1, double z1,
    double x2, double y2, double z2,
    double x3, double y3, double z3,
    double x4, double y4, double z4,
    DwgObject **out);

int32_t dwg_entity_solid_create(
    DwgData *data,
    double x1, double y1, double z1,
    double x2, double y2, double z2,
    double x3, double y3, double z3,
    double x4, double y4, double z4,
    DwgObject **out);
```

`dwg_entity_ellipse_create` passes the center, relative major-axis vector,
and minor-axis ratio to `Ellipse::from_center_axes`. The start/end parameters
default to a full ellipse and the normal defaults to +Z. The face and solid
constructors pass four WCS/OCS corner vectors to `Face3D::new` and
`Solid::new`, respectively. A triangular face or solid is represented by
passing the third corner again as the fourth corner.

The solid constructor creates a SOLID (`is_trace == false`); TRACE creation is
not part of this slice. Normal, thickness, invisible-edge flags, and other
entity properties retain their Rust constructor defaults.

## Error and Ownership Contract

- `data == NULL` or `out == NULL` returns `DWG_EINVAL` before dereferencing
  either pointer and does not mutate the document.
- Valid arguments return `DWG_OK`, clear the thread-local last error, and write
  a non-null borrowed `DwgObject *` with an allocated handle.
- Geometry values are passed directly to the Rust constructors. This slice
  adds no finite-value, ratio, degeneracy, or corner-order validation.
- The returned object is owned by `DwgData`; C must not free it and must stop
  using it after `dwg_object_remove` or `dwg_data_destroy`.

## Implementation and Testing

Extend the existing `src/ffi/entity_create.rs` module and its shared wrapper
for null validation and arena insertion; do not introduce another owner or
rebuild the object chain. Export exactly the three functions above and expose
them through `src/ffi/mod.rs` and the generated `include/dwg_api.h`.

Rust ABI tests must verify each entity type, one-object count delta, allocated
handle, iteration reachability, handle lookup identity, independent documents,
and null-argument non-mutation. The C consumer must use only
`include/dwg_api.h`, create at least one fixed-geometry entity, verify its type
and count/lookup result, and run through normal destruction. Run only the
focused tests, generated-header check, scoped rustfmt, GNU release build, and
GNU C smoke test; the full suite and local MSVC run remain separate gates.

## Non-Goals

This slice does not add TEXT/MTEXT, LWPOLYLINE/POLYLINE, SPLINE, TRACE,
layout- or block-targeted creation, batch creation, new setters, geometry
validation, or changes to `CadDocument` ownership/storage.

## Acceptance Criteria

1. The three declarations and symbols are present in the generated header and
   contain no Rust-internal types.
2. Each constructor adds exactly one correctly typed Model Space entity with a
   document-owned object and allocated handle.
3. New objects participate in counts, iteration, lookup, removal, and normal
   destruction without changing existing object addresses.
4. Null document/output arguments return `DWG_EINVAL` without mutation.
5. Focused Rust, header, GNU build, and GNU C checks pass without modifying
   `Cargo.lock` or unrelated behavior.
