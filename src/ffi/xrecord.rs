//! Typed XRECORD access for the C ABI.

use std::slice;

use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::objects::{ObjectType, XRecord, XRecordEntry, XRecordValue, XRecordValueType};
use crate::types::Handle;

/// A typed XRECORD value exchanged through the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DwgXrecordValue {
    /// Payload discriminator; one of the `DWG_XRECORD_*` constants.
    pub tag: u32,
    /// Reserved for ABI evolution; mutation callers must set this to zero.
    pub reserved: u32,
    /// Integer payload for Int16, Int32, Int64, Byte, and Bool values.
    pub integer: i64,
    /// Floating-point payload for Double values.
    pub real: f64,
    /// Three-dimensional payload for Point3D values.
    pub point: [f64; 3],
    /// Document-owned handle payload for Handle and ObjectId values.
    pub handle: u64,
    /// Caller-owned string or chunk storage.
    pub data: *mut core::ffi::c_void,
    /// Input length or output capacity/required length for `data`.
    pub data_len: usize,
}

/// String XRECORD value tag.
pub const DWG_XRECORD_STRING: u32 = 1;
/// Double XRECORD value tag.
pub const DWG_XRECORD_DOUBLE: u32 = 2;
/// Signed 16-bit integer XRECORD value tag.
pub const DWG_XRECORD_INT16: u32 = 3;
/// Signed 32-bit integer XRECORD value tag.
pub const DWG_XRECORD_INT32: u32 = 4;
/// Signed 64-bit integer XRECORD value tag.
pub const DWG_XRECORD_INT64: u32 = 5;
/// Unsigned byte XRECORD value tag.
pub const DWG_XRECORD_BYTE: u32 = 6;
/// Boolean XRECORD value tag.
pub const DWG_XRECORD_BOOL: u32 = 7;
/// Handle or ObjectId XRECORD value tag.
pub const DWG_XRECORD_HANDLE: u32 = 8;
/// Three-dimensional point XRECORD value tag.
pub const DWG_XRECORD_POINT3D: u32 = 9;
/// Binary chunk XRECORD value tag.
pub const DWG_XRECORD_CHUNK: u32 = 10;

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

fn xrecord<'a>(document: &'a FfiDocument, handle: u64) -> Result<&'a XRecord, i32> {
    match document.doc.objects.get(&Handle::new(handle)) {
        Some(ObjectType::XRecord(record)) => Ok(record),
        Some(_) => Err(fail(DWG_ETYPE, "object is not an XRECORD")),
        None => Err(fail(
            DWG_ENOTFOUND,
            "XRECORD handle is missing from document",
        )),
    }
}

fn xrecord_mut<'a>(document: &'a mut FfiDocument, handle: u64) -> Result<&'a mut XRecord, i32> {
    match document.doc.objects.get_mut(&Handle::new(handle)) {
        Some(ObjectType::XRecord(record)) => Ok(record),
        Some(_) => Err(fail(DWG_ETYPE, "object is not an XRECORD")),
        None => Err(fail(
            DWG_ENOTFOUND,
            "XRECORD handle is missing from document",
        )),
    }
}

fn tag_for(value: &XRecordValue) -> u32 {
    match value {
        XRecordValue::String(_) => DWG_XRECORD_STRING,
        XRecordValue::Double(_) => DWG_XRECORD_DOUBLE,
        XRecordValue::Int16(_) => DWG_XRECORD_INT16,
        XRecordValue::Int32(_) => DWG_XRECORD_INT32,
        XRecordValue::Int64(_) => DWG_XRECORD_INT64,
        XRecordValue::Byte(_) => DWG_XRECORD_BYTE,
        XRecordValue::Bool(_) => DWG_XRECORD_BOOL,
        XRecordValue::Handle(_) => DWG_XRECORD_HANDLE,
        XRecordValue::Point3D(_, _, _) => DWG_XRECORD_POINT3D,
        XRecordValue::Chunk(_) => DWG_XRECORD_CHUNK,
    }
}

fn tag_matches_code(tag: u32, code: i32) -> bool {
    matches!(
        (tag, XRecordValueType::from_code(code)),
        (DWG_XRECORD_STRING, XRecordValueType::String)
            | (DWG_XRECORD_DOUBLE, XRecordValueType::Double)
            | (DWG_XRECORD_INT16, XRecordValueType::Int16)
            | (DWG_XRECORD_INT32, XRecordValueType::Int32)
            | (DWG_XRECORD_INT64, XRecordValueType::Int64)
            | (DWG_XRECORD_BYTE, XRecordValueType::Byte)
            | (DWG_XRECORD_BOOL, XRecordValueType::Bool)
            | (
                DWG_XRECORD_HANDLE,
                XRecordValueType::Handle | XRecordValueType::ObjectId
            )
            | (DWG_XRECORD_POINT3D, XRecordValueType::Point3D)
            | (DWG_XRECORD_CHUNK, XRecordValueType::Chunk)
    )
}

unsafe fn input_bytes(value: &DwgXrecordValue) -> Result<&[u8], i32> {
    if value.data.is_null() {
        if value.data_len == 0 {
            return Ok(&[]);
        }
        return Err(fail(
            DWG_EINVAL,
            "XRECORD data is null with a nonzero length",
        ));
    }
    Ok(slice::from_raw_parts(
        value.data.cast::<u8>(),
        value.data_len,
    ))
}

unsafe fn value_from_c(
    document: &FfiDocument,
    code: i32,
    value: &DwgXrecordValue,
) -> Result<XRecordValue, i32> {
    if value.reserved != 0 {
        return Err(fail(DWG_EINVAL, "XRECORD value reserved bits are nonzero"));
    }
    if !(DWG_XRECORD_STRING..=DWG_XRECORD_CHUNK).contains(&value.tag) {
        return Err(fail(DWG_EINVAL, "XRECORD value tag is invalid"));
    }
    if !tag_matches_code(value.tag, code) {
        return Err(fail(
            DWG_ETYPE,
            "XRECORD value tag does not match group code",
        ));
    }

    match value.tag {
        DWG_XRECORD_STRING => {
            let text = std::str::from_utf8(input_bytes(value)?)
                .map_err(|_| fail(DWG_EINVAL, "XRECORD string is not valid UTF-8"))?;
            Ok(XRecordValue::String(text.to_owned()))
        }
        DWG_XRECORD_DOUBLE => {
            if !value.real.is_finite() {
                return Err(fail(DWG_EINVAL, "XRECORD double is not finite"));
            }
            Ok(XRecordValue::Double(value.real))
        }
        DWG_XRECORD_INT16 => match i16::try_from(value.integer) {
            Ok(integer) => Ok(XRecordValue::Int16(integer)),
            Err(_) => Err(fail(DWG_EINVAL, "XRECORD Int16 is out of range")),
        },
        DWG_XRECORD_INT32 => match i32::try_from(value.integer) {
            Ok(integer) => Ok(XRecordValue::Int32(integer)),
            Err(_) => Err(fail(DWG_EINVAL, "XRECORD Int32 is out of range")),
        },
        DWG_XRECORD_INT64 => Ok(XRecordValue::Int64(value.integer)),
        DWG_XRECORD_BYTE => match u8::try_from(value.integer) {
            Ok(integer) => Ok(XRecordValue::Byte(integer)),
            Err(_) => Err(fail(DWG_EINVAL, "XRECORD byte is out of range")),
        },
        DWG_XRECORD_BOOL => match value.integer {
            0 => Ok(XRecordValue::Bool(false)),
            1 => Ok(XRecordValue::Bool(true)),
            _ => Err(fail(DWG_EINVAL, "XRECORD boolean must be zero or one")),
        },
        DWG_XRECORD_HANDLE => {
            let handle = Handle::new(value.handle);
            if !document.doc.objects.contains_key(&handle) {
                return Err(fail(
                    DWG_ENOTFOUND,
                    "XRECORD handle value is missing from document",
                ));
            }
            Ok(XRecordValue::Handle(handle))
        }
        DWG_XRECORD_POINT3D => {
            if !value.point.iter().all(|coordinate| coordinate.is_finite()) {
                return Err(fail(DWG_EINVAL, "XRECORD point is not finite"));
            }
            Ok(XRecordValue::Point3D(
                value.point[0],
                value.point[1],
                value.point[2],
            ))
        }
        DWG_XRECORD_CHUNK => Ok(XRecordValue::Chunk(input_bytes(value)?.to_vec())),
        _ => unreachable!("validated XRECORD tag"),
    }
}

unsafe fn validate_output(value: &DwgXrecordValue, source: &XRecordValue) -> Result<(), i32> {
    let required = match source {
        XRecordValue::String(text) => text.len() + 1,
        XRecordValue::Chunk(bytes) => bytes.len(),
        _ => return Ok(()),
    };
    if value.data.is_null() {
        return Ok(());
    }
    if value.data_len == 0 {
        return Err(fail(DWG_EINVAL, "XRECORD output buffer has zero capacity"));
    }
    if value.data_len < required {
        return Err(fail(DWG_EBUFFER, "XRECORD output buffer is too small"));
    }
    Ok(())
}

unsafe fn write_value(value: *mut DwgXrecordValue, source: &XRecordValue) {
    let output = &mut *value;
    let data = output.data;
    *output = DwgXrecordValue {
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
        XRecordValue::String(text) => {
            let bytes = text.as_bytes();
            output.data_len = bytes.len() + 1;
            if !data.is_null() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
                *data.cast::<u8>().add(bytes.len()) = 0;
            }
        }
        XRecordValue::Double(real) => output.real = *real,
        XRecordValue::Int16(integer) => output.integer = i64::from(*integer),
        XRecordValue::Int32(integer) => output.integer = i64::from(*integer),
        XRecordValue::Int64(integer) => output.integer = *integer,
        XRecordValue::Byte(integer) => output.integer = i64::from(*integer),
        XRecordValue::Bool(boolean) => output.integer = i64::from(*boolean),
        XRecordValue::Handle(handle) => output.handle = handle.value(),
        XRecordValue::Point3D(x, y, z) => output.point = [*x, *y, *z],
        XRecordValue::Chunk(bytes) => {
            output.data_len = bytes.len();
            if !data.is_null() && !bytes.is_empty() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
            }
        }
    }
}

/// Return the number of entries in an XRECORD.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_count(
    data: *const DwgData,
    xrecord_handle: u64,
    count: *mut usize,
) -> i32 {
    if count.is_null() {
        return fail(DWG_EINVAL, "XRECORD count output is null");
    }
    let document = match document(data) {
        Ok(document) => document,
        Err(code) => return code,
    };
    let xrecord = match xrecord(document, xrecord_handle) {
        Ok(xrecord) => xrecord,
        Err(code) => return code,
    };
    *count = xrecord.entries.len();
    clear_last_error();
    DWG_OK
}

/// Find the physical index of a zero-based occurrence of an XRECORD group code.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_find(
    data: *const DwgData,
    xrecord_handle: u64,
    code: i32,
    occurrence: usize,
    index: *mut usize,
) -> i32 {
    if index.is_null() {
        return fail(DWG_EINVAL, "XRECORD index output is null");
    }
    let document = match document(data) {
        Ok(document) => document,
        Err(code) => return code,
    };
    let xrecord = match xrecord(document, xrecord_handle) {
        Ok(xrecord) => xrecord,
        Err(code) => return code,
    };
    let Some((index_value, _)) = xrecord
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.code == code)
        .nth(occurrence)
    else {
        return fail(DWG_ENOTFOUND, "XRECORD group code occurrence is missing");
    };
    *index = index_value;
    clear_last_error();
    DWG_OK
}

/// Read one XRECORD entry into caller-owned output storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_entry(
    data: *const DwgData,
    xrecord_handle: u64,
    index: usize,
    code: *mut i32,
    value: *mut DwgXrecordValue,
) -> i32 {
    if code.is_null() || value.is_null() {
        return fail(DWG_EINVAL, "XRECORD entry output is null");
    }
    let document = match document(data) {
        Ok(document) => document,
        Err(code) => return code,
    };
    let xrecord = match xrecord(document, xrecord_handle) {
        Ok(xrecord) => xrecord,
        Err(code) => return code,
    };
    let Some(entry) = xrecord.entries.get(index) else {
        return fail(DWG_ENOTFOUND, "XRECORD entry index is out of range");
    };
    if let Err(code_value) = validate_output(&*value, &entry.value) {
        return code_value;
    }
    *code = entry.code;
    write_value(value, &entry.value);
    clear_last_error();
    DWG_OK
}

fn mutate_xrecord(document: &mut FfiDocument, handle: u64, code: i32, entry: XRecordEntry) {
    let xrecord = xrecord_mut(document, handle).expect("XRECORD was validated before mutation");
    xrecord.entries.push(entry);
    if (330..=369).contains(&code) {
        xrecord.synchronize_object_references();
    }
    xrecord.preserve_object_reference_stream = false;
    xrecord.entries_complete = true;
}

/// Append one typed entry to an XRECORD.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_append(
    data: *mut DwgData,
    xrecord_handle: u64,
    code: i32,
    value: *const DwgXrecordValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XRECORD input value is null");
    }
    let document = match document_mut(data) {
        Ok(document) => document,
        Err(code) => return code,
    };
    if let Err(code_value) = xrecord(document, xrecord_handle) {
        return code_value;
    }
    let converted = match value_from_c(document, code, &*value) {
        Ok(value) => value,
        Err(code_value) => return code_value,
    };
    mutate_xrecord(
        document,
        xrecord_handle,
        code,
        XRecordEntry::new(code, converted),
    );
    clear_last_error();
    DWG_OK
}

/// Replace one XRECORD entry at a zero-based index.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_set(
    data: *mut DwgData,
    xrecord_handle: u64,
    index: usize,
    code: i32,
    value: *const DwgXrecordValue,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "XRECORD input value is null");
    }
    let document = match document_mut(data) {
        Ok(document) => document,
        Err(code_value) => return code_value,
    };
    let previous_code = match xrecord(document, xrecord_handle) {
        Ok(xrecord) => match xrecord.entries.get(index) {
            Some(entry) => entry.code,
            None => return fail(DWG_ENOTFOUND, "XRECORD entry index is out of range"),
        },
        Err(code_value) => return code_value,
    };
    let converted = match value_from_c(document, code, &*value) {
        Ok(value) => value,
        Err(code_value) => return code_value,
    };
    let xrecord =
        xrecord_mut(document, xrecord_handle).expect("XRECORD was validated before mutation");
    xrecord.entries[index] = XRecordEntry::new(code, converted);
    if (330..=369).contains(&previous_code) || (330..=369).contains(&code) {
        xrecord.synchronize_object_references();
    }
    xrecord.preserve_object_reference_stream = false;
    xrecord.entries_complete = true;
    clear_last_error();
    DWG_OK
}

/// Remove one XRECORD entry and return it through caller-owned storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_xrecord_remove(
    data: *mut DwgData,
    xrecord_handle: u64,
    index: usize,
    code: *mut i32,
    value: *mut DwgXrecordValue,
) -> i32 {
    if code.is_null() || value.is_null() {
        return fail(DWG_EINVAL, "XRECORD removal output is null");
    }
    let document = match document_mut(data) {
        Ok(document) => document,
        Err(code_value) => return code_value,
    };
    let entry = match xrecord(document, xrecord_handle) {
        Ok(xrecord) => match xrecord.entries.get(index) {
            Some(entry) => entry.clone(),
            None => return fail(DWG_ENOTFOUND, "XRECORD entry index is out of range"),
        },
        Err(code_value) => return code_value,
    };
    if let Err(code_value) = validate_output(&*value, &entry.value) {
        return code_value;
    }
    let xrecord =
        xrecord_mut(document, xrecord_handle).expect("XRECORD was validated before mutation");
    xrecord.entries.remove(index);
    if (330..=369).contains(&entry.code) {
        xrecord.synchronize_object_references();
    }
    xrecord.preserve_object_reference_stream = false;
    xrecord.entries_complete = true;
    *code = entry.code;
    write_value(value, &entry.value);
    clear_last_error();
    DWG_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::CadDocument;

    fn fixture(entries: Vec<XRecordEntry>) -> (FfiDocument, u64) {
        let mut document = CadDocument::new();
        let owner = document.header.named_objects_dict_handle;
        let handle = document.ensure_xrecord(owner, "FFI_XRECORD_TEST");
        let Some(ObjectType::XRecord(xrecord)) = document.objects.get_mut(&handle) else {
            panic!("test XRECORD is missing");
        };
        xrecord.entries = entries;
        xrecord.preserve_object_reference_stream = true;
        (FfiDocument::new(document), handle.value())
    }

    fn int32_value(value: i32) -> DwgXrecordValue {
        DwgXrecordValue {
            tag: DWG_XRECORD_INT32,
            reserved: 0,
            integer: i64::from(value),
            real: 0.0,
            point: [0.0; 3],
            handle: 0,
            data: std::ptr::null_mut(),
            data_len: 0,
        }
    }

    #[test]
    fn exported_append_clears_preserved_reference_stream_for_code_90() {
        let (mut document, handle) = fixture(Vec::new());
        let data = (&mut document as *mut FfiDocument).cast::<DwgData>();
        let value = int32_value(10);

        assert_eq!(
            unsafe { dwg_xrecord_append(data, handle, 90, &value) },
            DWG_OK
        );
        assert!(
            !xrecord(&document, handle)
                .unwrap()
                .preserve_object_reference_stream
        );
    }

    #[test]
    fn exported_set_clears_preserved_reference_stream_for_code_90() {
        let (mut document, handle) = fixture(vec![XRecordEntry::int32(90, 1)]);
        let data = (&mut document as *mut FfiDocument).cast::<DwgData>();
        let value = int32_value(10);

        assert_eq!(
            unsafe { dwg_xrecord_set(data, handle, 0, 90, &value) },
            DWG_OK
        );
        assert!(
            !xrecord(&document, handle)
                .unwrap()
                .preserve_object_reference_stream
        );
    }

    #[test]
    fn exported_remove_clears_preserved_reference_stream_for_code_90() {
        let (mut document, handle) = fixture(vec![XRecordEntry::int32(90, 1)]);
        let data = (&mut document as *mut FfiDocument).cast::<DwgData>();
        let mut code = 0;
        let mut value = int32_value(0);

        assert_eq!(
            unsafe { dwg_xrecord_remove(data, handle, 0, &mut code, &mut value) },
            DWG_OK
        );
        assert!(
            !xrecord(&document, handle)
                .unwrap()
                .preserve_object_reference_stream
        );
    }
}
