//! ABI-level tests calling the exported extern "C" symbols directly.

use acadrust::ffi::*;
use std::ffi::CString;

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
fn error_code_is_zero_by_default() {
    let code = unsafe { dwg_last_error_code() };
    assert_eq!(code, DWG_OK);
}

#[test]
fn version_and_parse_symbols_exported() {
    unsafe {
        let ptr = dwg_version();
        assert!(!ptr.is_null());
        let version = std::ffi::CStr::from_ptr(ptr).to_str().unwrap();
        assert!(version.starts_with("acadrust-0.4.1"));

        let s = std::ffi::CString::new("AC1032").unwrap();
        assert_eq!(dwg_parse_version(s.as_ptr()), 8);
    }
}

#[test]
fn data_create_destroy_and_version() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    let rc = unsafe { dwg_data_create(&mut data) };
    assert_eq!(rc, DWG_OK);
    assert!(!data.is_null());

    let mut buf = [0u8; 16];
    let mut needed = 0usize;
    let rc = unsafe { dwg_data_version(data, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"AC1032");

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn data_create_null_out_returns_einval() {
    let rc = unsafe { dwg_data_create(std::ptr::null_mut()) };
    assert_eq!(rc, DWG_EINVAL);
}

#[test]
fn dwg_roundtrip_file() {
    let dir = std::env::temp_dir().join("acadrust_ffi_p1");
    std::fs::create_dir_all(&dir).unwrap();
    let src = CString::new(dir.join("src.dwg").to_str().unwrap()).unwrap();
    let dst = CString::new(dir.join("dst.dwg").to_str().unwrap()).unwrap();
    let _ = std::fs::remove_file(&dir.join("src.dwg"));
    let _ = std::fs::remove_file(&dir.join("dst.dwg"));

    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    // 先写入一个空文档生成 src.dwg
    assert_eq!(unsafe { dwg_write_file(src.as_ptr(), data) }, DWG_OK);
    // 读回
    assert_eq!(unsafe { dwg_read_file(src.as_ptr(), data) }, DWG_OK);
    // 写到 dst.dwg
    assert_eq!(unsafe { dwg_write_file(dst.as_ptr(), data) }, DWG_OK);
    // 再读回 dst.dwg 确认无错误
    assert_eq!(unsafe { dwg_read_file(dst.as_ptr(), data) }, DWG_OK);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn read_missing_file_reports_error() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let missing = CString::new("Z:\\definitely_missing_file_xyz.dwg").unwrap();
    let rc = unsafe { dwg_read_file(missing.as_ptr(), data) };
    assert_eq!(rc, DWG_EREAD);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_EREAD);
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn c_smoke_compiles_links_and_runs() {
    // Locate a C compiler. Uses $CC if set, otherwise "gcc" on PATH (MinGW).
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
    if !dll.exists() {
        // cdylib not built for this configuration; nothing to link against.
        return;
    }
    let out_dir = std::env::temp_dir().join("acadrust_c_smoke");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_smoke.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_smoke.c");

    let status = std::process::Command::new(&cc)
        .arg(src.as_os_str())
        .arg(dll.as_os_str())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to launch C compiler");
    assert!(status.success(), "C compile/link failed");

    let out = std::process::Command::new(&exe)
        .output()
        .expect("failed to run c_smoke");
    assert!(
        out.status.success(),
        "c_smoke exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}
