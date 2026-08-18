//! Entity setter entry points.
//!
//! Concrete setters are added here as the FFI mutation surface expands.

use crate::entities::EntityType;
use crate::ffi::error::set_last_error;
use crate::ffi::object::{DwgObjectType, FfiDocument};
use crate::ffi::types::*;
use crate::ffi::util::{color_from_c, finite, finite3, nonzero3, read_c_string};
use crate::types::{Vector2, Vector3};

macro_rules! vec3_setter {
    ($name:ident, $kind:ident, $entity:ident, $field:ident) => {
        #[doc = concat!("Set the ", stringify!($field), " vector of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer and `obj` must be a valid entity object from that document."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            data: *mut DwgData,
            obj: *const DwgObject,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = Vector3::new(x, y, z);
                    Ok(())
                },
            )
        }
    };
}

macro_rules! f64_setter {
    ($name:ident, Ellipse, Ellipse, minor_axis_ratio, positive) => {
        f64_setter!(
            @impl $name,
            Ellipse,
            Ellipse,
            minor_axis_ratio,
            "minor axis ratio must be finite and within (0, 1]",
            valid_minor_axis_ratio
        );
    };
    ($name:ident, $kind:ident, $entity:ident, $field:ident, positive) => {
        f64_setter!(
            @impl $name,
            $kind,
            $entity,
            $field,
            "value must be finite and positive",
            valid_positive
        );
    };
    ($name:ident, $kind:ident, $entity:ident, $field:ident, finite_only) => {
        f64_setter!(
            @impl $name,
            $kind,
            $entity,
            $field,
            "value must be finite",
            finite
        );
    };
    (@impl $name:ident, $kind:ident, $entity:ident, $field:ident, $message:literal, $validator:ident) => {
        #[doc = concat!("Set the ", stringify!($field), " scalar of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer and `obj` must be a valid entity object from that document."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(data: *mut DwgData, obj: *const DwgObject, value: f64) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !$validator(value) {
                set_last_error(DWG_EINVAL, $message);
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = value;
                    Ok(())
                },
            )
        }
    };
}

macro_rules! string_setter {
    ($name:ident, $kind:ident, $entity:ident, $field:ident, allow_empty) => {
        #[doc = concat!("Set the ", stringify!($field), " of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer, `obj` must be a valid entity object from that document, and `value` must point to a valid NUL-terminated string."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            data: *mut DwgData,
            obj: *const DwgObject,
            value: *const i8,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            let Ok(value) = read_c_string(value) else {
                set_last_error(DWG_EINVAL, "value is not valid UTF-8");
                return DWG_EINVAL;
            };
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = value;
                    Ok(())
                },
            )
        }
    };
}

fn valid_positive(value: f64) -> bool {
    finite(value) && value > 0.0
}

fn valid_minor_axis_ratio(value: f64) -> bool {
    finite(value) && value > 0.0 && value <= 1.0
}

fn normalize_direction(x: f64, y: f64, z: f64) -> Vector3 {
    let scale = x.abs().max(y.abs()).max(z.abs());
    let x = x / scale;
    let y = y / scale;
    let z = z / scale;
    let length = (x * x + y * y + z * z).sqrt();
    Vector3::new(x / length, y / length, z / length)
}

/// Set an entity's layer name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid entity object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_layer(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "layer is not valid UTF-8");
        return DWG_EINVAL;
    };
    if value.is_empty() {
        set_last_error(DWG_EINVAL, "layer must not be empty");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().layer = value;
        Ok(())
    })
}

/// Set an entity's color using the C ABI color encoding.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid entity object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_color(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().color = color_from_c(value);
        Ok(())
    })
}

/// Set an entity's linetype name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid entity object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_linetype(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "linetype is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().linetype = value;
        Ok(())
    })
}

/// Set an entity's positive, finite linetype scale.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid entity object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_linetype_scale(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite(value) || value <= 0.0 {
        set_last_error(DWG_EINVAL, "linetype scale must be finite and positive");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().linetype_scale = value;
        Ok(())
    })
}

vec3_setter!(dwg_entity_line_set_start, Line, Line, start);
vec3_setter!(dwg_entity_line_set_end, Line, Line, end);
vec3_setter!(dwg_entity_circle_set_center, Circle, Circle, center);
f64_setter!(
    dwg_entity_circle_set_radius,
    Circle,
    Circle,
    radius,
    positive
);
vec3_setter!(dwg_entity_arc_set_center, Arc, Arc, center);
f64_setter!(dwg_entity_arc_set_radius, Arc, Arc, radius, positive);
f64_setter!(
    dwg_entity_arc_set_start_angle,
    Arc,
    Arc,
    start_angle,
    finite_only
);
f64_setter!(
    dwg_entity_arc_set_end_angle,
    Arc,
    Arc,
    end_angle,
    finite_only
);
vec3_setter!(dwg_entity_point_set_location, Point, Point, location);
vec3_setter!(dwg_entity_ellipse_set_center, Ellipse, Ellipse, center);
vec3_setter!(
    dwg_entity_ellipse_set_major_axis,
    Ellipse,
    Ellipse,
    major_axis
);
f64_setter!(
    dwg_entity_ellipse_set_minor_axis_ratio,
    Ellipse,
    Ellipse,
    minor_axis_ratio,
    positive
);
string_setter!(dwg_entity_text_set_value, Text, Text, value, allow_empty);
vec3_setter!(
    dwg_entity_text_set_insertion_point,
    Text,
    Text,
    insertion_point
);
f64_setter!(dwg_entity_text_set_height, Text, Text, height, positive);
string_setter!(dwg_entity_mtext_set_value, MText, MText, value, allow_empty);
vec3_setter!(
    dwg_entity_mtext_set_insertion_point,
    MText,
    MText,
    insertion_point
);
f64_setter!(dwg_entity_mtext_set_height, MText, MText, height, positive);
vec3_setter!(dwg_entity_ray_set_base_point, Ray, Ray, base_point);
vec3_setter!(dwg_entity_xline_set_base_point, XLine, XLine, base_point);
string_setter!(
    dwg_entity_attrdef_set_tag,
    AttributeDefinition,
    AttributeDefinition,
    tag,
    allow_empty
);
string_setter!(
    dwg_entity_attrdef_set_prompt,
    AttributeDefinition,
    AttributeDefinition,
    prompt,
    allow_empty
);
string_setter!(
    dwg_entity_attrdef_set_default_value,
    AttributeDefinition,
    AttributeDefinition,
    default_value,
    allow_empty
);
vec3_setter!(
    dwg_entity_attrdef_set_insertion_point,
    AttributeDefinition,
    AttributeDefinition,
    insertion_point
);
f64_setter!(
    dwg_entity_attrdef_set_height,
    AttributeDefinition,
    AttributeDefinition,
    height,
    positive
);
string_setter!(dwg_entity_tolerance_set_text, Tolerance, Tolerance, text, allow_empty);
vec3_setter!(
    dwg_entity_tolerance_set_insertion_point,
    Tolerance,
    Tolerance,
    insertion_point
);
string_setter!(dwg_entity_mline_set_style_name, MLine, MLine, style_name, allow_empty);
vec3_setter!(dwg_entity_mline_set_start_point, MLine, MLine, start_point);
f64_setter!(
    dwg_entity_mline_set_scale_factor,
    MLine,
    MLine,
    scale_factor,
    positive
);
vec3_setter!(
    dwg_entity_shape_set_insertion_point,
    Shape,
    Shape,
    insertion_point
);
f64_setter!(dwg_entity_shape_set_size, Shape, Shape, size, positive);
f64_setter!(
    dwg_entity_shape_set_rotation,
    Shape,
    Shape,
    rotation,
    finite_only
);

/// Set an INSERT entity's block name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid INSERT object from that
/// document, and `value` must point to a valid non-empty NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_block_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    if value.is_empty() {
        set_last_error(DWG_EINVAL, "block name must not be empty");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.block_name = value;
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_insert_set_insert_point,
    Insert,
    Insert,
    insert_point
);
f64_setter!(
    dwg_entity_insert_set_rotation,
    Insert,
    Insert,
    rotation,
    finite_only
);

/// Set an INSERT entity's normalized normal vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_normal(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "normal must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.normal = normalize_direction(x, y, z);
            Ok(())
        },
    )
}

/// Set an INSERT entity's array dimensions and spacing.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_array(
    data: *mut DwgData,
    obj: *const DwgObject,
    columns: u16,
    rows: u16,
    column_spacing: f64,
    row_spacing: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if columns == 0 || rows == 0 {
        set_last_error(DWG_EINVAL, "array dimensions must be at least 1");
        return DWG_EINVAL;
    }
    if !finite(column_spacing) || !finite(row_spacing) {
        set_last_error(DWG_EINVAL, "array spacing must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.column_count = columns;
            insert.row_count = rows;
            insert.column_spacing = column_spacing;
            insert.row_spacing = row_spacing;
            if columns > 1 || rows > 1 {
                insert.mark_as_minsert();
            }
            Ok(())
        },
    )
}

vec3_setter!(dwg_entity_viewport_set_center, Viewport, Viewport, center);
f64_setter!(
    dwg_entity_viewport_set_width,
    Viewport,
    Viewport,
    width,
    positive
);
f64_setter!(
    dwg_entity_viewport_set_height,
    Viewport,
    Viewport,
    height,
    positive
);

f64_setter!(
    dwg_entity_hatch_set_elevation,
    Hatch,
    Hatch,
    elevation,
    finite_only
);

/// Set a HATCH entity's pattern name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid HATCH object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_hatch_set_pattern_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Hatch),
        move |entity| {
            let EntityType::Hatch(hatch) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            hatch.pattern.name = value;
            Ok(())
        },
    )
}

f64_setter!(
    dwg_entity_hatch_set_pattern_angle,
    Hatch,
    Hatch,
    pattern_angle,
    finite_only
);
f64_setter!(
    dwg_entity_hatch_set_pattern_scale,
    Hatch,
    Hatch,
    pattern_scale,
    positive
);

/// Set a RAY entity's normalized direction vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid RAY object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_ray_set_direction(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "direction must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, Some(DwgObjectType::Ray), move |entity| {
        let EntityType::Ray(ray) = entity else {
            return Err((DWG_ETYPE, "entity type does not match setter"));
        };
        ray.direction = normalize_direction(x, y, z);
        Ok(())
    })
}

/// Set an XLINE entity's normalized direction vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid XLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xline_set_direction(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "direction must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::XLine),
        move |entity| {
            let EntityType::XLine(xline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            xline.direction = normalize_direction(x, y, z);
            Ok(())
        },
    )
}

/// Set one of a SOLID entity's four corners (`corner` 1..=4).
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SOLID object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_solid_set_corner(
    data: *mut DwgData,
    obj: *const DwgObject,
    corner: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !matches!(corner, 1..=4) {
        set_last_error(DWG_EINVAL, "corner must be between 1 and 4");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    let value = Vector3::new(x, y, z);
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Solid),
        move |entity| {
            let EntityType::Solid(solid) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            match corner {
                1 => solid.first_corner = value,
                2 => solid.second_corner = value,
                3 => solid.third_corner = value,
                4 => solid.fourth_corner = value,
                _ => unreachable!(),
            }
            Ok(())
        },
    )
}

/// Set one of a 3DFACE entity's four corners (`corner` 1..=4).
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid 3DFACE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_face3d_set_corner(
    data: *mut DwgData,
    obj: *const DwgObject,
    corner: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !matches!(corner, 1..=4) {
        set_last_error(DWG_EINVAL, "corner must be between 1 and 4");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    let value = Vector3::new(x, y, z);
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Face3D),
        move |entity| {
            let EntityType::Face3D(face3d) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            match corner {
                1 => face3d.first_corner = value,
                2 => face3d.second_corner = value,
                3 => face3d.third_corner = value,
                4 => face3d.fourth_corner = value,
                _ => unreachable!(),
            }
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_helix_set_axis_base_point,
    Helix,
    Helix,
    axis_base_point
);
f64_setter!(dwg_entity_helix_set_radius, Helix, Helix, radius, positive);
f64_setter!(dwg_entity_helix_set_turns, Helix, Helix, turns, positive);
f64_setter!(
    dwg_entity_helix_set_turn_height,
    Helix,
    Helix,
    turn_height,
    positive
);

/// Set a DIMENSION entity's style name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid DIMENSION object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_style_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().style_name = value;
            Ok(())
        },
    )
}

/// Set a DIMENSION entity's definition point.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_definition_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().definition_point = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

/// Set a DIMENSION entity's text middle point.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_text_middle_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().text_middle_point = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_raster_image_set_insertion_point,
    RasterImage,
    RasterImage,
    insertion_point
);

/// Set a RASTERIMAGE entity's pixel width and height atomically.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid RASTERIMAGE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_raster_image_set_size(
    data: *mut DwgData,
    obj: *const DwgObject,
    width: f64,
    height: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !valid_positive(width) || !valid_positive(height) {
        set_last_error(DWG_EINVAL, "width and height must be finite and positive");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::RasterImage),
        move |entity| {
            let EntityType::RasterImage(raster) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            raster.size = Vector2::new(width, height);
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_underlay_set_insertion_point,
    Underlay,
    Underlay,
    insertion_point
);
vec3_setter!(dwg_entity_light_set_position, Light, Light, position);
vec3_setter!(
    dwg_entity_wipeout_set_insertion_point,
    Wipeout,
    Wipeout,
    insertion_point
);
