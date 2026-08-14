//! Opaque C ABI types and error codes.

/// Opaque document handle. Points to `Box<CadDocument>`; the document is the
/// sole ownership root. Created by `dwg_data_create`, destroyed by
/// `dwg_data_destroy`.
#[repr(C)]
pub struct DwgData {
    _priv: [u8; 0],
}

/// Opaque reference to an object inside a document. Owned by the document;
/// never freed by the caller. Stable until the document is destroyed.
#[repr(C)]
pub struct DwgObject {
    _priv: [u8; 0],
}

/// Success.
pub const DWG_OK: i32 = 0;
/// Invalid argument or NULL pointer.
pub const DWG_EINVAL: i32 = 1;
/// Allocation failure.
pub const DWG_ENOMEM: i32 = 2;
/// Read failure.
pub const DWG_EREAD: i32 = 3;
/// Write failure.
pub const DWG_EWRITE: i32 = 4;
/// Unsupported version.
pub const DWG_EVERSION: i32 = 5;
/// Object/entity not found.
pub const DWG_ENOTFOUND: i32 = 6;
/// Buffer too small (`needed` carries the required size).
pub const DWG_EBUFFER: i32 = 7;
/// Type mismatch.
pub const DWG_ETYPE: i32 = 8;
