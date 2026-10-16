//! ABI-level tests calling the exported extern "C" symbols directly.

use acadrust::ffi::*;
use acadrust::objects::ObjectType;
use acadrust::objects::{XRecordEntry, XRecordValue};
use acadrust::types::Handle;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

static SAMPLE_DWG_COUNTER: AtomicUsize = AtomicUsize::new(0);
static NOTIFICATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static NOTIFICATION_LAST_CODE: AtomicI32 = AtomicI32::new(0);
static NOTIFICATION_LAST_USER: AtomicUsize = AtomicUsize::new(0);
static NOTIFICATION_LAST_MESSAGE_LEN: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn record_notification(code: i32, message: *const i8, user_data: *mut c_void) {
    NOTIFICATION_COUNT.fetch_add(1, Ordering::Relaxed);
    NOTIFICATION_LAST_CODE.store(code, Ordering::Relaxed);
    NOTIFICATION_LAST_USER.store(user_data as usize, Ordering::Relaxed);
    let length = if message.is_null() {
        0
    } else {
        unsafe { std::ffi::CStr::from_ptr(message) }
            .to_bytes()
            .len()
    };
    NOTIFICATION_LAST_MESSAGE_LEN.store(length, Ordering::Relaxed);
}

/// Locate the cdylib the C smoke tests link against. Preference order:
/// 1. `ACADRUST_DLL` env var, 2. GNU release build, 3. any debug fallback.
fn ffi_dll() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("ACADRUST_DLL") {
        let p = std::path::PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("target/x86_64-pc-windows-gnu/release/acadrust.dll"),
        format!("target/x86_64-pc-windows-gnu/debug/acadrust.dll"),
        format!("target/release/acadrust.dll"),
        format!("target/debug/acadrust.dll"),
    ];
    candidates
        .iter()
        .map(|c| std::path::Path::new(manifest).join(c))
        .find(|p| p.exists())
}

fn c_runtime_command(exe: &std::path::Path, dll: &std::path::Path) -> std::process::Command {
    let dll_dir = dll.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut search_path = vec![dll_dir.as_os_str().to_os_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        search_path.push(existing);
    }
    let mut command = std::process::Command::new(exe);
    command.env("PATH", std::env::join_paths(search_path).unwrap());
    command
}

/// A uniquely named sample DWG and its uniquely named parent directory.
struct SampleDwgFixture {
    path: std::path::PathBuf,
    c_path: CString,
    dir: std::path::PathBuf,
}

impl SampleDwgFixture {
    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn c_path(&self) -> &std::ffi::CStr {
        &self.c_path
    }
}

impl Drop for SampleDwgFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.dir);
    }
}

/// Build a temp DWG containing Line, Circle and Text via the Rust API.
fn write_sample_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{Circle, Line, Text};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.dwg");

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 2.0, 3.0,
    )))
    .unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(
        Vector3::new(5.0, 6.0, 0.0),
        2.5,
    )))
    .unwrap();
    let mut t = Text::new();
    t.value = "hello".to_string();
    t.height = 1.0;
    doc.add_entity(EntityType::Text(t)).unwrap();
    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn sample_dwg_paths_are_unique_per_invocation() {
    let first = write_sample_dwg("acadrust_ffi_unique_fixture");
    let second = write_sample_dwg("acadrust_ffi_unique_fixture");
    assert_ne!(first.path(), second.path());
}

#[test]
fn sample_dwg_fixture_cleanup_removes_its_parent_directory() {
    let fixture = write_sample_dwg("acadrust_ffi_cleanup_fixture");
    let parent = fixture.path().parent().unwrap().to_path_buf();

    drop(fixture);

    assert!(!parent.exists());
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
        assert!(version.starts_with(concat!("acadrust-", env!("CARGO_PKG_VERSION"))));

        let s = std::ffi::CString::new("AC1032").unwrap();
        assert_eq!(
            dwg_parse_version(s.as_ptr()),
            acadrust::types::DxfVersion::AC1032 as i32
        );
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

    // 鍏堝啓鍏ヤ竴涓┖鏂囨。鐢熸垚 src.dwg
    assert_eq!(unsafe { dwg_write_file(src.as_ptr(), data) }, DWG_OK);
    // 璇诲洖
    assert_eq!(unsafe { dwg_read_file(src.as_ptr(), data) }, DWG_OK);
    // 鍐欏埌 dst.dwg
    assert_eq!(unsafe { dwg_write_file(dst.as_ptr(), data) }, DWG_OK);
    // 鍐嶈鍥?dst.dwg 纭鏃犻敊璇?
    assert_eq!(unsafe { dwg_read_file(dst.as_ptr(), data) }, DWG_OK);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn read_missing_file_reports_error() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let dir = std::env::temp_dir().join("acadrust_ffi_p1");
    std::fs::create_dir_all(&dir).unwrap();
    let missing_path = dir.join("definitely_missing_file_xyz.dwg");
    let _ = std::fs::remove_file(&missing_path);
    let missing = CString::new(missing_path.to_str().unwrap()).unwrap();
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
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        // cdylib not built for this configuration; nothing to link against.
        eprintln!("SKIP: acadrust.dll not found; skipping C smoke test");
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

    let out = c_runtime_command(&exe, &dll)
        .output()
        .expect("failed to run c_smoke");
    assert!(
        out.status.success(),
        "c_smoke exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn c_object_remove_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_sample_dwg("acadrust_ffi_p4c1_c");
    let out_dir = std::env::temp_dir().join("acadrust_c_object_remove");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_object_remove.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_object_remove.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C object-remove compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }

        let run = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_object_remove: {error}"))?;
        if !run.status.success() {
            return Err(format!(
                "c_object_remove exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                run.status.code(),
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_notification_callback_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_p4c2_c_{}_{}",
        std::process::id(),
        SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let fixture = dir.join("notification.dxf");
    std::fs::write(
        &fixture,
        b"0\nSECTION\n2\nTHUMBNAILIMAGE\n0\nENDSEC\n0\nEOF\n",
    )
    .unwrap();
    let out_dir = std::env::temp_dir().join("acadrust_c_notification");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_notification.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_notification.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C notification compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }
        let run = c_runtime_command(&exe, &dll)
            .arg(fixture.to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_notification: {error}"))?;
        if !run.status.success() {
            return Err(format!(
                "c_notification exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                run.status.code(),
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(&fixture);
    let _ = std::fs::remove_dir(&dir);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_acis_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_phase4c3_dwg();
    let out_dir = std::env::temp_dir().join("acadrust_c_acis");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_acis.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_acis.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C ACIS compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }
        let run = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_acis: {error}"))?;
        if !run.status.success() {
            return Err(format!(
                "c_acis exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                run.status.code(),
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_bytes_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        eprintln!("SKIP: acadrust.dll not found; skipping C bytes test");
        return;
    }
    let out_dir = std::env::temp_dir().join("acadrust_c_bytes");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_bytes.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_bytes.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .status()
            .map_err(|e| format!("failed to launch C compiler: {e}"))?;
        if !compile.success() {
            return Err("C bytes compile/link failed".to_string());
        }

        let run = c_runtime_command(&exe, &dll)
            .output()
            .map_err(|e| format!("failed to run c_bytes: {e}"))?;
        if !run.status.success() {
            return Err(format!(
                "c_bytes exited with {:?}: {}",
                run.status.code(),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&exe);
    result.unwrap();
}

#[test]
fn c_dictionary_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let out_dir = std::env::temp_dir().join("acadrust_c_dictionary");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_dictionary.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_dictionary.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C dictionary compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }

        let run = c_runtime_command(&exe, &dll)
            .output()
            .map_err(|error| format!("failed to run c_dictionary: {error}"))?;
        if !run.status.success() {
            return Err(format!(
                "c_dictionary exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                run.status.code(),
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

fn write_xrecord_smoke_dwg(dir_name: &str, foreign: bool) -> SampleDwgFixture {
    use acadrust::entities::Line;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("xrecord-smoke.dwg");

    let mut doc = CadDocument::new();
    if foreign {
        doc.add_entity(EntityType::Line(Line::from_coords(
            10.0, 20.0, 0.0, 11.0, 21.0, 0.0,
        )))
        .unwrap();
    }
    let owner = doc.header.named_objects_dict_handle;
    let name = if foreign { "P4B2_FOREIGN" } else { "P4B2_TEST" };
    let xrecord = doc.ensure_xrecord(owner, name);
    let record = doc.xrecord_mut(owner, name).unwrap();
    record.entries = vec![
        XRecordEntry::string(1, "P4B2 string"),
        XRecordEntry::double(40, 12.5),
        XRecordEntry::int32(90, 123_456),
        XRecordEntry::new(10, XRecordValue::Point3D(1.0, 2.0, 3.0)),
        XRecordEntry::new(310, XRecordValue::Chunk(vec![0xCA, 0xFE, 0x01])),
    ];
    record.entries_complete = true;
    let Some(ObjectType::Dictionary(dictionary)) = doc.objects.get_mut(&owner) else {
        panic!("named objects dictionary is missing");
    };
    if let Some((_, handle)) = dictionary
        .entries
        .iter_mut()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
    {
        *handle = xrecord;
    } else {
        dictionary.entries.push((name.to_string(), xrecord));
    }

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn c_xrecord_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_xrecord_smoke_dwg("acadrust_c_xrecord", false);
    let foreign_fixture = write_xrecord_smoke_dwg("acadrust_c_xrecord_foreign", true);
    let out_dir = fixture.path().parent().unwrap();
    let exe = out_dir.join("c_xrecord.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_xrecord.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler {cc}: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C xrecord compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }

        let output = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .arg(foreign_fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_xrecord: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "c_xrecord exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_xdata_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let (fixture, entity, object) = write_phase4b3_xdata_dwg(
        "acadrust_c_xdata",
        acadrust::types::DxfVersion::AC1032,
        0,
        false,
    );
    let (foreign_fixture, _foreign_entity, _foreign_object) = write_phase4b3_xdata_dwg(
        "acadrust_c_xdata_foreign",
        acadrust::types::DxfVersion::AC1032,
        2,
        true,
    );
    let out_dir = fixture.path().parent().unwrap();
    let exe = out_dir.join("c_xdata.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_xdata.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-std=c11")
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler {cc}: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C xdata compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }

        let output = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .arg(foreign_fixture.c_path().to_str().unwrap())
            .arg(format!("0x{entity:X}"))
            .arg(format!("0x{object:X}"))
            .output()
            .map_err(|error| format!("failed to run c_xdata: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "c_xdata exited with {:?} (source entity {:X})\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                entity,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_iterate_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        eprintln!("SKIP: acadrust.dll not found; skipping C iterate test");
        return;
    }
    let out_dir = std::env::temp_dir().join("acadrust_c_iterate");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_iterate.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_iterate.c");

    let status = std::process::Command::new(&cc)
        .arg(src.as_os_str())
        .arg(dll.as_os_str())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to launch C compiler");
    assert!(status.success(), "C compile/link failed");

    let out = c_runtime_command(&exe, &dll)
        .output()
        .expect("failed to run c_iterate");
    assert!(
        out.status.success(),
        "c_iterate exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn c_table_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        eprintln!("SKIP: acadrust.dll not found; skipping C table test");
        return;
    }
    let out_dir = std::env::temp_dir().join("acadrust_c_table");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_table.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_table.c");

    let status = std::process::Command::new(&cc)
        .arg(src.as_os_str())
        .arg(dll.as_os_str())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to launch C compiler");
    assert!(status.success(), "C compile/link failed");

    let out = c_runtime_command(&exe, &dll)
        .output()
        .expect("failed to run c_table");
    assert!(
        out.status.success(),
        "c_table exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn c_entity_batch1_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        eprintln!("SKIP: acadrust.dll not found; skipping C entity batch1 test");
        return;
    }
    let out_dir = std::env::temp_dir().join("acadrust_c_entity_b1");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_entity_batch1.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_entity_batch1.c");

    let status = std::process::Command::new(&cc)
        .arg(src.as_os_str())
        .arg(dll.as_os_str())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to launch C compiler");
    assert!(status.success(), "C compile/link failed");

    let out = c_runtime_command(&exe, &dll)
        .output()
        .expect("failed to run c_entity_batch1");
    assert!(
        out.status.success(),
        "c_entity_batch1 exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn c_entity_set_basic_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_sample_dwg("acadrust_c_entity_set_basic");
    let out_dir = fixture.path().parent().unwrap();
    let exe = out_dir.join("c_entity_set_basic.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_entity_set_basic.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler {cc}: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }
        eprintln!(
            "C basic entity setter compiled and linked: compiler={cc}, exe={}",
            exe.display()
        );

        let output = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_entity_set_basic: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "c_entity_set_basic exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        eprintln!(
            "C basic entity setter executable ran successfully: {}",
            exe.display()
        );
        Ok(())
    })();

    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

fn write_fixed_setter_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{Dimension, DimensionLinear, Insert, RasterImage, Text};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fixed-setters.dwg");

    let mut doc = CadDocument::new();

    let mut text = Text::new();
    text.value = "before".to_string();
    text.height = 1.0;
    doc.add_entity(EntityType::Text(text)).unwrap();

    let insert = Insert::new("WALL", Vector3::new(1.0, 2.0, 3.0));
    doc.add_entity(EntityType::Insert(insert)).unwrap();

    let dimension = Dimension::Linear(DimensionLinear::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
    ));
    doc.add_entity(EntityType::Dimension(Box::new(dimension))).unwrap();

    let image = RasterImage::new("img.png", Vector3::new(1.0, 1.0, 0.0), 320.0, 240.0);
    doc.add_entity(EntityType::RasterImage(image)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn c_entity_set_fixed_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_fixed_setter_dwg("acadrust_c_entity_set_fixed");
    let out_dir = fixture.path().parent().unwrap();
    let exe = out_dir.join("c_entity_set_fixed.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_entity_set_fixed.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler {cc}: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }
        eprintln!(
            "C fixed entity setter compiled and linked: compiler={cc}, exe={}",
            exe.display()
        );

        let output = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_entity_set_fixed: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "c_entity_set_fixed exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        eprintln!(
            "C fixed entity setter executable ran successfully: {}",
            exe.display()
        );
        Ok(())
    })();

    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

#[test]
fn c_entity_set_collections_compiles_links_and_runs() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll()
        .expect("acadrust.dll not found; build the release ffi cdylib before running this test");
    let fixture = write_collection_setter_dwg("acadrust_c_entity_set_collections");
    let out_dir = fixture.path().parent().unwrap();
    let exe = out_dir.join("c_entity_set_collections.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_entity_set_collections.c");

    let result = (|| -> Result<(), String> {
        let compile = std::process::Command::new(&cc)
            .arg(src.as_os_str())
            .arg(dll.as_os_str())
            .arg("-o")
            .arg(&exe)
            .output()
            .map_err(|error| format!("failed to launch C compiler {cc}: {error}"))?;
        if !compile.status.success() {
            return Err(format!(
                "C compile/link failed with {:?}\nstdout:\n{}\nstderr:\n{}",
                compile.status.code(),
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            ));
        }
        let output = c_runtime_command(&exe, &dll)
            .arg(fixture.c_path().to_str().unwrap())
            .output()
            .map_err(|error| format!("failed to run c_entity_set_collections: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "c_entity_set_collections exited with {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&exe);
    result.unwrap_or_else(|message| panic!("{message}"));
}

fn write_entities_fixture() -> (CString, std::path::PathBuf) {
    use acadrust::entities::{
        Circle, Ellipse, Hatch, HatchPattern, Line, LwPolyline, LwVertex, MText,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join("acadrust_ffi_entities_fixture");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("entities.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 10.0, 20.0, 30.0,
    )))
    .unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(
        Vector3::new(5.0, 6.0, 0.0),
        2.5,
    )))
    .unwrap();
    let mut e = Ellipse::new();
    e.center = Vector3::new(1.0, 2.0, 0.0);
    e.major_axis = Vector3::new(5.0, 0.0, 0.0);
    e.minor_axis_ratio = 0.5;
    doc.add_entity(EntityType::Ellipse(e)).unwrap();
    let mut m = MText::new();
    m.value = "mt".to_string();
    doc.add_entity(EntityType::MText(Box::new(m))).unwrap();
    let mut lw = LwPolyline::new();
    lw.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lw.add_vertex(LwVertex::from_coords(10.0, 5.0));
    doc.add_entity(EntityType::LwPolyline(lw)).unwrap();
    let mut h = Hatch::new();
    h.pattern = HatchPattern::new("SOLID");
    doc.add_entity(EntityType::Hatch(Box::new(h))).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
}

#[test]
fn c_entities_value_smoke() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let manifest = env!("CARGO_MANIFEST_DIR");
    let dll = ffi_dll().unwrap_or_default();
    if !dll.exists() {
        eprintln!("SKIP: acadrust.dll not found; skipping C entities test");
        return;
    }
    let (fixture, fixture_path) = write_entities_fixture();

    let out_dir = std::env::temp_dir().join("acadrust_c_entities");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("c_entities.exe");
    let src = std::path::Path::new(manifest).join("tests/ffi/c_entities.c");

    let status = std::process::Command::new(&cc)
        .arg(src.as_os_str())
        .arg(dll.as_os_str())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to launch C compiler");
    assert!(status.success(), "C compile/link failed");

    let out = c_runtime_command(&exe, &dll)
        .arg(fixture.as_c_str().to_str().unwrap())
        .output()
        .expect("failed to run c_entities");
    assert!(
        out.status.success(),
        "c_entities exited with {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&fixture_path);
}

#[test]
fn iterate_sample_document_via_abi() {
    let fixture = write_sample_dwg("acadrust_ffi_p2_iter");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let mut err = 0;
    let mut obj = unsafe { dwg_data_first_object(data, &mut err) };
    assert_eq!(err, DWG_OK);
    assert!(!obj.is_null());

    let mut seen = Vec::new();
    while !obj.is_null() {
        seen.push(unsafe { dwg_object_get_type(obj) });
        let handle = unsafe { dwg_object_get_handle(obj) };
        assert!(handle != 0, "objects should carry a handle");
        obj = unsafe { dwg_object_next(obj) };
    }
    assert_eq!(
        &seen[..3],
        &[
            acadrust::ffi::DwgObjectType::Line as i32,
            acadrust::ffi::DwgObjectType::Circle as i32,
            acadrust::ffi::DwgObjectType::Text as i32,
        ]
    );

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn fresh_document_object_graph_has_default_objects() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let mut err = 0;
    let first = unsafe { dwg_data_first_object(data, &mut err) };
    assert_eq!(err, DWG_OK);
    // CadDocument::new() installs default objects (dictionaries, styles...),
    // so the object graph is never empty; the first object is non-entity.
    assert!(!first.is_null());
    assert_eq!(
        unsafe { dwg_object_get_type(first) },
        acadrust::ffi::DwgObjectType::Other as i32
    );
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase4c1_object_remove_invalidates_only_the_removed_object() {
    let (data, _fixture) = open_sample_document();
    let mut error = 0;
    let line = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    let circle = unsafe { dwg_object_next(line) };
    assert!(!line.is_null());
    assert!(!circle.is_null());
    let circle_handle = unsafe { dwg_object_get_handle(circle) };

    assert_eq!(unsafe { dwg_object_remove(data, line) }, DWG_OK);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(
        unsafe { dwg_object_get_type(line) },
        DwgObjectType::Unknown as i32
    );
    assert_eq!(unsafe { dwg_object_get_handle(line) }, 0);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_line_get_start(line, &mut x, &mut y, &mut z) },
        DWG_ENOTFOUND
    );

    let first_after = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    assert_eq!(first_after, circle);
    assert_eq!(unsafe { dwg_object_get_handle(circle) }, circle_handle);

    assert_eq!(unsafe { dwg_object_remove(data, line) }, DWG_ENOTFOUND);
    assert_eq!(
        unsafe { dwg_object_remove(data, std::ptr::null()) },
        DWG_EINVAL
    );
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase4c1_object_remove_handles_non_entity_and_foreign_objects() {
    let mut left = std::ptr::null_mut();
    let mut right = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut left) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut right) }, DWG_OK);

    let mut error = 0;
    let mut left_object = unsafe { dwg_data_first_object(left, &mut error) };
    assert_eq!(error, DWG_OK);
    assert!(!left_object.is_null());
    while unsafe { dwg_object_get_type(left_object) } != DwgObjectType::Other as i32 {
        left_object = unsafe { dwg_object_next(left_object) };
        assert!(!left_object.is_null());
    }
    let foreign = unsafe { dwg_data_first_object(right, &mut error) };
    assert_eq!(error, DWG_OK);

    assert_eq!(unsafe { dwg_object_remove(left, foreign) }, DWG_ENOTFOUND);
    assert_eq!(unsafe { dwg_object_remove(left, left_object) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_object_get_type(left_object) },
        DwgObjectType::Unknown as i32
    );

    unsafe {
        dwg_data_destroy(left);
        dwg_data_destroy(right);
    }
}

#[test]
fn phase4c2_notification_callback_receives_and_can_clear_notifications() {
    let dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_p4c2_{}_{}",
        std::process::id(),
        SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notification.dxf");
    std::fs::write(&path, b"0\nSECTION\n2\nTHUMBNAILIMAGE\n0\nENDSEC\n0\nEOF\n").unwrap();
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    let token = 0xC2_u8;
    let user_data = (&token as *const u8).cast_mut().cast::<c_void>();
    NOTIFICATION_COUNT.store(0, Ordering::Relaxed);
    NOTIFICATION_LAST_CODE.store(0, Ordering::Relaxed);
    NOTIFICATION_LAST_USER.store(0, Ordering::Relaxed);
    NOTIFICATION_LAST_MESSAGE_LEN.store(0, Ordering::Relaxed);

    let mut data = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe {
            dwg_set_notification_callback(
                std::ptr::null_mut(),
                Some(record_notification),
                user_data,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_set_notification_callback(data, Some(record_notification), user_data) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(unsafe { dxf_read_file(c_path.as_ptr(), data) }, DWG_OK);
    assert!(NOTIFICATION_COUNT.load(Ordering::Relaxed) > 0);
    assert_eq!(
        NOTIFICATION_LAST_CODE.load(Ordering::Relaxed),
        DWG_NOTIFICATION_NOT_IMPLEMENTED
    );
    assert_eq!(
        NOTIFICATION_LAST_USER.load(Ordering::Relaxed),
        user_data as usize
    );
    assert!(NOTIFICATION_LAST_MESSAGE_LEN.load(Ordering::Relaxed) > 0);

    NOTIFICATION_COUNT.store(0, Ordering::Relaxed);
    assert_eq!(
        unsafe { dwg_set_notification_callback(data, None, std::ptr::null_mut()) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(unsafe { dxf_read_file(c_path.as_ptr(), data) }, DWG_OK);
    assert_eq!(NOTIFICATION_COUNT.load(Ordering::Relaxed), 0);

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(dir);
}

const PHASE4C3_SAT: &str = "700 0 1 0\n\
 @7 unknown 12 ACIS 7.0 NT 24 Wed Jan 01 00:00:00 2025 1.0 9.9999999999999995e-007 1e-010\n\
 body $-1 $1 $-1 $-1 #\n\
 lump $-1 $-1 $2 $0 #\n\
 shell $-1 $-1 $-1 $3 $-1 $1 #\n\
 face $-1 $-1 $-1 $4 $2 $5 forward single #\n\
 loop $-1 $-1 $6 $3 #\n\
 plane-surface $-1 0 0 5 0 0 1 1 0 0 forward_v I I I I #\n\
 coedge $-1 $6 $6 $-1 $7 forward $4 $-1 #\n\
 edge $-1 $8 0 $8 1 $6 $9 forward #\n\
 vertex $-1 $7 $10 #\n\
 straight-curve $-1 -5 -5 5 1 0 0 I I #\n\
 point $-1 -5 -5 5 #\n";

fn phase4c3_data() -> *mut DwgData {
    use acadrust::entities::{AcisData, Body, EntityType, Region, Solid3D, Surface, SurfaceKind};
    use acadrust::CadDocument;

    let mut data = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Solid3D(Solid3D::from_sat(PHASE4C3_SAT)))
        .unwrap();
    doc.add_entity(EntityType::Region(Region::from_sat(PHASE4C3_SAT)))
        .unwrap();
    doc.add_entity(EntityType::Body(Body::from_sat(PHASE4C3_SAT)))
        .unwrap();
    let mut surface = Surface::new(SurfaceKind::Plane);
    surface.acis_data = AcisData::from_sat(PHASE4C3_SAT);
    doc.add_entity(EntityType::Surface(Box::new(surface)))
        .unwrap();
    unsafe {
        *(data as *mut FfiDocument) = FfiDocument::new(doc);
    }
    data
}

fn write_phase4c3_dwg() -> SampleDwgFixture {
    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_p4c3_{}_{}",
        std::process::id(),
        suffix
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("acis.dwg");
    let data = phase4c3_data();
    let c_path = CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { dwg_write_file(c_path.as_ptr(), data) }, DWG_OK);
    unsafe { dwg_data_destroy(data) };
    SampleDwgFixture { path, c_path, dir }
}

#[test]
fn phase4c3_acis_sat_sab_access_and_roundtrip() {
    let data = phase4c3_data();
    let mut error = 0;
    let solid = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    assert_eq!(
        unsafe { dwg_object_get_type(solid) },
        DwgObjectType::Solid3D as i32
    );
    let region = unsafe { dwg_object_next(solid) };
    let body = unsafe { dwg_object_next(region) };
    let surface = unsafe { dwg_object_next(body) };
    assert_eq!(
        unsafe { dwg_object_get_type(region) },
        DwgObjectType::Region as i32
    );
    assert_eq!(
        unsafe { dwg_object_get_type(body) },
        DwgObjectType::Body as i32
    );
    assert_eq!(
        unsafe { dwg_object_get_type(surface) },
        DwgObjectType::Surface as i32
    );

    let mut binary = -1;
    assert_eq!(
        unsafe { dwg_entity_acis_is_binary(solid, &mut binary) },
        DWG_OK
    );
    assert_eq!(binary, 0);
    let mut needed = 0usize;
    assert_eq!(
        unsafe { dwg_entity_acis_get_sat(solid, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    assert_eq!(needed, PHASE4C3_SAT.len() + 1);
    let mut sat = vec![0_i8; needed];
    assert_eq!(
        unsafe { dwg_entity_acis_get_sat(solid, sat.as_mut_ptr(), sat.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(sat.as_ptr()) }
            .to_str()
            .unwrap(),
        PHASE4C3_SAT
    );
    assert_eq!(
        unsafe { dwg_entity_acis_get_sab(solid, std::ptr::null_mut(), 0, &mut needed) },
        DWG_ETYPE
    );

    let replacement = CString::new("700 0 1 0\nreplacement\n").unwrap();
    assert_eq!(
        unsafe { dwg_entity_acis_set_sat(data, solid, replacement.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_acis_is_binary(solid, &mut binary) },
        DWG_OK
    );
    assert_eq!(binary, 0);
    assert_eq!(
        unsafe { dwg_entity_acis_get_sat(solid, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    assert_eq!(needed, replacement.as_bytes().len() + 1);

    let sab = [0x01_u8, 0x02, 0xA5];
    assert_eq!(
        unsafe { dwg_entity_acis_set_sab(data, region, sab.as_ptr(), sab.len()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_acis_is_binary(region, &mut binary) },
        DWG_OK
    );
    assert_eq!(binary, 1);
    assert_eq!(
        unsafe { dwg_entity_acis_get_sat(region, std::ptr::null_mut(), 0, &mut needed) },
        DWG_ETYPE
    );
    assert_eq!(
        unsafe { dwg_entity_acis_get_sab(region, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    assert_eq!(needed, sab.len());
    let mut sab_read = [0_u8; 3];
    assert_eq!(
        unsafe {
            dwg_entity_acis_get_sab(region, sab_read.as_mut_ptr(), sab_read.len(), &mut needed)
        },
        DWG_OK
    );
    assert_eq!(sab_read, sab);

    assert_eq!(
        unsafe { dwg_entity_acis_set_sat(data, body, replacement.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_acis_set_sat(data, surface, replacement.as_ptr()) },
        DWG_OK
    );
    let mut ordinary = unsafe { dwg_object_next(surface) };
    while !ordinary.is_null()
        && unsafe { dwg_object_get_type(ordinary) } != DwgObjectType::Other as i32
    {
        ordinary = unsafe { dwg_object_next(ordinary) };
    }
    assert!(!ordinary.is_null());
    assert_eq!(
        unsafe { dwg_entity_acis_is_binary(ordinary, &mut binary) },
        DWG_ETYPE
    );

    let roundtrip_data = phase4c3_data();
    let mut bytes = std::ptr::null_mut();
    let mut byte_len = 0usize;
    assert_eq!(
        unsafe { dwg_write_bytes(roundtrip_data, &mut bytes, &mut byte_len) },
        DWG_OK
    );
    let mut reopened = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_bytes(bytes, byte_len, reopened) }, DWG_OK);
    let mut reopened_solid = unsafe { dwg_data_first_object(reopened, &mut error) };
    while !reopened_solid.is_null()
        && unsafe { dwg_object_get_type(reopened_solid) } != DwgObjectType::Solid3D as i32
    {
        reopened_solid = unsafe { dwg_object_next(reopened_solid) };
    }
    assert!(!reopened_solid.is_null());
    assert_eq!(
        unsafe { dwg_entity_acis_is_binary(reopened_solid, &mut binary) },
        DWG_OK
    );
    if binary == 0 {
        assert_eq!(
            unsafe {
                dwg_entity_acis_get_sat(reopened_solid, std::ptr::null_mut(), 0, &mut needed)
            },
            DWG_OK
        );
        assert!(needed > 1);
    } else {
        assert_eq!(
            unsafe {
                dwg_entity_acis_get_sab(reopened_solid, std::ptr::null_mut(), 0, &mut needed)
            },
            DWG_OK
        );
        assert!(needed > 0);
    }

    unsafe {
        dwg_free_buffer(bytes.cast());
        dwg_data_destroy(reopened);
        dwg_data_destroy(roundtrip_data);
        dwg_data_destroy(data);
    }
}

fn open_sample_document() -> (*mut DwgData, SampleDwgFixture) {
    let fixture = write_sample_dwg("acadrust_ffi_p2_entity");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    (data, fixture)
}

#[test]
fn line_and_circle_getters_return_values() {
    let (data, _fixture) = open_sample_document();
    let mut err = 0;
    let line = unsafe { dwg_data_first_object(data, &mut err) };
    assert_eq!(err, DWG_OK);

    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    let rc = unsafe { dwg_entity_line_get_start(line, &mut x, &mut y, &mut z) };
    assert_eq!(rc, DWG_OK);
    assert!((x - 0.0).abs() < 1e-9);
    assert!((y - 0.0).abs() < 1e-9);
    assert!((z - 0.0).abs() < 1e-9);

    let rc = unsafe { dwg_entity_line_get_end(line, &mut x, &mut y, &mut z) };
    assert_eq!(rc, DWG_OK);
    assert!((x - 1.0).abs() < 1e-9);
    assert!((y - 2.0).abs() < 1e-9);
    assert!((z - 3.0).abs() < 1e-9);

    // Wrong family must report DWG_ETYPE.
    let mut r = 0.0;
    let rc = unsafe { dwg_entity_circle_get_radius(line, &mut r) };
    assert_eq!(rc, DWG_ETYPE);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn text_getter_reads_value_with_buffer_protocol() {
    let (data, _fixture) = open_sample_document();
    let mut err = 0;
    let mut obj = unsafe { dwg_data_first_object(data, &mut err) };
    // Walk to the TEXT entity.
    while !obj.is_null()
        && unsafe { dwg_object_get_type(obj) } != acadrust::ffi::DwgObjectType::Text as i32
    {
        obj = unsafe { dwg_object_next(obj) };
    }
    assert!(!obj.is_null(), "sample document must contain a TEXT entity");

    // Length query.
    let mut needed = 0usize;
    let rc = unsafe { dwg_entity_text_get_value(obj, std::ptr::null_mut(), 0, &mut needed) };
    assert_eq!(rc, DWG_OK);
    assert_eq!(needed, 6); // "hello" + NUL

    let mut buf = [0u8; 6];
    let rc = unsafe {
        dwg_entity_text_get_value(obj, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed)
    };
    assert_eq!(rc, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"hello");

    let mut h = 0.0;
    let rc = unsafe { dwg_entity_text_get_height(obj, &mut h) };
    assert_eq!(rc, DWG_OK);
    assert!((h - 1.0).abs() < 1e-9);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn layer_color_roundtrip_via_abi() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let mut count = 0;
    assert_eq!(unsafe { dwg_layer_table_count(data, &mut count) }, DWG_OK);
    assert!(count >= 1, "default document has at least layer 0");

    let color: i32 = 0x00FF0000; // red
    assert_eq!(unsafe { dwg_layer_set_color(data, 0, color) }, DWG_OK);
    let mut read = 0;
    assert_eq!(unsafe { dwg_layer_get_color(data, 0, &mut read) }, DWG_OK);
    assert_eq!(read, color);

    let name = CString::new("0").unwrap();
    let mut idx = -1;
    assert_eq!(
        unsafe { dwg_layer_index_by_name(data, name.as_ptr(), &mut idx) },
        DWG_OK
    );
    assert_eq!(idx, 0);

    let mut handle = 0u64;
    assert_eq!(
        unsafe { dwg_layer_get_handle(data, 0, &mut handle) },
        DWG_OK
    );
    assert!(handle != 0);

    // Out of range index.
    let rc = unsafe { dwg_layer_get_name(data, 9999, std::ptr::null_mut(), 0, &mut 0usize) };
    assert_eq!(rc, DWG_ENOTFOUND);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn generic_table_counts_and_names() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let mut count = 0;
    assert_eq!(
        unsafe {
            dwg_table_entry_count(data, acadrust::ffi::DwgTableKind::Layer as i32, &mut count)
        },
        DWG_OK
    );
    assert!(count >= 1);
    assert_eq!(
        unsafe {
            dwg_table_entry_count(data, acadrust::ffi::DwgTableKind::Block as i32, &mut count)
        },
        DWG_OK
    );
    assert!(count >= 2, "model + paper space block records");

    let mut buf = [0u8; 64];
    let mut needed = 0usize;
    assert_eq!(
        unsafe {
            dwg_table_entry_get_name(
                data,
                acadrust::ffi::DwgTableKind::Layer as i32,
                0,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                &mut needed,
            )
        },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"0");

    unsafe { dwg_data_destroy(data) };
}

fn write_batch1_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{
        Arc, Circle, Ellipse, Face3D, Line, LwPolyline, LwVertex, MText, Point, Ray, Solid, Spline,
        XLine,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch1.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 2.0, 3.0,
    )))
    .unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(
        Vector3::new(5.0, 6.0, 0.0),
        2.5,
    )))
    .unwrap();
    doc.add_entity(EntityType::Arc(Arc::from_center_radius_angles(
        Vector3::new(1.0, 2.0, 0.0),
        2.0,
        0.0,
        1.0,
    )))
    .unwrap();
    doc.add_entity(EntityType::Point(Point::from_coords(1.0, 2.0, 3.0)))
        .unwrap();
    let mut e = Ellipse::new();
    e.center = Vector3::new(1.0, 2.0, 0.0);
    e.major_axis = Vector3::new(5.0, 0.0, 0.0);
    e.minor_axis_ratio = 0.5;
    doc.add_entity(EntityType::Ellipse(e)).unwrap();

    let mut m = MText::new();
    m.value = "mt".to_string();
    m.height = 2.0;
    doc.add_entity(EntityType::MText(Box::new(m))).unwrap();

    let s = Spline::from_control_points(
        3,
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(3.0, 1.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
        ],
    );
    doc.add_entity(EntityType::Spline(s)).unwrap();

    let mut lw = LwPolyline::new();
    lw.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lw.add_vertex(LwVertex::from_coords(10.0, 5.0));
    doc.add_entity(EntityType::LwPolyline(lw)).unwrap();

    let ray = Ray::default();
    doc.add_entity(EntityType::Ray(ray)).unwrap();
    let xl = XLine::default();
    doc.add_entity(EntityType::XLine(xl)).unwrap();
    doc.add_entity(EntityType::Solid(Solid::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    )))
    .unwrap();
    doc.add_entity(EntityType::Face3D(Face3D::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    )))
    .unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
}

fn write_spline_collection_dwg(dir_name: &str, fit_point_scenario: bool) -> SampleDwgFixture {
    use acadrust::entities::{Line, Spline};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("spline-collections.dwg");

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
    )))
    .unwrap();
    let spline = if fit_point_scenario {
        Spline::from_fit_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 2.0, 0.0),
            Vector3::new(2.0, 1.0, 0.0),
            Vector3::new(3.0, 3.0, 0.0),
        ])
    } else {
        let mut spline = Spline::from_control_points(
            3,
            vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(2.0, 0.0, 0.0),
                Vector3::new(3.0, 1.0, 0.0),
                Vector3::new(4.0, 0.0, 0.0),
            ],
        );
        spline.flags.rational = true;
        spline.weights = vec![1.0, 1.5, 2.0, 2.5, 3.0];
        spline
    };
    doc.add_entity(EntityType::Spline(spline)).unwrap();
    DwgWriter::write_to_file(&path, &doc).unwrap();

    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

fn write_polyline_collection_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{
        Line, LwPolyline, LwVertex, PolyfaceMesh, PolyfaceVertex, Polyline2D, Polyline3D, Vertex2D,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("polyline-collections.dwg");

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
    )))
    .unwrap();

    let mut lwpolyline = LwPolyline::new();
    lwpolyline.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lwpolyline.add_vertex(LwVertex::from_coords(5.0, 6.0));
    doc.add_entity(EntityType::LwPolyline(lwpolyline)).unwrap();

    let mut polyline2d = Polyline2D::new();
    polyline2d.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
    polyline2d.add_vertex(Vertex2D::new(Vector3::new(5.0, 6.0, 7.0)));
    doc.add_entity(EntityType::Polyline2D(polyline2d)).unwrap();

    let mut polyline3d = Polyline3D::new();
    polyline3d.add_vertex(Vector3::new(0.0, 0.0, 0.0));
    polyline3d.add_vertex(Vector3::new(5.0, 6.0, 7.0));
    doc.add_entity(EntityType::Polyline3D(polyline3d)).unwrap();

    let mut polyface_mesh = PolyfaceMesh::new();
    polyface_mesh.add_vertex(PolyfaceVertex::from_xyz(0.0, 0.0, 0.0));
    polyface_mesh.add_vertex(PolyfaceVertex::from_xyz(5.0, 6.0, 7.0));
    doc.add_entity(EntityType::PolyfaceMesh(polyface_mesh))
        .unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

fn write_leader_mesh_collection_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{Leader, Line, Mesh, MeshEdge};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("leader-mesh-collections.dwg");

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Leader(Leader::from_vertices(vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    ])))
    .unwrap();
    doc.add_entity(EntityType::Line(Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 1.0, 1.0,
    )))
    .unwrap();
    let mut mesh = Mesh::new();
    mesh.vertices = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(2.0, 0.0, 0.0),
        Vector3::new(3.0, 0.0, 0.0),
    ];
    mesh.edges.push(MeshEdge::new(2, 3));
    doc.add_entity(EntityType::Mesh(mesh)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

fn write_mesh_face_collection_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{Mesh, MeshFace};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mesh-faces.dwg");
    let mut doc = CadDocument::new();
    let mut mesh = Mesh::new();
    mesh.vertices = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];
    mesh.faces.push(MeshFace::triangle(0, 1, 2));
    mesh.compute_edges();
    doc.add_entity(EntityType::Mesh(mesh)).unwrap();
    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

fn write_collection_setter_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{
        Leader, LwPolyline, LwVertex, Mesh, PolyfaceMesh, PolyfaceVertex, Polyline2D, Polyline3D,
        Spline, Vertex2D,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("collection-setters.dwg");

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Spline(Spline::from_control_points(
        3,
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(3.0, 1.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
        ],
    )))
    .unwrap();

    let mut lwpolyline = LwPolyline::new();
    lwpolyline.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lwpolyline.add_vertex(LwVertex::from_coords(1.0, 1.0));
    doc.add_entity(EntityType::LwPolyline(lwpolyline)).unwrap();

    let mut polyline2d = Polyline2D::new();
    polyline2d.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
    polyline2d.add_vertex(Vertex2D::new(Vector3::new(1.0, 1.0, 0.0)));
    doc.add_entity(EntityType::Polyline2D(polyline2d)).unwrap();

    let mut polyline3d = Polyline3D::new();
    polyline3d.add_vertex(Vector3::new(0.0, 0.0, 0.0));
    polyline3d.add_vertex(Vector3::new(1.0, 1.0, 0.0));
    doc.add_entity(EntityType::Polyline3D(polyline3d)).unwrap();

    doc.add_entity(EntityType::Leader(Leader::from_vertices(vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
    ])))
    .unwrap();

    let mesh = Mesh::from_triangles(
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ],
        &[(0, 1, 2)],
    );
    doc.add_entity(EntityType::Mesh(mesh)).unwrap();

    let mut polyface_mesh = PolyfaceMesh::new();
    polyface_mesh.add_vertex(PolyfaceVertex::from_xyz(0.0, 0.0, 0.0));
    polyface_mesh.add_vertex(PolyfaceVertex::from_xyz(1.0, 0.0, 0.0));
    doc.add_entity(EntityType::PolyfaceMesh(polyface_mesh))
        .unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

fn find_object_of_type(data: *mut DwgData, want: i32) -> *const DwgObject {
    let mut err = 0;
    let mut obj = unsafe { dwg_data_first_object(data, &mut err) };
    while !obj.is_null() {
        if unsafe { dwg_object_get_type(obj) } == want {
            return obj;
        }
        obj = unsafe { dwg_object_next(obj) };
    }
    std::ptr::null()
}

#[test]
fn common_entity_setters_roundtrip_through_getters() {
    let (data, _fixture) = open_sample_document();
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    assert!(!line.is_null());
    let layer = CString::new("P3C_LAYER").unwrap();
    let linetype = CString::new("DASHED").unwrap();

    assert_eq!(
        unsafe { dwg_entity_set_layer(data, line, layer.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_color(data, line, 0x00112233) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype(data, line, linetype.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype_scale(data, line, 2.5) },
        DWG_OK
    );

    let mut needed = 0;
    assert_eq!(
        unsafe { dwg_entity_get_layer(line, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    let mut layer_buf = vec![0_i8; needed];
    assert_eq!(
        unsafe { dwg_entity_get_layer(line, layer_buf.as_mut_ptr(), layer_buf.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(layer_buf.as_ptr()) }
            .to_str()
            .unwrap(),
        "P3C_LAYER"
    );

    assert_eq!(
        unsafe { dwg_entity_get_linetype(line, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    let mut linetype_buf = vec![0_i8; needed];
    assert_eq!(
        unsafe {
            dwg_entity_get_linetype(
                line,
                linetype_buf.as_mut_ptr(),
                linetype_buf.len(),
                &mut needed,
            )
        },
        DWG_OK
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(linetype_buf.as_ptr()) }
            .to_str()
            .unwrap(),
        "DASHED"
    );

    let mut color = 0;
    let mut scale = 0.0;
    assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_entity_get_linetype_scale(line, &mut scale) },
        DWG_OK
    );
    assert_eq!(color, 0x00112233);
    assert_eq!(scale, 2.5);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn common_entity_setters_reject_invalid_inputs_without_corruption() {
    let (data, _fixture) = open_sample_document();
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let stable_layer = CString::new("P3C_STABLE_LAYER").unwrap();
    let stable_linetype = CString::new("CONTINUOUS").unwrap();
    let empty_linetype = CString::new("").unwrap();
    assert!(!line.is_null());

    assert_eq!(
        unsafe { dwg_entity_set_linetype(data, line, empty_linetype.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_layer(data, line, stable_layer.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_color(data, line, 0x00010203) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype(data, line, stable_linetype.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype_scale(data, line, 3.0) },
        DWG_OK
    );

    assert_eq!(
        unsafe { dwg_entity_set_layer(std::ptr::null_mut(), line, stable_layer.as_ptr()) },
        DWG_EINVAL
    );
    assert_eq!(dwg_last_error_code(), DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_set_color(data, std::ptr::null(), 0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_set_layer(data, line, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_set_layer(data, line, empty_linetype.as_ptr()) },
        DWG_EINVAL
    );
    let invalid_utf8 = [0xff_u8, 0];
    assert_eq!(
        unsafe { dwg_entity_set_linetype(data, line, invalid_utf8.as_ptr().cast()) },
        DWG_EINVAL
    );
    let mut linetype_needed = 0;
    assert_eq!(
        unsafe { dwg_entity_get_linetype(line, std::ptr::null_mut(), 0, &mut linetype_needed) },
        DWG_OK
    );
    let mut linetype_buf = vec![0_i8; linetype_needed];
    assert_eq!(
        unsafe {
            dwg_entity_get_linetype(
                line,
                linetype_buf.as_mut_ptr(),
                linetype_buf.len(),
                &mut linetype_needed,
            )
        },
        DWG_OK
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(linetype_buf.as_ptr()) }
            .to_str()
            .unwrap(),
        "CONTINUOUS"
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype_scale(data, line, f64::NAN) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_set_linetype_scale(data, line, 0.0) },
        DWG_EINVAL
    );

    let (foreign_data, _foreign_fixture) = open_sample_document();
    let foreign_line = find_object_of_type(foreign_data, DwgObjectType::Line as i32);
    assert_eq!(
        unsafe { dwg_entity_set_color(data, foreign_line, 0x00ABCDEF) },
        DWG_ENOTFOUND
    );

    let mut needed = 0;
    assert_eq!(
        unsafe { dwg_entity_get_layer(line, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    let mut layer_buf = vec![0_i8; needed];
    assert_eq!(
        unsafe { dwg_entity_get_layer(line, layer_buf.as_mut_ptr(), layer_buf.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(layer_buf.as_ptr()) }
            .to_str()
            .unwrap(),
        "P3C_STABLE_LAYER"
    );
    let mut color = 0;
    let mut scale = 0.0;
    assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_entity_get_linetype_scale(line, &mut scale) },
        DWG_OK
    );
    assert_eq!(color, 0x00010203);
    assert_eq!(scale, 3.0);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase4a_dwg_bytes_roundtrip_and_error_state() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let mut output: *mut u8 = 1 as *mut u8;
    let mut output_len = usize::MAX;
    assert_eq!(
        unsafe { dwg_write_bytes(std::ptr::null(), &mut output, &mut output_len) },
        DWG_EINVAL
    );
    assert!(output.is_null());
    assert_eq!(output_len, 0);

    assert_eq!(
        unsafe { dwg_read_bytes(std::ptr::null(), 0, data) },
        DWG_EINVAL
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_EINVAL);

    assert_eq!(
        unsafe { dwg_write_bytes(data, &mut output, &mut output_len) },
        DWG_OK
    );
    assert!(!output.is_null());
    assert!(output_len > 0);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    let malformed = [0x41_u8, 0x42, 0x43, 0x44];
    assert_eq!(
        unsafe { dwg_read_bytes(malformed.as_ptr(), malformed.len(), data) },
        DWG_EREAD
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_EREAD);
    let mut error = [0u8; 128];
    let mut needed = 0usize;
    assert_eq!(
        unsafe { dwg_last_error(error.as_mut_ptr() as *mut _, error.len(), &mut needed) },
        DWG_EREAD
    );
    assert!(needed > 1);

    let mut version = [0u8; 16];
    let mut version_needed = 0usize;
    assert_eq!(
        unsafe {
            dwg_data_version(
                data,
                version.as_mut_ptr() as *mut _,
                version.len(),
                &mut version_needed,
            )
        },
        DWG_OK
    );
    assert_eq!(&version[..version_needed - 1], b"AC1032");

    let mut reopened: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_bytes(output, output_len, reopened) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    unsafe {
        dwg_free_buffer(output.cast());
        dwg_data_destroy(reopened);
        dwg_data_destroy(data);
    }
}

#[test]
fn phase4a_dwg_bytes_reject_null_output_parameters() {
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

    let mut output = std::ptr::null_mut();
    let mut output_len = 123usize;
    assert_eq!(
        unsafe { dwg_write_bytes(data, std::ptr::null_mut(), &mut output_len) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_write_bytes(data, &mut output, std::ptr::null_mut()) },
        DWG_EINVAL
    );

    unsafe { dwg_data_destroy(data) };
}

struct Phase4b1DictionaryFixture {
    data: *mut DwgData,
    root: u64,
}

impl Phase4b1DictionaryFixture {
    fn new() -> Self {
        let mut data = std::ptr::null_mut();
        assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
        let expected_root = unsafe {
            (&*(data as *const FfiDocument))
                .doc
                .header
                .named_objects_dict_handle
                .value()
        };
        let mut root = 0_u64;
        assert_eq!(
            unsafe { dwg_data_named_objects_dictionary(data, &mut root) },
            DWG_OK
        );
        assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
        assert_eq!(root, expected_root);
        assert_ne!(root, 0);
        Self { data, root }
    }
}

impl Drop for Phase4b1DictionaryFixture {
    fn drop(&mut self) {
        unsafe { dwg_data_destroy(self.data) };
    }
}

#[test]
fn phase4b1_dictionary_rejects_invalid_and_missing_arguments() {
    let fixture = Phase4b1DictionaryFixture::new();
    let key = CString::new("missing").unwrap();
    let mut value = 0_u64;
    let mut count = 0_usize;
    let mut needed = 0_usize;
    let mut key_buf = [0_i8; 8];

    assert_eq!(
        unsafe { dwg_data_named_objects_dictionary(std::ptr::null(), &mut value) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_data_named_objects_dictionary(fixture.data, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_dictionary_count(std::ptr::null(), fixture.root, &mut count) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_dictionary_count(fixture.data, fixture.root, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, std::ptr::null(), &mut value) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_get(
                fixture.data,
                fixture.root,
                key.as_ptr(),
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                key_buf.as_mut_ptr(),
                0,
                &mut needed,
                &mut value,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut value,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                std::ptr::null_mut(),
                0,
                &mut needed,
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_dictionary_set(std::ptr::null_mut(), fixture.root, key.as_ptr(), 1) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, std::ptr::null(), 1) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_remove(std::ptr::null_mut(), fixture.root, key.as_ptr(), &mut value)
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_dictionary_remove(
                fixture.data,
                fixture.root,
                key.as_ptr(),
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );

    assert_eq!(
        unsafe { dwg_dictionary_count(fixture.data, u64::MAX, &mut count) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, key.as_ptr(), &mut value) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_dictionary_remove(fixture.data, fixture.root, key.as_ptr(), &mut value) },
        DWG_ENOTFOUND
    );

    let mut err = 0;
    let mut object = unsafe { dwg_data_first_object(fixture.data, &mut err) };
    let mut non_dictionary = None;
    while !object.is_null() {
        let handle = unsafe { dwg_object_get_handle(object) };
        let result = unsafe { dwg_dictionary_count(fixture.data, handle, &mut count) };
        if result == DWG_OK {
            assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
        } else if result == DWG_ETYPE {
            non_dictionary = Some(handle);
            break;
        }
        object = unsafe { dwg_object_next(object) };
    }
    let non_dictionary = non_dictionary.expect("fresh document has a non-dictionary object");
    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, non_dictionary, key.as_ptr(), 1) },
        DWG_ETYPE
    );
}

#[test]
fn phase4b1_dictionary_lookup_enumeration_and_mutation() {
    let fixture = Phase4b1DictionaryFixture::new();
    let acad_group = CString::new("ACAD_GROUP").unwrap();
    let mut group_handle = 0_u64;
    assert_eq!(
        unsafe {
            dwg_dictionary_get(
                fixture.data,
                fixture.root,
                acad_group.as_ptr(),
                &mut group_handle,
            )
        },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let expected_group_handle = group_handle;

    let mut count = 0_usize;
    assert_eq!(
        unsafe { dwg_dictionary_count(fixture.data, fixture.root, &mut count) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert!(count > 0);
    let original_count = count;
    let mut found_group = false;
    for index in 0..count {
        let mut needed = 0_usize;
        let mut entry_value = 0_u64;
        assert_eq!(
            unsafe {
                dwg_dictionary_entry(
                    fixture.data,
                    fixture.root,
                    index,
                    std::ptr::null_mut(),
                    0,
                    &mut needed,
                    &mut entry_value,
                )
            },
            DWG_OK
        );
        assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
        let mut key = vec![0_i8; needed];
        assert_eq!(
            unsafe {
                dwg_dictionary_entry(
                    fixture.data,
                    fixture.root,
                    index,
                    key.as_mut_ptr(),
                    key.len(),
                    &mut needed,
                    &mut entry_value,
                )
            },
            DWG_OK
        );
        assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
        let key = unsafe { std::ffi::CStr::from_ptr(key.as_ptr()) }
            .to_str()
            .unwrap();
        if key.eq_ignore_ascii_case("ACAD_GROUP") {
            assert_eq!(entry_value, expected_group_handle);
            found_group = true;
        }
    }
    assert!(found_group);

    let test_key = CString::new("P4B1_TEST").unwrap();
    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, test_key.as_ptr(), group_handle) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut read = 0_u64;
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, test_key.as_ptr(), &mut read) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(read, group_handle);

    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, acad_group.as_ptr(), u64::MAX) },
        DWG_ENOTFOUND
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_ENOTFOUND);
    let mut count_after_invalid_set = 0_usize;
    assert_eq!(
        unsafe { dwg_dictionary_count(fixture.data, fixture.root, &mut count_after_invalid_set) },
        DWG_OK
    );
    assert_eq!(count_after_invalid_set, original_count + 1);
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, acad_group.as_ptr(), &mut read) },
        DWG_OK
    );
    assert_eq!(read, group_handle);

    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, test_key.as_ptr(), fixture.root) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, test_key.as_ptr(), &mut read) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(read, fixture.root);
    assert_eq!(
        unsafe { dwg_dictionary_remove(fixture.data, fixture.root, test_key.as_ptr(), &mut read) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(read, fixture.root);
    assert_eq!(
        unsafe { dwg_dictionary_remove(fixture.data, fixture.root, test_key.as_ptr(), &mut read) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_dictionary_get(fixture.data, fixture.root, test_key.as_ptr(), &mut read) },
        DWG_ENOTFOUND
    );
    let mut count_after_failed_remove = 0_usize;
    assert_eq!(
        unsafe { dwg_dictionary_count(fixture.data, fixture.root, &mut count_after_failed_remove) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(count_after_failed_remove, original_count);
    let mut unchanged_group_handle = 0_u64;
    assert_eq!(
        unsafe {
            dwg_dictionary_get(
                fixture.data,
                fixture.root,
                acad_group.as_ptr(),
                &mut unchanged_group_handle,
            )
        },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(unchanged_group_handle, expected_group_handle);
}

#[test]
fn phase4b1_dictionary_removal_clears_hard_owner_metadata() {
    let fixture = Phase4b1DictionaryFixture::new();
    let key = CString::new("P4B1_HARD_OWNER").unwrap();
    let mut group_handle = 0_u64;
    let acad_group = CString::new("ACAD_GROUP").unwrap();
    assert_eq!(
        unsafe {
            dwg_dictionary_get(
                fixture.data,
                fixture.root,
                acad_group.as_ptr(),
                &mut group_handle,
            )
        },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, key.as_ptr(), group_handle) },
        DWG_OK
    );

    unsafe {
        let ffi = &mut *(fixture.data as *mut FfiDocument);
        let Some(ObjectType::Dictionary(dictionary)) =
            ffi.doc.objects.get_mut(&Handle::new(fixture.root))
        else {
            panic!("named objects dictionary is missing");
        };
        dictionary.set_entry_hard_owner("P4B1_HARD_OWNER", true);
    }

    let mut removed = 0_u64;
    assert_eq!(
        unsafe { dwg_dictionary_remove(fixture.data, fixture.root, key.as_ptr(), &mut removed) },
        DWG_OK
    );
    assert_eq!(removed, group_handle);
    unsafe {
        let ffi = &*(fixture.data as *const FfiDocument);
        let Some(ObjectType::Dictionary(dictionary)) =
            ffi.doc.objects.get(&Handle::new(fixture.root))
        else {
            panic!("named objects dictionary is missing");
        };
        assert!(!dictionary.is_entry_hard_owner("P4B1_HARD_OWNER"));
    }
}

#[test]
fn phase4b1_dictionary_buffers_and_roundtrip() {
    let fixture = Phase4b1DictionaryFixture::new();
    let key = CString::new("P4B1_TEST").unwrap();
    assert_eq!(
        unsafe { dwg_dictionary_set(fixture.data, fixture.root, key.as_ptr(), fixture.root) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    let mut needed = 0_usize;
    let mut value = 0_u64;
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                std::ptr::null_mut(),
                0,
                &mut needed,
                &mut value,
            )
        },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert!(needed >= 2);
    let mut too_small = vec![0_i8; needed - 1];
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                too_small.as_mut_ptr(),
                too_small.len(),
                &mut needed,
                &mut value,
            )
        },
        DWG_EBUFFER
    );
    let mut output = vec![0_i8; needed];
    assert_eq!(
        unsafe {
            dwg_dictionary_entry(
                fixture.data,
                fixture.root,
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut needed,
                &mut value,
            )
        },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(output.as_ptr()) }.to_bytes(),
        b"ACAD_GROUP"
    );

    let mut bytes = std::ptr::null_mut();
    let mut byte_len = 0_usize;
    assert_eq!(
        unsafe { dwg_write_bytes(fixture.data, &mut bytes, &mut byte_len) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut reopened = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_bytes(bytes, byte_len, reopened) }, DWG_OK);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut root = 0_u64;
    assert_eq!(
        unsafe { dwg_data_named_objects_dictionary(reopened, &mut root) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut read = 0_u64;
    assert_eq!(
        unsafe { dwg_dictionary_get(reopened, root, key.as_ptr(), &mut read) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(read, fixture.root);
    unsafe {
        dwg_free_buffer(bytes.cast());
        dwg_data_destroy(reopened);
    }
}

struct Phase4b2XrecordFixture {
    data: *mut DwgData,
    xrecord: u64,
    owner: u64,
}

impl Phase4b2XrecordFixture {
    fn new() -> Self {
        Self::new_with_extra_record(false)
    }

    fn new_with_extra_record(extra_record: bool) -> Self {
        let mut data = std::ptr::null_mut();
        assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);

        let mut err = DWG_OK;
        let owner = unsafe { dwg_data_first_object(data, &mut err) };
        assert_eq!(err, DWG_OK);
        assert!(!owner.is_null());
        let owner = unsafe { dwg_object_get_handle(owner) };
        assert_ne!(owner, 0);

        let xrecord = unsafe {
            let ffi = &mut *(data as *mut FfiDocument);
            if extra_record {
                let _ = ffi
                    .doc
                    .ensure_xrecord(Handle::new(owner), "P4B2_FOREIGN_PADDING");
            }
            let xrecord = ffi.doc.ensure_xrecord(Handle::new(owner), "P4B2_TEST");
            let record = ffi
                .doc
                .xrecord_mut(Handle::new(owner), "P4B2_TEST")
                .unwrap();
            record.entries = vec![
                XRecordEntry::string(1, "P4B2 string"),
                XRecordEntry::double(40, 12.5),
                XRecordEntry::int32(90, 123_456),
                XRecordEntry::new(10, XRecordValue::Point3D(1.0, 2.0, 3.0)),
                XRecordEntry::new(310, XRecordValue::Chunk(vec![0xCA, 0xFE, 0x01])),
            ];
            record.entries_complete = true;
            let named_objects = ffi.doc.header.named_objects_dict_handle;
            let Some(ObjectType::Dictionary(dictionary)) = ffi.doc.objects.get_mut(&named_objects)
            else {
                panic!("named objects dictionary is missing");
            };
            dictionary.entries.push(("P4B2_TEST".to_string(), xrecord));
            xrecord.value()
        };
        Self {
            data,
            xrecord,
            owner,
        }
    }
}

impl Drop for Phase4b2XrecordFixture {
    fn drop(&mut self) {
        unsafe { dwg_data_destroy(self.data) };
    }
}

fn xrecord_value(tag: u32) -> DwgXrecordValue {
    DwgXrecordValue {
        tag,
        reserved: 0,
        integer: 0,
        real: 0.0,
        point: [0.0; 3],
        handle: 0,
        data: std::ptr::null_mut(),
        data_len: 0,
    }
}

fn assert_seeded_xrecord(fixture: &Phase4b2XrecordFixture) {
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_xrecord_count(fixture.data, fixture.xrecord, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 5);

    let mut string = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 0, &mut 0, &mut string) },
        DWG_OK
    );
    assert_eq!(string.tag, DWG_XRECORD_STRING);
    assert_eq!(string.data_len, b"P4B2 string\0".len());
    let mut string_bytes = vec![0_u8; string.data_len];
    string.data = string_bytes.as_mut_ptr().cast();
    string.data_len = string_bytes.len();
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 0, &mut 0, &mut string) },
        DWG_OK
    );
    assert_eq!(&string_bytes, b"P4B2 string\0");

    let mut double = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 1, &mut 0, &mut double) },
        DWG_OK
    );
    assert_eq!(double.tag, DWG_XRECORD_DOUBLE);
    assert_eq!(double.real, 12.5);

    let mut integer = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 2, &mut 0, &mut integer) },
        DWG_OK
    );
    assert_eq!(integer.tag, DWG_XRECORD_INT32);
    assert_eq!(integer.integer, 123_456);

    let mut point = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 3, &mut 0, &mut point) },
        DWG_OK
    );
    assert_eq!(point.tag, DWG_XRECORD_POINT3D);
    assert_eq!(point.point, [1.0, 2.0, 3.0]);

    let mut chunk = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 4, &mut 0, &mut chunk) },
        DWG_OK
    );
    assert_eq!(chunk.tag, DWG_XRECORD_CHUNK);
    assert_eq!(chunk.data_len, 3);
    let mut chunk_bytes = [0_u8; 3];
    chunk.data = chunk_bytes.as_mut_ptr().cast();
    chunk.data_len = chunk_bytes.len();
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 4, &mut 0, &mut chunk) },
        DWG_OK
    );
    assert_eq!(chunk_bytes, [0xCA, 0xFE, 0x01]);
}

#[test]
fn phase4b2_xrecord_argument_and_type_errors() {
    let fixture = Phase4b2XrecordFixture::new();
    let mut count = 0;
    let mut index = 0;
    let mut code = 0;
    let mut value = xrecord_value(DWG_XRECORD_INT32);

    assert_eq!(
        unsafe { dwg_xrecord_count(std::ptr::null(), fixture.xrecord, &mut count) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_xrecord_count(fixture.data, fixture.xrecord, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 1, 0, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                fixture.data,
                fixture.xrecord,
                0,
                std::ptr::null_mut(),
                &mut value,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                fixture.data,
                fixture.xrecord,
                0,
                &mut code,
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_xrecord_append(std::ptr::null_mut(), fixture.xrecord, 90, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 90, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    assert_eq!(
        unsafe { dwg_xrecord_set(fixture.data, fixture.xrecord, 0, 90, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    assert_eq!(
        unsafe {
            dwg_xrecord_remove(
                fixture.data,
                fixture.xrecord,
                0,
                std::ptr::null_mut(),
                &mut value,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    assert_eq!(
        unsafe {
            dwg_xrecord_remove(
                fixture.data,
                fixture.xrecord,
                0,
                &mut code,
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);

    value.reserved = 1;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 90, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);

    value = xrecord_value(DWG_XRECORD_STRING);
    value.data_len = 1;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 1, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);

    let malformed_utf8 = [0xff_u8];
    value.data = malformed_utf8.as_ptr().cast_mut().cast();
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 1, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);

    value = xrecord_value(DWG_XRECORD_DOUBLE);
    value.real = f64::NAN;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 40, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    value = xrecord_value(DWG_XRECORD_POINT3D);
    value.point = [1.0, f64::INFINITY, 3.0];
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 10, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);

    value = xrecord_value(99);
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 90, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xrecord(&fixture);
    value = xrecord_value(DWG_XRECORD_STRING);
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 90, &value) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_seeded_xrecord(&fixture);

    assert_eq!(
        unsafe { dwg_xrecord_count(fixture.data, u64::MAX, &mut count) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 5, &mut code, &mut value) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 1, 1, &mut index) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_xrecord_count(fixture.data, fixture.owner, &mut count) },
        DWG_ETYPE
    );
}

#[test]
fn phase4b2_xrecord_lookup_and_mutation() {
    let fixture = Phase4b2XrecordFixture::new();
    assert_seeded_xrecord(&fixture);

    let mut index = usize::MAX;
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 1, 0, &mut index) },
        DWG_OK
    );
    assert_eq!(index, 0);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 40, 0, &mut index) },
        DWG_OK
    );
    assert_eq!(index, 1);
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 90, 0, &mut index) },
        DWG_OK
    );
    assert_eq!(index, 2);

    let mut int16 = xrecord_value(DWG_XRECORD_INT16);
    int16.integer = -321;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 70, &int16) },
        DWG_OK
    );
    let mut byte = xrecord_value(DWG_XRECORD_BYTE);
    byte.integer = 0xAB;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 280, &byte) },
        DWG_OK
    );
    let mut duplicate = xrecord_value(DWG_XRECORD_DOUBLE);
    duplicate.real = 99.5;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 40, &duplicate) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 40, 0, &mut index) },
        DWG_OK
    );
    assert_eq!(index, 1);
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 40, 1, &mut index) },
        DWG_OK
    );
    assert_eq!(index, 7);
    assert_eq!(
        unsafe { dwg_xrecord_find(fixture.data, fixture.xrecord, 40, 2, &mut index) },
        DWG_ENOTFOUND
    );
    let mut read_code = 0;
    let mut read_int16 = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                fixture.data,
                fixture.xrecord,
                5,
                &mut read_code,
                &mut read_int16,
            )
        },
        DWG_OK
    );
    assert_eq!(read_code, 70);
    assert_eq!(read_int16.tag, DWG_XRECORD_INT16);
    assert_eq!(read_int16.integer, -321);
    let mut read_byte = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                fixture.data,
                fixture.xrecord,
                6,
                &mut read_code,
                &mut read_byte,
            )
        },
        DWG_OK
    );
    assert_eq!(read_code, 280);
    assert_eq!(read_byte.tag, DWG_XRECORD_BYTE);
    assert_eq!(read_byte.integer, 0xAB);

    let mut int64 = xrecord_value(DWG_XRECORD_INT64);
    int64.integer = -9_876_543_210;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 160, &int64) },
        DWG_OK
    );
    let mut boolean = xrecord_value(DWG_XRECORD_BOOL);
    boolean.integer = 1;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 290, &boolean) },
        DWG_OK
    );
    let mut replacement = xrecord_value(DWG_XRECORD_DOUBLE);
    replacement.real = 42.25;
    assert_eq!(
        unsafe { dwg_xrecord_set(fixture.data, fixture.xrecord, 1, 40, &replacement) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    let mut code = 0;
    let mut removed = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_remove(fixture.data, fixture.xrecord, 2, &mut code, &mut removed) },
        DWG_OK
    );
    assert_eq!(code, 90);
    assert_eq!(removed.tag, DWG_XRECORD_INT32);
    assert_eq!(removed.integer, 123_456);
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_xrecord_count(fixture.data, fixture.xrecord, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 9);
    let mut shifted = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 2, &mut code, &mut shifted) },
        DWG_OK
    );
    assert_eq!(code, 10);
    assert_eq!(shifted.tag, DWG_XRECORD_POINT3D);
    assert_eq!(shifted.point, [1.0, 2.0, 3.0]);
}

#[test]
fn phase4b2_xrecord_buffers_and_roundtrip() {
    let fixture = Phase4b2XrecordFixture::new();
    let mut code = 0;
    let mut string = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 0, &mut code, &mut string) },
        DWG_OK
    );
    assert_eq!(code, 1);
    assert_eq!(string.tag, DWG_XRECORD_STRING);
    assert_eq!(string.data_len, b"P4B2 string\0".len());
    let mut too_small = vec![0_u8; string.data_len - 1];
    string.data = too_small.as_mut_ptr().cast();
    string.data_len = too_small.len();
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 0, &mut code, &mut string) },
        DWG_EBUFFER
    );
    assert_last_error(DWG_EBUFFER);
    assert_seeded_xrecord(&fixture);
    let mut string_bytes = vec![0_u8; b"P4B2 string\0".len()];
    string.data = string_bytes.as_mut_ptr().cast();
    string.data_len = string_bytes.len();
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 0, &mut code, &mut string) },
        DWG_OK
    );
    assert_eq!(&string_bytes, b"P4B2 string\0");

    let mut chunk = xrecord_value(0);
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 4, &mut code, &mut chunk) },
        DWG_OK
    );
    assert_eq!(chunk.tag, DWG_XRECORD_CHUNK);
    assert_eq!(chunk.data_len, 3);
    let mut chunk_bytes = [0_u8; 3];
    chunk.data = chunk_bytes.as_mut_ptr().cast();
    chunk.data_len = chunk_bytes.len();
    assert_eq!(
        unsafe { dwg_xrecord_entry(fixture.data, fixture.xrecord, 4, &mut code, &mut chunk) },
        DWG_OK
    );
    assert_eq!(chunk_bytes, [0xCA, 0xFE, 0x01]);

    let mut reference = xrecord_value(DWG_XRECORD_HANDLE);
    reference.handle = fixture.owner;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 330, &reference) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut extra_int16 = xrecord_value(DWG_XRECORD_INT16);
    extra_int16.integer = -321;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 70, &extra_int16) },
        DWG_OK
    );
    let mut extra_byte = xrecord_value(DWG_XRECORD_BYTE);
    extra_byte.integer = 0xAB;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 280, &extra_byte) },
        DWG_OK
    );
    let mut extra_int64 = xrecord_value(DWG_XRECORD_INT64);
    extra_int64.integer = -9_876_543_210;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 160, &extra_int64) },
        DWG_OK
    );
    let mut extra_bool = xrecord_value(DWG_XRECORD_BOOL);
    extra_bool.integer = 1;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, fixture.xrecord, 290, &extra_bool) },
        DWG_OK
    );
    let mut bytes = std::ptr::null_mut();
    let mut byte_len = 0;
    assert_eq!(
        unsafe { dwg_write_bytes(fixture.data, &mut bytes, &mut byte_len) },
        DWG_OK
    );
    let mut reopened = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_bytes(bytes, byte_len, reopened) }, DWG_OK);
    let key = CString::new("P4B2_TEST").unwrap();
    let mut root = 0;
    let mut reopened_xrecord = 0;
    assert_eq!(
        unsafe { dwg_data_named_objects_dictionary(reopened, &mut root) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_dictionary_get(reopened, root, key.as_ptr(), &mut reopened_xrecord) },
        DWG_OK
    );
    let mut reopened_count = 0;
    assert_eq!(
        unsafe { dwg_xrecord_count(reopened, reopened_xrecord, &mut reopened_count) },
        DWG_OK
    );
    assert_eq!(reopened_count, 10);
    let mut reopened_reference = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                5,
                &mut code,
                &mut reopened_reference,
            )
        },
        DWG_OK
    );
    assert_eq!(code, 330);
    assert_eq!(reopened_reference.tag, DWG_XRECORD_HANDLE);
    assert_eq!(reopened_reference.handle, fixture.owner);
    let mut reopened_string = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                0,
                &mut code,
                &mut reopened_string,
            )
        },
        DWG_OK
    );
    let mut reopened_string_bytes = vec![0_u8; reopened_string.data_len];
    reopened_string.data = reopened_string_bytes.as_mut_ptr().cast();
    reopened_string.data_len = reopened_string_bytes.len();
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                0,
                &mut code,
                &mut reopened_string,
            )
        },
        DWG_OK
    );
    assert_eq!(reopened_string_bytes, b"P4B2 string\0");
    let mut reopened_double = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                1,
                &mut code,
                &mut reopened_double,
            )
        },
        DWG_OK
    );
    assert_eq!(code, 40);
    assert_eq!(reopened_double.real, 12.5);
    let mut reopened_int32 = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                2,
                &mut code,
                &mut reopened_int32,
            )
        },
        DWG_OK
    );
    assert_eq!(code, 90);
    assert_eq!(reopened_int32.integer, 123_456);
    let mut reopened_point = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                3,
                &mut code,
                &mut reopened_point,
            )
        },
        DWG_OK
    );
    assert_eq!(reopened_point.point, [1.0, 2.0, 3.0]);
    let mut reopened_chunk = xrecord_value(0);
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                4,
                &mut code,
                &mut reopened_chunk,
            )
        },
        DWG_OK
    );
    let mut reopened_chunk_bytes = [0_u8; 3];
    reopened_chunk.data = reopened_chunk_bytes.as_mut_ptr().cast();
    reopened_chunk.data_len = reopened_chunk_bytes.len();
    assert_eq!(
        unsafe {
            dwg_xrecord_entry(
                reopened,
                reopened_xrecord,
                4,
                &mut code,
                &mut reopened_chunk,
            )
        },
        DWG_OK
    );
    assert_eq!(reopened_chunk_bytes, [0xCA, 0xFE, 0x01]);
    for (index, (expected_tag, expected_integer)) in [
        (DWG_XRECORD_INT16, -321),
        (DWG_XRECORD_BYTE, 0xAB),
        (DWG_XRECORD_INT64, -9_876_543_210),
        (DWG_XRECORD_BOOL, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let mut value = xrecord_value(0);
        assert_eq!(
            unsafe {
                dwg_xrecord_entry(reopened, reopened_xrecord, index + 6, &mut code, &mut value)
            },
            DWG_OK
        );
        assert_eq!(value.tag, expected_tag);
        assert_eq!(value.integer, expected_integer);
    }
    unsafe { dwg_free_buffer(bytes.cast()) };

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_p4b2_dxf_{}_{}",
        std::process::id(),
        suffix
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("xrecord.dxf");
    let path_c = CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(
        unsafe { dxf_write_file(path_c.as_ptr(), fixture.data) },
        DWG_OK
    );
    let rust_document = acadrust::DxfReader::from_file(&path)
        .unwrap()
        .read()
        .unwrap();
    let rust_record = rust_document
        .xrecord(Handle::new(fixture.owner), "P4B2_TEST")
        .unwrap();
    assert_eq!(rust_record.entries.len(), 10);
    assert_eq!(
        rust_record.entries[0],
        XRecordEntry::string(1, "P4B2 string")
    );
    assert_eq!(rust_record.entries[1], XRecordEntry::double(40, 12.5));
    assert_eq!(rust_record.entries[2], XRecordEntry::int32(90, 123_456));
    assert_eq!(
        rust_record.entries[3],
        XRecordEntry::new(10, XRecordValue::Point3D(1.0, 2.0, 3.0))
    );
    assert_eq!(
        rust_record.entries[4],
        XRecordEntry::new(310, XRecordValue::Chunk(vec![0xCA, 0xFE, 0x01]))
    );
    assert_eq!(
        rust_record.entries[5],
        XRecordEntry::new(330, XRecordValue::Handle(Handle::new(fixture.owner)))
    );
    assert_eq!(rust_record.entries[6], XRecordEntry::int16(70, -321));
    assert_eq!(
        rust_record.entries[7],
        XRecordEntry::new(280, XRecordValue::Byte(0xAB))
    );
    assert_eq!(
        rust_record.entries[8],
        XRecordEntry::new(160, XRecordValue::Int64(-9_876_543_210))
    );
    assert_eq!(
        rust_record.entries[9],
        XRecordEntry::new(290, XRecordValue::Bool(true))
    );
    unsafe { dwg_data_destroy(reopened) };
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn phase4b2_xrecord_foreign_handle_preserves_document() {
    let fixture = Phase4b2XrecordFixture::new();
    let foreign = Phase4b2XrecordFixture::new_with_extra_record(true);
    assert_ne!(fixture.xrecord, foreign.xrecord);
    let mut value = xrecord_value(DWG_XRECORD_INT32);
    value.integer = 7;
    assert_eq!(
        unsafe { dwg_xrecord_append(fixture.data, foreign.xrecord, 90, &value) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_seeded_xrecord(&fixture);
}

#[test]
fn phase3c_spline_collection_setters() {
    let fixture = write_spline_collection_dwg("acadrust_ffi_p3c3_spline", false);
    let foreign_fixture = write_spline_collection_dwg("acadrust_ffi_p3c3_spline_foreign", false);
    let fit_fixture = write_spline_collection_dwg("acadrust_ffi_p3c3_spline_fit", true);

    let mut data: *mut DwgData = std::ptr::null_mut();
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    let mut fit_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut fit_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_read_file(fit_fixture.c_path().as_ptr(), fit_data) },
        DWG_OK
    );

    let spline = find_object_of_type(data, DwgObjectType::Spline as i32);
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let foreign_spline = find_object_of_type(foreign_data, DwgObjectType::Spline as i32);
    let fit_spline = find_object_of_type(fit_data, DwgObjectType::Spline as i32);
    assert!(!spline.is_null());
    assert!(!line.is_null());
    assert!(!foreign_spline.is_null());
    assert!(!fit_spline.is_null());
    assert_eq!(
        unsafe { read_spline_state(spline) },
        (
            3,
            vec![
                (0.0, 0.0, 0.0),
                (1.0, 1.0, 0.0),
                (2.0, 0.0, 0.0),
                (3.0, 1.0, 0.0),
                (4.0, 0.0, 0.0),
            ],
        )
    );

    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_set_degree(std::ptr::null_mut(), spline, 2)
    });
    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_append_control_point(data, std::ptr::null(), 5.0, 6.0, 7.0)
    });
    assert_spline_failure_preserves_state(spline, DWG_ETYPE, || unsafe {
        dwg_entity_spline_set_control_point(data, line, 0, 5.0, 6.0, 7.0)
    });
    assert_spline_failure_preserves_state(spline, DWG_ENOTFOUND, || unsafe {
        dwg_entity_spline_clear_control_points(data, foreign_spline)
    });

    for degree in [-1, 0, 26, 5] {
        assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
            dwg_entity_spline_set_degree(data, spline, degree)
        });
    }
    for index in [-1, 5] {
        assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
            dwg_entity_spline_set_control_point(data, spline, index, 5.0, 6.0, 7.0)
        });
        assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
            dwg_entity_spline_remove_control_point(data, spline, index)
        });
    }
    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_set_control_point(data, spline, 1, f64::NAN, 6.0, 7.0)
    });
    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_append_control_point(data, spline, 5.0, f64::INFINITY, 7.0)
    });

    assert_eq!(
        unsafe { dwg_entity_spline_set_control_point(data, spline, 1, 11.0, 12.0, 13.0) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    let mut needed = 0;
    assert_eq!(
        unsafe { dwg_last_error(std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    assert_eq!(needed, 1);
    assert_eq!(
        unsafe { read_spline_state(spline) }.1[1],
        (11.0, 12.0, 13.0)
    );

    assert_eq!(
        unsafe { dwg_entity_spline_append_control_point(data, spline, 21.0, 22.0, 23.0) },
        DWG_OK
    );
    let appended_state = unsafe { read_spline_state(spline) };
    assert_eq!(appended_state.1.len(), 6);
    assert_eq!(appended_state.1[5], (21.0, 22.0, 23.0));
    assert_eq!(
        unsafe { dwg_entity_spline_remove_control_point(data, spline, 5) },
        DWG_OK
    );
    let removed_state = unsafe { read_spline_state(spline) };
    assert_eq!(removed_state.1.len(), 5);
    assert_eq!(removed_state.1[1], (11.0, 12.0, 13.0));

    assert_eq!(
        unsafe { dwg_entity_spline_set_degree(data, spline, 2) },
        DWG_OK
    );
    assert_eq!(unsafe { read_spline_state(spline) }.0, 2);
    assert_eq!(
        unsafe { dwg_entity_spline_remove_control_point(data, spline, 4) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_spline_remove_control_point(data, spline, 3) },
        DWG_OK
    );
    assert_eq!(unsafe { read_spline_state(spline) }.1.len(), 3);
    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_remove_control_point(data, spline, 2)
    });
    assert_eq!(
        unsafe { dwg_entity_spline_clear_control_points(data, spline) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    assert_eq!(unsafe { read_spline_state(spline) }, (2, Vec::new()));
    assert_spline_failure_preserves_state(spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_append_control_point(data, spline, 1.0, 2.0, 3.0)
    });

    assert_eq!(unsafe { read_spline_state(fit_spline) }, (3, Vec::new()));
    assert_spline_failure_preserves_state(fit_spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_set_control_point(fit_data, fit_spline, 0, 1.0, 2.0, 3.0)
    });
    assert!(last_error_message().contains("fit points"));
    assert_spline_failure_preserves_state(fit_spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_append_control_point(fit_data, fit_spline, 1.0, 2.0, 3.0)
    });
    assert!(last_error_message().contains("fit points"));
    assert_spline_failure_preserves_state(fit_spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_remove_control_point(fit_data, fit_spline, 0)
    });
    assert!(last_error_message().contains("fit points"));
    assert_spline_failure_preserves_state(fit_spline, DWG_EINVAL, || unsafe {
        dwg_entity_spline_clear_control_points(fit_data, fit_spline)
    });
    assert!(last_error_message().contains("fit points"));

    unsafe { dwg_data_destroy(fit_data) };
    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

unsafe fn read_collection_count(
    getter: unsafe extern "C" fn(*const DwgObject, *mut i32) -> i32,
    obj: *const DwgObject,
) -> i32 {
    let mut count = -1;
    assert_eq!(getter(obj, &mut count), DWG_OK);
    count
}

unsafe fn read_collection_vertex2(
    getter: unsafe extern "C" fn(*const DwgObject, i32, *mut f64, *mut f64) -> i32,
    obj: *const DwgObject,
    index: i32,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    assert_eq!(getter(obj, index, &mut x, &mut y), DWG_OK);
    (x, y)
}

unsafe fn read_collection_vertex3(
    getter: unsafe extern "C" fn(*const DwgObject, i32, *mut f64, *mut f64, *mut f64) -> i32,
    obj: *const DwgObject,
    index: i32,
) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(getter(obj, index, &mut x, &mut y, &mut z), DWG_OK);
    (x, y, z)
}

#[test]
fn phase3c_polyline_collection_setters() {
    let fixture = write_polyline_collection_dwg("acadrust_ffi_p3c3_polyline");
    let foreign_fixture = write_polyline_collection_dwg("acadrust_ffi_p3c3_polyline_foreign");
    let mut data: *mut DwgData = std::ptr::null_mut();
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );

    let lwpolyline = find_object_of_type(data, DwgObjectType::LwPolyline as i32);
    let polyline2d = find_object_of_type(data, DwgObjectType::Polyline2D as i32);
    let polyline3d = find_object_of_type(data, DwgObjectType::Polyline3D as i32);
    let polyface_mesh = find_object_of_type(data, DwgObjectType::PolyfaceMesh as i32);
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let foreign_lwpolyline = find_object_of_type(foreign_data, DwgObjectType::LwPolyline as i32);
    for obj in [
        lwpolyline,
        polyline2d,
        polyline3d,
        polyface_mesh,
        line,
        foreign_lwpolyline,
    ] {
        assert!(!obj.is_null());
    }

    assert_eq!(
        unsafe { dwg_entity_lwpolyline_get_vertex_count(lwpolyline, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(std::ptr::null_mut(), lwpolyline, 0, 1.0, 2.0,) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_append_vertex(data, std::ptr::null(), 1.0, 2.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_clear_vertices(data, line) },
        DWG_ETYPE
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_remove_vertex(data, foreign_lwpolyline, 0) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { read_collection_count(dwg_entity_lwpolyline_get_vertex_count, lwpolyline) },
        2
    );
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 0) },
        (0.0, 0.0)
    );
    for index in [-1, 2] {
        assert_eq!(
            unsafe { dwg_entity_lwpolyline_set_vertex(data, lwpolyline, index, 7.0, 8.0) },
            DWG_EINVAL
        );
        assert_eq!(
            unsafe { dwg_entity_lwpolyline_remove_vertex(data, lwpolyline, index) },
            DWG_EINVAL
        );
    }
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_append_vertex(data, lwpolyline, f64::NAN, 8.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { read_collection_count(dwg_entity_lwpolyline_get_vertex_count, lwpolyline) },
        2
    );
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 0) },
        (0.0, 0.0)
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(data, lwpolyline, 0, 7.0, 8.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 0) },
        (7.0, 8.0)
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_append_vertex(data, lwpolyline, 11.0, 12.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 2) },
        (11.0, 12.0)
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_remove_vertex(data, lwpolyline, 2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_clear_vertices(data, lwpolyline) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_collection_count(dwg_entity_lwpolyline_get_vertex_count, lwpolyline) },
        0
    );

    macro_rules! exercise_vec3_collection {
        ($obj:ident, $count:ident, $get:ident, $set:ident, $append:ident, $remove:ident, $clear:ident) => {{
            assert_eq!(unsafe { read_collection_count($count, $obj) }, 2);
            assert_eq!(
                unsafe { read_collection_vertex3($get, $obj, 0) },
                (0.0, 0.0, 0.0)
            );
            for index in [-1, 2] {
                assert_eq!(
                    unsafe { $set(data, $obj, index, 7.0, 8.0, 9.0) },
                    DWG_EINVAL
                );
                assert_eq!(unsafe { $remove(data, $obj, index) }, DWG_EINVAL);
            }
            assert_eq!(
                unsafe { $append(data, $obj, 7.0, f64::NAN, 9.0) },
                DWG_EINVAL
            );
            assert_eq!(unsafe { read_collection_count($count, $obj) }, 2);
            assert_eq!(
                unsafe { read_collection_vertex3($get, $obj, 0) },
                (0.0, 0.0, 0.0)
            );
            assert_eq!(unsafe { $set(data, $obj, 0, 7.0, 8.0, 9.0) }, DWG_OK);
            assert_eq!(
                unsafe { read_collection_vertex3($get, $obj, 0) },
                (7.0, 8.0, 9.0)
            );
            assert_eq!(unsafe { $append(data, $obj, 21.0, 22.0, 23.0) }, DWG_OK);
            assert_eq!(
                unsafe { read_collection_vertex3($get, $obj, 2) },
                (21.0, 22.0, 23.0)
            );
            assert_eq!(unsafe { $remove(data, $obj, 2) }, DWG_OK);
            assert_eq!(unsafe { $clear(data, $obj) }, DWG_OK);
            assert_eq!(unsafe { read_collection_count($count, $obj) }, 0);
        }};
    }

    exercise_vec3_collection!(
        polyline2d,
        dwg_entity_polyline2d_get_vertex_count,
        dwg_entity_polyline2d_get_vertex,
        dwg_entity_polyline2d_set_vertex,
        dwg_entity_polyline2d_append_vertex,
        dwg_entity_polyline2d_remove_vertex,
        dwg_entity_polyline2d_clear_vertices
    );
    exercise_vec3_collection!(
        polyline3d,
        dwg_entity_polyline3d_get_vertex_count,
        dwg_entity_polyline3d_get_vertex,
        dwg_entity_polyline3d_set_vertex,
        dwg_entity_polyline3d_append_vertex,
        dwg_entity_polyline3d_remove_vertex,
        dwg_entity_polyline3d_clear_vertices
    );
    exercise_vec3_collection!(
        polyface_mesh,
        dwg_entity_polyface_mesh_get_vertex_count,
        dwg_entity_polyface_mesh_get_vertex,
        dwg_entity_polyface_mesh_set_vertex,
        dwg_entity_polyface_mesh_append_vertex,
        dwg_entity_polyface_mesh_remove_vertex,
        dwg_entity_polyface_mesh_clear_vertices
    );

    assert_eq!(dwg_last_error_code(), DWG_OK);
    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_leader_and_mesh_vertex_setters() {
    let fixture = write_leader_mesh_collection_dwg("acadrust_ffi_p3c3_leader_mesh");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let leader = find_object_of_type(data, DwgObjectType::Leader as i32);
    let mesh = find_object_of_type(data, DwgObjectType::Mesh as i32);
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    assert!(!leader.is_null());
    assert!(!mesh.is_null());
    assert!(!line.is_null());

    let mut count = -1;
    assert_eq!(
        unsafe { dwg_entity_leader_get_vertex_count(leader, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 2);
    assert_eq!(
        unsafe { dwg_entity_leader_set_vertex(data, leader, 0, 10.0, 11.0, 12.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_leader_append_vertex(data, leader, 2.0, 3.0, 4.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_leader_remove_vertex(data, leader, 2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_leader_clear_vertices(data, leader) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_leader_get_vertex_count(leader, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 0);
    assert_eq!(
        unsafe { dwg_entity_leader_set_vertex(data, leader, -1, 1.0, 2.0, 3.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_leader_append_vertex(data, leader, f64::NAN, 2.0, 3.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_leader_clear_vertices(data, line) },
        DWG_ETYPE
    );

    count = -1;
    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex_count(mesh, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 4);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_vertex(data, mesh, 0, 20.0, 21.0, 22.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_append_vertex(data, mesh, 4.0, 5.0, 6.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_remove_vertex(data, mesh, 4) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_clear_vertices(data, mesh) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex_count(mesh, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 0);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_mesh_face_setters() {
    let fixture = write_mesh_face_collection_dwg("acadrust_ffi_p3c3_mesh_faces");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    let mesh = find_object_of_type(data, DwgObjectType::Mesh as i32);
    assert!(!mesh.is_null());

    let mut face_count = -1;
    let mut slot_count = -1;
    let mut vertex = u32::MAX;
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_count(mesh, &mut face_count) },
        DWG_OK
    );
    assert_eq!(face_count, 1);
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_vertex_count(mesh, 0, &mut slot_count) },
        DWG_OK
    );
    assert_eq!(slot_count, 3);
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_vertex(mesh, 0, 1, &mut vertex) },
        DWG_OK
    );
    assert_eq!(vertex, 1);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face_vertex(data, mesh, 0, 1, 3) },
        DWG_OK
    );
    let replacement = [0_u32, 2, 3];
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face(data, mesh, 0, replacement.as_ptr(), 3) },
        DWG_OK
    );
    let appended = [1_u32, 2, 3];
    assert_eq!(
        unsafe { dwg_entity_mesh_append_face(data, mesh, appended.as_ptr(), 3) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_count(mesh, &mut face_count) },
        DWG_OK
    );
    assert_eq!(face_count, 2);
    assert_eq!(
        unsafe { dwg_entity_mesh_remove_face(data, mesh, 0) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_entity_mesh_clear_faces(data, mesh) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_count(mesh, &mut face_count) },
        DWG_OK
    );
    assert_eq!(face_count, 0);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face(data, mesh, 0, replacement.as_ptr(), 2) },
        DWG_EINVAL
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_EINVAL);
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_setter_error_matrix() {
    let collection_fixture = write_collection_setter_dwg("acadrust_ffi_p3c4_errors");
    let foreign_collection_fixture =
        write_collection_setter_dwg("acadrust_ffi_p3c4_errors_foreign");
    let sample_fixture = write_sample_dwg("acadrust_ffi_p3c4_errors_sample");
    let foreign_sample_fixture = write_sample_dwg("acadrust_ffi_p3c4_errors_sample_foreign");

    let mut data: *mut DwgData = std::ptr::null_mut();
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    let mut sample_data: *mut DwgData = std::ptr::null_mut();
    let mut foreign_sample_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut sample_data) }, DWG_OK);
    assert_eq!(unsafe { dwg_data_create(&mut foreign_sample_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(collection_fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_read_file(foreign_collection_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_read_file(sample_fixture.c_path().as_ptr(), sample_data) },
        DWG_OK
    );
    assert_eq!(
        unsafe {
            dwg_read_file(
                foreign_sample_fixture.c_path().as_ptr(),
                foreign_sample_data,
            )
        },
        DWG_OK
    );

    let spline = find_object_of_type(data, DwgObjectType::Spline as i32);
    let lwpolyline = find_object_of_type(data, DwgObjectType::LwPolyline as i32);
    let mesh = find_object_of_type(data, DwgObjectType::Mesh as i32);
    let foreign_lwpolyline = find_object_of_type(foreign_data, DwgObjectType::LwPolyline as i32);
    let line = find_object_of_type(sample_data, DwgObjectType::Line as i32);
    let circle = find_object_of_type(sample_data, DwgObjectType::Circle as i32);
    let text = find_object_of_type(sample_data, DwgObjectType::Text as i32);
    let foreign_line = find_object_of_type(foreign_sample_data, DwgObjectType::Line as i32);
    for object in [
        spline,
        lwpolyline,
        mesh,
        foreign_lwpolyline,
        line,
        circle,
        text,
        foreign_line,
    ] {
        assert!(!object.is_null());
    }

    let layer = CString::new("P3C4_LAYER").unwrap();
    assert_eq!(
        unsafe { dwg_entity_set_layer(std::ptr::null_mut(), line, layer.as_ptr()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert!(!last_error_message().is_empty());
    assert_eq!(
        unsafe { dwg_entity_line_set_start(sample_data, circle, 1.0, 2.0, 3.0) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_eq!(
        unsafe { dwg_entity_line_set_start(sample_data, foreign_line, 1.0, 2.0, 3.0) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_eq!(
        unsafe { dwg_entity_set_layer(sample_data, line, layer.as_ptr()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    assert_eq!(last_error_message(), "");

    assert_eq!(
        unsafe { dwg_entity_spline_set_degree(data, spline, -1) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert!(last_error_message().contains("degree"));
    let mut degree = 0;
    assert_eq!(
        unsafe { dwg_entity_spline_get_degree(spline, &mut degree) },
        DWG_OK
    );
    assert_eq!(degree, 3);

    assert_eq!(
        unsafe { dwg_entity_mesh_set_vertex(data, mesh, 0, f64::NAN, 1.0, 2.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex(mesh, 0, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert_eq!((x, y, z), (0.0, 0.0, 0.0));

    assert_eq!(
        unsafe { dwg_entity_text_set_value(sample_data, text, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert!(!last_error_message().is_empty());
    let text_value = CString::new("P3C4_TEXT").unwrap();
    assert_eq!(
        unsafe { dwg_entity_text_set_value(sample_data, text, text_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(data, lwpolyline, -1, 1.0, 2.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(data, mesh, 0, 1.0, 2.0) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(data, foreign_lwpolyline, 0, 1.0, 2.0) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);

    let face = [0_u32, 2, 3];
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face(data, mesh, 0, std::ptr::null(), face.len()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face(data, mesh, 0, face.as_ptr(), 2) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face_vertex(data, mesh, -1, 0, 0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_mesh_set_face(data, mesh, 0, face.as_ptr(), face.len()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);

    unsafe { dwg_data_destroy(foreign_sample_data) };
    unsafe { dwg_data_destroy(sample_data) };
    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_dwg_setters_roundtrip_all_versions() {
    use acadrust::entities::{Circle, Line, Text};
    use acadrust::tables::Layer;
    use acadrust::types::DxfVersion;
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let versions = [
        ("R13", DxfVersion::AC1012),
        ("R14", DxfVersion::AC1014),
        ("R2000", DxfVersion::AC1015),
        ("R2004", DxfVersion::AC1018),
        ("R2007", DxfVersion::AC1021),
        ("R2010", DxfVersion::AC1024),
        ("R2013", DxfVersion::AC1027),
        ("R2018", DxfVersion::AC1032),
    ];

    for (label, version) in versions {
        let dir = std::env::temp_dir().join(format!(
            "acadrust_ffi_p3c4_dwg_{label}_{}_{}",
            std::process::id(),
            SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("source.dwg");
        let output_path = dir.join("mutated.dwg");

        let mut document = CadDocument::with_version(version);
        let mut target_layer = Layer::new("P3C_RT");
        target_layer.handle = document.allocate_handle();
        document.layers.add(target_layer).unwrap();
        document
            .add_entity(EntityType::Line(Line::from_coords(
                0.0, 0.0, 0.0, 1.0, 2.0, 3.0,
            )))
            .unwrap();
        let mut circle = Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5);
        circle.common.layer = "P3C_RT".to_string();
        document.add_entity(EntityType::Circle(circle)).unwrap();
        document
            .add_entity(EntityType::Text(Text::with_value(
                "P3C SOURCE",
                Vector3::new(0.0, 0.0, 0.0),
            )))
            .unwrap();
        DwgWriter::write_to_file(&source_path, &document).unwrap();

        let source = CString::new(source_path.to_str().unwrap()).unwrap();
        let output = CString::new(output_path.to_str().unwrap()).unwrap();
        let mut data: *mut DwgData = std::ptr::null_mut();
        assert_eq!(
            unsafe { dwg_data_create(&mut data) },
            DWG_OK,
            "{label}: create"
        );
        assert_eq!(
            unsafe { dwg_read_file(source.as_ptr(), data) },
            DWG_OK,
            "{label}: read source {}",
            source_path.display()
        );

        let line = find_object_of_type(data, DwgObjectType::Line as i32);
        let circle = find_object_of_type(data, DwgObjectType::Circle as i32);
        let text = find_object_of_type(data, DwgObjectType::Text as i32);
        assert!(!line.is_null(), "{label}: line missing");
        assert!(!circle.is_null(), "{label}: circle missing");
        assert!(!text.is_null(), "{label}: text missing");

        let layer = CString::new("P3C_RT").unwrap();
        let text_value = CString::new("P3C DWG").unwrap();
        assert_eq!(
            unsafe { dwg_entity_set_layer(data, line, layer.as_ptr()) },
            DWG_OK,
            "{label}: set layer"
        );
        assert_eq!(
            unsafe { dwg_entity_set_color(data, line, 0x00112233) },
            DWG_OK,
            "{label}: set color"
        );
        assert_eq!(
            unsafe { dwg_entity_line_set_start(data, line, 11.0, 12.0, 13.0) },
            DWG_OK,
            "{label}: set line start"
        );
        assert_eq!(
            unsafe { dwg_entity_circle_set_radius(data, circle, 4.5) },
            DWG_OK,
            "{label}: set circle radius"
        );
        assert_eq!(
            unsafe { dwg_entity_text_set_value(data, text, text_value.as_ptr()) },
            DWG_OK,
            "{label}: set text"
        );
        assert_eq!(
            unsafe { read_string(dwg_entity_get_layer, line) },
            "P3C_RT",
            "{label}: in-memory layer"
        );
        assert_eq!(
            unsafe { dwg_write_file(output.as_ptr(), data) },
            DWG_OK,
            "{label}: write {}",
            output_path.display()
        );
        unsafe { dwg_data_destroy(data) };

        let mut reopened: *mut DwgData = std::ptr::null_mut();
        assert_eq!(
            unsafe { dwg_data_create(&mut reopened) },
            DWG_OK,
            "{label}: reopen create"
        );
        assert_eq!(
            unsafe { dwg_read_file(output.as_ptr(), reopened) },
            DWG_OK,
            "{label}: reopen read {}",
            output_path.display()
        );
        let line = find_object_of_type(reopened, DwgObjectType::Line as i32);
        let circle = find_object_of_type(reopened, DwgObjectType::Circle as i32);
        let text = find_object_of_type(reopened, DwgObjectType::Text as i32);
        assert_eq!(
            unsafe { read_vec3(dwg_entity_line_get_start, line) },
            (11.0, 12.0, 13.0)
        );
        assert_eq!(
            unsafe { read_f64(dwg_entity_circle_get_radius, circle) },
            4.5
        );
        assert_eq!(
            unsafe { read_string(dwg_entity_get_layer, line) },
            "P3C_RT",
            "{label}: reopened layer"
        );
        let mut color = 0;
        assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
        let expected_color = if version >= DxfVersion::AC1018 {
            0x00112233
        } else {
            -(acadrust::types::Color::from_rgb(0x11, 0x22, 0x33).approximate_index() as i32)
        };
        assert_eq!(color, expected_color, "{label}: color");
        assert_eq!(
            unsafe { read_string(dwg_entity_text_get_value, text) },
            "P3C DWG"
        );
        unsafe { dwg_data_destroy(reopened) };

        let _ = std::fs::remove_file(&source_path);
        let _ = std::fs::remove_file(&output_path);
        let _ = std::fs::remove_dir(&dir);
    }
}

#[test]
fn phase3c_dwg_collection_setters_roundtrip() {
    let fixture = write_collection_setter_dwg("acadrust_ffi_p3c4_collection_roundtrip");
    let output_path = fixture.dir.join("collection-setters-mutated.dwg");
    let output = CString::new(output_path.to_str().unwrap()).unwrap();
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let spline = find_object_of_type(data, DwgObjectType::Spline as i32);
    let lwpolyline = find_object_of_type(data, DwgObjectType::LwPolyline as i32);
    let mesh = find_object_of_type(data, DwgObjectType::Mesh as i32);
    assert!(!spline.is_null());
    assert!(!lwpolyline.is_null());
    assert!(!mesh.is_null());

    assert_eq!(
        unsafe { dwg_entity_spline_set_control_point(data, spline, 1, 10.0, 11.0, 12.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_spline_append_control_point(data, spline, 5.0, 6.0, 7.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_set_vertex(data, lwpolyline, 0, 10.0, 20.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_append_vertex(data, lwpolyline, 2.0, 3.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mesh_append_vertex(data, mesh, 2.0, 2.0, 2.0) },
        DWG_OK
    );
    let appended_face = [0_u32, 1, 4];
    assert_eq!(
        unsafe {
            dwg_entity_mesh_append_face(data, mesh, appended_face.as_ptr(), appended_face.len())
        },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_write_file(output.as_ptr(), data) }, DWG_OK);
    unsafe { dwg_data_destroy(data) };

    let mut reopened: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(output.as_ptr(), reopened) }, DWG_OK);
    let spline = find_object_of_type(reopened, DwgObjectType::Spline as i32);
    let lwpolyline = find_object_of_type(reopened, DwgObjectType::LwPolyline as i32);
    let mesh = find_object_of_type(reopened, DwgObjectType::Mesh as i32);
    assert!(!spline.is_null());
    assert!(!lwpolyline.is_null());
    assert!(!mesh.is_null());

    let mut count = 0;
    assert_eq!(
        unsafe { dwg_entity_spline_get_control_point_count(spline, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 6);
    let (_, spline_points) = unsafe { read_spline_state(spline) };
    assert_eq!(spline_points[1], (10.0, 11.0, 12.0));
    assert_eq!(spline_points[5], (5.0, 6.0, 7.0));

    assert_eq!(
        unsafe { dwg_entity_lwpolyline_get_vertex_count(lwpolyline, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 3);
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 0) },
        (10.0, 20.0)
    );
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 2) },
        (2.0, 3.0)
    );

    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex_count(mesh, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 5);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex(mesh, 4, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert_eq!((x, y, z), (2.0, 2.0, 2.0));
    assert_eq!(
        unsafe { dwg_entity_mesh_get_face_count(mesh, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 2);
    for (slot, expected) in appended_face.iter().copied().enumerate() {
        let mut vertex = u32::MAX;
        assert_eq!(
            unsafe { dwg_entity_mesh_get_face_vertex(mesh, 1, slot as i32, &mut vertex) },
            DWG_OK
        );
        assert_eq!(vertex, expected);
    }

    unsafe { dwg_data_destroy(reopened) };
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&fixture.path);
    let _ = std::fs::remove_dir(&fixture.dir);
}

#[test]
fn phase3c_dxf_setters_roundtrip() {
    use acadrust::entities::{Line, LwPolyline, LwVertex, Text};
    use acadrust::tables::Layer;
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DxfWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_p3c4_dxf_{}_{}",
        std::process::id(),
        suffix
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let source_path = dir.join("source.dxf");
    let output_path = dir.join("mutated.dxf");

    let mut document = CadDocument::new();
    document.layers.add(Layer::new("P3C_DXF")).unwrap();
    document
        .add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 1.0, 2.0, 3.0,
        )))
        .unwrap();
    document
        .add_entity(EntityType::Text(Text::with_value(
            "P3C SOURCE",
            Vector3::new(0.0, 0.0, 0.0),
        )))
        .unwrap();
    let mut lwpolyline = LwPolyline::new();
    lwpolyline.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lwpolyline.add_vertex(LwVertex::from_coords(1.0, 1.0));
    document
        .add_entity(EntityType::LwPolyline(lwpolyline))
        .unwrap();
    DxfWriter::new(&document)
        .write_to_file(&source_path)
        .unwrap();

    let source = CString::new(source_path.to_str().unwrap()).unwrap();
    let output = CString::new(output_path.to_str().unwrap()).unwrap();
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dxf_read_file(source.as_ptr(), data) }, DWG_OK);
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let text = find_object_of_type(data, DwgObjectType::Text as i32);
    let lwpolyline = find_object_of_type(data, DwgObjectType::LwPolyline as i32);
    assert!(!line.is_null());
    assert!(!text.is_null());
    assert!(!lwpolyline.is_null());

    let layer = CString::new("P3C_DXF").unwrap();
    let text_value = CString::new("P3C DXF").unwrap();
    assert_eq!(
        unsafe { dwg_entity_set_layer(data, line, layer.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, line, 11.0, 12.0, 13.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_text_set_value(data, text, text_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_append_vertex(data, lwpolyline, 2.0, 3.0) },
        DWG_OK
    );
    assert_eq!(unsafe { dxf_write_file(output.as_ptr(), data) }, DWG_OK);
    unsafe { dwg_data_destroy(data) };

    let mut reopened: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
    assert_eq!(unsafe { dxf_read_file(output.as_ptr(), reopened) }, DWG_OK);
    let line = find_object_of_type(reopened, DwgObjectType::Line as i32);
    let text = find_object_of_type(reopened, DwgObjectType::Text as i32);
    let lwpolyline = find_object_of_type(reopened, DwgObjectType::LwPolyline as i32);
    assert_eq!(
        unsafe { read_string(dwg_entity_get_layer, line) },
        "P3C_DXF"
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_line_get_start, line) },
        (11.0, 12.0, 13.0)
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_text_get_value, text) },
        "P3C DXF"
    );
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_get_vertex_count(lwpolyline, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 3);
    assert_eq!(
        unsafe { read_collection_vertex2(dwg_entity_lwpolyline_get_vertex, lwpolyline, 2) },
        (2.0, 3.0)
    );

    unsafe { dwg_data_destroy(reopened) };
    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_dir(&dir);
}

unsafe fn read_vec3(
    getter: unsafe extern "C" fn(*const DwgObject, *mut f64, *mut f64, *mut f64) -> i32,
    obj: *const DwgObject,
) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(getter(obj, &mut x, &mut y, &mut z), DWG_OK);
    (x, y, z)
}

unsafe fn read_vec2(
    getter: unsafe extern "C" fn(*const DwgObject, *mut f64, *mut f64) -> i32,
    obj: *const DwgObject,
) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    assert_eq!(getter(obj, &mut x, &mut y), DWG_OK);
    (x, y)
}

unsafe fn read_f64(
    getter: unsafe extern "C" fn(*const DwgObject, *mut f64) -> i32,
    obj: *const DwgObject,
) -> f64 {
    let mut value = 0.0;
    assert_eq!(getter(obj, &mut value), DWG_OK);
    value
}

unsafe fn read_corner(
    getter: unsafe extern "C" fn(*const DwgObject, i32, *mut f64, *mut f64, *mut f64) -> i32,
    obj: *const DwgObject,
    corner: i32,
) -> (f64, f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(getter(obj, corner, &mut x, &mut y, &mut z), DWG_OK);
    (x, y, z)
}

fn assert_last_error(code: i32) {
    assert_eq!(dwg_last_error_code(), code);
    let mut needed = 0;
    assert_eq!(dwg_last_error(std::ptr::null_mut(), 0, &mut needed), DWG_OK);
    assert!(needed > 1);
    let mut buffer = vec![0_i8; needed];
    assert_eq!(
        dwg_last_error(buffer.as_mut_ptr(), buffer.len(), &mut needed),
        code
    );
}

fn last_error_message() -> String {
    let code = dwg_last_error_code();
    let mut needed = 0;
    assert_eq!(dwg_last_error(std::ptr::null_mut(), 0, &mut needed), DWG_OK);
    let mut buffer = vec![0_i8; needed];
    assert_eq!(
        dwg_last_error(buffer.as_mut_ptr(), buffer.len(), &mut needed),
        code
    );
    unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }
        .to_str()
        .unwrap()
        .to_owned()
}

unsafe fn read_spline_state(obj: *const DwgObject) -> (i32, Vec<(f64, f64, f64)>) {
    let mut degree = 0;
    let mut count = 0;
    assert_eq!(dwg_entity_spline_get_degree(obj, &mut degree), DWG_OK);
    assert_eq!(
        dwg_entity_spline_get_control_point_count(obj, &mut count),
        DWG_OK
    );
    let mut points = Vec::with_capacity(count as usize);
    for index in 0..count {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        assert_eq!(
            dwg_entity_spline_get_control_point(obj, index, &mut x, &mut y, &mut z),
            DWG_OK
        );
        points.push((x, y, z));
    }
    (degree, points)
}

fn assert_spline_failure_preserves_state(
    spline: *const DwgObject,
    expected_code: i32,
    operation: impl FnOnce() -> i32,
) {
    let before = unsafe { read_spline_state(spline) };
    assert_eq!(operation(), expected_code);
    assert_last_error(expected_code);
    assert_eq!(unsafe { read_spline_state(spline) }, before);
}

unsafe fn read_string(
    getter: unsafe extern "C" fn(*const DwgObject, *mut i8, usize, *mut usize) -> i32,
    obj: *const DwgObject,
) -> String {
    let mut needed = 0;
    assert_eq!(getter(obj, std::ptr::null_mut(), 0, &mut needed), DWG_OK);
    let mut buffer = vec![0_i8; needed];
    assert_eq!(
        getter(obj, buffer.as_mut_ptr(), buffer.len(), &mut needed),
        DWG_OK
    );
    std::ffi::CStr::from_ptr(buffer.as_ptr())
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn phase3c_text_setters() {
    let (text_data, _text_fixture) = open_sample_document();
    let text = find_object_of_type(text_data, DwgObjectType::Text as i32);
    let line = find_object_of_type(text_data, DwgObjectType::Line as i32);
    assert!(!text.is_null());
    assert!(!line.is_null());

    let (mtext_path, mtext_path_buf) = write_batch1_dwg("acadrust_ffi_p3c2_text");
    let mut mtext_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut mtext_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(mtext_path.as_ptr(), mtext_data) },
        DWG_OK
    );
    let mtext = find_object_of_type(mtext_data, DwgObjectType::MText as i32);
    let mtext_line = find_object_of_type(mtext_data, DwgObjectType::Line as i32);
    assert!(!mtext.is_null());
    assert!(!mtext_line.is_null());

    let text_value = CString::new("P3C TEXT").unwrap();
    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, text, text_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_text_set_insertion_point(text_data, text, 1.0, 2.0, 3.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_text_set_height(text_data, text, 4.0) },
        DWG_OK
    );

    let mtext_value = CString::new("P3C\\PMTEXT").unwrap();
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, mtext, mtext_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_insertion_point(mtext_data, mtext, 5.0, 6.0, 7.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_height(mtext_data, mtext, 2.25) },
        DWG_OK
    );

    assert_eq!(
        unsafe { read_string(dwg_entity_text_get_value, text) },
        "P3C TEXT"
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(unsafe { read_f64(dwg_entity_text_get_height, text) }, 4.0);
    assert_eq!(
        unsafe { read_string(dwg_entity_mtext_get_value, mtext) },
        "P3C\\PMTEXT"
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_mtext_get_height, mtext) },
        2.25
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_value(std::ptr::null_mut(), text, text_value.as_ptr()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_string(dwg_entity_text_get_value, text) },
        "P3C TEXT"
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, std::ptr::null(), mtext_value.as_ptr()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_string(dwg_entity_mtext_get_value, mtext) },
        "P3C\\PMTEXT"
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_insertion_point(text_data, std::ptr::null(), 8.0, 9.0, 10.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe {
            dwg_entity_mtext_set_insertion_point(std::ptr::null_mut(), mtext, 8.0, 9.0, 10.0)
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_height(text_data, std::ptr::null(), 6.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_text_get_height, text) }, 4.0);
    assert_eq!(
        unsafe { dwg_entity_mtext_set_height(std::ptr::null_mut(), mtext, 3.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_f64(dwg_entity_mtext_get_height, mtext) },
        2.25
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_insertion_point(text_data, text, f64::NAN, 2.0, 3.0,) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe { dwg_entity_text_set_insertion_point(text_data, text, 1.0, f64::INFINITY, 3.0,) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_insertion_point(mtext_data, mtext, f64::NAN, 6.0, 7.0,) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_insertion_point(mtext_data, mtext, 5.0, f64::INFINITY, 7.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );

    let empty = CString::new("").unwrap();
    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, text, empty.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, mtext, empty.as_ptr()) },
        DWG_OK
    );
    assert_eq!(unsafe { read_string(dwg_entity_text_get_value, text) }, "");
    assert_eq!(
        unsafe { read_string(dwg_entity_mtext_get_value, mtext) },
        ""
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, text, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_text_get_value, text) }, "");
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, mtext, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_string(dwg_entity_mtext_get_value, mtext) },
        ""
    );

    let invalid_utf8 = [0xff_u8, 0];
    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, text, invalid_utf8.as_ptr().cast()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_text_get_value, text) }, "");
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, mtext, invalid_utf8.as_ptr().cast()) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_string(dwg_entity_mtext_get_value, mtext) },
        ""
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_height(text_data, text, 0.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_text_get_height, text) }, 4.0);
    assert_eq!(
        unsafe { dwg_entity_mtext_set_height(mtext_data, mtext, f64::NAN) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_f64(dwg_entity_mtext_get_height, mtext) },
        2.25
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, line, text_value.as_ptr()) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_eq!(unsafe { read_string(dwg_entity_text_get_value, text) }, "");
    assert_eq!(
        unsafe { dwg_entity_mtext_set_insertion_point(mtext_data, mtext_line, 8.0, 9.0, 10.0) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );

    let (foreign_data, _foreign_fixture) = open_sample_document();
    let foreign_text = find_object_of_type(foreign_data, DwgObjectType::Text as i32);
    assert_eq!(
        unsafe { dwg_entity_text_set_height(text_data, foreign_text, 6.0) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_eq!(unsafe { read_f64(dwg_entity_text_get_height, text) }, 4.0);

    let (foreign_mtext_path, foreign_mtext_path_buf) =
        write_batch1_dwg("acadrust_ffi_p3c2_text_foreign");
    let mut foreign_mtext_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_mtext_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(foreign_mtext_path.as_ptr(), foreign_mtext_data) },
        DWG_OK
    );
    let foreign_mtext = find_object_of_type(foreign_mtext_data, DwgObjectType::MText as i32);
    assert!(!foreign_mtext.is_null());
    assert_eq!(
        unsafe { dwg_entity_mtext_set_height(mtext_data, foreign_mtext, 3.25) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_eq!(
        unsafe { read_f64(dwg_entity_mtext_get_height, mtext) },
        2.25
    );

    assert_eq!(
        unsafe { dwg_entity_text_set_value(text_data, text, text_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_mtext_set_value(mtext_data, mtext, mtext_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(foreign_mtext_data) };
    unsafe { dwg_data_destroy(mtext_data) };
    let foreign_mtext_dir = foreign_mtext_path_buf
        .parent()
        .map(|path| path.to_path_buf());
    let _ = std::fs::remove_file(foreign_mtext_path_buf);
    if let Some(foreign_mtext_dir) = foreign_mtext_dir {
        let _ = std::fs::remove_dir(foreign_mtext_dir);
    }
    let mtext_dir = mtext_path_buf.parent().map(|path| path.to_path_buf());
    let _ = std::fs::remove_file(mtext_path_buf);
    if let Some(mtext_dir) = mtext_dir {
        let _ = std::fs::remove_dir(mtext_dir);
    }
    unsafe { dwg_data_destroy(text_data) };
}

#[test]
fn phase3c_basic_geometry_setters_roundtrip_through_getters() {
    let (path, path_buf) = write_batch1_dwg("acadrust_ffi_p3c1");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(dwg_data_create(&mut data), DWG_OK);
    assert_eq!(dwg_read_file(path.as_ptr(), data), DWG_OK);

    let p1 = (11.0, 12.0, 13.0);
    let p2 = (21.0, 22.0, 23.0);
    let radius = 4.5;
    let start_angle = 0.25;
    let end_angle = 2.75;
    let ratio = 0.625;
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let circle = find_object_of_type(data, DwgObjectType::Circle as i32);
    let arc = find_object_of_type(data, DwgObjectType::Arc as i32);
    let point = find_object_of_type(data, DwgObjectType::Point as i32);
    let ellipse = find_object_of_type(data, DwgObjectType::Ellipse as i32);
    let ray = find_object_of_type(data, DwgObjectType::Ray as i32);
    let xline = find_object_of_type(data, DwgObjectType::XLine as i32);
    let solid = find_object_of_type(data, DwgObjectType::Solid as i32);
    let face3d = find_object_of_type(data, DwgObjectType::Face3D as i32);

    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, line, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_line_set_end(data, line, p2.0, p2.1, p2.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_circle_set_center(data, circle, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_circle_set_radius(data, circle, radius) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_center(data, arc, p2.0, p2.1, p2.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_radius(data, arc, radius) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_start_angle(data, arc, start_angle) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_end_angle(data, arc, end_angle) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_point_set_location(data, point, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_center(data, ellipse, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_major_axis(data, ellipse, p2.0, p2.1, p2.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, ratio) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ray_set_base_point(data, ray, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ray_set_direction(data, ray, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_xline_set_base_point(data, xline, p2.0, p2.1, p2.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_xline_set_direction(data, xline, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_solid_set_corner(data, solid, 1, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_solid_set_corner(data, solid, 4, p2.0, p2.1, p2.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_face3d_set_corner(data, face3d, 1, p1.0, p1.1, p1.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_face3d_set_corner(data, face3d, 4, p2.0, p2.1, p2.2) },
        DWG_OK
    );

    assert_eq!(unsafe { read_vec3(dwg_entity_line_get_start, line) }, p1);
    assert_eq!(unsafe { read_vec3(dwg_entity_line_get_end, line) }, p2);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_circle_get_center, circle) },
        p1
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_circle_get_radius, circle) },
        radius
    );
    assert_eq!(unsafe { read_vec3(dwg_entity_arc_get_center, arc) }, p2);
    assert_eq!(unsafe { read_f64(dwg_entity_arc_get_radius, arc) }, radius);
    assert_eq!(
        unsafe { read_f64(dwg_entity_arc_get_start_angle, arc) },
        start_angle
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_arc_get_end_angle, arc) },
        end_angle
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_point_get_location, point) },
        p1
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_ellipse_get_center, ellipse) },
        p1
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_ellipse_get_major_axis, ellipse) },
        p2
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_ellipse_get_minor_axis_ratio, ellipse) },
        ratio
    );
    assert_eq!(unsafe { read_vec3(dwg_entity_ray_get_base_point, ray) }, p1);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_xline_get_base_point, xline) },
        p2
    );
    for direction in [
        unsafe { read_vec3(dwg_entity_ray_get_direction, ray) },
        unsafe { read_vec3(dwg_entity_xline_get_direction, xline) },
    ] {
        assert!((direction.0 - 11.0 / 434.0_f64.sqrt()).abs() < 1e-12);
        assert!((direction.1 - 12.0 / 434.0_f64.sqrt()).abs() < 1e-12);
        assert!((direction.2 - 13.0 / 434.0_f64.sqrt()).abs() < 1e-12);
    }
    assert_eq!(
        unsafe { read_corner(dwg_entity_solid_get_corner, solid, 1) },
        p1
    );
    assert_eq!(
        unsafe { read_corner(dwg_entity_solid_get_corner, solid, 4) },
        p2
    );
    assert_eq!(
        unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 1) },
        p1
    );
    assert_eq!(
        unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 4) },
        p2
    );

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(path_buf);
}

#[test]
fn phase3c_direction_setters_normalize_extreme_finite_vectors() {
    let (path, path_buf) = write_batch1_dwg("acadrust_ffi_p3c1_extreme");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(dwg_data_create(&mut data), DWG_OK);
    assert_eq!(dwg_read_file(path.as_ptr(), data), DWG_OK);
    let ray = find_object_of_type(data, DwgObjectType::Ray as i32);
    let xline = find_object_of_type(data, DwgObjectType::XLine as i32);

    assert_eq!(
        unsafe {
            dwg_entity_ray_set_direction(data, ray, f64::MAX, f64::MAX / 2.0, -f64::MAX / 4.0)
        },
        DWG_OK
    );
    assert_eq!(
        unsafe {
            dwg_entity_xline_set_direction(
                data,
                xline,
                f64::MIN_POSITIVE,
                f64::MIN_POSITIVE / 2.0,
                -f64::MIN_POSITIVE / 4.0,
            )
        },
        DWG_OK
    );
    for direction in [
        unsafe { read_vec3(dwg_entity_ray_get_direction, ray) },
        unsafe { read_vec3(dwg_entity_xline_get_direction, xline) },
    ] {
        assert!(direction.0.is_finite() && direction.1.is_finite() && direction.2.is_finite());
        assert!(
            (direction.0 * direction.0 + direction.1 * direction.1 + direction.2 * direction.2
                - 1.0)
                .abs()
                < 1e-12
        );
    }

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(path_buf);
}

#[test]
fn phase3c_geometry_setters_reject_invalid_inputs_without_mutation() {
    let (path, path_buf) = write_batch1_dwg("acadrust_ffi_p3c1_invalid");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(dwg_data_create(&mut data), DWG_OK);
    assert_eq!(dwg_read_file(path.as_ptr(), data), DWG_OK);
    let line = find_object_of_type(data, DwgObjectType::Line as i32);
    let circle = find_object_of_type(data, DwgObjectType::Circle as i32);
    let arc = find_object_of_type(data, DwgObjectType::Arc as i32);
    let ellipse = find_object_of_type(data, DwgObjectType::Ellipse as i32);
    let ray = find_object_of_type(data, DwgObjectType::Ray as i32);
    let xline = find_object_of_type(data, DwgObjectType::XLine as i32);
    let solid = find_object_of_type(data, DwgObjectType::Solid as i32);
    let face3d = find_object_of_type(data, DwgObjectType::Face3D as i32);
    let stable = (11.0, 12.0, 13.0);

    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, line, stable.0, stable.1, stable.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_circle_set_center(data, circle, stable.0, stable.1, stable.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_circle_set_radius(data, circle, 4.5) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_start_angle(data, arc, 0.25) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 0.625) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_ray_set_direction(data, ray, 3.0, 4.0, 0.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_xline_set_direction(data, xline, 3.0, 4.0, 0.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_solid_set_corner(data, solid, 1, stable.0, stable.1, stable.2) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_face3d_set_corner(data, face3d, 1, stable.0, stable.1, stable.2) },
        DWG_OK
    );

    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, line, f64::NAN, 0.0, 0.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_line_get_start, line) },
        stable
    );
    assert_eq!(
        unsafe { dwg_entity_arc_set_start_angle(data, arc, f64::INFINITY) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_f64(dwg_entity_arc_get_start_angle, arc) },
        0.25
    );
    assert_eq!(
        unsafe { dwg_entity_circle_set_radius(data, circle, 0.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_entity_circle_set_radius(data, circle, f64::NAN) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_f64(dwg_entity_circle_get_radius, circle) },
        4.5
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 0.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 1.1) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, f64::NAN) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_f64(dwg_entity_ellipse_get_minor_axis_ratio, ellipse) },
        0.625
    );
    assert_eq!(
        unsafe { dwg_entity_ray_set_direction(data, ray, 0.0, 0.0, 0.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_xline_set_direction(data, xline, f64::NAN, 0.0, 0.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_ray_get_direction, ray) },
        (0.6, 0.8, 0.0)
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_xline_get_direction, xline) },
        (0.6, 0.8, 0.0)
    );
    assert_eq!(
        unsafe { dwg_entity_solid_set_corner(data, solid, 0, 1.0, 2.0, 3.0) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_face3d_set_corner(data, face3d, 5, 1.0, 2.0, 3.0) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_corner(dwg_entity_solid_get_corner, solid, 1) },
        stable
    );
    assert_eq!(
        unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 1) },
        stable
    );
    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, circle, 1.0, 2.0, 3.0) },
        DWG_ETYPE
    );
    assert_last_error(DWG_ETYPE);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_circle_get_center, circle) },
        stable
    );
    assert_eq!(
        unsafe { dwg_entity_line_set_start(data, line, 21.0, 22.0, 23.0) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(path_buf);
}

#[test]
fn batch1_geometry_getters() {
    let (path, path_buf) = write_batch1_dwg("acadrust_ffi_p3b");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);

    let ell = find_object_of_type(data, acadrust::ffi::DwgObjectType::Ellipse as i32);
    assert!(!ell.is_null());
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    assert_eq!(
        unsafe { dwg_entity_ellipse_get_center(ell, &mut cx, &mut cy, &mut cz) },
        DWG_OK
    );
    assert!((cx - 1.0).abs() < 1e-9 && (cy - 2.0).abs() < 1e-9);
    let mut ratio = 0.0;
    assert_eq!(
        unsafe { dwg_entity_ellipse_get_minor_axis_ratio(ell, &mut ratio) },
        DWG_OK
    );
    assert!((ratio - 0.5).abs() < 1e-9);

    let mtx = find_object_of_type(data, acadrust::ffi::DwgObjectType::MText as i32);
    assert!(!mtx.is_null());
    let mut needed = 0usize;
    assert_eq!(
        unsafe { dwg_entity_mtext_get_value(mtx, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    assert_eq!(needed, 3);
    let mut h = 0.0;
    assert_eq!(unsafe { dwg_entity_mtext_get_height(mtx, &mut h) }, DWG_OK);
    assert!((h - 2.0).abs() < 1e-9);

    let sp = find_object_of_type(data, acadrust::ffi::DwgObjectType::Spline as i32);
    assert!(!sp.is_null());
    let mut degree = 0;
    assert_eq!(
        unsafe { dwg_entity_spline_get_degree(sp, &mut degree) },
        DWG_OK
    );
    assert_eq!(degree, 3);
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_entity_spline_get_control_point_count(sp, &mut count) },
        DWG_OK
    );
    assert!(count >= 3);

    let lw = find_object_of_type(data, acadrust::ffi::DwgObjectType::LwPolyline as i32);
    assert!(!lw.is_null());
    let mut vcount = 0;
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_get_vertex_count(lw, &mut vcount) },
        DWG_OK
    );
    assert_eq!(vcount, 2);
    let mut vx = 0.0;
    let mut vy = 0.0;
    assert_eq!(
        unsafe { dwg_entity_lwpolyline_get_vertex(lw, 1, &mut vx, &mut vy) },
        DWG_OK
    );
    assert!((vx - 10.0).abs() < 1e-9 && (vy - 5.0).abs() < 1e-9);

    // Common getters on the line-family sample from the first document.
    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(&path_buf);
}

fn write_batch2_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{Insert, Mesh, Polyline2D, Vertex2D, Viewport};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch2.dwg");

    let mut doc = CadDocument::new();
    let ins = Insert::new("WALL", Vector3::new(1.0, 2.0, 3.0));
    doc.add_entity(EntityType::Insert(ins)).unwrap();

    let mut p2 = Polyline2D::new();
    p2.add_vertex(Vertex2D::new(acadrust::types::Vector3::new(0.0, 0.0, 0.0)));
    p2.add_vertex(Vertex2D::new(acadrust::types::Vector3::new(5.0, 6.0, 0.0)));
    doc.add_entity(EntityType::Polyline2D(p2)).unwrap();

    let mut mesh = Mesh::new();
    mesh.add_vertex(Vector3::new(0.0, 0.0, 0.0));
    mesh.add_vertex(Vector3::new(1.0, 0.0, 0.0));
    mesh.add_vertex(Vector3::new(0.0, 1.0, 0.0));
    doc.add_entity(EntityType::Mesh(mesh)).unwrap();

    let vp = Viewport::new();
    doc.add_entity(EntityType::Viewport(Box::new(vp))).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn batch2_block_mesh_getters() {
    let fixture = write_batch2_dwg("acadrust_ffi_p3b2");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let ins = find_object_of_type(data, acadrust::ffi::DwgObjectType::Insert as i32);
    assert!(!ins.is_null());
    let mut needed = 0usize;
    assert_eq!(
        unsafe { dwg_entity_insert_get_block_name(ins, std::ptr::null_mut(), 0, &mut needed) },
        DWG_OK
    );
    let mut name_buf = [0u8; 64];
    assert_eq!(
        unsafe {
            dwg_entity_insert_get_block_name(
                ins,
                name_buf.as_mut_ptr() as *mut _,
                name_buf.len(),
                &mut needed,
            )
        },
        DWG_OK
    );
    eprintln!("block_name bytes: {:?}", &name_buf[..needed - 1]);
    assert!(needed >= 2, "block_name must be non-empty after round-trip");
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_insert_get_insert_point(ins, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert!((x - 1.0).abs() < 1e-9 && (y - 2.0).abs() < 1e-9 && (z - 3.0).abs() < 1e-9);

    let p2 = find_object_of_type(data, acadrust::ffi::DwgObjectType::Polyline2D as i32);
    assert!(!p2.is_null());
    let mut vcount = 0;
    assert_eq!(
        unsafe { dwg_entity_polyline2d_get_vertex_count(p2, &mut vcount) },
        DWG_OK
    );
    assert_eq!(vcount, 2);
    assert_eq!(
        unsafe { dwg_entity_polyline2d_get_vertex(p2, 1, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert!((x - 5.0).abs() < 1e-9 && (y - 6.0).abs() < 1e-9);

    let mesh = find_object_of_type(data, acadrust::ffi::DwgObjectType::Mesh as i32);
    assert!(!mesh.is_null());
    let mut mcount = 0;
    assert_eq!(
        unsafe { dwg_entity_mesh_get_vertex_count(mesh, &mut mcount) },
        DWG_OK
    );
    assert_eq!(mcount, 3);

    let vp = find_object_of_type(data, acadrust::ffi::DwgObjectType::Viewport as i32);
    assert!(!vp.is_null());
    let mut w = 0.0;
    let mut h = 0.0;
    assert_eq!(unsafe { dwg_entity_viewport_get_width(vp, &mut w) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_entity_viewport_get_height(vp, &mut h) },
        DWG_OK
    );
    assert!(w > 0.0 && h > 0.0);

    unsafe { dwg_data_destroy(data) };
}

fn write_batch3_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{
        AttributeDefinition, Hatch, HatchPattern, Helix, Leader, MLine, Shape, Tolerance,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch3.dwg");

    let mut doc = CadDocument::new();

    let mut h = Hatch::new();
    h.pattern = HatchPattern::new("SOLID");
    h.pattern_angle = 0.25;
    doc.add_entity(EntityType::Hatch(Box::new(h))).unwrap();

    let mut tol = Tolerance::new();
    tol.insertion_point = Vector3::new(1.0, 2.0, 0.0);
    tol.text = "TOL".to_string();
    doc.add_entity(EntityType::Tolerance(tol)).unwrap();

    let mut l = Leader::new();
    l.vertices = vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 10.0, 0.0)];
    doc.add_entity(EntityType::Leader(l)).unwrap();

    let mut a = AttributeDefinition::new("TAG1".to_string(), "Enter".to_string(), "D".to_string());
    a.insertion_point = Vector3::new(3.0, 4.0, 0.0);
    doc.add_entity(EntityType::AttributeDefinition(a)).unwrap();

    let mut s = Shape::new();
    s.insertion_point = Vector3::new(5.0, 6.0, 0.0);
    s.size = 2.0;
    s.rotation = 0.5;
    doc.add_entity(EntityType::Shape(s)).unwrap();

    let mut ml = MLine::from_points(&[Vector3::new(7.0, 8.0, 0.0), Vector3::new(9.0, 10.0, 0.0)]);
    ml.style_name = "Standard".to_string();
    doc.add_entity(EntityType::MLine(ml)).unwrap();

    let mut hx = Helix::new();
    hx.axis_base_point = Vector3::new(0.0, 0.0, 0.0);
    hx.radius = 3.0;
    hx.turns = 4.0;
    doc.add_entity(EntityType::Helix(Box::new(hx))).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn batch3_annotation_text_getters() {
    let fixture = write_batch3_dwg("acadrust_ffi_p3b3");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let mut needed = 0usize;
    let mut buf = [0u8; 64];

    let h = find_object_of_type(data, acadrust::ffi::DwgObjectType::Hatch as i32);
    assert!(!h.is_null());
    assert_eq!(
        unsafe {
            dwg_entity_hatch_get_pattern_name(h, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed)
        },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"SOLID");

    let tol = find_object_of_type(data, acadrust::ffi::DwgObjectType::Tolerance as i32);
    assert!(!tol.is_null());
    assert_eq!(
        unsafe {
            dwg_entity_tolerance_get_text(tol, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed)
        },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"TOL");

    let ldr = find_object_of_type(data, acadrust::ffi::DwgObjectType::Leader as i32);
    assert!(!ldr.is_null());
    let mut vcount = 0;
    assert_eq!(
        unsafe { dwg_entity_leader_get_vertex_count(ldr, &mut vcount) },
        DWG_OK
    );
    assert_eq!(vcount, 2);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_leader_get_vertex(ldr, 1, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert!((x - 10.0).abs() < 1e-9 && (y - 10.0).abs() < 1e-9);

    let att = find_object_of_type(
        data,
        acadrust::ffi::DwgObjectType::AttributeDefinition as i32,
    );
    assert!(!att.is_null());
    assert_eq!(
        unsafe {
            dwg_entity_attrdef_get_tag(att, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed)
        },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"TAG1");

    let shp = find_object_of_type(data, acadrust::ffi::DwgObjectType::Shape as i32);
    assert!(!shp.is_null());
    let mut size = 0.0;
    assert_eq!(unsafe { dwg_entity_shape_get_size(shp, &mut size) }, DWG_OK);
    assert!((size - 2.0).abs() < 1e-9);

    let hx = find_object_of_type(data, acadrust::ffi::DwgObjectType::Helix as i32);
    assert!(!hx.is_null());
    let mut radius = 0.0;
    assert_eq!(
        unsafe { dwg_entity_helix_get_radius(hx, &mut radius) },
        DWG_OK
    );
    assert!((radius - 3.0).abs() < 1e-9);
    let mut turns = 0.0;
    assert_eq!(
        unsafe { dwg_entity_helix_get_turns(hx, &mut turns) },
        DWG_OK
    );
    assert!((turns - 4.0).abs() < 1e-9);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_annotation_setters() {
    macro_rules! assert_failure_preserves {
        ($call:expr, $code:expr, $read:expr, $expected:expr) => {{
            assert_eq!(unsafe { $call }, $code);
            assert_last_error($code);
            assert_eq!(unsafe { $read }, $expected);
        }};
    }

    let fixture = write_batch3_dwg("acadrust_ffi_p3c2_annotation");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let attrdef = find_object_of_type(data, DwgObjectType::AttributeDefinition as i32);
    let tolerance = find_object_of_type(data, DwgObjectType::Tolerance as i32);
    let mline = find_object_of_type(data, DwgObjectType::MLine as i32);
    let shape = find_object_of_type(data, DwgObjectType::Shape as i32);
    assert!(!attrdef.is_null() && !tolerance.is_null() && !mline.is_null() && !shape.is_null());

    let foreign_fixture = write_batch3_dwg("acadrust_ffi_p3c2_annotation_foreign");
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );
    let foreign_attrdef =
        find_object_of_type(foreign_data, DwgObjectType::AttributeDefinition as i32);
    let foreign_tolerance = find_object_of_type(foreign_data, DwgObjectType::Tolerance as i32);
    let foreign_mline = find_object_of_type(foreign_data, DwgObjectType::MLine as i32);
    let foreign_shape = find_object_of_type(foreign_data, DwgObjectType::Shape as i32);
    assert!(
        !foreign_attrdef.is_null()
            && !foreign_tolerance.is_null()
            && !foreign_mline.is_null()
            && !foreign_shape.is_null()
    );

    let tag = CString::new("P3C_TAG").unwrap();
    let prompt = CString::new("P3C_PROMPT").unwrap();
    let default_value = CString::new("P3C_DEFAULT").unwrap();
    let text = CString::new("P3C_TOL").unwrap();
    let style = CString::new("P3C_STYLE").unwrap();
    let invalid_utf8 = [0xff_u8, 0];

    // AttributeDefinition success paths: every setter is read back immediately.
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_tag(data, attrdef, tag.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_attrdef_get_tag, attrdef) },
        "P3C_TAG"
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_prompt(data, attrdef, prompt.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_attrdef_get_prompt, attrdef) },
        "P3C_PROMPT"
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_default_value(data, attrdef, default_value.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_attrdef_get_default_value, attrdef) },
        "P3C_DEFAULT"
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_insertion_point(data, attrdef, 10.0, 11.0, 12.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef) },
        (10.0, 11.0, 12.0)
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_height(data, attrdef, 3.5) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_attrdef_get_height, attrdef) },
        3.5
    );

    // AttributeDefinition string validation and routing.
    assert_failure_preserves!(
        dwg_entity_attrdef_set_tag(std::ptr::null_mut(), attrdef, tag.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_attrdef_get_tag, attrdef),
        "P3C_TAG"
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_prompt(data, std::ptr::null(), prompt.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_attrdef_get_prompt, attrdef),
        "P3C_PROMPT"
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_default_value(data, attrdef, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_attrdef_get_default_value, attrdef),
        "P3C_DEFAULT"
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_tag(data, attrdef, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_attrdef_get_tag, attrdef),
        "P3C_TAG"
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_tag(data, tolerance, tag.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_attrdef_get_tag, attrdef),
        "P3C_TAG"
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_tag(data, foreign_attrdef, tag.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_attrdef_get_tag, attrdef),
        "P3C_TAG"
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_tag(data, attrdef, tag.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_attrdef_get_tag, attrdef) },
        "P3C_TAG"
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    // AttributeDefinition vector validation and routing.
    assert_failure_preserves!(
        dwg_entity_attrdef_set_insertion_point(std::ptr::null_mut(), attrdef, 10.0, 11.0, 12.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef),
        (10.0, 11.0, 12.0)
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_insertion_point(data, std::ptr::null(), 10.0, 11.0, 12.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef),
        (10.0, 11.0, 12.0)
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_insertion_point(data, attrdef, f64::NAN, 11.0, 12.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef),
        (10.0, 11.0, 12.0)
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_insertion_point(data, tolerance, 10.0, 11.0, 12.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef),
        (10.0, 11.0, 12.0)
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_insertion_point(data, foreign_attrdef, 10.0, 11.0, 12.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef),
        (10.0, 11.0, 12.0)
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_insertion_point(data, attrdef, 10.0, 11.0, 12.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef) },
        (10.0, 11.0, 12.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    // AttributeDefinition positive-scalar validation and routing.
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(std::ptr::null_mut(), attrdef, 3.5),
        DWG_EINVAL,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(data, std::ptr::null(), 3.5),
        DWG_EINVAL,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(data, attrdef, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(data, attrdef, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(data, tolerance, 3.5),
        DWG_ETYPE,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_failure_preserves!(
        dwg_entity_attrdef_set_height(data, foreign_attrdef, 3.5),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_attrdef_get_height, attrdef),
        3.5
    );
    assert_eq!(
        unsafe { dwg_entity_attrdef_set_height(data, attrdef, 3.5) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_attrdef_get_height, attrdef) },
        3.5
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    // Tolerance success paths.
    assert_eq!(
        unsafe { dwg_entity_tolerance_set_text(data, tolerance, text.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_tolerance_get_text, tolerance) },
        "P3C_TOL"
    );
    assert_eq!(
        unsafe { dwg_entity_tolerance_set_insertion_point(data, tolerance, 20.0, 21.0, 22.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance) },
        (20.0, 21.0, 22.0)
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(std::ptr::null_mut(), tolerance, text.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(data, std::ptr::null(), text.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(data, tolerance, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(data, tolerance, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(data, attrdef, text.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_text(data, foreign_tolerance, text.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_tolerance_get_text, tolerance),
        "P3C_TOL"
    );
    assert_eq!(
        unsafe { dwg_entity_tolerance_set_text(data, tolerance, text.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_tolerance_get_text, tolerance) },
        "P3C_TOL"
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_failure_preserves!(
        dwg_entity_tolerance_set_insertion_point(std::ptr::null_mut(), tolerance, 20.0, 21.0, 22.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance),
        (20.0, 21.0, 22.0)
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_insertion_point(data, std::ptr::null(), 20.0, 21.0, 22.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance),
        (20.0, 21.0, 22.0)
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_insertion_point(data, tolerance, f64::INFINITY, 21.0, 22.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance),
        (20.0, 21.0, 22.0)
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_insertion_point(data, attrdef, 20.0, 21.0, 22.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance),
        (20.0, 21.0, 22.0)
    );
    assert_failure_preserves!(
        dwg_entity_tolerance_set_insertion_point(data, foreign_tolerance, 20.0, 21.0, 22.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance),
        (20.0, 21.0, 22.0)
    );
    assert_eq!(
        unsafe { dwg_entity_tolerance_set_insertion_point(data, tolerance, 20.0, 21.0, 22.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance) },
        (20.0, 21.0, 22.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    // MLine success paths.
    assert_eq!(
        unsafe { dwg_entity_mline_set_style_name(data, mline, style.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_mline_get_style_name, mline) },
        "P3C_STYLE"
    );
    assert_eq!(
        unsafe { dwg_entity_mline_set_start_point(data, mline, 30.0, 31.0, 32.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mline_get_start_point, mline) },
        (30.0, 31.0, 32.0)
    );
    assert_eq!(
        unsafe { dwg_entity_mline_set_scale_factor(data, mline, 1.75) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_mline_get_scale_factor, mline) },
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(std::ptr::null_mut(), mline, style.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(data, std::ptr::null(), style.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(data, mline, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(data, mline, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(data, shape, style.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_style_name(data, foreign_mline, style.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_mline_get_style_name, mline),
        "P3C_STYLE"
    );
    assert_eq!(
        unsafe { dwg_entity_mline_set_style_name(data, mline, style.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_mline_get_style_name, mline) },
        "P3C_STYLE"
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_failure_preserves!(
        dwg_entity_mline_set_start_point(std::ptr::null_mut(), mline, 30.0, 31.0, 32.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_mline_get_start_point, mline),
        (30.0, 31.0, 32.0)
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_start_point(data, std::ptr::null(), 30.0, 31.0, 32.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_mline_get_start_point, mline),
        (30.0, 31.0, 32.0)
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_start_point(data, mline, 30.0, f64::NAN, 32.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_mline_get_start_point, mline),
        (30.0, 31.0, 32.0)
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_start_point(data, shape, 30.0, 31.0, 32.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_mline_get_start_point, mline),
        (30.0, 31.0, 32.0)
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_start_point(data, foreign_mline, 30.0, 31.0, 32.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_mline_get_start_point, mline),
        (30.0, 31.0, 32.0)
    );
    assert_eq!(
        unsafe { dwg_entity_mline_set_start_point(data, mline, 30.0, 31.0, 32.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mline_get_start_point, mline) },
        (30.0, 31.0, 32.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(std::ptr::null_mut(), mline, 1.75),
        DWG_EINVAL,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(data, std::ptr::null(), 1.75),
        DWG_EINVAL,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(data, mline, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(data, mline, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(data, shape, 1.75),
        DWG_ETYPE,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_failure_preserves!(
        dwg_entity_mline_set_scale_factor(data, foreign_mline, 1.75),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_mline_get_scale_factor, mline),
        1.75
    );
    assert_eq!(
        unsafe { dwg_entity_mline_set_scale_factor(data, mline, 1.75) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_mline_get_scale_factor, mline) },
        1.75
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    // Shape success paths.
    assert_eq!(
        unsafe { dwg_entity_shape_set_insertion_point(data, shape, 40.0, 41.0, 42.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_shape_get_insertion_point, shape) },
        (40.0, 41.0, 42.0)
    );
    assert_eq!(
        unsafe { dwg_entity_shape_set_size(data, shape, 2.5) },
        DWG_OK
    );
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);
    assert_eq!(
        unsafe { dwg_entity_shape_set_rotation(data, shape, 0.5) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_shape_get_rotation, shape) },
        0.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_insertion_point(std::ptr::null_mut(), shape, 40.0, 41.0, 42.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_shape_get_insertion_point, shape),
        (40.0, 41.0, 42.0)
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_insertion_point(data, std::ptr::null(), 40.0, 41.0, 42.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_shape_get_insertion_point, shape),
        (40.0, 41.0, 42.0)
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_insertion_point(data, shape, 40.0, f64::INFINITY, 42.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_shape_get_insertion_point, shape),
        (40.0, 41.0, 42.0)
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_insertion_point(data, tolerance, 40.0, 41.0, 42.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_shape_get_insertion_point, shape),
        (40.0, 41.0, 42.0)
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_insertion_point(data, foreign_shape, 40.0, 41.0, 42.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_shape_get_insertion_point, shape),
        (40.0, 41.0, 42.0)
    );
    assert_eq!(
        unsafe { dwg_entity_shape_set_insertion_point(data, shape, 40.0, 41.0, 42.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_shape_get_insertion_point, shape) },
        (40.0, 41.0, 42.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_failure_preserves!(
        dwg_entity_shape_set_size(std::ptr::null_mut(), shape, 2.5),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_size(data, std::ptr::null(), 2.5),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_size(data, shape, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_size(data, shape, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_size(data, tolerance, 2.5),
        DWG_ETYPE,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_size(data, foreign_shape, 2.5),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_shape_get_size, shape),
        2.5
    );
    assert_eq!(
        unsafe { dwg_entity_shape_set_size(data, shape, 2.5) },
        DWG_OK
    );
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_failure_preserves!(
        dwg_entity_shape_set_rotation(std::ptr::null_mut(), shape, 0.5),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_rotation, shape),
        0.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_rotation(data, std::ptr::null(), 0.5),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_rotation, shape),
        0.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_rotation(data, shape, f64::INFINITY),
        DWG_EINVAL,
        read_f64(dwg_entity_shape_get_rotation, shape),
        0.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_rotation(data, tolerance, 0.5),
        DWG_ETYPE,
        read_f64(dwg_entity_shape_get_rotation, shape),
        0.5
    );
    assert_failure_preserves!(
        dwg_entity_shape_set_rotation(data, foreign_shape, 0.5),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_shape_get_rotation, shape),
        0.5
    );
    assert_eq!(
        unsafe { dwg_entity_shape_set_rotation(data, shape, 0.5) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_shape_get_rotation, shape) },
        0.5
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_insert_viewport_hatch_setters() {
    macro_rules! assert_failure_preserves {
        ($call:expr, $code:expr, $read:expr, $expected:expr) => {{
            assert_eq!(unsafe { $call }, $code);
            assert_last_error($code);
            assert_eq!(unsafe { $read }, $expected);
        }};
    }
    macro_rules! read_array {
        ($obj:expr) => {{
            let mut columns = 0;
            let mut rows = 0;
            assert_eq!(
                unsafe { dwg_entity_insert_get_array($obj, &mut columns, &mut rows) },
                DWG_OK
            );
            (columns, rows)
        }};
    }

    let fixture = write_batch2_dwg("acadrust_ffi_p3c2_insert_viewport");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );
    let insert = find_object_of_type(data, DwgObjectType::Insert as i32);
    let viewport = find_object_of_type(data, DwgObjectType::Viewport as i32);
    assert!(!insert.is_null() && !viewport.is_null());

    let foreign_fixture = write_batch2_dwg("acadrust_ffi_p3c2_insert_viewport_foreign");
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );
    let foreign_insert = find_object_of_type(foreign_data, DwgObjectType::Insert as i32);
    let foreign_viewport = find_object_of_type(foreign_data, DwgObjectType::Viewport as i32);
    assert!(!foreign_insert.is_null() && !foreign_viewport.is_null());

    let hatch_fixture = write_batch3_dwg("acadrust_ffi_p3c2_hatch");
    let mut hatch_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut hatch_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(hatch_fixture.c_path().as_ptr(), hatch_data) },
        DWG_OK
    );
    let hatch = find_object_of_type(hatch_data, DwgObjectType::Hatch as i32);
    let tolerance = find_object_of_type(hatch_data, DwgObjectType::Tolerance as i32);

    assert!(!hatch.is_null() && !tolerance.is_null());

    let foreign_hatch_fixture = write_batch3_dwg("acadrust_ffi_p3c2_hatch_foreign");
    let mut foreign_hatch_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_hatch_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(foreign_hatch_fixture.c_path().as_ptr(), foreign_hatch_data) },
        DWG_OK
    );
    let foreign_hatch = find_object_of_type(foreign_hatch_data, DwgObjectType::Hatch as i32);
    assert!(!foreign_hatch.is_null());

    let block_name = CString::new("P3C_BLOCK").unwrap();
    let pattern_name = CString::new("ANSI31").unwrap();
    let empty = CString::new("").unwrap();
    let invalid_utf8 = [0xff_u8, 0];

    assert_eq!(
        unsafe { dwg_entity_insert_set_block_name(data, insert, block_name.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_insert_get_block_name, insert) },
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(std::ptr::null_mut(), insert, block_name.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, std::ptr::null(), block_name.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, insert, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, insert, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, insert, empty.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, viewport, block_name.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_block_name(data, foreign_insert, block_name.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_insert_get_block_name, insert),
        "P3C_BLOCK"
    );
    assert_eq!(
        unsafe { dwg_entity_insert_set_block_name(data, insert, block_name.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_insert_get_block_name, insert) },
        "P3C_BLOCK"
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_insert_set_insert_point(data, insert, 1.0, 2.0, 3.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_insert_get_insert_point, insert) },
        (1.0, 2.0, 3.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_insert_point(std::ptr::null_mut(), insert, 1.0, 2.0, 3.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_insert_point, insert),
        (1.0, 2.0, 3.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_insert_point(data, std::ptr::null(), 1.0, 2.0, 3.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_insert_point, insert),
        (1.0, 2.0, 3.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_insert_point(data, insert, f64::NAN, 2.0, 3.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_insert_point, insert),
        (1.0, 2.0, 3.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_insert_point(data, viewport, 1.0, 2.0, 3.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_insert_get_insert_point, insert),
        (1.0, 2.0, 3.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_insert_point(data, foreign_insert, 1.0, 2.0, 3.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_insert_get_insert_point, insert),
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe { dwg_entity_insert_set_insert_point(data, insert, 1.0, 2.0, 3.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_insert_get_insert_point, insert) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_insert_set_rotation(data, insert, 0.75) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_insert_get_rotation, insert) },
        0.75
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_rotation(std::ptr::null_mut(), insert, 0.75),
        DWG_EINVAL,
        read_f64(dwg_entity_insert_get_rotation, insert),
        0.75
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_rotation(data, std::ptr::null(), 0.75),
        DWG_EINVAL,
        read_f64(dwg_entity_insert_get_rotation, insert),
        0.75
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_rotation(data, insert, f64::INFINITY),
        DWG_EINVAL,
        read_f64(dwg_entity_insert_get_rotation, insert),
        0.75
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_rotation(data, viewport, 0.75),
        DWG_ETYPE,
        read_f64(dwg_entity_insert_get_rotation, insert),
        0.75
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_rotation(data, foreign_insert, 0.75),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_insert_get_rotation, insert),
        0.75
    );
    assert_eq!(
        unsafe { dwg_entity_insert_set_rotation(data, insert, 0.75) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_insert_get_rotation, insert) },
        0.75
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_insert_set_normal(data, insert, 3.0, 4.0, 0.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_insert_get_normal, insert) },
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(std::ptr::null_mut(), insert, 3.0, 4.0, 0.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(data, std::ptr::null(), 3.0, 4.0, 0.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(data, insert, 0.0, 0.0, 0.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(data, insert, f64::INFINITY, 4.0, 0.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(data, viewport, 3.0, 4.0, 0.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_normal(data, foreign_insert, 3.0, 4.0, 0.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_insert_get_normal, insert),
        (0.6, 0.8, 0.0)
    );
    assert_eq!(
        unsafe { dwg_entity_insert_set_normal(data, insert, 3.0, 4.0, 0.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_insert_get_normal, insert) },
        (0.6, 0.8, 0.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_insert_set_array(data, insert, 3, 2, 10.0, 20.0) },
        DWG_OK
    );
    assert_eq!(read_array!(insert), (3, 2));
    assert_failure_preserves!(
        dwg_entity_insert_set_array(std::ptr::null_mut(), insert, 3, 2, 10.0, 20.0),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, std::ptr::null(), 3, 2, 10.0, 20.0),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, insert, 0, 2, 10.0, 20.0),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );

    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, insert, 3, 0, 10.0, 20.0),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, insert, 3, 2, f64::NAN, 20.0),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, insert, 3, 2, 10.0, f64::INFINITY),
        DWG_EINVAL,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, viewport, 3, 2, 10.0, 20.0),
        DWG_ETYPE,
        read_array!(insert),
        (3, 2)
    );
    assert_failure_preserves!(
        dwg_entity_insert_set_array(data, foreign_insert, 3, 2, 10.0, 20.0),
        DWG_ENOTFOUND,
        read_array!(insert),
        (3, 2)
    );
    assert_eq!(
        unsafe { dwg_entity_insert_set_array(data, insert, 3, 2, 10.0, 20.0) },
        DWG_OK
    );
    assert_eq!(read_array!(insert), (3, 2));
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_viewport_set_center(data, viewport, 4.0, 5.0, 6.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_viewport_get_center, viewport) },
        (4.0, 5.0, 6.0)
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_center(std::ptr::null_mut(), viewport, 4.0, 5.0, 6.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_viewport_get_center, viewport),
        (4.0, 5.0, 6.0)
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_center(data, std::ptr::null(), 4.0, 5.0, 6.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_viewport_get_center, viewport),
        (4.0, 5.0, 6.0)
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_center(data, viewport, 4.0, f64::NAN, 6.0),
        DWG_EINVAL,
        read_vec3(dwg_entity_viewport_get_center, viewport),
        (4.0, 5.0, 6.0)
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_center(data, insert, 4.0, 5.0, 6.0),
        DWG_ETYPE,
        read_vec3(dwg_entity_viewport_get_center, viewport),
        (4.0, 5.0, 6.0)
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_center(data, foreign_viewport, 4.0, 5.0, 6.0),
        DWG_ENOTFOUND,
        read_vec3(dwg_entity_viewport_get_center, viewport),
        (4.0, 5.0, 6.0)
    );
    assert_eq!(
        unsafe { dwg_entity_viewport_set_center(data, viewport, 4.0, 5.0, 6.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_vec3(dwg_entity_viewport_get_center, viewport) },
        (4.0, 5.0, 6.0)
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_viewport_set_width(data, viewport, 200.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_viewport_get_width, viewport) },
        200.0
    );
    assert_eq!(
        unsafe { dwg_entity_viewport_set_height(data, viewport, 100.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_viewport_get_height, viewport) },
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(std::ptr::null_mut(), viewport, 200.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(data, std::ptr::null(), 200.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(data, viewport, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(data, viewport, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(data, insert, 200.0),
        DWG_ETYPE,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_width(data, foreign_viewport, 200.0),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_viewport_get_width, viewport),
        200.0
    );
    assert_eq!(
        unsafe { dwg_entity_viewport_set_width(data, viewport, 200.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_viewport_get_width, viewport) },
        200.0
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_failure_preserves!(
        dwg_entity_viewport_set_height(std::ptr::null_mut(), viewport, 100.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_height(data, std::ptr::null(), 100.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_height(data, viewport, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_height(data, viewport, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_height(data, insert, 100.0),
        DWG_ETYPE,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_failure_preserves!(
        dwg_entity_viewport_set_height(data, foreign_viewport, 100.0),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_viewport_get_height, viewport),
        100.0
    );
    assert_eq!(
        unsafe { dwg_entity_viewport_set_height(data, viewport, 100.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_viewport_get_height, viewport) },
        100.0
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe { dwg_entity_hatch_set_elevation(hatch_data, hatch, 7.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_elevation, hatch) },
        7.0
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_name(hatch_data, hatch, pattern_name.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_hatch_get_pattern_name, hatch) },
        "ANSI31"
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_angle(hatch_data, hatch, 0.25) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_pattern_angle, hatch) },
        0.25
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_scale(hatch_data, hatch, 2.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_pattern_scale, hatch) },
        2.0
    );

    assert_failure_preserves!(
        dwg_entity_hatch_set_elevation(std::ptr::null_mut(), hatch, 7.0),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_elevation, hatch),
        7.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_elevation(hatch_data, std::ptr::null(), 7.0),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_elevation, hatch),
        7.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_elevation(hatch_data, hatch, f64::INFINITY),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_elevation, hatch),
        7.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_elevation(hatch_data, tolerance, 7.0),
        DWG_ETYPE,
        read_f64(dwg_entity_hatch_get_elevation, hatch),
        7.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_elevation(hatch_data, foreign_hatch, 7.0),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_hatch_get_elevation, hatch),
        7.0
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_elevation(hatch_data, hatch, 7.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_elevation, hatch) },
        7.0
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(std::ptr::null_mut(), hatch, pattern_name.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(hatch_data, std::ptr::null(), pattern_name.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(hatch_data, hatch, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(hatch_data, hatch, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(hatch_data, tolerance, pattern_name.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );

    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_name(hatch_data, foreign_hatch, pattern_name.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_hatch_get_pattern_name, hatch),
        "ANSI31"
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_name(hatch_data, hatch, pattern_name.as_ptr()) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_string(dwg_entity_hatch_get_pattern_name, hatch) },
        "ANSI31"
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_angle(std::ptr::null_mut(), hatch, 0.25),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_angle, hatch),
        0.25
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_angle(hatch_data, std::ptr::null(), 0.25),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_angle, hatch),
        0.25
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_angle(hatch_data, hatch, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_angle, hatch),
        0.25
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_angle(hatch_data, tolerance, 0.25),
        DWG_ETYPE,
        read_f64(dwg_entity_hatch_get_pattern_angle, hatch),
        0.25
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_angle(hatch_data, foreign_hatch, 0.25),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_hatch_get_pattern_angle, hatch),
        0.25
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_angle(hatch_data, hatch, 0.25) },
        DWG_OK
    );

    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_pattern_angle, hatch) },
        0.25
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(std::ptr::null_mut(), hatch, 2.0),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(hatch_data, std::ptr::null(), 2.0),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(hatch_data, hatch, 0.0),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(hatch_data, hatch, f64::NAN),
        DWG_EINVAL,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(hatch_data, tolerance, 2.0),
        DWG_ETYPE,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_failure_preserves!(
        dwg_entity_hatch_set_pattern_scale(hatch_data, foreign_hatch, 2.0),
        DWG_ENOTFOUND,
        read_f64(dwg_entity_hatch_get_pattern_scale, hatch),
        2.0
    );
    assert_eq!(
        unsafe { dwg_entity_hatch_set_pattern_scale(hatch_data, hatch, 2.0) },
        DWG_OK
    );
    assert_eq!(
        unsafe { read_f64(dwg_entity_hatch_get_pattern_scale, hatch) },
        2.0
    );

    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    unsafe { dwg_data_destroy(foreign_hatch_data) };
    unsafe { dwg_data_destroy(hatch_data) };
    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

fn write_batch4_dwg(dir_name: &str) -> SampleDwgFixture {
    use acadrust::entities::{
        Dimension, DimensionLinear, Helix, Light, RasterImage, Underlay, Wipeout,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch4.dwg");

    let mut doc = CadDocument::new();

    let mut helix = Helix::new();
    helix.axis_base_point = Vector3::new(-1.0, -2.0, -3.0);
    helix.radius = 3.0;
    helix.turns = 2.0;
    helix.turn_height = 0.75;
    doc.add_entity(EntityType::Helix(Box::new(helix))).unwrap();

    let dim = Dimension::Linear(DimensionLinear::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
    ));
    doc.add_entity(EntityType::Dimension(Box::new(dim))).unwrap();

    let img = RasterImage::new("img.png", Vector3::new(1.0, 1.0, 0.0), 320.0, 240.0);
    doc.add_entity(EntityType::RasterImage(img)).unwrap();

    let mut underlay = Underlay::pdf();
    underlay.insertion_point = Vector3::new(2.0, 3.0, 4.0);
    doc.add_entity(EntityType::Underlay(underlay)).unwrap();

    let mut light = Light::new();
    light.position = Vector3::new(5.0, 6.0, 7.0);
    doc.add_entity(EntityType::Light(Box::new(light))).unwrap();

    let mut wipeout = Wipeout::new();
    wipeout.insertion_point = Vector3::new(8.0, 9.0, 10.0);
    doc.add_entity(EntityType::Wipeout(wipeout)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    SampleDwgFixture {
        c_path: CString::new(path.to_str().unwrap()).unwrap(),
        path,
        dir,
    }
}

#[test]
fn batch4_dimension_image_getters() {
    let fixture = write_batch4_dwg("acadrust_ffi_p3b4");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let dim = find_object_of_type(data, acadrust::ffi::DwgObjectType::Dimension as i32);
    assert!(!dim.is_null());
    let mut subtype = 0;
    assert_eq!(
        unsafe { dwg_entity_dimension_get_subtype(dim, &mut subtype) },
        DWG_OK
    );
    assert_eq!(subtype, 2); // Linear
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(
        unsafe { dwg_entity_dimension_get_definition_point(dim, &mut x, &mut y, &mut z) },
        DWG_OK
    );
    assert!(x.abs() < 1e-9 && y.abs() < 1e-9 && z.abs() < 1e-9);
    let mut needed = 0usize;
    let mut buf = [0u8; 64];
    assert_eq!(
        unsafe {
            dwg_entity_dimension_get_style_name(
                dim,
                buf.as_mut_ptr() as *mut _,
                buf.len(),
                &mut needed,
            )
        },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"Standard");
    let mut m = 0.0;
    assert_eq!(
        unsafe { dwg_entity_dimension_get_measurement(dim, &mut m) },
        DWG_OK
    );
    assert!(m > 0.0);

    let img = find_object_of_type(data, acadrust::ffi::DwgObjectType::RasterImage as i32);
    assert!(!img.is_null());
    let mut w = 0.0;
    let mut h = 0.0;
    assert_eq!(
        unsafe { dwg_entity_raster_image_get_size(img, &mut w, &mut h) },
        DWG_OK
    );
    assert!((w - 320.0).abs() < 1e-9 && (h - 240.0).abs() < 1e-9);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_reference_setters() {
    macro_rules! assert_failure_preserves {
        ($call:expr, $code:expr, $read:expr, $expected:expr) => {{
            assert_eq!(unsafe { $call }, $code);
            assert_last_error($code);
            assert_eq!(unsafe { $read }, $expected);
        }};
    }

    macro_rules! assert_vec3_setter {
        ($data:expr, $setter:ident, $getter:ident, $obj:expr, $wrong:expr, $foreign:expr, $before:expr, $after:expr) => {{
            let before = $before;
            let after = $after;
            assert_failure_preserves!(
                $setter(std::ptr::null_mut(), $obj, after.0, after.1, after.2),
                DWG_EINVAL,
                read_vec3($getter, $obj),
                before
            );
            assert_failure_preserves!(
                $setter($data, std::ptr::null(), after.0, after.1, after.2),
                DWG_EINVAL,
                read_vec3($getter, $obj),
                before
            );
            assert_failure_preserves!(
                $setter($data, $obj, f64::NAN, after.1, after.2),
                DWG_EINVAL,
                read_vec3($getter, $obj),
                before
            );
            assert_failure_preserves!(
                $setter($data, $wrong, after.0, after.1, after.2),
                DWG_ETYPE,
                read_vec3($getter, $obj),
                before
            );
            assert_failure_preserves!(
                $setter($data, $foreign, after.0, after.1, after.2),
                DWG_ENOTFOUND,
                read_vec3($getter, $obj),
                before
            );
            assert_eq!(
                unsafe { $setter($data, $obj, after.0, after.1, after.2) },
                DWG_OK
            );
            assert_eq!(dwg_last_error_code(), DWG_OK);
            assert_eq!(unsafe { read_vec3($getter, $obj) }, after);
        }};
    }

    macro_rules! assert_positive_setter {
        ($data:expr, $setter:ident, $getter:ident, $obj:expr, $wrong:expr, $foreign:expr, $before:expr, $after:expr) => {{
            assert_failure_preserves!(
                $setter(std::ptr::null_mut(), $obj, $after),
                DWG_EINVAL,
                read_f64($getter, $obj),
                $before
            );
            assert_failure_preserves!(
                $setter($data, std::ptr::null(), $after),
                DWG_EINVAL,
                read_f64($getter, $obj),
                $before
            );
            assert_failure_preserves!(
                $setter($data, $obj, 0.0),
                DWG_EINVAL,
                read_f64($getter, $obj),
                $before
            );
            assert_failure_preserves!(
                $setter($data, $obj, f64::NAN),
                DWG_EINVAL,
                read_f64($getter, $obj),
                $before
            );
            assert_failure_preserves!(
                $setter($data, $wrong, $after),
                DWG_ETYPE,
                read_f64($getter, $obj),
                $before
            );
            assert_failure_preserves!(
                $setter($data, $foreign, $after),
                DWG_ENOTFOUND,
                read_f64($getter, $obj),
                $before
            );
            assert_eq!(unsafe { $setter($data, $obj, $after) }, DWG_OK);
            assert_eq!(dwg_last_error_code(), DWG_OK);
            assert_eq!(unsafe { read_f64($getter, $obj) }, $after);
        }};
    }

    let fixture = write_batch4_dwg("acadrust_ffi_p3c2_reference");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) },
        DWG_OK
    );

    let helix = find_object_of_type(data, DwgObjectType::Helix as i32);
    let dimension = find_object_of_type(data, DwgObjectType::Dimension as i32);
    let raster = find_object_of_type(data, DwgObjectType::RasterImage as i32);
    let underlay = find_object_of_type(data, DwgObjectType::Underlay as i32);
    let light = find_object_of_type(data, DwgObjectType::Light as i32);
    let wipeout = find_object_of_type(data, DwgObjectType::Wipeout as i32);
    assert!(
        !helix.is_null()
            && !dimension.is_null()
            && !raster.is_null()
            && !underlay.is_null()
            && !light.is_null()
            && !wipeout.is_null()
    );

    let foreign_fixture = write_batch4_dwg("acadrust_ffi_p3c2_reference_foreign");
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) },
        DWG_OK
    );
    let foreign_helix = find_object_of_type(foreign_data, DwgObjectType::Helix as i32);
    let foreign_dimension = find_object_of_type(foreign_data, DwgObjectType::Dimension as i32);
    let foreign_raster = find_object_of_type(foreign_data, DwgObjectType::RasterImage as i32);
    let foreign_underlay = find_object_of_type(foreign_data, DwgObjectType::Underlay as i32);
    let foreign_light = find_object_of_type(foreign_data, DwgObjectType::Light as i32);
    let foreign_wipeout = find_object_of_type(foreign_data, DwgObjectType::Wipeout as i32);

    assert_vec3_setter!(
        data,
        dwg_entity_helix_set_axis_base_point,
        dwg_entity_helix_get_axis_base_point,
        helix,
        dimension,
        foreign_helix,
        (-1.0, -2.0, -3.0),
        (1.0, 2.0, 3.0)
    );
    assert_positive_setter!(
        data,
        dwg_entity_helix_set_radius,
        dwg_entity_helix_get_radius,
        helix,
        dimension,
        foreign_helix,
        3.0,
        5.0
    );
    assert_positive_setter!(
        data,
        dwg_entity_helix_set_turns,
        dwg_entity_helix_get_turns,
        helix,
        dimension,
        foreign_helix,
        2.0,
        4.0
    );
    assert_positive_setter!(
        data,
        dwg_entity_helix_set_turn_height,
        dwg_entity_helix_get_turn_height,
        helix,
        dimension,
        foreign_helix,
        0.75,
        1.5
    );

    let style = CString::new("P3C_DIM").unwrap();
    let empty_style = CString::new("").unwrap();
    let invalid_utf8 = [0xff_u8, 0];
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(std::ptr::null_mut(), dimension, style.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(data, std::ptr::null(), style.as_ptr()),
        DWG_EINVAL,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(data, dimension, std::ptr::null()),
        DWG_EINVAL,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(data, dimension, invalid_utf8.as_ptr().cast()),
        DWG_EINVAL,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(data, helix, style.as_ptr()),
        DWG_ETYPE,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_failure_preserves!(
        dwg_entity_dimension_set_style_name(data, foreign_dimension, style.as_ptr()),
        DWG_ENOTFOUND,
        read_string(dwg_entity_dimension_get_style_name, dimension),
        "Standard"
    );
    assert_eq!(
        unsafe { dwg_entity_dimension_set_style_name(data, dimension, empty_style.as_ptr()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    assert_eq!(
        unsafe { read_string(dwg_entity_dimension_get_style_name, dimension) },
        ""
    );
    assert_eq!(
        unsafe { dwg_entity_dimension_set_style_name(data, dimension, style.as_ptr()) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    assert_eq!(
        unsafe { read_string(dwg_entity_dimension_get_style_name, dimension) },
        "P3C_DIM"
    );

    assert_vec3_setter!(
        data,
        dwg_entity_dimension_set_definition_point,
        dwg_entity_dimension_get_definition_point,
        dimension,
        helix,
        foreign_dimension,
        (0.0, 0.0, 0.0),
        (10.0, 11.0, 12.0)
    );
    assert_vec3_setter!(
        data,
        dwg_entity_dimension_set_text_middle_point,
        dwg_entity_dimension_get_text_middle_point,
        dimension,
        helix,
        foreign_dimension,
        (0.0, 0.0, 0.0),
        (13.0, 14.0, 15.0)
    );
    assert_vec3_setter!(
        data,
        dwg_entity_raster_image_set_insertion_point,
        dwg_entity_raster_image_get_insertion_point,
        raster,
        dimension,
        foreign_raster,
        (1.0, 1.0, 0.0),
        (20.0, 21.0, 22.0)
    );

    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(std::ptr::null_mut(), raster, 640.0, 480.0),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, std::ptr::null(), 640.0, 480.0),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, raster, 0.0, 480.0),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, raster, 640.0, 0.0),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, raster, f64::NAN, 480.0),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, raster, 640.0, f64::INFINITY),
        DWG_EINVAL,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, dimension, 640.0, 480.0),
        DWG_ETYPE,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_failure_preserves!(
        dwg_entity_raster_image_set_size(data, foreign_raster, 640.0, 480.0),
        DWG_ENOTFOUND,
        read_vec2(dwg_entity_raster_image_get_size, raster),
        (320.0, 240.0)
    );
    assert_eq!(
        unsafe { dwg_entity_raster_image_set_size(data, raster, 640.0, 480.0) },
        DWG_OK
    );
    assert_eq!(dwg_last_error_code(), DWG_OK);
    assert_eq!(
        unsafe { read_vec2(dwg_entity_raster_image_get_size, raster) },
        (640.0, 480.0)
    );

    assert_vec3_setter!(
        data,
        dwg_entity_underlay_set_insertion_point,
        dwg_entity_underlay_get_insertion_point,
        underlay,
        light,
        foreign_underlay,
        (2.0, 3.0, 4.0),
        (30.0, 31.0, 32.0)
    );
    assert_vec3_setter!(
        data,
        dwg_entity_light_set_position,
        dwg_entity_light_get_position,
        light,
        wipeout,
        foreign_light,
        (5.0, 6.0, 7.0),
        (40.0, 41.0, 42.0)
    );
    assert_vec3_setter!(
        data,
        dwg_entity_wipeout_set_insertion_point,
        dwg_entity_wipeout_get_insertion_point,
        wipeout,
        underlay,
        foreign_wipeout,
        (8.0, 9.0, 10.0),
        (50.0, 51.0, 52.0)
    );

    let mut measurement = 0.0;
    assert_eq!(
        unsafe { dwg_entity_dimension_get_measurement(dimension, &mut measurement) },
        DWG_OK
    );
    assert_eq!(measurement, 10.0);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

struct Phase4b3XdataFixture {
    data: *mut DwgData,
    entity: u64,
    object: u64,
    foreign_data: *mut DwgData,
    foreign_entity: u64,
    foreign_object: u64,
    source: SampleDwgFixture,
    foreign_source: SampleDwgFixture,
}

impl Phase4b3XdataFixture {
    fn new() -> Self {
        Self::new_for_version(acadrust::types::DxfVersion::AC1032)
    }

    fn new_for_version(version: acadrust::types::DxfVersion) -> Self {
        let (source, entity, object) =
            write_phase4b3_xdata_dwg("acadrust_ffi_p4b3_xdata", version, 0, false);
        let (foreign_source, foreign_entity, foreign_object) =
            write_phase4b3_xdata_dwg("acadrust_ffi_p4b3_xdata_foreign", version, 2, true);
        let mut data = std::ptr::null_mut();
        let mut foreign_data = std::ptr::null_mut();
        assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
        assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
        assert_eq!(
            unsafe { dwg_read_file(source.c_path().as_ptr(), data) },
            DWG_OK
        );
        assert_eq!(
            unsafe { dwg_read_file(foreign_source.c_path().as_ptr(), foreign_data) },
            DWG_OK
        );
        assert_ne!(
            entity, foreign_entity,
            "foreign entity must not alias the fixture"
        );
        assert_ne!(
            object, foreign_object,
            "foreign object must not alias the fixture"
        );
        Self {
            data,
            entity,
            object,
            foreign_data,
            foreign_entity,
            foreign_object,
            source,
            foreign_source,
        }
    }
}

impl Drop for Phase4b3XdataFixture {
    fn drop(&mut self) {
        unsafe {
            dwg_data_destroy(self.foreign_data);
            dwg_data_destroy(self.data);
        }
    }
}

fn write_phase4b3_xdata_dwg(
    dir_name: &str,
    version: acadrust::types::DxfVersion,
    padding_entities: usize,
    add_foreign_object: bool,
) -> (SampleDwgFixture, u64, u64) {
    use acadrust::entities::Line;
    use acadrust::tables::{AppId, Layer, TableEntry};
    use acadrust::types::Vector3;
    use acadrust::xdata::{ExtendedDataRecord, XDataValue};
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let suffix = SAMPLE_DWG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{dir_name}_{}_{}", std::process::id(), suffix));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("xdata.dwg");

    let mut doc = CadDocument::with_version(version);
    for index in 0..padding_entities {
        doc.add_entity(EntityType::Line(Line::from_coords(
            index as f64,
            0.0,
            0.0,
            index as f64 + 1.0,
            1.0,
            0.0,
        )))
        .unwrap();
    }
    let entity = doc
        .add_entity(EntityType::Line(Line::from_coords(
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
        )))
        .unwrap();
    let object = doc.header.named_objects_dict_handle;
    let mut appid = AppId::new("P4B3_APP");
    appid.set_handle(doc.allocate_handle());
    doc.app_ids.add(appid).unwrap();
    let mut layer = Layer::new("P4B3_LAYER");
    layer.handle = doc.allocate_handle();
    doc.layers.add(layer).unwrap();

    let mut record = ExtendedDataRecord::new("P4B3_APP");
    record.values = vec![
        XDataValue::String("P4B3 string".to_string()),
        XDataValue::ControlString("{".to_string()),
        XDataValue::LayerName("P4B3_LAYER".to_string()),
        XDataValue::BinaryData(vec![0xCA, 0xFE, 0x01]),
        XDataValue::Handle(object),
        XDataValue::Point3D(Vector3::new(1.0, 2.0, 3.0)),
        XDataValue::Position3D(Vector3::new(4.0, 5.0, 6.0)),
        XDataValue::Displacement3D(Vector3::new(7.0, 8.0, 9.0)),
        XDataValue::Direction3D(Vector3::new(10.0, 11.0, 12.0)),
        XDataValue::Real(13.25),
        XDataValue::Distance(14.5),
        XDataValue::ScaleFactor(15.75),
        XDataValue::Integer16(-321),
        XDataValue::Integer32(123_456),
    ];
    doc.get_entity_mut(entity)
        .unwrap()
        .common_mut()
        .extended_data
        .add_record(record);

    let foreign_object = if add_foreign_object {
        doc.ensure_xrecord(object, "P4B3_FOREIGN_OBJECT")
    } else {
        object
    };
    DwgWriter::write_to_file(&path, &doc).unwrap();
    (
        SampleDwgFixture {
            c_path: CString::new(path.to_str().unwrap()).unwrap(),
            path,
            dir,
        },
        entity.value(),
        foreign_object.value(),
    )
}

fn xdata_value(tag: u32) -> DwgXdataValue {
    DwgXdataValue {
        tag,
        reserved: 0,
        integer: 0,
        real: 0.0,
        point: [0.0; 3],
        handle: 0,
        data: std::ptr::null_mut(),
        data_len: 0,
    }
}

fn read_xdata_value(
    data: *const DwgData,
    entity: u64,
    record: usize,
    index: usize,
) -> (DwgXdataValue, Vec<u8>) {
    let mut value = xdata_value(0);
    assert_eq!(
        unsafe { dwg_entity_xdata_value(data, entity, record, index, &mut value) },
        DWG_OK
    );
    let mut bytes = vec![0_u8; value.data_len];
    if !bytes.is_empty() {
        value.data = bytes.as_mut_ptr().cast();
        value.data_len = bytes.len();
        assert_eq!(
            unsafe { dwg_entity_xdata_value(data, entity, record, index, &mut value) },
            DWG_OK
        );
    }
    (value, bytes)
}

fn assert_xdata_value(
    data: *const DwgData,
    entity: u64,
    index: usize,
    tag: u32,
    integer: i64,
    real: f64,
    point: [f64; 3],
    handle: u64,
    bytes: &[u8],
) {
    let (value, actual_bytes) = read_xdata_value(data, entity, 0, index);
    assert_eq!(value.tag, tag, "value {index} tag");
    assert_eq!(value.integer, integer, "value {index} integer");
    assert_eq!(value.real, real, "value {index} real");
    assert_eq!(value.point, point, "value {index} point");
    assert_eq!(value.handle, handle, "value {index} handle");
    assert_eq!(actual_bytes, bytes, "value {index} bytes");
}

fn assert_seeded_xdata(data: *const DwgData, entity: u64, object: u64) {
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_entity_xdata_record_count(data, entity, &mut count) },
        DWG_OK
    );
    assert_eq!(count, 1);

    let mut required = 0;
    let mut values = 0;
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                data,
                entity,
                0,
                std::ptr::null_mut(),
                0,
                &mut required,
                &mut values,
            )
        },
        DWG_OK
    );
    assert_eq!(required, b"P4B3_APP\0".len());
    assert_eq!(values, 14);
    let mut name = vec![0_u8; required];
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                data,
                entity,
                0,
                name.as_mut_ptr().cast(),
                name.len(),
                &mut required,
                &mut values,
            )
        },
        DWG_OK
    );
    assert_eq!(name, b"P4B3_APP\0");

    assert_xdata_value(
        data,
        entity,
        0,
        DWG_XDATA_STRING,
        0,
        0.0,
        [0.0; 3],
        0,
        b"P4B3 string\0",
    );
    assert_xdata_value(
        data,
        entity,
        1,
        DWG_XDATA_CONTROL_STRING,
        0,
        0.0,
        [0.0; 3],
        0,
        b"{\0",
    );
    assert_xdata_value(
        data,
        entity,
        2,
        DWG_XDATA_LAYER_NAME,
        0,
        0.0,
        [0.0; 3],
        0,
        b"P4B3_LAYER\0",
    );
    assert_xdata_value(
        data,
        entity,
        3,
        DWG_XDATA_BINARY,
        0,
        0.0,
        [0.0; 3],
        0,
        &[0xCA, 0xFE, 0x01],
    );
    assert_xdata_value(
        data,
        entity,
        4,
        DWG_XDATA_HANDLE,
        0,
        0.0,
        [0.0; 3],
        object,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        5,
        DWG_XDATA_POINT3D,
        0,
        0.0,
        [1.0, 2.0, 3.0],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        6,
        DWG_XDATA_POSITION3D,
        0,
        0.0,
        [4.0, 5.0, 6.0],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        7,
        DWG_XDATA_DISPLACEMENT3D,
        0,
        0.0,
        [7.0, 8.0, 9.0],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        8,
        DWG_XDATA_DIRECTION3D,
        0,
        0.0,
        [10.0, 11.0, 12.0],
        0,
        &[],
    );
    assert_xdata_value(data, entity, 9, DWG_XDATA_REAL, 0, 13.25, [0.0; 3], 0, &[]);
    assert_xdata_value(
        data,
        entity,
        10,
        DWG_XDATA_DISTANCE,
        0,
        14.5,
        [0.0; 3],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        11,
        DWG_XDATA_SCALE_FACTOR,
        0,
        15.75,
        [0.0; 3],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        12,
        DWG_XDATA_INTEGER16,
        -321,
        0.0,
        [0.0; 3],
        0,
        &[],
    );
    assert_xdata_value(
        data,
        entity,
        13,
        DWG_XDATA_INTEGER32,
        123_456,
        0.0,
        [0.0; 3],
        0,
        &[],
    );
}

fn find_entity_by_handle_and_type(data: *mut DwgData, handle: u64, kind: DwgObjectType) -> u64 {
    let mut error = DWG_OK;
    let mut object = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    while !object.is_null() {
        if unsafe { dwg_object_get_handle(object) } == handle
            && unsafe { dwg_object_get_type(object) } == kind as i32
        {
            return handle;
        }
        object = unsafe { dwg_object_next(object) };
    }
    panic!("entity {handle:X} with type {kind:?} is missing");
}

#[test]
fn phase4b3_xdata_argument_and_type_errors() {
    let fixture = Phase4b3XdataFixture::new();
    assert_eq!(std::mem::size_of::<DwgXdataValue>(), 72);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, integer), 8);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, real), 16);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, point), 24);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, handle), 48);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, data), 56);
    assert_eq!(std::mem::offset_of!(DwgXdataValue, data_len), 64);

    let name = CString::new("P4B3_NEW").unwrap();
    let empty_name = CString::new("").unwrap();
    let malformed_name = [0xff_u8, 0];
    let mut count = 0;
    let mut index = 0;
    let mut required = 0;
    let mut values = 0;
    let mut value = xdata_value(DWG_XDATA_INTEGER32);

    assert_eq!(
        unsafe { dwg_entity_xdata_record_count(std::ptr::null(), fixture.entity, &mut count) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_count(fixture.data, fixture.entity, std::ptr::null_mut())
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_find(
                fixture.data,
                fixture.entity,
                std::ptr::null(),
                0,
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_find(
                fixture.data,
                fixture.entity,
                name.as_ptr(),
                0,
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                fixture.data,
                fixture.entity,
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut values,
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                fixture.data,
                fixture.entity,
                0,
                std::ptr::null_mut(),
                0,
                &mut required,
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 0, std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_record_count(fixture.data, fixture.object, &mut count) },
        DWG_ETYPE
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_record_count(fixture.data, fixture.foreign_entity, &mut count) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 1, 0, &mut value) },
        DWG_ENOTFOUND
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 14, &mut value) },
        DWG_ENOTFOUND
    );

    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                std::ptr::null_mut(),
                fixture.entity,
                name.as_ptr(),
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                fixture.data,
                fixture.entity,
                std::ptr::null(),
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                fixture.data,
                fixture.entity,
                empty_name.as_ptr(),
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                fixture.data,
                fixture.entity,
                malformed_name.as_ptr().cast(),
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                fixture.data,
                fixture.entity,
                name.as_ptr(),
                std::ptr::null_mut(),
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(
                fixture.data,
                fixture.entity,
                CString::new("P4B3_APP").unwrap().as_ptr(),
                &mut index,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_remove(
                fixture.data,
                fixture.entity,
                1,
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);

    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(std::ptr::null_mut(), fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe { dwg_entity_xdata_value_set(fixture.data, fixture.entity, 0, 0, std::ptr::null()) },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_value_remove(fixture.data, fixture.entity, 0, 0, std::ptr::null_mut())
        },
        DWG_EINVAL
    );
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);

    value.reserved = 1;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(99);
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_STRING);
    value.data_len = 1;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    let malformed_utf8 = [0xff_u8];
    value.data = malformed_utf8.as_ptr().cast_mut().cast();
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_CONTROL_STRING);
    let invalid_control = b"[";
    value.data = invalid_control.as_ptr().cast_mut().cast();
    value.data_len = invalid_control.len();
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_REAL);
    value.real = f64::NAN;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_POINT3D);
    value.point = [1.0, f64::INFINITY, 3.0];
    assert_eq!(
        unsafe { dwg_entity_xdata_value_set(fixture.data, fixture.entity, 0, 0, &value) },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_HANDLE);
    value.handle = fixture.foreign_object;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &value) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    value = xdata_value(DWG_XDATA_INTEGER32);
    value.integer = 7;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 1, &value) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);

    let mut wrong_shape = xdata_value(DWG_XDATA_BINARY);
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 0, &mut wrong_shape) },
        DWG_ETYPE
    );
    let mut zero_capacity = xdata_value(0);
    zero_capacity.data = std::ptr::dangling_mut();
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 0, &mut zero_capacity) },
        DWG_EINVAL
    );
}

#[test]
fn phase4b3_xdata_record_and_value_mutations() {
    let fixture = Phase4b3XdataFixture::new();
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    let app = CString::new("P4B3_APP").unwrap();
    let empty = CString::new("P4B3_EMPTY").unwrap();
    let next = CString::new("P4B3_NEXT").unwrap();
    let mut index = usize::MAX;
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_find(fixture.data, fixture.entity, app.as_ptr(), 0, &mut index)
        },
        DWG_OK
    );
    assert_eq!(index, 0);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(fixture.data, fixture.entity, empty.as_ptr(), &mut index)
        },
        DWG_OK
    );
    assert_eq!(index, 1);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let mut required = 0;
    let mut values = usize::MAX;
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                fixture.data,
                fixture.entity,
                1,
                std::ptr::null_mut(),
                0,
                &mut required,
                &mut values,
            )
        },
        DWG_OK
    );
    assert_eq!(required, b"P4B3_EMPTY\0".len());
    assert_eq!(values, 0);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_append(fixture.data, fixture.entity, next.as_ptr(), &mut index)
        },
        DWG_OK
    );
    assert_eq!(index, 2);

    let mut removed_name = [0_u8; 32];
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_remove(
                fixture.data,
                fixture.entity,
                1,
                removed_name.as_mut_ptr().cast(),
                removed_name.len(),
                &mut required,
            )
        },
        DWG_OK
    );
    assert_eq!(&removed_name[..required], b"P4B3_EMPTY\0");
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_find(fixture.data, fixture.entity, next.as_ptr(), 0, &mut index)
        },
        DWG_OK
    );
    assert_eq!(index, 1, "record indices shift after removal");

    let mut appended = xdata_value(DWG_XDATA_HANDLE);
    appended.handle = fixture.object;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 1, &appended) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let (appended_value, _) = read_xdata_value(fixture.data, fixture.entity, 1, 0);
    assert_eq!(appended_value.tag, DWG_XDATA_HANDLE);
    assert_eq!(appended_value.handle, fixture.object);
    let mut removed = xdata_value(0);
    assert_eq!(
        unsafe { dwg_entity_xdata_value_remove(fixture.data, fixture.entity, 1, 0, &mut removed) },
        DWG_OK
    );
    assert_eq!(removed.tag, DWG_XDATA_HANDLE);
    assert_eq!(removed.handle, fixture.object);

    let mut replacement = xdata_value(DWG_XDATA_REAL);
    replacement.real = 42.25;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_set(fixture.data, fixture.entity, 0, 1, &replacement) },
        DWG_OK
    );
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);
    let (value, _) = read_xdata_value(fixture.data, fixture.entity, 0, 1);
    assert_eq!(value.tag, DWG_XDATA_REAL);
    assert_eq!(value.real, 42.25);
    assert_eq!(
        unsafe { dwg_entity_xdata_value_remove(fixture.data, fixture.entity, 0, 1, &mut removed) },
        DWG_OK
    );
    assert_eq!(removed.tag, DWG_XDATA_REAL);
    assert_eq!(removed.real, 42.25);
    let (shifted, shifted_bytes) = read_xdata_value(fixture.data, fixture.entity, 0, 1);
    assert_eq!(shifted.tag, DWG_XDATA_LAYER_NAME);
    assert_eq!(shifted_bytes, b"P4B3_LAYER\0");

    let mut singleton = xdata_value(DWG_XDATA_INTEGER16);
    singleton.integer = -7;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 1, &singleton) },
        DWG_OK
    );
    assert_eq!(
        unsafe { dwg_entity_xdata_value_remove(fixture.data, fixture.entity, 1, 0, &mut removed) },
        DWG_OK
    );
    assert_eq!(removed.tag, DWG_XDATA_INTEGER16);
    assert_eq!(removed.integer, -7);
    assert_eq!(
        unsafe {
            dwg_entity_xdata_record_entry(
                fixture.data,
                fixture.entity,
                1,
                std::ptr::null_mut(),
                0,
                &mut required,
                &mut values,
            )
        },
        DWG_OK
    );
    assert_eq!(
        values, 0,
        "removing the last value preserves the empty record"
    );
}

#[test]
fn phase4b3_xdata_buffers_and_foreign_handle() {
    let fixture = Phase4b3XdataFixture::new();
    let mut string = xdata_value(0);
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 0, &mut string) },
        DWG_OK
    );
    assert_eq!(string.data_len, b"P4B3 string\0".len());
    let mut too_small = vec![0_u8; string.data_len - 1];
    string.data = too_small.as_mut_ptr().cast();
    string.data_len = too_small.len();
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 0, &mut string) },
        DWG_EBUFFER
    );
    assert_last_error(DWG_EBUFFER);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    let (string, string_bytes) = read_xdata_value(fixture.data, fixture.entity, 0, 0);
    assert_eq!(string.tag, DWG_XDATA_STRING);
    assert_eq!(string_bytes, b"P4B3 string\0");

    let mut binary = xdata_value(0);
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 3, &mut binary) },
        DWG_OK
    );
    assert_eq!(binary.data_len, 3);
    let mut binary_too_small = [0_u8; 2];
    binary.data = binary_too_small.as_mut_ptr().cast();
    binary.data_len = binary_too_small.len();
    assert_eq!(
        unsafe { dwg_entity_xdata_value(fixture.data, fixture.entity, 0, 3, &mut binary) },
        DWG_EBUFFER
    );
    assert_last_error(DWG_EBUFFER);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    let (binary, binary_bytes) = read_xdata_value(fixture.data, fixture.entity, 0, 3);
    assert_eq!(binary.tag, DWG_XDATA_BINARY);
    assert_eq!(binary_bytes, [0xCA, 0xFE, 0x01]);

    let mut foreign = xdata_value(DWG_XDATA_HANDLE);
    foreign.handle = fixture.foreign_object;
    assert_eq!(
        unsafe { dwg_entity_xdata_value_append(fixture.data, fixture.entity, 0, &foreign) },
        DWG_ENOTFOUND
    );
    assert_last_error(DWG_ENOTFOUND);
    assert_seeded_xdata(fixture.data, fixture.entity, fixture.object);
    let mut count = 0;
    assert_eq!(
        unsafe { dwg_entity_xdata_record_count(fixture.data, fixture.foreign_entity, &mut count) },
        DWG_ENOTFOUND
    );
}

#[test]
fn phase4b3_xdata_dwg_dxf_roundtrip() {
    for (label, version) in [
        ("AC1018", acadrust::types::DxfVersion::AC1018),
        ("AC1032", acadrust::types::DxfVersion::AC1032),
    ] {
        let fixture = Phase4b3XdataFixture::new_for_version(version);
        let replacement_bytes = b"P4B3 changed";
        let mut replacement = xdata_value(DWG_XDATA_STRING);
        replacement.data = replacement_bytes.as_ptr().cast_mut().cast();
        replacement.data_len = replacement_bytes.len();
        assert_eq!(
            unsafe { dwg_entity_xdata_value_set(fixture.data, fixture.entity, 0, 0, &replacement) },
            DWG_OK,
            "{label}: mutate structured XData after raw EED read"
        );

        let dwg_path = fixture.source.dir.join(format!("{label}-mutated.dwg"));
        let dwg_path_c = CString::new(dwg_path.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { dwg_write_file(dwg_path_c.as_ptr(), fixture.data) },
            DWG_OK,
            "{label}: write DWG"
        );
        let mut reopened = std::ptr::null_mut();
        assert_eq!(unsafe { dwg_data_create(&mut reopened) }, DWG_OK);
        assert_eq!(
            unsafe { dwg_read_file(dwg_path_c.as_ptr(), reopened) },
            DWG_OK,
            "{label}: reopen DWG"
        );
        let entity = find_entity_by_handle_and_type(reopened, fixture.entity, DwgObjectType::Line);
        let mut record_count = 0;
        assert_eq!(
            unsafe { dwg_entity_xdata_record_count(reopened, entity, &mut record_count) },
            DWG_OK
        );
        assert_eq!(record_count, 1, "{label}: DWG application order");
        let mut app_name = [0_u8; 16];
        let mut app_name_len = 0;
        let mut value_count = 0;
        assert_eq!(
            unsafe {
                dwg_entity_xdata_record_entry(
                    reopened,
                    entity,
                    0,
                    app_name.as_mut_ptr().cast(),
                    app_name.len(),
                    &mut app_name_len,
                    &mut value_count,
                )
            },
            DWG_OK
        );
        assert_eq!(
            &app_name[..app_name_len],
            b"P4B3_APP\0",
            "{label}: DWG application name"
        );
        assert_eq!(value_count, 14, "{label}: DWG value count");
        assert_xdata_value(
            reopened,
            entity,
            0,
            DWG_XDATA_STRING,
            0,
            0.0,
            [0.0; 3],
            0,
            b"P4B3 changed\0",
        );
        assert_xdata_value(
            reopened,
            entity,
            1,
            DWG_XDATA_CONTROL_STRING,
            0,
            0.0,
            [0.0; 3],
            0,
            b"{\0",
        );
        assert_xdata_value(
            reopened,
            entity,
            2,
            DWG_XDATA_LAYER_NAME,
            0,
            0.0,
            [0.0; 3],
            0,
            b"P4B3_LAYER\0",
        );
        assert_xdata_value(
            reopened,
            entity,
            3,
            DWG_XDATA_BINARY,
            0,
            0.0,
            [0.0; 3],
            0,
            &[0xCA, 0xFE, 0x01],
        );
        assert_xdata_value(
            reopened,
            entity,
            4,
            DWG_XDATA_HANDLE,
            0,
            0.0,
            [0.0; 3],
            fixture.object,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            5,
            DWG_XDATA_POINT3D,
            0,
            0.0,
            [1.0, 2.0, 3.0],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            6,
            DWG_XDATA_POSITION3D,
            0,
            0.0,
            [4.0, 5.0, 6.0],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            7,
            DWG_XDATA_DISPLACEMENT3D,
            0,
            0.0,
            [7.0, 8.0, 9.0],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            8,
            DWG_XDATA_DIRECTION3D,
            0,
            0.0,
            [10.0, 11.0, 12.0],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            9,
            DWG_XDATA_REAL,
            0,
            13.25,
            [0.0; 3],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            10,
            DWG_XDATA_DISTANCE,
            0,
            14.5,
            [0.0; 3],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            11,
            DWG_XDATA_SCALE_FACTOR,
            0,
            15.75,
            [0.0; 3],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            12,
            DWG_XDATA_INTEGER16,
            -321,
            0.0,
            [0.0; 3],
            0,
            &[],
        );
        assert_xdata_value(
            reopened,
            entity,
            13,
            DWG_XDATA_INTEGER32,
            123_456,
            0.0,
            [0.0; 3],
            0,
            &[],
        );

        let dxf_path = fixture.source.dir.join(format!("{label}-mutated.dxf"));
        let dxf_path_c = CString::new(dxf_path.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { dxf_write_file(dxf_path_c.as_ptr(), fixture.data) },
            DWG_OK,
            "{label}: write DXF"
        );
        let mut dxf_reopened = std::ptr::null_mut();
        assert_eq!(unsafe { dwg_data_create(&mut dxf_reopened) }, DWG_OK);
        assert_eq!(
            unsafe { dxf_read_file(dxf_path_c.as_ptr(), dxf_reopened) },
            DWG_OK,
            "{label}: reopen DXF"
        );
        let dxf_entity =
            find_entity_by_handle_and_type(dxf_reopened, fixture.entity, DwgObjectType::Line);
        assert_xdata_value(
            dxf_reopened,
            dxf_entity,
            0,
            DWG_XDATA_STRING,
            0,
            0.0,
            [0.0; 3],
            0,
            b"P4B3 changed\0",
        );
        let mut record_count = 0;
        assert_eq!(
            unsafe { dwg_entity_xdata_record_count(dxf_reopened, dxf_entity, &mut record_count) },
            DWG_OK
        );
        assert_eq!(record_count, 1, "{label}: DXF application order");
        let mut app_name = [0_u8; 16];
        let mut app_name_len = 0;
        let mut value_count = 0;
        assert_eq!(
            unsafe {
                dwg_entity_xdata_record_entry(
                    dxf_reopened,
                    dxf_entity,
                    0,
                    app_name.as_mut_ptr().cast(),
                    app_name.len(),
                    &mut app_name_len,
                    &mut value_count,
                )
            },
            DWG_OK
        );
        assert_eq!(
            &app_name[..app_name_len],
            b"P4B3_APP\0",
            "{label}: DXF application name"
        );
        assert_eq!(value_count, 14, "{label}: DXF value count");
        for (index, tag, integer, real, point, handle, bytes) in [
            (
                1,
                DWG_XDATA_CONTROL_STRING,
                0,
                0.0,
                [0.0; 3],
                0,
                b"{\0".as_slice(),
            ),
            (
                2,
                DWG_XDATA_LAYER_NAME,
                0,
                0.0,
                [0.0; 3],
                0,
                b"P4B3_LAYER\0".as_slice(),
            ),
            (
                3,
                DWG_XDATA_BINARY,
                0,
                0.0,
                [0.0; 3],
                0,
                &[0xCA, 0xFE, 0x01][..],
            ),
            (
                4,
                DWG_XDATA_HANDLE,
                0,
                0.0,
                [0.0; 3],
                fixture.object,
                &[][..],
            ),
            (5, DWG_XDATA_POINT3D, 0, 0.0, [1.0, 2.0, 3.0], 0, &[][..]),
            (6, DWG_XDATA_POSITION3D, 0, 0.0, [4.0, 5.0, 6.0], 0, &[][..]),
            (
                7,
                DWG_XDATA_DISPLACEMENT3D,
                0,
                0.0,
                [7.0, 8.0, 9.0],
                0,
                &[][..],
            ),
            (
                8,
                DWG_XDATA_DIRECTION3D,
                0,
                0.0,
                [10.0, 11.0, 12.0],
                0,
                &[][..],
            ),
            (9, DWG_XDATA_REAL, 0, 13.25, [0.0; 3], 0, &[][..]),
            (10, DWG_XDATA_DISTANCE, 0, 14.5, [0.0; 3], 0, &[][..]),
            (11, DWG_XDATA_SCALE_FACTOR, 0, 15.75, [0.0; 3], 0, &[][..]),
            (12, DWG_XDATA_INTEGER16, -321, 0.0, [0.0; 3], 0, &[][..]),
            (13, DWG_XDATA_INTEGER32, 123_456, 0.0, [0.0; 3], 0, &[][..]),
        ] {
            assert_xdata_value(
                dxf_reopened,
                dxf_entity,
                index,
                tag,
                integer,
                real,
                point,
                handle,
                bytes,
            );
        }

        unsafe {
            dwg_data_destroy(dxf_reopened);
            dwg_data_destroy(reopened);
        }
        let _ = std::fs::remove_file(dxf_path);
        let _ = std::fs::remove_file(dwg_path);
    }
}
