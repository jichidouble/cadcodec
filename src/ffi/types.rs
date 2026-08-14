//! Opaque handle types and C error-code constants.

/// Opaque handle to a drawing document (`Box<CadDocument>` behind the
/// pointer).  Allocated with [`dwg_data_create`](super::data::dwg_data_create)
/// and released with [`dwg_data_destroy`](super::data::dwg_data_destroy).
#[repr(C)]
pub struct DwgData {
    _priv: [u8; 0],
}

/// Opaque handle to a document object (reserved for the traversal phase).
#[repr(C)]
pub struct DwgObject {
    _priv: [u8; 0],
}

/// Success.
pub const DWG_OK: i32 = 0;
/// Invalid argument (NULL handle/output pointer).
pub const DWG_EINVAL: i32 = 1;
/// Allocation failure.
pub const DWG_ENOMEM: i32 = 2;
/// File read failure.
pub const DWG_EREAD: i32 = 3;
/// File write failure.
pub const DWG_EWRITE: i32 = 4;
/// Unsupported or unknown file version.
pub const DWG_EVERSION: i32 = 5;
/// Required object or file not found.
pub const DWG_ENOTFOUND: i32 = 6;
/// Caller-owned buffer too small (`*needed` holds the required size).
pub const DWG_EBUFFER: i32 = 7;
/// Wrong object/entity type for the requested accessor.
pub const DWG_ETYPE: i32 = 8;
