//! Raw ACIS/SAT/SAB access through the C ABI.

use std::ffi::{c_char, CStr};

use crate::entities::{AcisData, EntityType};
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::{FfiDocument, FfiObject};
use crate::ffi::types::*;

fn fail(code: i32, message: &'static str) -> i32 {
    set_last_error(code, message);
    code
}

unsafe fn acis_data<'a>(obj: *const DwgObject) -> Result<&'a AcisData, i32> {
    if obj.is_null() {
        return Err(fail(DWG_EINVAL, "object pointer is null"));
    }
    let ffi = &*(obj as *const FfiObject);
    if !ffi.alive {
        return Err(fail(DWG_ENOTFOUND, "object has been removed"));
    }
    if ffi.entity.is_null() {
        return Err(fail(DWG_ETYPE, "object is not an ACIS entity"));
    }
    match &*ffi.entity {
        EntityType::Solid3D(entity) => Ok(&entity.acis_data),
        EntityType::Region(entity) => Ok(&entity.acis_data),
        EntityType::Body(entity) => Ok(&entity.acis_data),
        EntityType::Surface(entity) => Ok(&entity.acis_data),
        _ => Err(fail(DWG_ETYPE, "entity does not carry ACIS data")),
    }
}

fn copy_bytes(bytes: &[u8], buf: *mut u8, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return fail(DWG_EINVAL, "invalid output buffer arguments");
    }
    unsafe {
        *needed = bytes.len();
    }
    if buf.is_null() {
        clear_last_error();
        return DWG_OK;
    }
    if cap < bytes.len() {
        return fail(DWG_EBUFFER, "output buffer is too small");
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len()) };
    clear_last_error();
    DWG_OK
}

fn copy_sat(sat: &str, buf: *mut c_char, cap: usize, needed: *mut usize) -> i32 {
    if needed.is_null() || (!buf.is_null() && cap == 0) {
        return fail(DWG_EINVAL, "invalid output buffer arguments");
    }
    let bytes = sat.as_bytes();
    let total = bytes.len() + 1;
    unsafe {
        *needed = total;
    }
    if buf.is_null() {
        clear_last_error();
        return DWG_OK;
    }
    if cap < total {
        return fail(DWG_EBUFFER, "output buffer is too small");
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
    clear_last_error();
    DWG_OK
}

/// Return whether an ACIS entity stores binary SAB data (`1`) or SAT text (`0`).
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_acis_is_binary(obj: *const DwgObject, binary: *mut i32) -> i32 {
    if binary.is_null() {
        return fail(DWG_EINVAL, "binary output pointer is null");
    }
    match acis_data(obj) {
        Ok(data) => {
            *binary = i32::from(data.is_binary);
            clear_last_error();
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Copy SAT text, including its terminating NUL, into caller-owned storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_acis_get_sat(
    obj: *const DwgObject,
    buf: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match acis_data(obj) {
        Ok(data) if !data.is_binary => copy_sat(&data.sat_data, buf, cap, needed),
        Ok(_) => fail(DWG_ETYPE, "ACIS entity stores SAB data"),
        Err(code) => code,
    }
}

/// Copy SAB bytes into caller-owned storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_acis_get_sab(
    obj: *const DwgObject,
    buf: *mut u8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match acis_data(obj) {
        Ok(data) if data.is_binary => copy_bytes(&data.sab_data, buf, cap, needed),
        Ok(_) => fail(DWG_ETYPE, "ACIS entity stores SAT data"),
        Err(code) => code,
    }
}

/// Replace an ACIS entity's payload with copied SAT text.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_acis_set_sat(
    data: *mut DwgData,
    obj: *const DwgObject,
    sat: *const c_char,
) -> i32 {
    if data.is_null() || obj.is_null() || sat.is_null() {
        return fail(
            DWG_EINVAL,
            "document, object, and SAT pointers are required",
        );
    }
    let sat = match CStr::from_ptr(sat).to_str() {
        Ok(sat) => sat.to_owned(),
        Err(_) => return fail(DWG_EINVAL, "SAT data is not valid UTF-8"),
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        match entity {
            EntityType::Solid3D(entity) => entity.acis_data = AcisData::from_sat(&sat),
            EntityType::Region(entity) => entity.acis_data = AcisData::from_sat(&sat),
            EntityType::Body(entity) => entity.acis_data = AcisData::from_sat(&sat),
            EntityType::Surface(entity) => entity.acis_data = AcisData::from_sat(&sat),
            _ => return Err((DWG_ETYPE, "entity does not carry ACIS data")),
        }
        Ok(())
    })
}

/// Replace an ACIS entity's payload with copied SAB bytes.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_acis_set_sab(
    data: *mut DwgData,
    obj: *const DwgObject,
    sab: *const u8,
    len: usize,
) -> i32 {
    if data.is_null() || obj.is_null() || (sab.is_null() && len != 0) {
        return fail(DWG_EINVAL, "invalid document, object, or SAB buffer");
    }
    let sab = if len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(sab, len).to_vec()
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        match entity {
            EntityType::Solid3D(entity) => entity.acis_data = AcisData::from_sab(sab),
            EntityType::Region(entity) => entity.acis_data = AcisData::from_sab(sab),
            EntityType::Body(entity) => entity.acis_data = AcisData::from_sab(sab),
            EntityType::Surface(entity) => entity.acis_data = AcisData::from_sab(sab),
            _ => return Err((DWG_ETYPE, "entity does not carry ACIS data")),
        }
        Ok(())
    })
}
