//! Utility exports (version and version parsing).

use std::ffi::CStr;
use std::os::raw::c_char;

/// NUL-terminated version string for `dwg_version`. The explicit trailing
/// `\0` guarantees the pointer returned to C is a valid C string.
const VERSION: &[u8] = concat!("acadrust-", env!("CARGO_PKG_VERSION"), "-ffi\0").as_bytes();

/// Return the library version as a NUL-terminated static string.
#[no_mangle]
pub extern "C" fn dwg_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// Parse a version string like `AC1032` and return its numeric DXF version,
/// or 0 when unrecognised.
#[no_mangle]
pub extern "C" fn dwg_parse_version(text: *const c_char) -> i32 {
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
