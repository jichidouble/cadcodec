//! Document lifecycle and version query.

use std::ptr;

use crate::document::CadDocument;

use super::error::clear_last_error;
use super::types::{DwgData, DWG_EINVAL, DWG_EBUFFER, DWG_OK};

/// Allocate a new empty document handle.
///
/// `out` receives the new opaque handle and must be released with
/// [`dwg_data_destroy`].  Returns `DWG_EINVAL` when `out` is NULL.
#[no_mangle]
pub extern "C" fn dwg_data_create(out: *mut *mut DwgData) -> i32 {
    if out.is_null() {
        return DWG_EINVAL;
    }
    let data = Box::new(CadDocument::new());
    unsafe { *out = Box::into_raw(data) as *mut DwgData };
    clear_last_error();
    DWG_OK
}

/// Release a document handle previously returned by [`dwg_data_create`].
///
/// NULL is a no-op; the handle must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_destroy(data: *mut DwgData) {
    if data.is_null() {
        return;
    }
    drop(Box::from_raw(data as *mut CadDocument));
}

/// Query the document's DXF/DWG version code (e.g. `AC1032`).
///
/// Writes the NUL-terminated string into the caller-owned `buf`; `needed`
/// receives the total bytes required including the NUL.  A length-only
/// query uses `buf == NULL, cap == 0`.  Returns `DWG_EBUFFER` when the
/// buffer is too small.
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
    let doc = &*(data as *const CadDocument);
    let ver = doc.version.as_str();
    let bytes = ver.as_bytes();
    let total = bytes.len() + 1;
    *needed = total;
    if buf.is_null() {
        return DWG_OK;
    }
    if cap < total {
        return DWG_EBUFFER;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
    *buf.add(bytes.len()) = 0;
    DWG_OK
}
