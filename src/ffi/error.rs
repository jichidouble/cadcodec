//! Thread-local error state for the C ABI.

use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<Option<(i32, String)>> = const { RefCell::new(None) };
}

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

pub(crate) fn last_error_message() -> Option<String> {
    LAST_ERROR.with(|s| s.borrow().as_ref().map(|(_, m)| m.clone()))
}

/// Copy the current thread's last error message into `buf` (caller-owned).
/// `needed` receives the total bytes required including the NUL terminator.
/// Returns the error code that was stored, or `DWG_OK` when none is pending.
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
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
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
}
