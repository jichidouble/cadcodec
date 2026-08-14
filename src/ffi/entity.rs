//! Typed getters for the basic entity families (Phase 2).

use crate::entities::{Arc, Circle, Ellipse, EntityType, Face3D, Line, LwPolyline, MText, Point, Ray, Solid, Spline, Text, XLine};
use crate::ffi::object::FfiObject;
use crate::ffi::types::*;
use crate::ffi::util::{color_to_c, copy_c_string};

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

fn as_ellipse<'a>(obj: *const DwgObject) -> Result<&'a Ellipse, i32> {
    match as_entity(obj)? {
        EntityType::Ellipse(e) => Ok(e),
        _ => Err(DWG_ETYPE),
    }
}

fn as_ray<'a>(obj: *const DwgObject) -> Result<&'a Ray, i32> {
    match as_entity(obj)? {
        EntityType::Ray(r) => Ok(r),
        _ => Err(DWG_ETYPE),
    }
}

fn as_xline<'a>(obj: *const DwgObject) -> Result<&'a XLine, i32> {
    match as_entity(obj)? {
        EntityType::XLine(x) => Ok(x),
        _ => Err(DWG_ETYPE),
    }
}

fn as_solid<'a>(obj: *const DwgObject) -> Result<&'a Solid, i32> {
    match as_entity(obj)? {
        EntityType::Solid(s) => Ok(s),
        _ => Err(DWG_ETYPE),
    }
}

fn as_face3d<'a>(obj: *const DwgObject) -> Result<&'a Face3D, i32> {
    match as_entity(obj)? {
        EntityType::Face3D(f) => Ok(f),
        _ => Err(DWG_ETYPE),
    }
}

fn as_spline<'a>(obj: *const DwgObject) -> Result<&'a Spline, i32> {
    match as_entity(obj)? {
        EntityType::Spline(s) => Ok(s),
        _ => Err(DWG_ETYPE),
    }
}

fn as_mtext<'a>(obj: *const DwgObject) -> Result<&'a MText, i32> {
    match as_entity(obj)? {
        EntityType::MText(m) => Ok(m),
        _ => Err(DWG_ETYPE),
    }
}

fn as_lwpolyline<'a>(obj: *const DwgObject) -> Result<&'a LwPolyline, i32> {
    match as_entity(obj)? {
        EntityType::LwPolyline(l) => Ok(l),
        _ => Err(DWG_ETYPE),
    }
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
vec3_getter!(dwg_entity_ellipse_get_center, as_ellipse, center);
vec3_getter!(dwg_entity_ellipse_get_major_axis, as_ellipse, major_axis);
f64_getter!(dwg_entity_ellipse_get_minor_axis_ratio, as_ellipse, minor_axis_ratio);
vec3_getter!(dwg_entity_ray_get_base_point, as_ray, base_point);
vec3_getter!(dwg_entity_ray_get_direction, as_ray, direction);
vec3_getter!(dwg_entity_xline_get_base_point, as_xline, base_point);
vec3_getter!(dwg_entity_xline_get_direction, as_xline, direction);
vec3_getter!(dwg_entity_mtext_get_insertion_point, as_mtext, insertion_point);
f64_getter!(dwg_entity_mtext_get_height, as_mtext, height);

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

/// Copy the MTEXT entity's value into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mtext_get_value(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_mtext(obj) {
        Ok(m) => copy_c_string(&m.value, buf, cap, needed),
        Err(code) => code,
    }
}

/// Copy the entity's layer name into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_get_layer(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_entity(obj) {
        Ok(e) => copy_c_string(e.common().layer.as_str(), buf, cap, needed),
        Err(code) => code,
    }
}

/// Read the entity's color (RGB >= 0, ACI index negative).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_get_color(obj: *const DwgObject, color: *mut i32) -> i32 {
    if color.is_null() {
        return DWG_EINVAL;
    }
    match as_entity(obj) {
        Ok(e) => {
            *color = color_to_c(&e.common().color);
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Copy the entity's linetype name into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_get_linetype(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_entity(obj) {
        Ok(e) => copy_c_string(e.common().linetype.as_str(), buf, cap, needed),
        Err(code) => code,
    }
}

/// Read the entity's linetype scale.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_get_linetype_scale(obj: *const DwgObject, scale: *mut f64) -> i32 {
    if scale.is_null() {
        return DWG_EINVAL;
    }
    match as_entity(obj) {
        Ok(e) => {
            *scale = e.common().linetype_scale;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read one of the four SOLID corners (`corner` 1..=4).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_solid_get_corner(
    obj: *const DwgObject,
    corner: i32,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    let solid = match as_solid(obj) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let v = match corner {
        1 => &solid.first_corner,
        2 => &solid.second_corner,
        3 => &solid.third_corner,
        4 => &solid.fourth_corner,
        _ => return DWG_EINVAL,
    };
    write_vec3(v, x, y, z)
}

/// Read one of the four 3DFACE corners (`corner` 1..=4).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_face3d_get_corner(
    obj: *const DwgObject,
    corner: i32,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    let face = match as_face3d(obj) {
        Ok(f) => f,
        Err(code) => return code,
    };
    let v = match corner {
        1 => &face.first_corner,
        2 => &face.second_corner,
        3 => &face.third_corner,
        4 => &face.fourth_corner,
        _ => return DWG_EINVAL,
    };
    write_vec3(v, x, y, z)
}

/// Read the SPLINE degree.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_get_degree(obj: *const DwgObject, degree: *mut i32) -> i32 {
    if degree.is_null() {
        return DWG_EINVAL;
    }
    match as_spline(obj) {
        Ok(s) => {
            *degree = s.degree;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read the SPLINE control point count.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_get_control_point_count(
    obj: *const DwgObject,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match as_spline(obj) {
        Ok(s) => {
            *count = s.control_points.len() as i32;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read one SPLINE control point by index.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_get_control_point(
    obj: *const DwgObject,
    index: i32,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    match as_spline(obj) {
        Ok(s) => match s.control_points.get(index as usize) {
            Some(v) => write_vec3(v, x, y, z),
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Read the LWPOLYLINE vertex count.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_get_vertex_count(
    obj: *const DwgObject,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match as_lwpolyline(obj) {
        Ok(l) => {
            *count = l.vertices.len() as i32;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read one LWPOLYLINE vertex (2D) by index.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_get_vertex(
    obj: *const DwgObject,
    index: i32,
    x: *mut f64,
    y: *mut f64,
) -> i32 {
    if x.is_null() || y.is_null() {
        return DWG_EINVAL;
    }
    match as_lwpolyline(obj) {
        Ok(l) => match l.vertices.get(index as usize) {
            Some(v) => {
                *x = v.location.x;
                *y = v.location.y;
                DWG_OK
            }
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}
