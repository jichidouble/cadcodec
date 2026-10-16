# FFI Phase 4b Objects Design

## Goal

Expose Dictionary, XRECORD, and EED/XData access through the existing C ABI
with stable document-owned handles, caller-owned strings, and explicit typed
values.

## Scope and Order

P4b is delivered as three independently testable slices:

1. Dictionary lookup, enumeration, insertion, update, and removal.
2. XRECORD typed entry lookup, enumeration, insertion, update, and removal.
3. Entity XData/EED record lookup, enumeration, insertion, update, and removal.

Each slice adds Rust ABI tests, a C compile/link/run smoke test, and a focused
round-trip test. ACIS, callbacks, cbindgen, and unrelated object types remain
outside P4b.

## ABI Model

All operations receive `DwgData *` and use the existing document-owned object
storage. No Rust `Vec` address or temporary object pointer crosses the ABI.
Handles are represented by the existing fixed-width handle type. String
results use the established caller-provided `char *` plus capacity/needed
contract. Variable binary data uses `uint8_t *` plus length and is released
with `dwg_free_buffer`.

Typed XRECORD and XData values use a `repr(C)` tagged value structure. The tag
selects one scalar or pointer/length payload, and every pointer is validated
before reading. Unsupported or mismatched group-code/value combinations return
`DWG_ETYPE`; missing owners, keys, or records return `DWG_ENOTFOUND`.

## Mutation Rules

Mutations validate all arguments before changing the document. Failed
operations leave the document unchanged and set the existing thread-local
error. Successful operations clear it. Dictionary and XRECORD mutations use
the existing `CadDocument` helpers so owner handles, object handles, and
`HANDSEED` remain consistent. EED/XData mutations preserve application order
and raw DWG EED data needed for round-trip writing.

## Testing and Compatibility

Rust tests first establish argument validation, lookup/enumeration, mutation,
error preservation, and round-trip behavior. C smoke tests then verify the
opaque ABI, tagged value layout, string/binary ownership, and symbol linkage.
Focused commands use the on-demand aliases from the companion test workflow
spec; full FFI and all-features suites remain release gates.
