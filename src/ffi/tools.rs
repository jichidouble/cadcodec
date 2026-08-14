//! Version utilities for C consumers.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::types::DxfVersion;

const VERSION: &str = concat!("acadrust-", env!("CARGO_PKG_VERSION"), "-ffi");

/// Return the library version as a NUL-terminated static string.
///
/// The pointer is owned by the library and stays valid for the process
/// lifetime; the caller must not free it.
#[no_mangle]
pub extern "C" fn dwg_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// Alias of [`dwg_version`] matching the spec's `dwg_version_to_string`.
#[no_mangle]
pub extern "C" fn dwg_version_to_string() -> *const c_char {
    dwg_version()
}

/// Parse a version string such as `AC1032` and return its numeric
/// `DxfVersion` discriminant (0 for `Unknown` / unrecognised input).
#[no_mangle]
pub extern "C" fn dwg_parse_version(text: *const c_char) -> i32 {
    if text.is_null() {
        return 0;
    }
    // SAFETY: the caller promises a NUL-terminated string.
    let Ok(s) = (unsafe { CStr::from_ptr(text) }).to_str() else {
        return 0;
    };
    let Some(v) = DxfVersion::parse(s) else {
        return 0;
    };
    v as i32
}
