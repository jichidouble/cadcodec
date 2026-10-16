//! DWG/DXF file read/write through the C ABI.

use std::ffi::CStr;
use std::io::Cursor;
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::document::CadDocument;
use crate::ffi::buffer::{free_ffi_buffer, into_ffi_buffer};
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;

fn c_path<'a>(text: *const c_char) -> Result<PathBuf, i32> {
    if text.is_null() {
        return Err(DWG_EINVAL);
    }
    let s = unsafe { CStr::from_ptr(text) }
        .to_str()
        .map_err(|_| DWG_EINVAL)?;
    Ok(PathBuf::from(s))
}

fn replace_document(data: *mut DwgData, doc: CadDocument) {
    unsafe {
        let ffi = &mut *(data as *mut FfiDocument);
        let callback = ffi.notification_callback;
        let user_data = ffi.notification_user_data;
        *ffi = FfiDocument::new(doc);
        ffi.notification_callback = callback;
        ffi.notification_user_data = user_data;
    }
}

fn dispatch_notifications(data: *const DwgData) {
    unsafe {
        (&*(data as *const FfiDocument)).dispatch_notifications();
    }
}

fn read_impl(path: *const c_char, data: *mut DwgData, dwg: bool) -> i32 {
    if data.is_null() {
        return DWG_EINVAL;
    }
    let path = match c_path(path) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let result = if dwg {
        crate::io::dwg::DwgReader::from_file(&path).and_then(|mut r| r.read())
    } else {
        crate::io::dxf::DxfReader::from_file(&path).and_then(|r| r.read())
    };
    match result {
        Ok(doc) => {
            replace_document(data, doc);
            dispatch_notifications(data);
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

fn read_bytes_impl(input: *const u8, input_len: usize, data: *mut DwgData) -> i32 {
    if input.is_null() || input_len == 0 || data.is_null() {
        set_last_error(DWG_EINVAL, "input bytes and data are required");
        return DWG_EINVAL;
    }

    let bytes = unsafe { std::slice::from_raw_parts(input, input_len) };
    let result = crate::io::dwg::DwgReader::from_stream(Cursor::new(bytes)).read();
    match result {
        Ok(doc) => {
            replace_document(data, doc);
            dispatch_notifications(data);
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

/// Read a DWG byte sequence into `data`, replacing its contents only after a
/// complete parse succeeds.
///
/// # Safety
/// `input` must point to `input_len` readable bytes and `data` must be a valid
/// mutable document handle created by `dwg_data_create`.
#[no_mangle]
pub unsafe extern "C" fn dwg_read_bytes(
    input: *const u8,
    input_len: usize,
    data: *mut DwgData,
) -> i32 {
    read_bytes_impl(input, input_len, data)
}

/// Write a DWG document to a newly allocated byte buffer.
///
/// # Safety
/// `data` must be a valid document handle. `output` and `output_len` must be
/// valid writable pointers. The returned buffer must be released with
/// `dwg_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn dwg_write_bytes(
    data: *const DwgData,
    output: *mut *mut u8,
    output_len: *mut usize,
) -> i32 {
    if output.is_null() || output_len.is_null() {
        set_last_error(DWG_EINVAL, "output pointers are required");
        return DWG_EINVAL;
    }
    *output = std::ptr::null_mut();
    *output_len = 0;
    if data.is_null() {
        set_last_error(DWG_EINVAL, "data is required");
        return DWG_EINVAL;
    }

    let ffi = &*(data as *const FfiDocument);
    let bytes = match crate::io::dwg::DwgWriter::write_to_vec(&ffi.doc) {
        Ok(bytes) => bytes,
        Err(e) => {
            let msg = format!("{e}");
            set_last_error(DWG_EWRITE, &msg);
            return DWG_EWRITE;
        }
    };
    let (buffer, len) = match into_ffi_buffer(bytes) {
        Ok(result) => result,
        Err(code) => {
            set_last_error(code, "failed to allocate output buffer");
            return code;
        }
    };
    *output = buffer;
    *output_len = len;
    clear_last_error();
    DWG_OK
}

/// Release a byte buffer returned by `dwg_write_bytes`.
///
/// # Safety
/// `buffer` must be NULL or a pointer returned by a successful
/// `dwg_write_bytes` call that has not already been released.
#[no_mangle]
pub unsafe extern "C" fn dwg_free_buffer(buffer: *mut std::ffi::c_void) {
    free_ffi_buffer(buffer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::error::dwg_last_error_code;
    use crate::ffi::object::FfiDocument;
    use crate::types::DxfVersion;

    #[test]
    fn byte_write_failure_clears_output_and_reports_error() {
        let mut ffi = FfiDocument::new(CadDocument::new());
        ffi.doc.version = DxfVersion::Unknown;
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();
        let mut output = 1usize as *mut u8;
        let mut output_len = usize::MAX;

        assert_eq!(
            unsafe { dwg_write_bytes(data, &mut output, &mut output_len) },
            DWG_EWRITE
        );
        assert!(output.is_null());
        assert_eq!(output_len, 0);
        assert_eq!(dwg_last_error_code(), DWG_EWRITE);
    }
}
