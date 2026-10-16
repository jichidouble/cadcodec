//! Shared helpers for the C ABI surface.

use crate::ffi::types::DWG_EBUFFER;
use crate::ffi::types::DWG_EINVAL;
use crate::ffi::types::DWG_OK;
use crate::types::Color;

/// C-side color encoding:
/// - non-negative: RGB true color `0x00RRGGBB`
/// - negative: ACI index (`-1` ByLayer, `-2` ByBlock, `-3` None, else Index)
pub(crate) fn color_to_c(c: &Color) -> i32 {
    match c {
        Color::Rgb { r, g, b } => ((*r as i32) << 16) | ((*g as i32) << 8) | (*b as i32),
        Color::Index(i) => -(*i as i32),
        Color::ByLayer => -1,
        Color::ByBlock => -2,
        Color::None => -3,
    }
}

pub(crate) fn color_from_c(v: i32) -> Color {
    if v >= 0 {
        Color::Rgb {
            r: ((v >> 16) & 0xFF) as u8,
            g: ((v >> 8) & 0xFF) as u8,
            b: (v & 0xFF) as u8,
        }
    } else {
        match v {
            -1 => Color::ByLayer,
            -2 => Color::ByBlock,
            -3 => Color::None,
            i => Color::Index(i.unsigned_abs() as u8),
        }
    }
}

/// Copy `text` into a caller-owned buffer with the `buf`/`cap`/`needed`
/// protocol: `buf == NULL` queries the required size; `cap < needed`
/// returns `DWG_EBUFFER` without writing.
pub(crate) fn copy_c_string(text: &str, buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let bytes = text.as_bytes();
    let total = bytes.len() + 1;
    unsafe {
        *needed = total;
    }
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

#[allow(dead_code)]
pub(crate) unsafe fn read_c_string(value: *const i8) -> Result<String, i32> {
    if value.is_null() {
        return Err(DWG_EINVAL);
    }
    std::ffi::CStr::from_ptr(value)
        .to_str()
        .map(str::to_owned)
        .map_err(|_| DWG_EINVAL)
}

#[allow(dead_code)]
pub(crate) fn finite(value: f64) -> bool {
    value.is_finite()
}

#[allow(dead_code)]
pub(crate) fn finite2(x: f64, y: f64) -> bool {
    x.is_finite() && y.is_finite()
}

#[allow(dead_code)]
pub(crate) fn finite3(x: f64, y: f64, z: f64) -> bool {
    x.is_finite() && y.is_finite() && z.is_finite()
}

#[allow(dead_code)]
pub(crate) fn nonzero3(x: f64, y: f64, z: f64) -> bool {
    finite3(x, y, z) && (x != 0.0 || y != 0.0 || z != 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_input_helpers_reject_invalid_values() {
        assert!(!finite(f64::NAN));
        assert!(!finite(f64::INFINITY));
        assert!(finite3(1.0, 2.0, 3.0));
        assert!(!nonzero3(0.0, 0.0, 0.0));
        assert_eq!(unsafe { read_c_string(std::ptr::null()) }, Err(DWG_EINVAL));
        let invalid = [0xff_u8, 0];
        assert_eq!(
            unsafe { read_c_string(invalid.as_ptr() as *const i8) },
            Err(DWG_EINVAL)
        );
    }
}
