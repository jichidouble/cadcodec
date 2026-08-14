//! DWG/DXF file read/write entry points.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::document::CadDocument;
use crate::error::DxfError;
use crate::io::dxf::{DxfReader, DxfWriter};
use crate::io::dwg::{DwgReader, DwgWriter};

use super::error::{clear_last_error, set_last_error};
use super::types::{DwgData, DWG_EINVAL, DWG_ENOTFOUND, DWG_EREAD, DWG_EVERSION, DWG_EWRITE, DWG_OK};

fn path_from_cstr(path: *const c_char) -> Result<PathBuf, i32> {
    if path.is_null() {
        return Err(DWG_EINVAL);
    }
    // SAFETY: the caller promises `path` is a NUL-terminated string for the
    // duration of the call.
    let s = unsafe { CStr::from_ptr(path) };
    let s = s.to_str().map_err(|_| DWG_EINVAL)?;
    Ok(PathBuf::from(s))
}

fn fail(code: i32, msg: String) -> i32 {
    set_last_error(code, msg);
    code
}

fn map_read_error(e: &DxfError) -> i32 {
    match e {
        DxfError::UnsupportedVersion(_) => DWG_EVERSION,
        DxfError::ObjectNotFound(_) => DWG_ENOTFOUND,
        _ => DWG_EREAD,
    }
}

fn map_write_error(e: &DxfError) -> i32 {
    match e {
        DxfError::UnsupportedVersion(_) => DWG_EVERSION,
        DxfError::ObjectNotFound(_) => DWG_ENOTFOUND,
        _ => DWG_EWRITE,
    }
}

fn write_document(
    path: *const c_char,
    data: *const DwgData,
    write: impl FnOnce(&PathBuf, &CadDocument) -> Result<(), DxfError>,
) -> i32 {
    let path = match path_from_cstr(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    if data.is_null() {
        return DWG_EINVAL;
    }
    // SAFETY: `data` is a handle returned by `dwg_data_create` and must stay
    // valid for the call.
    let doc = unsafe { &*(data as *const CadDocument) };
    match write(&path, doc) {
        Ok(()) => {
            clear_last_error();
            DWG_OK
        }
        Err(e) => {
            let code = map_write_error(&e);
            fail(code, format!("{e}"))
        }
    }
}

/// Read a DWG file into an existing document handle.
#[no_mangle]
pub extern "C" fn dwg_read_file(path: *const c_char, data: *mut DwgData) -> i32 {
    let path = match path_from_cstr(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    if data.is_null() {
        return DWG_EINVAL;
    }
    // SAFETY: `data` is a handle returned by `dwg_data_create` and must stay
    // valid for the call.
    let doc = unsafe { &mut *(data as *mut CadDocument) };
    let mut reader = match DwgReader::from_file(&path) {
        Ok(r) => r,
        Err(e) => return fail(map_read_error(&e), format!("{e}")),
    };
    match reader.read() {
        Ok(loaded) => {
            *doc = loaded;
            clear_last_error();
            DWG_OK
        }
        Err(e) => fail(map_read_error(&e), format!("{e}")),
    }
}

/// Write the document handle to a DWG file.
#[no_mangle]
pub extern "C" fn dwg_write_file(path: *const c_char, data: *const DwgData) -> i32 {
    write_document(path, data, |p, doc| DwgWriter::write_to_file(p, doc))
}

/// Read a DXF file into an existing document handle.
#[no_mangle]
pub extern "C" fn dxf_read_file(path: *const c_char, data: *mut DwgData) -> i32 {
    let path = match path_from_cstr(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    if data.is_null() {
        return DWG_EINVAL;
    }
    // SAFETY: `data` is a handle returned by `dwg_data_create` and must stay
    // valid for the call.
    let doc = unsafe { &mut *(data as *mut CadDocument) };
    match DxfReader::from_file(&path).and_then(|r| r.read()) {
        Ok(loaded) => {
            *doc = loaded;
            clear_last_error();
            DWG_OK
        }
        Err(e) => fail(map_read_error(&e), format!("{e}")),
    }
}

/// Write the document handle to a DXF file.
#[no_mangle]
pub extern "C" fn dxf_write_file(path: *const c_char, data: *const DwgData) -> i32 {
    write_document(path, data, |p, doc| DxfWriter::new(doc).write_to_file(p))
}
