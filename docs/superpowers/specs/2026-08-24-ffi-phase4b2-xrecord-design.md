# FFI Phase 4b2 XRECORD Design

## Goal

Expose safe typed XRECORD access through the existing C ABI without exposing
Rust collection addresses, temporary pointers, or allocator ownership.

## Scope and Order

P4b2 follows the completed Dictionary slice. C callers first obtain an
XRECORD handle through a Dictionary entry, then use that handle for typed
lookup, enumeration, insertion, replacement, and removal. Entity XData/EED,
ACIS, callbacks, cbindgen, and unrelated object types remain outside this
slice.

## ABI Value Model

The public value is a `repr(C)` tagged structure with fixed-width scalar
slots, three `double` point slots, and a caller-owned `void *data` plus
`size_t data_len` for strings and binary chunks. The tag selects exactly one
payload interpretation; unused slots are ignored. The same structure is used
for input mutations and output reads:

- input string/chunk: `data` points to readable bytes and `data_len` is length;
- output string/chunk: `data` points to writable storage and `data_len` is
  capacity on input and required/written length on output;
- output size queries accept `data == NULL` and return the required length;
- output strings include the terminating NUL in `data_len`.

Supported tags are String, Double, Int16, Int32, Int64, Byte, Bool, Handle,
Point3D, and Chunk. An unknown tag returns `DWG_EINVAL`; a known tag that
does not match the group code returns `DWG_ETYPE`.

## Public Operations

The ABI provides `dwg_xrecord_count`, `dwg_xrecord_find`,
`dwg_xrecord_entry`, `dwg_xrecord_append`, `dwg_xrecord_set`, and
`dwg_xrecord_remove`. `find` searches by group code and zero-based occurrence;
`entry` and `remove` use zero-based entry indexes. `entry` returns the group
code and decodes the value; `remove` returns the removed code and value.

## Validation and Mutation

Null pointers, invalid capacities, malformed input UTF-8, unknown tags, and
non-finite input numeric values return `DWG_EINVAL`. Missing XRECORD handles or
indexes return `DWG_ENOTFOUND`; non-XRECORD handles return `DWG_ETYPE`; an
insufficient output buffer returns `DWG_EBUFFER`. Failed calls preserve the
document and set TLS; successful calls clear TLS.

Handle/ObjectId values must reference objects in the same document before a
mutation is applied. Mutations validate the complete code/tag/value first,
then update `XRecord.entries`, set `entries_complete`, regenerate object
references for codes 330–369, and disable preservation of the old decoded
handle stream. No caller-owned pointer is retained by the document.

## Testing

Rust ABI tests cover every tag, size queries, invalid combinations, foreign
handles, failure preservation, and occurrence-based group-code lookup. A C smoke test
checks exact prototypes, caller-owned buffers, all mutation paths, and symbol
linkage. DWG and DXF round-trip tests verify strings, numeric values, points,
chunks, and object references survive persistence.
