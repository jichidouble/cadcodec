//! Entity setter entry points.
//!
//! Concrete setters are added here as the FFI mutation surface expands.

use crate::entities::EntityType;
use crate::ffi::error::set_last_error;
use crate::ffi::object::{DwgObjectType, FfiDocument};
use crate::ffi::types::*;
use crate::ffi::util::{color_from_c, finite, finite3, nonzero3, read_c_string};
use crate::types::Vector3;

macro_rules! vec3_setter {
    ($name:ident, $kind:ident, $entity:ident, $field:ident) => {
        #[doc = concat!("Set the ", stringify!($field), " vector of a ", stringify!($kind), " entity.")]
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

fn valid_positive(value: f64) -> bool {
    finite(value) && value > 0.0
}

fn valid_minor_axis_ratio(value: f64) -> bool {
    finite(value) && value > 0.0 && value <= 1.0
}

/// Set an entity's layer name.
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
vec3_setter!(dwg_entity_ray_set_base_point, Ray, Ray, base_point);
vec3_setter!(dwg_entity_xline_set_base_point, XLine, XLine, base_point);

/// Set a RAY entity's normalized direction vector.
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
        ray.direction = Vector3::new(x, y, z).normalize();
        Ok(())
    })
}

/// Set an XLINE entity's normalized direction vector.
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
            xline.direction = Vector3::new(x, y, z).normalize();
            Ok(())
        },
    )
}

/// Set one of a SOLID entity's four corners (`corner` 1..=4).
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
