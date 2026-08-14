//! Minimal C ABI surface for FFI consumers (e.g. MinGW-hosted bindings).
//!
//! Enable with `--features ffi`. Keep every exported symbol `#[no_mangle]`
//! with `extern "C"` and stable signatures; changing them breaks downstream
//! binaries.

use std::ffi::CStr;
use std::os::raw::c_char;

const VERSION: &str = concat!("acadrust-", env!("CARGO_PKG_VERSION"), "-ffi");

/// Return the library version as a NUL-terminated static string.
#[no_mangle]
pub extern "C" fn acadrust_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// Return the last error message of the calling thread as a NUL-terminated
/// string, or a static "(no error)" when none is pending.
#[no_mangle]
pub extern "C" fn acadrust_last_error() -> *const c_char {
    match LAST_ERROR.with(|slot| slot.borrow().clone()) {
        Some(msg) => match ERROR_BUF.with(|b| {
            let mut buf = b.borrow_mut();
            let bytes = msg.as_bytes();
            let n = bytes.len().min(buf.len() - 1);
            buf[..n].copy_from_slice(&bytes[..n]);
            buf[n] = 0;
            buf.as_ptr() as *const c_char
        }) {
            ptr => ptr,
        },
        None => c"(no error)".as_ptr(),
    }
}

/// Parse a version string like `AC1032` and return its numeric DXF version,
/// or 0 when unrecognised. Set by callers before use.
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

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static ERROR_BUF: std::cell::RefCell<[u8; 1024]> = const { std::cell::RefCell::new([0u8; 1024]) };
}
