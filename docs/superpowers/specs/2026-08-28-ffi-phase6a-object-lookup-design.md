# FFI Phase 6a: Object Lookup and Document Counts

## Context

The current C ABI can create an empty document, load a file, iterate active
objects, read handles, mutate supported entities, and remove objects. It has
no direct way to count the document or resolve a borrowed `DwgObject*` from a
known handle. C consumers therefore have to walk the entire object chain and
cannot efficiently reconnect references returned by dictionaries, tables, or
XRECORD APIs.

## Goals

- Add stable document entity and active-object counts.
- Add lookup of an active object by its 64-bit handle.
- Preserve opaque handles, stable arena addresses, and document-owned
  lifetimes.
- Return the existing `DWG_*` error codes consistently for invalid, removed,
  and missing values.
- Validate the new functions through focused Rust ABI tests and a C consumer.

## Proposed API

```c
int32_t dwg_data_entity_count(const DwgData *data, int32_t *count);
int32_t dwg_data_object_count(const DwgData *data, int32_t *count);
const DwgObject *dwg_object_by_handle(const DwgData *data,
                                      uint64_t handle,
                                      int32_t *error);
```

`dwg_data_entity_count` reports the document's current `entity_count()`, which
excludes structural block markers in the same way as `CadDocument::entities()`.
The object count reports active entries exposed by `dwg_data_first_object` and
`dwg_object_next`, including entities and non-entity objects. Removed entries
are excluded from both counts.

`dwg_object_by_handle` returns a borrowed active object reference. It sets
`*error` to `DWG_OK` on success, `DWG_EINVAL` for a null document or error
output, and `DWG_ENOTFOUND` for handle zero, a missing handle, or a removed
object. A NULL return always indicates failure. The returned pointer remains
owned by the document and must not be freed by C; it becomes unusable when the
object is removed or the document is destroyed.

## Implementation Boundary

Implement the functions in `src/ffi/data.rs` and `src/ffi/iterate.rs`, using
the existing `FfiDocument` arena, `CadDocument::entity_count()`, and active
arena entries. Do not change `CadDocument` storage, object ownership, pointer
allocation, or the meaning of existing iteration functions. Regenerate
`include/dwg_api.h` instead of editing it manually.

## Testing

- Add Rust ABI tests for empty and populated documents, lookup success, zero
  and missing handles, removed objects, independent documents, NULL output
  pointers, and count changes after removal.
- Extend the focused C consumer to include only `dwg_api.h`, create a document,
  resolve an iterated handle, verify counts, and confirm missing lookup errors.
- Run only the focused FFI test, header drift check, GNU C smoke test, and
  FFI-scoped formatting check. The full test matrix is out of scope.

## Non-Goals

This slice does not add entity creation, object-type expansion, table APIs,
DXF byte-stream APIs, or changes to the public ownership model.

## Acceptance Criteria

1. All three declarations are present in the generated header.
2. Counts match the active iteration chain before and after object removal.
3. Successful lookup returns the same stable pointer found by iteration.
4. Invalid, missing, and removed lookups never return a live object.
5. Focused Rust and GNU C checks pass without modifying `Cargo.lock` or
   unrelated source files.
