//! Thread-local error slot shared by every FFI entry point.

use std::cell::RefCell;

use super::types::{DWG_EINVAL, DWG_EBUFFER, DWG_OK};

thread_local! {
    static LAST_ERROR: RefCell<Option<(i32, String)>> = const { RefCell::new(None) };
}

/// Record the most recent failure for the calling thread.
///
/// Exposed so integration tests can drive the error slot through the public
/// ABI surface; not part of the stable C API.
#[doc(hidden)]
pub fn set_last_error(code: i32, msg: impl Into<String>) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some((code, msg.into())));
}

/// Clear the calling thread's error slot.
#[doc(hidden)]
pub fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Return the last error code of the calling thread (`DWG_OK` when none).
#[no_mangle]
pub extern "C" fn dwg_last_error_code() -> i32 {
    LAST_ERROR.with(|slot| slot.borrow().as_ref().map(|(code, _)| *code).unwrap_or(DWG_OK))
}

/// Copy the last error message into a caller-owned buffer.
///
/// `needed` is always written with the total bytes required including the
/// NUL terminator.  A length-only query uses `buf == NULL, cap == 0`.
/// Returns `DWG_OK` on success (including the length-only query), or
/// `DWG_EBUFFER` when the buffer is too small.
#[no_mangle]
pub extern "C" fn dwg_last_error(buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let (_, msg) = LAST_ERROR.with(|slot| slot.borrow().clone()).unwrap_or_default();
    let bytes = msg.as_bytes();
    let total = bytes.len() + 1;
    // `needed` is validated above; the write itself is on our own thread.
    unsafe { *needed = total };
    if buf.is_null() {
        return DWG_OK;
    }
    if cap < total {
        return DWG_EBUFFER;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
    DWG_OK
}
