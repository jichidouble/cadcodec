//! Document lifecycle: create/destroy an opaque `DwgData` handle and query
//! its version.

use crate::document::CadDocument;
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;

/// Create a new empty document. On success `*out` receives a document handle
/// owned by the caller (release with `dwg_data_destroy`).
#[no_mangle]
pub extern "C" fn dwg_data_create(out: *mut *mut DwgData) -> i32 {
    if out.is_null() {
        return DWG_EINVAL;
    }
    let data = Box::new(FfiDocument::new(CadDocument::new()));
    unsafe {
        *out = Box::into_raw(data) as *mut DwgData;
    }
    clear_last_error();
    DWG_OK
}

/// Write the number of live document entities to `count`.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_entity_count(data: *const DwgData, count: *mut i32) -> i32 {
    if data.is_null() || count.is_null() {
        return DWG_EINVAL;
    }
    let ffi = &*(data as *const FfiDocument);
    *count = ffi.doc.entity_count() as i32;
    clear_last_error();
    DWG_OK
}

/// Write the number of live entities and objects to `count`.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_object_count(data: *const DwgData, count: *mut i32) -> i32 {
    if data.is_null() || count.is_null() {
        return DWG_EINVAL;
    }
    let ffi = &*(data as *const FfiDocument);
    *count = ffi.objects.iter().filter(|object| object.alive).count() as i32;
    clear_last_error();
    DWG_OK
}

/// Destroy a document previously created by `dwg_data_create`. All object
/// references obtained from this document become invalid.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_destroy(data: *mut DwgData) {
    if data.is_null() {
        return;
    }
    drop(Box::from_raw(data as *mut FfiDocument));
}

/// Copy the document's DWG version string (e.g. "AC1032"; a fresh document
/// created by `dwg_data_create` is AC1032) into `buf` (caller-owned).
/// `needed` receives the total bytes required including NUL.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_version(
    data: *const DwgData,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if data.is_null() || needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let ffi = &*(data as *const FfiDocument);
    let ver = ffi.doc.version.as_str();
    let bytes = ver.as_bytes();
    let total = bytes.len() + 1;
    *needed = total;
    if buf.is_null() {
        return DWG_OK;
    }
    if cap < total {
        set_last_error(DWG_EBUFFER, "buffer too small");
        return DWG_EBUFFER;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
    *buf.add(bytes.len()) = 0;
    DWG_OK
}
