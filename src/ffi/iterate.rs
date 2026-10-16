//! Object iteration through the C ABI.

use crate::ffi::object::{DwgObjectType, FfiDocument, FfiObject};
use crate::ffi::types::*;

/// Return the first object of the document, or NULL when the document is
/// empty. `*error` receives `DWG_OK` on success, `DWG_EINVAL` on bad args.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_first_object(
    data: *const DwgData,
    error: *mut i32,
) -> *const DwgObject {
    if data.is_null() || error.is_null() {
        if !error.is_null() {
            *error = DWG_EINVAL;
        }
        return std::ptr::null();
    }
    let ffi = &*(data as *const FfiDocument);
    *error = DWG_OK;
    ffi.objects
        .iter()
        .find(|object| object.alive)
        .map_or(std::ptr::null(), |object| {
            object.as_ref() as *const FfiObject as *const DwgObject
        })
}

/// Find a live document object by its 64-bit handle. The returned pointer is
/// borrowed from `data` and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn dwg_object_by_handle(
    data: *const DwgData,
    handle: u64,
    error: *mut i32,
) -> *const DwgObject {
    if data.is_null() || error.is_null() {
        if !error.is_null() {
            *error = DWG_EINVAL;
        }
        return std::ptr::null();
    }
    if handle == 0 {
        *error = DWG_ENOTFOUND;
        return std::ptr::null();
    }

    let ffi = &*(data as *const FfiDocument);
    let Some(object) = ffi
        .objects
        .iter()
        .find(|object| object.alive && object.handle.value() == handle)
    else {
        *error = DWG_ENOTFOUND;
        return std::ptr::null();
    };

    *error = DWG_OK;
    object.as_ref() as *const FfiObject as *const DwgObject
}

/// Return the next object after `obj`, or NULL at the end of the document.
#[no_mangle]
pub unsafe extern "C" fn dwg_object_next(obj: *const DwgObject) -> *const DwgObject {
    if obj.is_null() {
        return std::ptr::null();
    }
    let ffi = &*(obj as *const FfiObject);
    if !ffi.alive || ffi.next.is_null() {
        std::ptr::null()
    } else {
        ffi.next as *const DwgObject
    }
}

/// Return the object type as a `DwgObjectType` value, or
/// `DWG_OBJECT_UNKNOWN` for a NULL pointer.
#[no_mangle]
pub unsafe extern "C" fn dwg_object_get_type(obj: *const DwgObject) -> i32 {
    if obj.is_null() {
        return DwgObjectType::Unknown as i32;
    }
    let ffi = &*(obj as *const FfiObject);
    if !ffi.alive {
        return DwgObjectType::Unknown as i32;
    }
    ffi.kind as i32
}

/// Return the object's handle value (0 for NULL).
#[no_mangle]
pub unsafe extern "C" fn dwg_object_get_handle(obj: *const DwgObject) -> u64 {
    if obj.is_null() {
        return 0;
    }
    let ffi = &*(obj as *const FfiObject);
    if !ffi.alive {
        return 0;
    }
    ffi.handle.value()
}
