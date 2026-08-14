//! Typed getters for the basic entity families (Phase 2).

use crate::entities::{Arc, AttributeDefinition, Circle, Dimension, Ellipse, EntityType, Face3D, Hatch, Helix, Insert, Leader, Light, Line, LwPolyline, MLine, MText, Mesh, Point, PolyfaceMesh, Polyline2D, Polyline3D, RasterImage, Ray, Shape, Solid, Spline, Text, Tolerance, Underlay, Viewport, Wipeout, XLine};
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

fn as_insert<'a>(obj: *const DwgObject) -> Result<&'a Insert, i32> {
    match as_entity(obj)? {
        EntityType::Insert(i) => Ok(i),
        _ => Err(DWG_ETYPE),
    }
}

fn as_polyline2d<'a>(obj: *const DwgObject) -> Result<&'a Polyline2D, i32> {
    match as_entity(obj)? {
        EntityType::Polyline2D(p) => Ok(p),
        _ => Err(DWG_ETYPE),
    }
}

fn as_polyline3d<'a>(obj: *const DwgObject) -> Result<&'a Polyline3D, i32> {
    match as_entity(obj)? {
        EntityType::Polyline3D(p) => Ok(p),
        _ => Err(DWG_ETYPE),
    }
}

fn as_mesh<'a>(obj: *const DwgObject) -> Result<&'a Mesh, i32> {
    match as_entity(obj)? {
        EntityType::Mesh(m) => Ok(m),
        _ => Err(DWG_ETYPE),
    }
}

fn as_viewport<'a>(obj: *const DwgObject) -> Result<&'a Viewport, i32> {
    match as_entity(obj)? {
        EntityType::Viewport(v) => Ok(v),
        _ => Err(DWG_ETYPE),
    }
}

fn as_polyface_mesh<'a>(obj: *const DwgObject) -> Result<&'a PolyfaceMesh, i32> {
    match as_entity(obj)? {
        EntityType::PolyfaceMesh(p) => Ok(p),
        _ => Err(DWG_ETYPE),
    }
}

fn as_hatch<'a>(obj: *const DwgObject) -> Result<&'a Hatch, i32> {
    match as_entity(obj)? {
        EntityType::Hatch(h) => Ok(h),
        _ => Err(DWG_ETYPE),
    }
}

fn as_tolerance<'a>(obj: *const DwgObject) -> Result<&'a Tolerance, i32> {
    match as_entity(obj)? {
        EntityType::Tolerance(t) => Ok(t),
        _ => Err(DWG_ETYPE),
    }
}

fn as_leader<'a>(obj: *const DwgObject) -> Result<&'a Leader, i32> {
    match as_entity(obj)? {
        EntityType::Leader(l) => Ok(l),
        _ => Err(DWG_ETYPE),
    }
}

fn as_mline<'a>(obj: *const DwgObject) -> Result<&'a MLine, i32> {
    match as_entity(obj)? {
        EntityType::MLine(m) => Ok(m),
        _ => Err(DWG_ETYPE),
    }
}

fn as_attribute_definition<'a>(obj: *const DwgObject) -> Result<&'a AttributeDefinition, i32> {
    match as_entity(obj)? {
        EntityType::AttributeDefinition(a) => Ok(a),
        _ => Err(DWG_ETYPE),
    }
}

fn as_shape<'a>(obj: *const DwgObject) -> Result<&'a Shape, i32> {
    match as_entity(obj)? {
        EntityType::Shape(s) => Ok(s),
        _ => Err(DWG_ETYPE),
    }
}

fn as_helix<'a>(obj: *const DwgObject) -> Result<&'a Helix, i32> {
    match as_entity(obj)? {
        EntityType::Helix(h) => Ok(h),
        _ => Err(DWG_ETYPE),
    }
}

fn as_dimension<'a>(obj: *const DwgObject) -> Result<&'a Dimension, i32> {
    match as_entity(obj)? {
        EntityType::Dimension(d) => Ok(d),
        _ => Err(DWG_ETYPE),
    }
}

fn as_raster_image<'a>(obj: *const DwgObject) -> Result<&'a RasterImage, i32> {
    match as_entity(obj)? {
        EntityType::RasterImage(r) => Ok(r),
        _ => Err(DWG_ETYPE),
    }
}

fn as_underlay<'a>(obj: *const DwgObject) -> Result<&'a Underlay, i32> {
    match as_entity(obj)? {
        EntityType::Underlay(u) => Ok(u),
        _ => Err(DWG_ETYPE),
    }
}

fn as_light<'a>(obj: *const DwgObject) -> Result<&'a Light, i32> {
    match as_entity(obj)? {
        EntityType::Light(l) => Ok(l),
        _ => Err(DWG_ETYPE),
    }
}

fn as_wipeout<'a>(obj: *const DwgObject) -> Result<&'a Wipeout, i32> {
    match as_entity(obj)? {
        EntityType::Wipeout(w) => Ok(w),
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
vec3_getter!(dwg_entity_insert_get_insert_point, as_insert, insert_point);
f64_getter!(dwg_entity_insert_get_rotation, as_insert, rotation);
vec3_getter!(dwg_entity_insert_get_normal, as_insert, normal);
vec3_getter!(dwg_entity_viewport_get_center, as_viewport, center);
f64_getter!(dwg_entity_viewport_get_width, as_viewport, width);
f64_getter!(dwg_entity_viewport_get_height, as_viewport, height);
f64_getter!(dwg_entity_hatch_get_elevation, as_hatch, elevation);
f64_getter!(dwg_entity_hatch_get_pattern_angle, as_hatch, pattern_angle);
f64_getter!(dwg_entity_hatch_get_pattern_scale, as_hatch, pattern_scale);
vec3_getter!(dwg_entity_tolerance_get_insertion_point, as_tolerance, insertion_point);
vec3_getter!(dwg_entity_attrdef_get_insertion_point, as_attribute_definition, insertion_point);
f64_getter!(dwg_entity_attrdef_get_height, as_attribute_definition, height);
vec3_getter!(dwg_entity_shape_get_insertion_point, as_shape, insertion_point);
f64_getter!(dwg_entity_shape_get_size, as_shape, size);
f64_getter!(dwg_entity_shape_get_rotation, as_shape, rotation);
vec3_getter!(dwg_entity_helix_get_axis_base_point, as_helix, axis_base_point);
f64_getter!(dwg_entity_helix_get_radius, as_helix, radius);
f64_getter!(dwg_entity_helix_get_turns, as_helix, turns);
f64_getter!(dwg_entity_helix_get_turn_height, as_helix, turn_height);
vec3_getter!(dwg_entity_mline_get_start_point, as_mline, start_point);
f64_getter!(dwg_entity_mline_get_scale_factor, as_mline, scale_factor);
vec3_getter!(dwg_entity_raster_image_get_insertion_point, as_raster_image, insertion_point);
vec3_getter!(dwg_entity_underlay_get_insertion_point, as_underlay, insertion_point);
vec3_getter!(dwg_entity_light_get_position, as_light, position);
vec3_getter!(dwg_entity_wipeout_get_insertion_point, as_wipeout, insertion_point);

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

/// Copy the INSERT block name into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_get_block_name(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_insert(obj) {
        Ok(i) => copy_c_string(&i.block_name, buf, cap, needed),
        Err(code) => code,
    }
}

/// Read the INSERT column/row counts.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_get_array(
    obj: *const DwgObject,
    columns: *mut u16,
    rows: *mut u16,
) -> i32 {
    if columns.is_null() || rows.is_null() {
        return DWG_EINVAL;
    }
    match as_insert(obj) {
        Ok(i) => {
            *columns = i.column_count;
            *rows = i.row_count;
            DWG_OK
        }
        Err(code) => code,
    }
}

macro_rules! vertex_list_getters {
    ($count:ident, $get:ident, $as_ty:ident, $vec_ty:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $count(obj: *const DwgObject, count: *mut i32) -> i32 {
            if count.is_null() {
                return DWG_EINVAL;
            }
            match $as_ty(obj) {
                Ok(e) => {
                    *count = e.vertices.len() as i32;
                    DWG_OK
                }
                Err(code) => code,
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $get(
            obj: *const DwgObject,
            index: i32,
            x: *mut f64,
            y: *mut f64,
            z: *mut f64,
        ) -> i32 {
            match $as_ty(obj) {
                Ok(e) => match e.vertices.get(index as usize) {
                    Some(v) => write_vec3(&v.$vec_ty, x, y, z),
                    None => DWG_ENOTFOUND,
                },
                Err(code) => code,
            }
        }
    };
}

vertex_list_getters!(
    dwg_entity_polyline2d_get_vertex_count,
    dwg_entity_polyline2d_get_vertex,
    as_polyline2d,
    location
);
vertex_list_getters!(
    dwg_entity_polyline3d_get_vertex_count,
    dwg_entity_polyline3d_get_vertex,
    as_polyline3d,
    position
);
vertex_list_getters!(
    dwg_entity_polyface_mesh_get_vertex_count,
    dwg_entity_polyface_mesh_get_vertex,
    as_polyface_mesh,
    location
);

/// Read the MESH vertex count / a vertex by index (vertices are Vector3).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_get_vertex_count(
    obj: *const DwgObject,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match as_mesh(obj) {
        Ok(m) => {
            *count = m.vertices.len() as i32;
            DWG_OK
        }
        Err(code) => code,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_get_vertex(
    obj: *const DwgObject,
    index: i32,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    match as_mesh(obj) {
        Ok(m) => match m.vertices.get(index as usize) {
            Some(v) => write_vec3(v, x, y, z),
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Read the MESH face count and the number of vertices of one face.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_get_face_count(
    obj: *const DwgObject,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match as_mesh(obj) {
        Ok(m) => {
            *count = m.faces.len() as i32;
            DWG_OK
        }
        Err(code) => code,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_get_face_vertex_count(
    obj: *const DwgObject,
    face: i32,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match as_mesh(obj) {
        Ok(m) => match m.faces.get(face as usize) {
            Some(f) => {
                *count = f.vertices.len() as i32;
                DWG_OK
            }
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

macro_rules! string_field_getter {
    ($name:ident, $as_ty:ident, $field:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            obj: *const DwgObject,
            buf: *mut i8,
            cap: usize,
            needed: *mut usize,
        ) -> i32 {
            match $as_ty(obj) {
                Ok(e) => copy_c_string(&e.$field, buf, cap, needed),
                Err(code) => code,
            }
        }
    };
}

string_field_getter!(dwg_entity_tolerance_get_text, as_tolerance, text);
string_field_getter!(dwg_entity_attrdef_get_tag, as_attribute_definition, tag);
string_field_getter!(dwg_entity_attrdef_get_prompt, as_attribute_definition, prompt);
string_field_getter!(dwg_entity_attrdef_get_default_value, as_attribute_definition, default_value);
string_field_getter!(dwg_entity_mline_get_style_name, as_mline, style_name);

/// Copy the DIMENSION style name into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_get_style_name(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_dimension(obj) {
        Ok(d) => copy_c_string(&d.base().style_name, buf, cap, needed),
        Err(code) => code,
    }
}

/// Read the DIMENSION definition point (WCS).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_get_definition_point(
    obj: *const DwgObject,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    match as_dimension(obj) {
        Ok(d) => write_vec3(&d.base().definition_point, x, y, z),
        Err(code) => code,
    }
}

/// Read the DIMENSION text middle point (WCS).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_get_text_middle_point(
    obj: *const DwgObject,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> i32 {
    match as_dimension(obj) {
        Ok(d) => write_vec3(&d.base().text_middle_point, x, y, z),
        Err(code) => code,
    }
}

/// Read the DIMENSION subtype (1 Aligned, 2 Linear, 3 Radius, 4 Diameter,
/// 5 Angular2Ln, 6 Angular3Pt, 7 Ordinate, 8 Arc, 9 LargeRadial).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_get_subtype(obj: *const DwgObject, subtype: *mut i32) -> i32 {
    if subtype.is_null() {
        return DWG_EINVAL;
    }
    match as_dimension(obj) {
        Ok(d) => {
            *subtype = match d {
                Dimension::Aligned(_) => 1,
                Dimension::Linear(_) => 2,
                Dimension::Radius(_) => 3,
                Dimension::Diameter(_) => 4,
                Dimension::Angular2Ln(_) => 5,
                Dimension::Angular3Pt(_) => 6,
                Dimension::Ordinate(_) => 7,
                Dimension::Arc(_) => 8,
                Dimension::LargeRadial(_) => 9,
            };
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read the DIMENSION measurement value.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_get_measurement(obj: *const DwgObject, value: *mut f64) -> i32 {
    if value.is_null() {
        return DWG_EINVAL;
    }
    match as_dimension(obj) {
        Ok(d) => {
            *value = d.measurement();
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Read the RASTERIMAGE pixel size (width/height).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_raster_image_get_size(
    obj: *const DwgObject,
    width: *mut f64,
    height: *mut f64,
) -> i32 {
    if width.is_null() || height.is_null() {
        return DWG_EINVAL;
    }
    match as_raster_image(obj) {
        Ok(r) => {
            *width = r.size.x;
            *height = r.size.y;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Copy the HATCH pattern name into a caller-owned buffer.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_hatch_get_pattern_name(
    obj: *const DwgObject,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match as_hatch(obj) {
        Ok(h) => copy_c_string(&h.pattern.name, buf, cap, needed),
        Err(code) => code,
    }
}

macro_rules! vec3_list_getters {
    ($count:ident, $get:ident, $as_ty:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $count(obj: *const DwgObject, count: *mut i32) -> i32 {
            if count.is_null() {
                return DWG_EINVAL;
            }
            match $as_ty(obj) {
                Ok(e) => {
                    *count = e.vertices.len() as i32;
                    DWG_OK
                }
                Err(code) => code,
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $get(
            obj: *const DwgObject,
            index: i32,
            x: *mut f64,
            y: *mut f64,
            z: *mut f64,
        ) -> i32 {
            match $as_ty(obj) {
                Ok(e) => match e.vertices.get(index as usize) {
                    Some(v) => write_vec3(v, x, y, z),
                    None => DWG_ENOTFOUND,
                },
                Err(code) => code,
            }
        }
    };
}

vec3_list_getters!(
    dwg_entity_leader_get_vertex_count,
    dwg_entity_leader_get_vertex,
    as_leader
);
