//! Shared helpers for the C ABI surface.

use crate::ffi::types::DWG_EINVAL;
use crate::ffi::types::DWG_EBUFFER;
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
    unsafe { *needed = total; }
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
