//! ABI smoke tests for the FFI surface.
//!
//! These tests call the `extern "C"` symbols directly (bypassing a real C
//! compiler) so they run on any host toolchain.  The C integration smoke
//! test lives next to this file and is compiled by the `c_smoke` helper.

#![allow(unused_unsafe)]

use acadrust::ffi::*;
use acadrust::{CadDocument, DxfVersion, DwgReader, DwgWriter, EntityType};
use std::ffi::CString;
use std::path::PathBuf;

/// Shared temp directory for FFI round-trip fixtures.
fn ffi_temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("acadrust_ffi_p1");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write a small real DWG fixture using the library's own writer so the FFI
/// read path exercises a valid file without committing binary fixtures.
fn write_dwg_fixture(path: &std::path::Path) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let line = acadrust::entities::Line::from_coords(0.0, 0.0, 0.0, 100.0, 50.0, 0.0);
    doc.add_entity(EntityType::Line(line)).unwrap();
    DwgWriter::write_to_file(path, &doc).unwrap();
}

#[test]
fn last_error_is_empty_by_default() {
    let mut buf = [0u8; 64];
    let mut needed = 0usize;
    let rc = unsafe { dwg_last_error(buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, 1);
    assert_eq!(buf[0], 0);
}

#[test]
fn error_slot_roundtrip() {
    unsafe { set_last_error(DWG_EREAD, "bad read") };
    let mut buf = [0u8; 32];
    let mut needed = 0usize;
    let code = unsafe { dwg_last_error_code() };
    let rc = unsafe { dwg_last_error(buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(code, DWG_EREAD);
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"bad read");

    // Querying the message keeps the slot (LibreDWG-style: the message stays
    // until the next operation overwrites it).
    let code = unsafe { dwg_last_error_code() };
    assert_eq!(code, DWG_EREAD);
    unsafe { clear_last_error() };
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
}

#[test]
fn error_helpers_validate_arguments() {
    // NULL `needed` is invalid.
    let rc = unsafe { dwg_last_error(std::ptr::null_mut(), 0, std::ptr::null_mut()) };
    assert_eq!(rc, DWG_EINVAL);

    // `cap == 0` with a non-NULL buffer is invalid.
    let mut byte = 0u8;
    let mut needed = 0usize;
    let rc = unsafe { dwg_last_error(&mut byte as *mut u8 as *mut _, 0, &mut needed) };
    assert_eq!(rc, DWG_EINVAL);

    // Length-only query: NULL buffer with cap 0 is legal and reports the
    // required size.
    unsafe { set_last_error(DWG_EWRITE, "disk full") };
    let rc = unsafe { dwg_last_error(std::ptr::null_mut(), 0, &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, "disk full".len() + 1);

    // A too-small buffer reports DWG_EBUFFER but still writes `needed`.
    let mut small = [0u8; 4];
    let rc = unsafe { dwg_last_error(small.as_mut_ptr() as *mut _, small.len(), &mut needed) };
    assert_eq!(rc, DWG_EBUFFER);
    assert_eq!(needed, "disk full".len() + 1);
}

#[test]
fn error_codes_match_c_header() {
    assert_eq!(DWG_OK, 0);
    assert_eq!(DWG_EINVAL, 1);
    assert_eq!(DWG_ENOMEM, 2);
    assert_eq!(DWG_EREAD, 3);
    assert_eq!(DWG_EWRITE, 4);
    assert_eq!(DWG_EVERSION, 5);
    assert_eq!(DWG_ENOTFOUND, 6);
    assert_eq!(DWG_EBUFFER, 7);
    assert_eq!(DWG_ETYPE, 8);
}

#[test]
fn data_create_destroy_and_version() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    let rc = unsafe { dwg_data_create(&mut data) };
    assert_eq!(rc, DWG_OK);
    assert!(!data.is_null());

    // Fresh documents default to the newest supported version (AC1032).
    let expected = acadrust::CadDocument::new().version.as_str();
    let mut buf = vec![0u8; expected.len() + 1];
    let mut needed = 0usize;
    let rc = unsafe { dwg_data_version(data, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, expected.len() + 1);
    assert_eq!(&buf[..needed - 1], expected.as_bytes());

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn data_helpers_validate_arguments() {
    // NULL out pointer.
    assert_eq!(unsafe { dwg_data_create(std::ptr::null_mut()) }, DWG_EINVAL);

    // NULL data handle for version query.
    let mut buf = [0u8; 16];
    let mut needed = 0usize;
    let rc = unsafe { dwg_data_version(
        std::ptr::null(),
        buf.as_mut_ptr() as *mut _,
        buf.len(),
        &mut needed,
    ) };
    assert_eq!(rc, DWG_EINVAL);

    // Length-only query reports the version size without a buffer.
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let rc = unsafe { dwg_data_version(data, std::ptr::null_mut(), 0, &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, acadrust::CadDocument::new().version.as_str().len() + 1);
    unsafe { dwg_data_destroy(data) };

    // Destroying NULL is a no-op.
    unsafe { dwg_data_destroy(std::ptr::null_mut()) };
}

#[test]
fn dwg_roundtrip_file() {
    let dir = ffi_temp_dir();
    let src = dir.join("src.dwg");
    let dst = dir.join("dst.dwg");
    write_dwg_fixture(&src);

    let src_c = CString::new(src.to_str().unwrap()).unwrap();
    let dst_c = CString::new(dst.to_str().unwrap()).unwrap();

    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let rc = unsafe { dwg_read_file(src_c.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "read must succeed: {}", last_error_text());

    // Version follows the file, not the empty-document default.
    let mut buf = [0u8; 16];
    let mut needed = 0usize;
    let rc = unsafe { dwg_data_version(data, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"AC1032");

    let rc = unsafe { dwg_write_file(dst_c.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "write must succeed: {}", last_error_text());
    unsafe { dwg_data_destroy(data) };

    // Verify the FFI output is a valid DWG the library can re-read.
    let mut reader = DwgReader::from_file(&dst).unwrap();
    let doc = reader.read().unwrap();
    assert_eq!(doc.entities().count(), 1);
    let _ = std::fs::remove_file(&dst);
}

#[test]
fn dxf_roundtrip_file() {
    let dir = ffi_temp_dir();
    let src = dir.join("src.dxf");
    let dst = dir.join("dst.dxf");

    // Library-generated DXF fixture.
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let line = acadrust::entities::Line::from_coords(1.0, 2.0, 0.0, 3.0, 4.0, 0.0);
    doc.add_entity(EntityType::Line(line)).unwrap();
    acadrust::DxfWriter::new(&doc).write_to_file(&src).unwrap();

    let src_c = CString::new(src.to_str().unwrap()).unwrap();
    let dst_c = CString::new(dst.to_str().unwrap()).unwrap();

    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let rc = unsafe { dxf_read_file(src_c.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "dxf read must succeed: {}", last_error_text());
    let rc = unsafe { dxf_write_file(dst_c.as_ptr(), data) };
    assert_eq!(rc, DWG_OK, "dxf write must succeed: {}", last_error_text());
    unsafe { dwg_data_destroy(data) };

    let doc = acadrust::DxfReader::from_file(&dst).unwrap().read().unwrap();
    assert_eq!(doc.entities().count(), 1);
    let _ = std::fs::remove_file(&dst);
}

#[test]
fn io_functions_validate_arguments() {
    let null_c = std::ptr::null();
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    // NULL path.
    assert_eq!(unsafe { dwg_read_file(null_c, data) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_write_file(null_c, data) }, DWG_EINVAL);
    assert_eq!(unsafe { dxf_read_file(null_c, data) }, DWG_EINVAL);
    assert_eq!(unsafe { dxf_write_file(null_c, data) }, DWG_EINVAL);

    // NULL data handle.
    let path_c = CString::new("x.dwg").unwrap();
    assert_eq!(unsafe { dwg_read_file(path_c.as_ptr(), std::ptr::null_mut()) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_write_file(path_c.as_ptr(), std::ptr::null()) }, DWG_EINVAL);

    // Missing file surfaces DWG_EREAD with a message.
    let missing = CString::new(ffi_temp_dir().join("does_not_exist.dwg").to_str().unwrap()).unwrap();
    let rc = unsafe { dwg_read_file(missing.as_ptr(), data) };
    assert_eq!(rc, DWG_EREAD);
    let code = unsafe { dwg_last_error_code() };
    assert_eq!(code, DWG_EREAD);

    unsafe { dwg_data_destroy(data) };
}

fn last_error_text() -> String {
    let mut buf = [0u8; 256];
    let mut needed = 0usize;
    let rc = unsafe { dwg_last_error(buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    if rc == DWG_OK {
        let n = needed.saturating_sub(1).min(buf.len());
        String::from_utf8_lossy(&buf[..n]).into_owned()
    } else {
        format!("error code {rc}")
    }
}
