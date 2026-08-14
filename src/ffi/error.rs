<<<<<<< HEAD
//! Thread-local error slot shared by every FFI entry point.

use std::cell::RefCell;

use super::types::{DWG_EINVAL, DWG_EBUFFER, DWG_OK};

=======
//! Thread-local error state for the C ABI.

use std::cell::RefCell;

>>>>>>> feat/ffi-c-api
thread_local! {
    static LAST_ERROR: RefCell<Option<(i32, String)>> = const { RefCell::new(None) };
}

<<<<<<< HEAD
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
=======
// Used by the I/O layer (Task 3) and by internal tests; allowed while the
// I/O layer is not yet implemented.
#[allow(dead_code)]
pub(crate) fn set_last_error(code: i32, msg: &str) {
    LAST_ERROR.with(|s| *s.borrow_mut() = Some((code, msg.to_string())));
}

#[allow(dead_code)]
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|s| *s.borrow_mut() = None);
}

/// Copy the current thread's last error message into `buf` (caller-owned).
/// `needed` receives the total bytes required including the NUL terminator.
///
/// Return value is the error code that was stored (`DWG_OK` when none is
/// pending), matching `dwg_last_error_code`; callers that only need the code
/// may use `dwg_last_error_code` directly, callers that need the message use
/// this function. The slot is sticky: it is only replaced by the next failed
/// operation and cleared by the next successful one.
#[no_mangle]
pub extern "C" fn dwg_last_error(buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return crate::ffi::types::DWG_EINVAL;
    }
    let (code, msg) = LAST_ERROR.with(|s| s.borrow().clone()).unwrap_or((0, String::new()));
    let bytes = msg.as_bytes();
    let total = bytes.len() + 1;
    unsafe { *needed = total; }
    if buf.is_null() {
        return crate::ffi::types::DWG_OK;
    }
    if cap < total {
        return crate::ffi::types::DWG_EBUFFER;
>>>>>>> feat/ffi-c-api
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
<<<<<<< HEAD
    DWG_OK
=======
    code
}

/// Return the current thread's last error code, or `DWG_OK` when none.
#[no_mangle]
pub extern "C" fn dwg_last_error_code() -> i32 {
    LAST_ERROR.with(|s| s.borrow().as_ref().map(|(c, _)| *c).unwrap_or(crate::ffi::types::DWG_OK))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::types::*;

    #[test]
    fn internal_error_slot_roundtrip() {
        clear_last_error();
        set_last_error(DWG_EREAD, "bad read");
        let (code, msg) = LAST_ERROR.with(|s| s.borrow().clone()).unwrap();
        assert_eq!(code, DWG_EREAD);
        assert_eq!(msg, "bad read");
    }

    #[test]
    fn last_error_reports_code_and_message() {
        clear_last_error();
        set_last_error(DWG_EWRITE, "disk full");
        let mut buf = [0u8; 32];
        let mut needed = 0usize;
        let code = dwg_last_error(buf.as_mut_ptr() as *mut i8, buf.len(), &mut needed);
        assert_eq!(code, DWG_EWRITE);
        assert_eq!(needed, 10); // "disk full" + NUL
        assert_eq!(&buf[..needed - 1], b"disk full");
    }

    #[test]
    fn null_output_returns_einval() {
        let rc = dwg_last_error(std::ptr::null_mut(), 0, std::ptr::null_mut());
        assert_eq!(rc, DWG_EINVAL);
    }
>>>>>>> feat/ffi-c-api
}
