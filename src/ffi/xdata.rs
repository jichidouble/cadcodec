//! Typed entity XData access for the C ABI.

use crate::entities::EntityType;
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::tables::{AppId, TableEntry};
use crate::types::{Handle, Vector3};
use crate::xdata::{ExtendedDataRecord, XDataValue};
use std::ffi::CStr;
use std::slice;

/// A typed XData value exchanged through the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DwgXdataValue {
    /// Payload discriminator; one of the `DWG_XDATA_*` constants.
    pub tag: u32,
    /// Reserved for ABI evolution; mutation callers must set this to zero.
    pub reserved: u32,
    /// Integer payload for Integer16 and Integer32 values.
    pub integer: i64,
    /// Floating-point payload for real, distance, and scale-factor values.
    pub real: f64,
    /// Three-dimensional point, position, displacement, or direction payload.
    pub point: [f64; 3],
    /// Document-owned handle payload.
    pub handle: u64,
    /// Caller-owned string or binary storage.
    pub data: *mut core::ffi::c_void,
    /// Input length or output capacity/required length for `data`.
    pub data_len: usize,
}

/// String XData value tag.
pub const DWG_XDATA_STRING: u32 = 1;
/// Control-string XData value tag.
pub const DWG_XDATA_CONTROL_STRING: u32 = 2;
/// Layer-name XData value tag.
pub const DWG_XDATA_LAYER_NAME: u32 = 3;
/// Binary XData value tag.
pub const DWG_XDATA_BINARY: u32 = 4;
/// Handle XData value tag.
pub const DWG_XDATA_HANDLE: u32 = 5;
/// Three-dimensional point XData value tag.
pub const DWG_XDATA_POINT3D: u32 = 6;
/// Three-dimensional position XData value tag.
pub const DWG_XDATA_POSITION3D: u32 = 7;
/// Three-dimensional displacement XData value tag.
pub const DWG_XDATA_DISPLACEMENT3D: u32 = 8;
/// Three-dimensional direction XData value tag.
pub const DWG_XDATA_DIRECTION3D: u32 = 9;
/// Real XData value tag.
pub const DWG_XDATA_REAL: u32 = 10;
/// Distance XData value tag.
pub const DWG_XDATA_DISTANCE: u32 = 11;
/// Scale-factor XData value tag.
pub const DWG_XDATA_SCALE_FACTOR: u32 = 12;
/// Signed 16-bit integer XData value tag.
pub const DWG_XDATA_INTEGER16: u32 = 13;
/// Signed 32-bit integer XData value tag.
pub const DWG_XDATA_INTEGER32: u32 = 14;

fn fail(code: i32, message: &'static str) -> i32 {
    set_last_error(code, message);
    code
}

unsafe fn document<'a>(data: *const DwgData) -> Result<&'a FfiDocument, i32> {
    if data.is_null() {
        return Err(fail(DWG_EINVAL, "document pointer is null"));
    }
    Ok(&*(data as *const FfiDocument))
}

unsafe fn document_mut<'a>(data: *mut DwgData) -> Result<&'a mut FfiDocument, i32> {
    if data.is_null() {
        return Err(fail(DWG_EINVAL, "document pointer is null"));
    }
    Ok(&mut *(data as *mut FfiDocument))
}

fn entity(document: &FfiDocument, handle: u64) -> Result<&EntityType, i32> {
    let handle = Handle::new(handle);
    if let Some(entity) = document.doc.get_entity(handle) {
        return Ok(entity);
    }
    if document.doc.objects.contains_key(&handle) {
        return Err(fail(DWG_ETYPE, "handle does not identify an entity"));
    }
    Err(fail(
        DWG_ENOTFOUND,
        "entity handle is missing from document",
    ))
}

fn entity_mut(document: &mut FfiDocument, handle: u64) -> Result<&mut EntityType, i32> {
    let handle = Handle::new(handle);
    if document.doc.get_entity(handle).is_some() {
        return Ok(document
            .doc
            .get_entity_mut(handle)
            .expect("entity was validated before mutation"));
    }
    if document.doc.objects.contains_key(&handle) {
        return Err(fail(DWG_ETYPE, "handle does not identify an entity"));
    }
    Err(fail(
        DWG_ENOTFOUND,
        "entity handle is missing from document",
    ))
}

fn c_string<'a>(value: *const core::ffi::c_char, what: &'static str) -> Result<&'a str, i32> {
    if value.is_null() {
        return Err(fail(DWG_EINVAL, what));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| fail(DWG_EINVAL, "application name is not valid UTF-8"))
}

fn application_name<'a>(value: *const core::ffi::c_char) -> Result<&'a str, i32> {
    let name = c_string(value, "application name is null")?;
    if name.is_empty() {
        return Err(fail(DWG_EINVAL, "application name is empty"));
    }
    Ok(name)
}

unsafe fn input_bytes(value: &DwgXdataValue) -> Result<&[u8], i32> {
    if value.data.is_null() {
        if value.data_len == 0 {
            return Ok(&[]);
        }
        return Err(fail(DWG_EINVAL, "XData data is null with a nonzero length"));
    }
    Ok(slice::from_raw_parts(
        value.data.cast::<u8>(),
        value.data_len,
    ))
}

fn tag_for(value: &XDataValue) -> u32 {
    match value {
        XDataValue::String(_) => DWG_XDATA_STRING,
        XDataValue::ControlString(_) => DWG_XDATA_CONTROL_STRING,
        XDataValue::LayerName(_) => DWG_XDATA_LAYER_NAME,
        XDataValue::BinaryData(_) => DWG_XDATA_BINARY,
        XDataValue::Handle(_) => DWG_XDATA_HANDLE,
        XDataValue::Point3D(_) => DWG_XDATA_POINT3D,
        XDataValue::Position3D(_) => DWG_XDATA_POSITION3D,
        XDataValue::Displacement3D(_) => DWG_XDATA_DISPLACEMENT3D,
        XDataValue::Direction3D(_) => DWG_XDATA_DIRECTION3D,
        XDataValue::Real(_) => DWG_XDATA_REAL,
        XDataValue::Distance(_) => DWG_XDATA_DISTANCE,
        XDataValue::ScaleFactor(_) => DWG_XDATA_SCALE_FACTOR,
        XDataValue::Integer16(_) => DWG_XDATA_INTEGER16,
        XDataValue::Integer32(_) => DWG_XDATA_INTEGER32,
    }
}

unsafe fn value_from_c(document: &FfiDocument, value: &DwgXdataValue) -> Result<XDataValue, i32> {
    if value.reserved != 0 {
        return Err(fail(DWG_EINVAL, "XData value reserved bits are nonzero"));
    }
    if !(DWG_XDATA_STRING..=DWG_XDATA_INTEGER32).contains(&value.tag) {
        return Err(fail(DWG_EINVAL, "XData value tag is invalid"));
    }
    let bytes = input_bytes(value)?;
    match value.tag {
        DWG_XDATA_STRING => Ok(XDataValue::String(
            std::str::from_utf8(bytes)
                .map_err(|_| fail(DWG_EINVAL, "XData string is not valid UTF-8"))?
                .to_owned(),
        )),
        DWG_XDATA_CONTROL_STRING => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| fail(DWG_EINVAL, "XData control string is not valid UTF-8"))?;
            if text != "{" && text != "}" {
                return Err(fail(DWG_EINVAL, "XData control string is invalid"));
            }
            Ok(XDataValue::ControlString(text.to_owned()))
        }
        DWG_XDATA_LAYER_NAME => Ok(XDataValue::LayerName(
            std::str::from_utf8(bytes)
                .map_err(|_| fail(DWG_EINVAL, "XData layer name is not valid UTF-8"))?
                .to_owned(),
        )),
        DWG_XDATA_BINARY => Ok(XDataValue::BinaryData(bytes.to_vec())),
        DWG_XDATA_HANDLE => {
            let handle = Handle::new(value.handle);
            if document.doc.get_entity(handle).is_none()
                && !document.doc.objects.contains_key(&handle)
            {
                return Err(fail(
                    DWG_ENOTFOUND,
                    "XData handle value is missing from document",
                ));
            }
            Ok(XDataValue::Handle(handle))
        }
        DWG_XDATA_POINT3D
        | DWG_XDATA_POSITION3D
        | DWG_XDATA_DISPLACEMENT3D
        | DWG_XDATA_DIRECTION3D => {
            if !value.point.iter().all(|coordinate| coordinate.is_finite()) {
                return Err(fail(DWG_EINVAL, "XData point is not finite"));
            }
            let point = Vector3::new(value.point[0], value.point[1], value.point[2]);
            Ok(match value.tag {
                DWG_XDATA_POINT3D => XDataValue::Point3D(point),
                DWG_XDATA_POSITION3D => XDataValue::Position3D(point),
                DWG_XDATA_DISPLACEMENT3D => XDataValue::Displacement3D(point),
                _ => XDataValue::Direction3D(point),
            })
        }
        DWG_XDATA_REAL => {
            if !value.real.is_finite() {
                return Err(fail(DWG_EINVAL, "XData real is not finite"));
            }
            Ok(XDataValue::Real(value.real))
        }
        DWG_XDATA_DISTANCE => {
            if !value.real.is_finite() {
                return Err(fail(DWG_EINVAL, "XData distance is not finite"));
            }
            Ok(XDataValue::Distance(value.real))
        }
        DWG_XDATA_SCALE_FACTOR => {
            if !value.real.is_finite() {
                return Err(fail(DWG_EINVAL, "XData scale factor is not finite"));
            }
            Ok(XDataValue::ScaleFactor(value.real))
        }
        DWG_XDATA_INTEGER16 => i16::try_from(value.integer)
            .map(XDataValue::Integer16)
            .map_err(|_| fail(DWG_EINVAL, "XData Integer16 is out of range")),
        DWG_XDATA_INTEGER32 => i32::try_from(value.integer)
            .map(XDataValue::Integer32)
            .map_err(|_| fail(DWG_EINVAL, "XData Integer32 is out of range")),
        _ => unreachable!("validated XData tag"),
    }
}

unsafe fn validate_output(value: &DwgXdataValue, source: &XDataValue) -> Result<(), i32> {
    let tag = tag_for(source);
    if value.tag != 0 && value.tag != tag {
        return Err(fail(DWG_ETYPE, "XData output tag does not match value"));
    }
    let required = match source {
        XDataValue::String(text)
        | XDataValue::ControlString(text)
        | XDataValue::LayerName(text) => text.len() + 1,
        XDataValue::BinaryData(bytes) => bytes.len(),
        _ => 0,
    };
    if value.data.is_null() {
        if value.data_len != 0 {
            return Err(fail(
                DWG_EINVAL,
                "XData output data is null with a nonzero length",
            ));
        }
    } else if value.data_len == 0 {
        return Err(fail(DWG_EINVAL, "XData output buffer has zero capacity"));
    } else if value.data_len < required {
        return Err(fail(DWG_EBUFFER, "XData output buffer is too small"));
    }
    Ok(())
}

unsafe fn write_value(output: *mut DwgXdataValue, source: &XDataValue) {
    let data = (*output).data;
    *output = DwgXdataValue {
        tag: tag_for(source),
        reserved: 0,
        integer: 0,
        real: 0.0,
        point: [0.0; 3],
        handle: 0,
        data,
        data_len: 0,
    };
    match source {
        XDataValue::String(text)
        | XDataValue::ControlString(text)
        | XDataValue::LayerName(text) => {
            let bytes = text.as_bytes();
            (*output).data_len = bytes.len() + 1;
            if !data.is_null() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
                *data.cast::<u8>().add(bytes.len()) = 0;
            }
        }
        XDataValue::BinaryData(bytes) => {
            (*output).data_len = bytes.len();
            if !data.is_null() && !bytes.is_empty() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
            }
        }
        XDataValue::Handle(handle) => (*output).handle = handle.value(),
        XDataValue::Point3D(point)
        | XDataValue::Position3D(point)
        | XDataValue::Displacement3D(point)
        | XDataValue::Direction3D(point) => (*output).point = [point.x, point.y, point.z],
        XDataValue::Real(real) | XDataValue::Distance(real) | XDataValue::ScaleFactor(real) => {
            (*output).real = *real
        }
        XDataValue::Integer16(integer) => (*output).integer = i64::from(*integer),
        XDataValue::Integer32(integer) => (*output).integer = i64::from(*integer),
    }
}

fn record<'a>(
    document: &'a FfiDocument,
    entity_handle: u64,
    index: usize,
) -> Result<&'a ExtendedDataRecord, i32> {
    let entity = entity(document, entity_handle)?;
    entity
        .common()
        .extended_data
        .record(index)
        .ok_or_else(|| fail(DWG_ENOTFOUND, "XData record index is out of range"))
}

fn record_name(document: &FfiDocument, entity_handle: u64, index: usize) -> Result<String, i32> {
    Ok(record(document, entity_handle, index)?
        .application_name
        .clone())
}

fn invalidate_raw_eed(
    document: &mut FfiDocument,
    entity_handle: u64,
    name: &str,
) -> Result<(), i32> {
    let app_handle = document.doc.app_ids.get(name).map(|app| app.handle.value());
    if let Some(app_handle) = app_handle {
        let target = entity_mut(document, entity_handle)?;
        target
            .common_mut()
            .extended_data
            .raw_dwg_eed
            .retain(|(handle, _)| *handle != app_handle);
    }
    Ok(())
}

unsafe fn write_name(output: *mut core::ffi::c_char, capacity: usize, name: &str) {
    let bytes = name.as_bytes();
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast::<u8>(), bytes.len());
    *output.add(bytes.len()) = 0;
    let _ = capacity;
}

fn validate_name_output(
    output: *mut core::ffi::c_char,
    capacity: usize,
    required: *mut usize,
    name: &str,
) -> Result<(), i32> {
    if required.is_null() {
        return Err(fail(DWG_EINVAL, "XData name length output is null"));
    }
    let needed = name.len() + 1;
    unsafe {
        *required = needed;
    }
    if output.is_null() {
        return Ok(());
    }
    if capacity == 0 {
        return Err(fail(DWG_EINVAL, "XData name buffer has zero capacity"));
    }
    if capacity < needed {
        return Err(fail(DWG_EBUFFER, "XData name buffer is too small"));
    }
    Ok(())
}

/// Return the number of XData records attached to an entity.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_record_count(
    data: *const DwgData,
    entity_handle: u64,
    count: *mut usize,
) -> i32 {
    if count.is_null() {
        return fail(DWG_EINVAL, "XData count output is null");
    }
    let document = match document(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let entity = match entity(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    *count = entity.common().extended_data.len();
    clear_last_error();
    DWG_OK
}

/// Find a zero-based occurrence of an XData application record.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_record_find(
    data: *const DwgData,
    entity_handle: u64,
    app_name: *const core::ffi::c_char,
    occurrence: usize,
    index: *mut usize,
) -> i32 {
    if index.is_null() {
        return fail(DWG_EINVAL, "XData index output is null");
    }
    let name = match application_name(app_name) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let document = match document(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let entity = match entity(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let Some((found, _)) = entity
        .common()
        .extended_data
        .records()
        .iter()
        .enumerate()
        .filter(|(_, record)| record.application_name == name)
        .nth(occurrence)
    else {
        return fail(DWG_ENOTFOUND, "XData application record is missing");
    };
    *index = found;
    clear_last_error();
    DWG_OK
}

/// Read an XData record name and value count.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_record_entry(
    data: *const DwgData,
    entity_handle: u64,
    record_index: usize,
    application_name: *mut core::ffi::c_char,
    capacity: usize,
    required: *mut usize,
    value_count: *mut usize,
) -> i32 {
    if value_count.is_null() || required.is_null() {
        return fail(DWG_EINVAL, "XData value count output is null");
    }
    let document = match document(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let source = match record(document, entity_handle, record_index) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = validate_name_output(
        application_name,
        capacity,
        required,
        &source.application_name,
    ) {
        return code;
    }
    *value_count = source.values.len();
    if !application_name.is_null() {
        write_name(application_name, capacity, &source.application_name);
    }
    clear_last_error();
    DWG_OK
}

/// Append an empty XData record, registering its APPID when necessary.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_record_append(
    data: *mut DwgData,
    entity_handle: u64,
    app_name: *const core::ffi::c_char,
    record_index: *mut usize,
) -> i32 {
    if record_index.is_null() {
        return fail(DWG_EINVAL, "XData record index output is null");
    }
    let name = match application_name(app_name) {
        Ok(value) => value.to_owned(),
        Err(code) => return code,
    };
    let document = match document_mut(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = entity(document, entity_handle) {
        return code;
    }
    if document
        .doc
        .get_entity(Handle::new(entity_handle))
        .unwrap()
        .common()
        .extended_data
        .get_record(&name)
        .is_some()
    {
        return fail(DWG_EINVAL, "XData application record already exists");
    }
    if !document.doc.app_ids.contains(&name) {
        let mut app = AppId::new(&name);
        app.set_handle(document.doc.allocate_handle());
        if document.doc.app_ids.add(app).is_err() {
            return fail(DWG_EINVAL, "XData APPID already exists");
        }
    }
    if let Err(code) = invalidate_raw_eed(document, entity_handle, &name) {
        return code;
    }
    let target = match entity_mut(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let extended_data = &mut target.common_mut().extended_data;
    *record_index = extended_data.len();
    extended_data.add_record(ExtendedDataRecord::new(name));
    clear_last_error();
    DWG_OK
}

/// Remove an XData record and return its application name.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_record_remove(
    data: *mut DwgData,
    entity_handle: u64,
    record_index: usize,
    application_name: *mut core::ffi::c_char,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    if required.is_null() {
        return fail(DWG_EINVAL, "XData name length output is null");
    }
    let document = match document_mut(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let name = match record_name(document, entity_handle, record_index) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = validate_name_output(application_name, capacity, required, &name) {
        return code;
    }
    if let Err(code) = invalidate_raw_eed(document, entity_handle, &name) {
        return code;
    }
    let target = match entity_mut(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    target
        .common_mut()
        .extended_data
        .remove_record_at(record_index);
    if !application_name.is_null() {
        write_name(application_name, capacity, &name);
    }
    clear_last_error();
    DWG_OK
}

/// Read one XData value into caller-owned storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_value(
    data: *const DwgData,
    entity_handle: u64,
    record_index: usize,
    value_index: usize,
    value: *mut DwgXdataValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XData value output is null");
    }
    let document = match document(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let source = match record(document, entity_handle, record_index).and_then(|record| {
        record
            .values
            .get(value_index)
            .ok_or_else(|| fail(DWG_ENOTFOUND, "XData value index is out of range"))
    }) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = validate_output(&*value, source) {
        return code;
    }
    write_value(value, source);
    clear_last_error();
    DWG_OK
}

/// Append one XData value.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_value_append(
    data: *mut DwgData,
    entity_handle: u64,
    record_index: usize,
    value: *const DwgXdataValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XData value input is null");
    }
    let document = match document_mut(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let name = match record_name(document, entity_handle, record_index) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let converted = match value_from_c(document, &*value) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = invalidate_raw_eed(document, entity_handle, &name) {
        return code;
    }
    let target = match entity_mut(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    target
        .common_mut()
        .extended_data
        .record_mut(record_index)
        .unwrap()
        .values
        .push(converted);
    clear_last_error();
    DWG_OK
}

/// Replace one XData value.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_value_set(
    data: *mut DwgData,
    entity_handle: u64,
    record_index: usize,
    value_index: usize,
    value: *const DwgXdataValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XData value input is null");
    }
    let document = match document_mut(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let name = match record_name(document, entity_handle, record_index) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if record(document, entity_handle, record_index)
        .unwrap()
        .values
        .get(value_index)
        .is_none()
    {
        return fail(DWG_ENOTFOUND, "XData value index is out of range");
    }
    let converted = match value_from_c(document, &*value) {
        Ok(value) => value,
        Err(code) => return code,
    };
    if let Err(code) = invalidate_raw_eed(document, entity_handle, &name) {
        return code;
    }
    let target = match entity_mut(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    target
        .common_mut()
        .extended_data
        .record_mut(record_index)
        .unwrap()
        .values[value_index] = converted;
    clear_last_error();
    DWG_OK
}

/// Remove one XData value and return it through caller-owned storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xdata_value_remove(
    data: *mut DwgData,
    entity_handle: u64,
    record_index: usize,
    value_index: usize,
    value: *mut DwgXdataValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XData value output is null");
    }
    let document = match document_mut(data) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let name = match record_name(document, entity_handle, record_index) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let source = match record(document, entity_handle, record_index).and_then(|record| {
        record
            .values
            .get(value_index)
            .ok_or_else(|| fail(DWG_ENOTFOUND, "XData value index is out of range"))
    }) {
        Ok(value) => value.clone(),
        Err(code) => return code,
    };
    // Removal outputs are overwritten wholesale; an existing tag in the
    // caller's reusable struct is not an input shape constraint.
    let mut output_shape = *value;
    output_shape.tag = 0;
    if let Err(code) = validate_output(&output_shape, &source) {
        return code;
    }
    if let Err(code) = invalidate_raw_eed(document, entity_handle, &name) {
        return code;
    }
    let target = match entity_mut(document, entity_handle) {
        Ok(value) => value,
        Err(code) => return code,
    };
    target
        .common_mut()
        .extended_data
        .record_mut(record_index)
        .unwrap()
        .values
        .remove(value_index);
    write_value(value, &source);
    clear_last_error();
    DWG_OK
}
