//! Entity setter entry points.
//!
//! Concrete setters are added here as the FFI mutation surface expands.

use crate::ffi::error::set_last_error;
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::ffi::util::{color_from_c, finite, read_c_string};

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
