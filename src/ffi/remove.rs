//! Document-owned object removal through the C ABI.

use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;

/// Remove an object from `data`.
///
/// The object pointer remains a safe, invalid arena token until
/// `dwg_data_destroy`; all other active object pointers retain their address.
#[no_mangle]
pub unsafe extern "C" fn dwg_object_remove(data: *mut DwgData, obj: *const DwgObject) -> i32 {
    if data.is_null() || obj.is_null() {
        set_last_error(DWG_EINVAL, "document and object pointers are required");
        return DWG_EINVAL;
    }
    let ffi = &mut *(data as *mut FfiDocument);
    match ffi.remove_object(obj) {
        Ok(()) => {
            clear_last_error();
            DWG_OK
        }
        Err(DWG_ENOTFOUND) => {
            set_last_error(DWG_ENOTFOUND, "object was not found or was already removed");
            DWG_ENOTFOUND
        }
        Err(code) => {
            set_last_error(code, "object removal failed");
            code
        }
    }
}
