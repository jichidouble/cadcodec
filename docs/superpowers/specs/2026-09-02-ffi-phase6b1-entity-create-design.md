# FFI Phase 6b-1: Basic Entity Creation

## Context

The C ABI can create and destroy documents, load files, iterate active
objects, resolve handles, remove objects, and mutate supported entities. It
cannot create new drawing entities, so a C consumer cannot build a document
from geometry without first producing a DWG/DXF file elsewhere.

## Goals

- Add C constructors for LINE, CIRCLE, ARC, and POINT.
- Add each new entity directly to the document's Model Space.
- Allocate handles through the existing `CadDocument::add_entity()` path.
- Return the new object through the existing document-owned FFI arena.
- Preserve stable object pointers, existing ownership, and first/next iteration
  semantics.
- Validate the ABI through focused Rust tests, the generated header, and a C
  consumer.

## Proposed API

```c
int32_t dwg_entity_line_create(
    DwgData *data,
    double x1, double y1, double z1,
    double x2, double y2, double z2,
    DwgObject **out);

int32_t dwg_entity_circle_create(
    DwgData *data,
    double x, double y, double z,
    double radius,
    DwgObject **out);

int32_t dwg_entity_arc_create(
    DwgData *data,
    double x, double y, double z,
    double radius,
    double start_angle, double end_angle,
    DwgObject **out);

int32_t dwg_entity_point_create(
    DwgData *data,
    double x, double y, double z,
    DwgObject **out);
```

The functions return `DWG_OK` and write a borrowed object pointer on success.
The caller must not free or retain the pointer after `dwg_object_remove` or
`dwg_data_destroy`. Null `data` or `out` returns `DWG_EINVAL`. Constructor
arguments are passed directly to the corresponding Rust constructors; no
additional geometric validation is added in this slice. A failed operation
does not add an entity.

## Architecture and Ownership

Implement a small `src/ffi/entity_create.rs` module for the four exported
functions and a private `FfiDocument` helper in `src/ffi/object.rs` that adds
an entity, creates its boxed arena record, and relinks the active chain. The
helper uses `CadDocument::add_entity()` so handle allocation and Model Space
routing remain centralized. The new boxed record stores a pointer to the
entity already owned by `CadDocument`; no second entity owner is introduced.

## Error Handling

Validate `data` and `out` before dereferencing either pointer. Return
`DWG_EINVAL` for invalid arguments and `DWG_OK` otherwise. On success, the
returned object has a nonzero allocated handle, the entity count increases by
one, and the object is discoverable through `dwg_data_first_object` and
`dwg_object_by_handle`.

## Testing

- Rust tests create all four entity kinds, verify type/handle/pointer identity,
  counts, first/next reachability, and independent documents.
- Rust tests cover null document/output arguments and confirm no entity is
  added on invalid calls.
- The C header smoke consumer creates a LINE using only `dwg_api.h`, checks its
  returned handle and count, and resolves it through handle lookup.
- Run only the focused creation/header tests, generated-header check,
  FFI-scoped rustfmt, GNU release build, and GNU C smoke test.

## Non-Goals

This slice does not add complex-entity constructors, arbitrary entity
deserialization, layout/block-targeted creation, batch creation, setter
changes, or changes to `CadDocument` ownership/storage.

## Acceptance Criteria

1. All four declarations are present in the generated header.
2. Each constructor adds exactly one Model Space entity with an allocated
   handle and a document-owned object pointer.
3. New objects participate in counts, iteration, and handle lookup.
4. Null document/output arguments return `DWG_EINVAL` without mutation.
5. Focused Rust and GNU C checks pass without modifying `Cargo.lock` or
   unrelated source files.
