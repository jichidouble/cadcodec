<<<<<<< HEAD
//! DWG/DXF file read/write entry points.
=======
//! DWG/DXF file read/write through the C ABI.
>>>>>>> feat/ffi-c-api

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::document::CadDocument;
<<<<<<< HEAD
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
=======
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;

fn c_path<'a>(text: *const c_char) -> Result<PathBuf, i32> {
    if text.is_null() {
        return Err(DWG_EINVAL);
    }
    let s = unsafe { CStr::from_ptr(text) }.to_str().map_err(|_| DWG_EINVAL)?;
    Ok(PathBuf::from(s))
}

fn replace_document(data: *mut DwgData, doc: CadDocument) {
    unsafe {
        let ffi = &mut *(data as *mut FfiDocument);
        *ffi = FfiDocument::new(doc);
    }
}

fn read_impl(
    path: *const c_char,
    data: *mut DwgData,
    dwg: bool,
) -> i32 {
    if data.is_null() {
        return DWG_EINVAL;
    }
    let path = match c_path(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let result = if dwg {
        crate::io::dwg::DwgReader::from_file(&path)
            .and_then(|mut r| r.read())
    } else {
        crate::io::dxf::DxfReader::from_file(&path)
            .and_then(|r| r.read())
    };
    match result {
        Ok(doc) => {
            replace_document(data, doc);
            clear_last_error();
            DWG_OK
        }
        Err(e) => {
            let msg = format!("{e}");
            set_last_error(DWG_EREAD, &msg);
            DWG_EREAD
        }
    }
}

fn write_impl(path: *const c_char, data: *const DwgData, dwg: bool) -> i32 {
    if data.is_null() {
        return DWG_EINVAL;
    }
    let path = match c_path(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let ffi = unsafe { &*(data as *const FfiDocument) };
    let doc = &ffi.doc;
    let result = if dwg {
        crate::io::dwg::DwgWriter::write_to_file(&path, doc)
    } else {
        crate::io::dxf::DxfWriter::new(doc).write_to_file(&path)
    };
    match result {
>>>>>>> feat/ffi-c-api
        Ok(()) => {
            clear_last_error();
            DWG_OK
        }
        Err(e) => {
<<<<<<< HEAD
            let code = map_write_error(&e);
            fail(code, format!("{e}"))
=======
            let msg = format!("{e}");
            set_last_error(DWG_EWRITE, &msg);
            DWG_EWRITE
>>>>>>> feat/ffi-c-api
        }
    }
}

<<<<<<< HEAD
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
=======
/// Read a DWG file into `data`, replacing its contents.
#[no_mangle]
pub extern "C" fn dwg_read_file(path: *const c_char, data: *mut DwgData) -> i32 {
    read_impl(path, data, true)
}

/// Write `data` to a DWG file.
#[no_mangle]
pub extern "C" fn dwg_write_file(path: *const c_char, data: *const DwgData) -> i32 {
    write_impl(path, data, true)
}

/// Read a DXF file into `data`, replacing its contents.
#[no_mangle]
pub extern "C" fn dxf_read_file(path: *const c_char, data: *mut DwgData) -> i32 {
    read_impl(path, data, false)
}

/// Write `data` to a DXF file.
#[no_mangle]
pub extern "C" fn dxf_write_file(path: *const c_char, data: *const DwgData) -> i32 {
    write_impl(path, data, false)
>>>>>>> feat/ffi-c-api
}
