<<<<<<< HEAD
//! Document lifecycle and version query.

use std::ptr;

use crate::document::CadDocument;

use super::error::clear_last_error;
use super::types::{DwgData, DWG_EINVAL, DWG_EBUFFER, DWG_OK};

/// Allocate a new empty document handle.
///
/// `out` receives the new opaque handle and must be released with
/// [`dwg_data_destroy`].  Returns `DWG_EINVAL` when `out` is NULL.
=======
//! Document lifecycle: create/destroy an opaque `DwgData` handle and query
//! its version.

use crate::document::CadDocument;
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;

/// Create a new empty document. On success `*out` receives a document handle
/// owned by the caller (release with `dwg_data_destroy`).
>>>>>>> feat/ffi-c-api
#[no_mangle]
pub extern "C" fn dwg_data_create(out: *mut *mut DwgData) -> i32 {
    if out.is_null() {
        return DWG_EINVAL;
    }
<<<<<<< HEAD
    let data = Box::new(CadDocument::new());
    unsafe { *out = Box::into_raw(data) as *mut DwgData };
=======
    let data = Box::new(FfiDocument::new(CadDocument::new()));
    unsafe {
        *out = Box::into_raw(data) as *mut DwgData;
    }
>>>>>>> feat/ffi-c-api
    clear_last_error();
    DWG_OK
}

<<<<<<< HEAD
/// Release a document handle previously returned by [`dwg_data_create`].
///
/// NULL is a no-op; the handle must not be used afterwards.
=======
/// Destroy a document previously created by `dwg_data_create`. All object
/// references obtained from this document become invalid.
>>>>>>> feat/ffi-c-api
#[no_mangle]
pub unsafe extern "C" fn dwg_data_destroy(data: *mut DwgData) {
    if data.is_null() {
        return;
    }
<<<<<<< HEAD
    drop(Box::from_raw(data as *mut CadDocument));
}

/// Query the document's DXF/DWG version code (e.g. `AC1032`).
///
/// Writes the NUL-terminated string into the caller-owned `buf`; `needed`
/// receives the total bytes required including the NUL.  A length-only
/// query uses `buf == NULL, cap == 0`.  Returns `DWG_EBUFFER` when the
/// buffer is too small.
=======
    drop(Box::from_raw(data as *mut FfiDocument));
}

/// Copy the document's DWG version string (e.g. "AC1032"; a fresh document
/// created by `dwg_data_create` is AC1032) into `buf` (caller-owned).
/// `needed` receives the total bytes required including NUL.
>>>>>>> feat/ffi-c-api
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
<<<<<<< HEAD
    let doc = &*(data as *const CadDocument);
    let ver = doc.version.as_str();
=======
    let ffi = &*(data as *const FfiDocument);
    let ver = ffi.doc.version.as_str();
>>>>>>> feat/ffi-c-api
    let bytes = ver.as_bytes();
    let total = bytes.len() + 1;
    *needed = total;
    if buf.is_null() {
        return DWG_OK;
    }
    if cap < total {
<<<<<<< HEAD
        return DWG_EBUFFER;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
=======
        set_last_error(DWG_EBUFFER, "buffer too small");
        return DWG_EBUFFER;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
>>>>>>> feat/ffi-c-api
    *buf.add(bytes.len()) = 0;
    DWG_OK
}
