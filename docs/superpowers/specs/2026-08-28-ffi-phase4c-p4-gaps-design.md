# FFI Phase 4c P4 Gap Design

## Goal

Complete the remaining P4 C ABI capabilities without changing the Rust core's
ownership model: object removal, diagnostic callbacks, and raw ACIS payload
access.

## Scope and Order

The work is split into three independently testable slices:

1. P4c1 removes document-owned entities or objects through a stable
   `DwgObject*` handle.
2. P4c2 dispatches the existing `NotificationCollection` to a caller-owned
   callback after read/write operations.
3. P4c3 reads and replaces SAT/SAB payloads for `3DSOLID`, `REGION`, `BODY`,
   and `SURFACE`.

ACIS parsing/building, new entity families, and cbindgen remain outside this
slice. Existing Dictionary, XRECORD, XData, and byte-stream APIs are not
changed.

## Ownership and Invalid References

`DwgData` remains the sole owner of all objects. `dwg_object_remove` removes
the corresponding core entry and marks the FFI arena record dead instead of
freeing it early. This keeps a previously returned pointer memory-safe until
`dwg_data_destroy`; all operations on a dead pointer return
`DWG_ENOTFOUND`. The iteration chain skips dead records. Entity removal also
updates the core entity index, block membership, and associated solid-history
objects where applicable.

## Callback ABI

```c
typedef void (*DwgNotificationCallback)(int type, const char *message,
                                        void *user_data);
int dwg_set_notification_callback(DwgData *data,
                                  DwgNotificationCallback callback,
                                  void *user_data);
```

The callback is stored per document, invoked synchronously after a successful
read that produced notifications, and receives a temporary NUL-terminated
message valid only during the call. Passing `NULL` clears it. Callback
panics/unwinding must not cross the C ABI. Writers currently do not produce
new notifications, so a successful write does not replay stale read messages.

## ACIS ABI

The raw representation is explicit: SAT uses a NUL-terminated UTF-8 string;
SAB uses caller-owned bytes. Getters use `buf/cap/needed`, while setters copy
all input before returning. Separate SAT and SAB setters prevent accidental
reinterpretation of binary data. Setting a payload resets only the entity's
`AcisData` payload representation and preserves the entity handle/ownership.

## Validation

Null, foreign, dead, wrong-type, invalid UTF-8, and insufficient-buffer cases
return the existing FFI error codes and leave the document unchanged. Every
slice has Rust ABI tests, a C smoke test, and a minimal DWG/DXF round-trip
where persistence is relevant.
