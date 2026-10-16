# FFI Phase 4b3 XData/EED Design

## Goal

Expose entity XData through the existing C ABI with document-owned entity
handles, caller-owned buffers, typed values, and durable DXF/DWG persistence.
The API must operate on the structured `ExtendedData` model while preserving
the existing raw DWG EED round-trip behavior for untouched applications.

## Scope and Non-Goals

This slice covers application-record lookup, enumeration, insertion, removal,
and record-value read/append/replace/remove. It does not expose raw EED bytes,
ACIS, callbacks, cbindgen-generated headers, or XRECORD operations. It keeps
the current `DwgData` lifecycle and does not add a new object-pointer lifetime
contract.

## ABI Model

All functions use a document-owned entity handle (`uint64_t`) and validate it
against the target `DwgData`. No `DwgObject *`, Rust `Vec` address, or Rust
allocation is retained across the ABI.

```c
typedef struct DwgXdataValue {
    uint32_t tag;
    uint32_t reserved;
    int64_t integer;
    double real;
    double point[3];
    uint64_t handle;
    void *data;
    size_t data_len;
} DwgXdataValue;
```

Tags are fixed: `STRING=1`, `CONTROL_STRING=2`, `LAYER_NAME=3`, `BINARY=4`,
`HANDLE=5`, `POINT3D=6`, `POSITION3D=7`, `DISPLACEMENT3D=8`, `DIRECTION3D=9`,
`REAL=10`, `DISTANCE=11`, `SCALE_FACTOR=12`, `INTEGER16=13`, and `INTEGER32=14`.
String output is NUL-inclusive; binary output is byte-counted. `data_len` is
input length for mutations and output capacity/required length for reads.

## Public Operations

The ABI consists of these nine operations:

```c
int dwg_entity_xdata_record_count(const DwgData *, uint64_t entity, size_t *count);
int dwg_entity_xdata_record_find(const DwgData *, uint64_t entity,
                                 const char *application_name,
                                 size_t occurrence, size_t *index);
int dwg_entity_xdata_record_entry(const DwgData *, uint64_t entity,
                                  size_t record_index, char *application_name,
                                  size_t capacity, size_t *required,
                                  size_t *value_count);
int dwg_entity_xdata_record_append(DwgData *, uint64_t entity,
                                   const char *application_name,
                                   size_t *record_index);
int dwg_entity_xdata_record_remove(DwgData *, uint64_t entity,
                                   size_t record_index, char *application_name,
                                   size_t capacity, size_t *required);
int dwg_entity_xdata_value(const DwgData *, uint64_t entity,
                           size_t record_index, size_t value_index,
                           DwgXdataValue *value);
int dwg_entity_xdata_value_append(DwgData *, uint64_t entity,
                                  size_t record_index,
                                  const DwgXdataValue *value);
int dwg_entity_xdata_value_set(DwgData *, uint64_t entity,
                               size_t record_index, size_t value_index,
                               const DwgXdataValue *value);
int dwg_entity_xdata_value_remove(DwgData *, uint64_t entity,
                                  size_t record_index, size_t value_index,
                                  DwgXdataValue *value);
```

`record_append` creates an empty record and registers the application in the
APPID table when necessary. New records reject duplicate application names;
`record_find` still accepts an occurrence index for legacy documents containing
duplicates. Removing the last value does not implicitly remove its record;
record lifetime is controlled only by the record operations.

## Validation and Ownership

Null pointers, empty application names, invalid UTF-8, unknown tags, nonzero
`reserved`, malformed buffers, non-finite numbers, and invalid indices return
`DWG_EINVAL`. Missing entities, records, values, or document-owned handle
payloads return `DWG_ENOTFOUND`; non-entity handles and incompatible outputs
return `DWG_ETYPE`; insufficient output capacity returns `DWG_EBUFFER`.
Every failed mutation leaves the entity unchanged and sets TLS error state;
successful calls clear it. Handle payloads must resolve in the same document.
Control strings accept only `{` and `}`.

## Persistence Rules

Before any record or value mutation, remove all matching application handles
from `EntityCommon::extended_data.raw_dwg_eed`. This prevents stale verbatim
EED from winning over the new structured values. The existing DWG writer then
encodes the structured record using the target version, while the DXF writer
emits the same `XDataValue` sequence. Untouched applications retain their raw
DWG blocks byte-for-byte.

## Verification

Rust ABI tests cover all tags, record/value lifecycle, invalid arguments,
foreign entities and handles, buffer contracts, TLS behavior, and preservation
after failed mutations. A MinGW C smoke test verifies the exact C layout,
symbol linkage, caller-owned buffers, and mutations. Focused DWG (R2004 and
R2018) and DXF round-trips reopen the files and compare application order and
all structured values.
