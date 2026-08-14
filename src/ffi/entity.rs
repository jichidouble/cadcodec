//! Typed getters for the basic entity families (Phase 2).

use crate::entities::{Arc, Circle, EntityType, Line, Point, Text};
use crate::ffi::object::FfiObject;
use crate::ffi::types::*;

fn as_entity<'a>(obj: *const DwgObject) -> Result<&'a EntityType, i32> {
    if obj.is_null() {
        return Err(DWG_EINVAL);
    }
    let ffi = unsafe { &*(obj as *const FfiObject) };
    if ffi.entity.is_null() {
        return Err(DWG_ETYPE);
    }
    Ok(unsafe { &*ffi.entity })
}

fn as_line<'a>(obj: *const DwgObject) -> Result<&'a Line, i32> {
    match as_entity(obj)? {
        EntityType::Line(l) => Ok(l),
        _ => Err(DWG_ETYPE),
    }
}

fn as_circle<'a>(obj: *const DwgObject) -> Result<&'a Circle, i32> {
    match as_entity(obj)? {
        EntityType::Circle(c) => Ok(c),
        _ => Err(DWG_ETYPE),
    }
}

fn as_arc<'a>(obj: *const DwgObject) -> Result<&'a Arc, i32> {
    match as_entity(obj)? {
        EntityType::Arc(a) => Ok(a),
        _ => Err(DWG_ETYPE),
    }
}

fn as_text<'a>(obj: *const DwgObject) -> Result<&'a Text, i32> {
    match as_entity(obj)? {
        EntityType::Text(t) => Ok(t),
        _ => Err(DWG_ETYPE),
    }
}

fn as_point<'a>(obj: *const DwgObject) -> Result<&'a Point, i32> {
    match as_entity(obj)? {
        EntityType::Point(p) => Ok(p),
        _ => Err(DWG_ETYPE),
    }
}

/// Copy `text` into a caller-owned buffer, `buf`/`cap`/`needed` style.
fn copy_c_string(text: &str, buf: *mut i8, cap: usize, needed: *mut usize) -> i32 {
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

fn write_vec3(v: &crate::types::Vector3, x: *mut f64, y: *mut f64, z: *mut f64) -> i32 {
    if x.is_null() || y.is_null() || z.is_null() {
        return DWG_EINVAL;
    }
    unsafe {
        *x = v.x;
        *y = v.y;
        *z = v.z;
    }
    DWG_OK
}

macro_rules! vec3_getter {
    ($name:ident, $as_ty:ident, $field:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            obj: *const DwgObject,
            x: *mut f64,
            y: *mut f64,
            z: *mut f64,
        ) -> i32 {
            match $as_ty(obj) {
                Ok(e) => write_vec3(&e.$field, x, y, z),
                Err(code) => code,
            }
        }
    };
}

macro_rules! f64_getter {
    ($name:ident, $as_ty:ident, $field:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(obj: *const DwgObject, out: *mut f64) -> i32 {
            if out.is_null() {
                return DWG_EINVAL;
            }
            match $as_ty(obj) {
                Ok(e) => {
                    *out = e.$field;
                    DWG_OK
                }
                Err(code) => code,
            }
        }
    };
}

vec3_getter!(dwg_entity_line_get_start, as_line, start);
vec3_getter!(dwg_entity_line_get_end, as_line, end);
vec3_getter!(dwg_entity_circle_get_center, as_circle, center);
f64_getter!(dwg_entity_circle_get_radius, as_circle, radius);
vec3_getter!(dwg_entity_arc_get_center, as_arc, center);
f64_getter!(dwg_entity_arc_get_radius, as_arc, radius);
f64_getter!(dwg_entity_arc_get_start_angle, as_arc, start_angle);
f64_getter!(dwg_entity_arc_get_end_angle, as_arc, end_angle);
vec3_getter!(dwg_entity_text_get_insertion_point, as_text, insertion_point);
f64_getter!(dwg_entity_text_get_height, as_text, height);
vec3_getter!(dwg_entity_point_get_location, as_point, location);

/// Copy the TEXT entity's value into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_text_get_value(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_text(obj) {
        Ok(t) => copy_c_string(&t.value, buf, cap, needed),
        Err(code) => code,
    }
}
