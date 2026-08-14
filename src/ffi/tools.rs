//! Utility exports (version and version parsing).

use std::ffi::CStr;
use std::os::raw::c_char;

const VERSION: &str = concat!("acadrust-", env!("CARGO_PKG_VERSION"), "-ffi");

thread_local! {
    static ERROR_BUF: std::cell::RefCell<[u8; 1024]> = const { std::cell::RefCell::new([0u8; 1024]) };
}

/// Return the library version as a NUL-terminated static string.
#[no_mangle]
pub extern "C" fn acadrust_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// Parse a version string like `AC1032` and return its numeric DXF version,
/// or 0 when unrecognised.
#[no_mangle]
pub extern "C" fn acadrust_parse_version(text: *const c_char) -> i32 {
    if text.is_null() {
        return 0;
    }
    let Ok(s) = (unsafe { CStr::from_ptr(text) }).to_str() else {
        return 0;
    };
    let Some(v) = crate::types::DxfVersion::parse(s) else {
        return 0;
    };
    v as i32
}

/// Legacy: return the last error message as a NUL-terminated static string,
/// or "(no error)" when none is pending. Superseded by `dwg_last_error`.
#[no_mangle]
pub extern "C" fn acadrust_last_error() -> *const c_char {
    match super::error::last_error_message() {
        Some(msg) => ERROR_BUF.with(|b| {
            let mut buf = b.borrow_mut();
            let bytes = msg.as_bytes();
            let n = bytes.len().min(buf.len() - 1);
            buf[..n].copy_from_slice(&bytes[..n]);
            buf[n] = 0;
            buf.as_ptr() as *const c_char
        }),
        None => c"(no error)".as_ptr(),
    }
}
