//! DWG/DXF file read/write through the C ABI.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::document::CadDocument;
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
        Ok(()) => {
            clear_last_error();
            DWG_OK
        }
        Err(e) => {
            let msg = format!("{e}");
            set_last_error(DWG_EWRITE, &msg);
            DWG_EWRITE
        }
    }
}

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
}
