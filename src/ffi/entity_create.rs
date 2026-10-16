//! Basic entity creation through the C ABI.

use crate::entities::{Arc, Circle, EntityType, Line, Point};
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::{DwgData, DwgObject, DWG_EINVAL, DWG_OK};

unsafe fn create_entity(data: *mut DwgData, out: *mut *mut DwgObject, entity: EntityType) -> i32 {
    if data.is_null() || out.is_null() {
        return DWG_EINVAL;
    }

    let ffi = &mut *(data as *mut FfiDocument);
    match ffi.add_entity(entity) {
        Ok(object) => {
            *out = object;
            clear_last_error();
            DWG_OK
        }
        Err(code) => {
            set_last_error(code, "entity insertion failed");
            code
        }
    }
}

/// Create a LINE entity in model space.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_line_create(
    data: *mut DwgData,
    x1: f64,
    y1: f64,
    z1: f64,
    x2: f64,
    y2: f64,
    z2: f64,
    out: *mut *mut DwgObject,
) -> i32 {
    create_entity(
        data,
        out,
        EntityType::Line(Line::from_coords(x1, y1, z1, x2, y2, z2)),
    )
}

/// Create a CIRCLE entity in model space.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_circle_create(
    data: *mut DwgData,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
    out: *mut *mut DwgObject,
) -> i32 {
    create_entity(
        data,
        out,
        EntityType::Circle(Circle::from_coords(x, y, z, radius)),
    )
}

/// Create an ARC entity in model space.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_arc_create(
    data: *mut DwgData,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    out: *mut *mut DwgObject,
) -> i32 {
    create_entity(
        data,
        out,
        EntityType::Arc(Arc::from_coords(x, y, z, radius, start_angle, end_angle)),
    )
}

/// Create a POINT entity in model space.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_point_create(
    data: *mut DwgData,
    x: f64,
    y: f64,
    z: f64,
    out: *mut *mut DwgObject,
) -> i32 {
    create_entity(data, out, EntityType::Point(Point::from_coords(x, y, z)))
}
