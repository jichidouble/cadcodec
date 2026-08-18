//! ABI-level tests calling the exported extern "C" symbols directly.

use acadrust::ffi::*;
use std::ffi::CString;
use std::sync::atomic::{AtomicUsize, Ordering};

static SAMPLE_DWG_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 2.0, 3.0))).unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
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

    let out = std::process::Command::new(&exe)
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

    let out = std::process::Command::new(&exe)
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

    let out = std::process::Command::new(&exe)
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

        let output = std::process::Command::new(&exe)
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

fn write_entities_fixture() -> (CString, std::path::PathBuf) {
    use acadrust::entities::{Circle, Ellipse, Hatch, HatchPattern, Line, LwPolyline, LwVertex, MText};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join("acadrust_ffi_entities_fixture");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("entities.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 10.0, 20.0, 30.0))).unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
    let mut e = Ellipse::new();
    e.center = Vector3::new(1.0, 2.0, 0.0);
    e.major_axis = Vector3::new(5.0, 0.0, 0.0);
    e.minor_axis_ratio = 0.5;
    doc.add_entity(EntityType::Ellipse(e)).unwrap();
    let mut m = MText::new();
    m.value = "mt".to_string();
    doc.add_entity(EntityType::MText(m)).unwrap();
    let mut lw = LwPolyline::new();
    lw.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lw.add_vertex(LwVertex::from_coords(10.0, 5.0));
    doc.add_entity(EntityType::LwPolyline(lw)).unwrap();
    let mut h = Hatch::new();
    h.pattern = HatchPattern::new("SOLID");
    doc.add_entity(EntityType::Hatch(h)).unwrap();

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

    let out = std::process::Command::new(&exe)
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
    assert_eq!(unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) }, DWG_OK);

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
    assert_eq!(unsafe { dwg_object_get_type(first) }, acadrust::ffi::DwgObjectType::Other as i32);
    unsafe { dwg_data_destroy(data) };
}

fn open_sample_document() -> (*mut DwgData, SampleDwgFixture) {
    let fixture = write_sample_dwg("acadrust_ffi_p2_entity");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) }, DWG_OK);
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
    while !obj.is_null() && unsafe { dwg_object_get_type(obj) } != acadrust::ffi::DwgObjectType::Text as i32
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
    let rc = unsafe { dwg_entity_text_get_value(obj, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) };
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
    assert_eq!(unsafe { dwg_layer_index_by_name(data, name.as_ptr(), &mut idx) }, DWG_OK);
    assert_eq!(idx, 0);

    let mut handle = 0u64;
    assert_eq!(unsafe { dwg_layer_get_handle(data, 0, &mut handle) }, DWG_OK);
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
    assert_eq!(unsafe { dwg_table_entry_count(data, acadrust::ffi::DwgTableKind::Layer as i32, &mut count) }, DWG_OK);
    assert!(count >= 1);
    assert_eq!(unsafe { dwg_table_entry_count(data, acadrust::ffi::DwgTableKind::Block as i32, &mut count) }, DWG_OK);
    assert!(count >= 2, "model + paper space block records");

    let mut buf = [0u8; 64];
    let mut needed = 0usize;
    assert_eq!(unsafe {
        dwg_table_entry_get_name(
            data,
            acadrust::ffi::DwgTableKind::Layer as i32,
            0,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            &mut needed,
        )
    }, DWG_OK);
    assert_eq!(&buf[..needed - 1], b"0");

    unsafe { dwg_data_destroy(data) };
}

fn write_batch1_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{
        Arc, Circle, Ellipse, Face3D, Line, LwPolyline, LwVertex, MText, Point, Ray, Solid, Spline, XLine,
    };
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch1.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 2.0, 3.0))).unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
    doc.add_entity(EntityType::Arc(Arc::from_center_radius_angles(
        Vector3::new(1.0, 2.0, 0.0),
        2.0,
        0.0,
        1.0,
    )))
    .unwrap();
    doc.add_entity(EntityType::Point(Point::from_coords(1.0, 2.0, 3.0))).unwrap();
    let mut e = Ellipse::new();
    e.center = Vector3::new(1.0, 2.0, 0.0);
    e.major_axis = Vector3::new(5.0, 0.0, 0.0);
    e.minor_axis_ratio = 0.5;
    doc.add_entity(EntityType::Ellipse(e)).unwrap();

    let mut m = MText::new();
    m.value = "mt".to_string();
    m.height = 2.0;
    doc.add_entity(EntityType::MText(m)).unwrap();

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

    assert_eq!(unsafe { dwg_entity_set_layer(data, line, layer.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_color(data, line, 0x00112233) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype(data, line, linetype.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype_scale(data, line, 2.5) }, DWG_OK);

    let mut needed = 0;
    assert_eq!(unsafe { dwg_entity_get_layer(line, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    let mut layer_buf = vec![0_i8; needed];
    assert_eq!(unsafe { dwg_entity_get_layer(line, layer_buf.as_mut_ptr(), layer_buf.len(), &mut needed) }, DWG_OK);
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(layer_buf.as_ptr()) }.to_str().unwrap(), "P3C_LAYER");

    assert_eq!(unsafe { dwg_entity_get_linetype(line, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    let mut linetype_buf = vec![0_i8; needed];
    assert_eq!(unsafe { dwg_entity_get_linetype(line, linetype_buf.as_mut_ptr(), linetype_buf.len(), &mut needed) }, DWG_OK);
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(linetype_buf.as_ptr()) }.to_str().unwrap(), "DASHED");

    let mut color = 0;
    let mut scale = 0.0;
    assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_get_linetype_scale(line, &mut scale) }, DWG_OK);
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

    assert_eq!(unsafe { dwg_entity_set_linetype(data, line, empty_linetype.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_layer(data, line, stable_layer.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_color(data, line, 0x00010203) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype(data, line, stable_linetype.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_set_linetype_scale(data, line, 3.0) }, DWG_OK);

    assert_eq!(unsafe { dwg_entity_set_layer(std::ptr::null_mut(), line, stable_layer.as_ptr()) }, DWG_EINVAL);
    assert_eq!(dwg_last_error_code(), DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_set_color(data, std::ptr::null(), 0) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_set_layer(data, line, std::ptr::null()) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_set_layer(data, line, empty_linetype.as_ptr()) }, DWG_EINVAL);
    let invalid_utf8 = [0xff_u8, 0];
    assert_eq!(unsafe { dwg_entity_set_linetype(data, line, invalid_utf8.as_ptr().cast()) }, DWG_EINVAL);
    let mut linetype_needed = 0;
    assert_eq!(unsafe { dwg_entity_get_linetype(line, std::ptr::null_mut(), 0, &mut linetype_needed) }, DWG_OK);
    let mut linetype_buf = vec![0_i8; linetype_needed];
    assert_eq!(unsafe {
        dwg_entity_get_linetype(line, linetype_buf.as_mut_ptr(), linetype_buf.len(), &mut linetype_needed)
    }, DWG_OK);
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(linetype_buf.as_ptr()) }.to_str().unwrap(), "CONTINUOUS");
    assert_eq!(unsafe { dwg_entity_set_linetype_scale(data, line, f64::NAN) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_set_linetype_scale(data, line, 0.0) }, DWG_EINVAL);

    let (foreign_data, _foreign_fixture) = open_sample_document();
    let foreign_line = find_object_of_type(foreign_data, DwgObjectType::Line as i32);
    assert_eq!(unsafe { dwg_entity_set_color(data, foreign_line, 0x00ABCDEF) }, DWG_ENOTFOUND);

    let mut needed = 0;
    assert_eq!(unsafe { dwg_entity_get_layer(line, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    let mut layer_buf = vec![0_i8; needed];
    assert_eq!(unsafe { dwg_entity_get_layer(line, layer_buf.as_mut_ptr(), layer_buf.len(), &mut needed) }, DWG_OK);
    assert_eq!(unsafe { std::ffi::CStr::from_ptr(layer_buf.as_ptr()) }.to_str().unwrap(), "P3C_STABLE_LAYER");
    let mut color = 0;
    let mut scale = 0.0;
    assert_eq!(unsafe { dwg_entity_get_color(line, &mut color) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_get_linetype_scale(line, &mut scale) }, DWG_OK);
    assert_eq!(color, 0x00010203);
    assert_eq!(scale, 3.0);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
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

unsafe fn read_f64(getter: unsafe extern "C" fn(*const DwgObject, *mut f64) -> i32, obj: *const DwgObject) -> f64 {
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
    assert_eq!(dwg_last_error(buffer.as_mut_ptr(), buffer.len(), &mut needed), code);
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
        unsafe {
            dwg_entity_text_set_insertion_point(
                text_data,
                std::ptr::null(),
                8.0,
                9.0,
                10.0,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe {
            dwg_entity_mtext_set_insertion_point(
                std::ptr::null_mut(),
                mtext,
                8.0,
                9.0,
                10.0,
            )
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
        unsafe {
            dwg_entity_text_set_insertion_point(
                text_data,
                text,
                f64::NAN,
                2.0,
                3.0,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe {
            dwg_entity_text_set_insertion_point(
                text_data,
                text,
                1.0,
                f64::INFINITY,
                3.0,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_text_get_insertion_point, text) },
        (1.0, 2.0, 3.0)
    );
    assert_eq!(
        unsafe {
            dwg_entity_mtext_set_insertion_point(
                mtext_data,
                mtext,
                f64::NAN,
                6.0,
                7.0,
            )
        },
        DWG_EINVAL
    );
    assert_last_error(DWG_EINVAL);
    assert_eq!(
        unsafe { read_vec3(dwg_entity_mtext_get_insertion_point, mtext) },
        (5.0, 6.0, 7.0)
    );
    assert_eq!(
        unsafe {
            dwg_entity_mtext_set_insertion_point(
                mtext_data,
                mtext,
                5.0,
                f64::INFINITY,
                7.0,
            )
        },
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
        unsafe {
            dwg_entity_mtext_set_insertion_point(mtext_data, mtext_line, 8.0, 9.0, 10.0)
        },
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

    assert_eq!(unsafe { dwg_entity_line_set_start(data, line, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_line_set_end(data, line, p2.0, p2.1, p2.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_circle_set_center(data, circle, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_circle_set_radius(data, circle, radius) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_arc_set_center(data, arc, p2.0, p2.1, p2.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_arc_set_radius(data, arc, radius) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_arc_set_start_angle(data, arc, start_angle) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_arc_set_end_angle(data, arc, end_angle) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_point_set_location(data, point, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ellipse_set_center(data, ellipse, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ellipse_set_major_axis(data, ellipse, p2.0, p2.1, p2.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, ratio) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ray_set_base_point(data, ray, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ray_set_direction(data, ray, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_xline_set_base_point(data, xline, p2.0, p2.1, p2.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_xline_set_direction(data, xline, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_solid_set_corner(data, solid, 1, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_solid_set_corner(data, solid, 4, p2.0, p2.1, p2.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_face3d_set_corner(data, face3d, 1, p1.0, p1.1, p1.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_face3d_set_corner(data, face3d, 4, p2.0, p2.1, p2.2) }, DWG_OK);

    assert_eq!(unsafe { read_vec3(dwg_entity_line_get_start, line) }, p1);
    assert_eq!(unsafe { read_vec3(dwg_entity_line_get_end, line) }, p2);
    assert_eq!(unsafe { read_vec3(dwg_entity_circle_get_center, circle) }, p1);
    assert_eq!(unsafe { read_f64(dwg_entity_circle_get_radius, circle) }, radius);
    assert_eq!(unsafe { read_vec3(dwg_entity_arc_get_center, arc) }, p2);
    assert_eq!(unsafe { read_f64(dwg_entity_arc_get_radius, arc) }, radius);
    assert_eq!(unsafe { read_f64(dwg_entity_arc_get_start_angle, arc) }, start_angle);
    assert_eq!(unsafe { read_f64(dwg_entity_arc_get_end_angle, arc) }, end_angle);
    assert_eq!(unsafe { read_vec3(dwg_entity_point_get_location, point) }, p1);
    assert_eq!(unsafe { read_vec3(dwg_entity_ellipse_get_center, ellipse) }, p1);
    assert_eq!(unsafe { read_vec3(dwg_entity_ellipse_get_major_axis, ellipse) }, p2);
    assert_eq!(unsafe { read_f64(dwg_entity_ellipse_get_minor_axis_ratio, ellipse) }, ratio);
    assert_eq!(unsafe { read_vec3(dwg_entity_ray_get_base_point, ray) }, p1);
    assert_eq!(unsafe { read_vec3(dwg_entity_xline_get_base_point, xline) }, p2);
    for direction in [
        unsafe { read_vec3(dwg_entity_ray_get_direction, ray) },
        unsafe { read_vec3(dwg_entity_xline_get_direction, xline) },
    ] {
        assert!((direction.0 - 11.0 / 434.0_f64.sqrt()).abs() < 1e-12);
        assert!((direction.1 - 12.0 / 434.0_f64.sqrt()).abs() < 1e-12);
        assert!((direction.2 - 13.0 / 434.0_f64.sqrt()).abs() < 1e-12);
    }
    assert_eq!(unsafe { read_corner(dwg_entity_solid_get_corner, solid, 1) }, p1);
    assert_eq!(unsafe { read_corner(dwg_entity_solid_get_corner, solid, 4) }, p2);
    assert_eq!(unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 1) }, p1);
    assert_eq!(unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 4) }, p2);

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

    assert_eq!(unsafe { dwg_entity_ray_set_direction(data, ray, f64::MAX, f64::MAX / 2.0, -f64::MAX / 4.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_xline_set_direction(data, xline, f64::MIN_POSITIVE, f64::MIN_POSITIVE / 2.0, -f64::MIN_POSITIVE / 4.0) }, DWG_OK);
    for direction in [
        unsafe { read_vec3(dwg_entity_ray_get_direction, ray) },
        unsafe { read_vec3(dwg_entity_xline_get_direction, xline) },
    ] {
        assert!(direction.0.is_finite() && direction.1.is_finite() && direction.2.is_finite());
        assert!((direction.0 * direction.0 + direction.1 * direction.1 + direction.2 * direction.2 - 1.0).abs() < 1e-12);
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

    assert_eq!(unsafe { dwg_entity_line_set_start(data, line, stable.0, stable.1, stable.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_circle_set_center(data, circle, stable.0, stable.1, stable.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_circle_set_radius(data, circle, 4.5) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_arc_set_start_angle(data, arc, 0.25) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 0.625) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_ray_set_direction(data, ray, 3.0, 4.0, 0.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_xline_set_direction(data, xline, 3.0, 4.0, 0.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_solid_set_corner(data, solid, 1, stable.0, stable.1, stable.2) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_face3d_set_corner(data, face3d, 1, stable.0, stable.1, stable.2) }, DWG_OK);

    assert_eq!(unsafe { dwg_entity_line_set_start(data, line, f64::NAN, 0.0, 0.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_line_get_start, line) }, stable);
    assert_eq!(unsafe { dwg_entity_arc_set_start_angle(data, arc, f64::INFINITY) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_arc_get_start_angle, arc) }, 0.25);
    assert_eq!(unsafe { dwg_entity_circle_set_radius(data, circle, 0.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_circle_set_radius(data, circle, f64::NAN) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_circle_get_radius, circle) }, 4.5);
    assert_eq!(unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 0.0) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, 1.1) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_ellipse_set_minor_axis_ratio(data, ellipse, f64::NAN) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_ellipse_get_minor_axis_ratio, ellipse) }, 0.625);
    assert_eq!(unsafe { dwg_entity_ray_set_direction(data, ray, 0.0, 0.0, 0.0) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_xline_set_direction(data, xline, f64::NAN, 0.0, 0.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_ray_get_direction, ray) }, (0.6, 0.8, 0.0));
    assert_eq!(unsafe { read_vec3(dwg_entity_xline_get_direction, xline) }, (0.6, 0.8, 0.0));
    assert_eq!(unsafe { dwg_entity_solid_set_corner(data, solid, 0, 1.0, 2.0, 3.0) }, DWG_EINVAL);
    assert_eq!(unsafe { dwg_entity_face3d_set_corner(data, face3d, 5, 1.0, 2.0, 3.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_corner(dwg_entity_solid_get_corner, solid, 1) }, stable);
    assert_eq!(unsafe { read_corner(dwg_entity_face3d_get_corner, face3d, 1) }, stable);
    assert_eq!(unsafe { dwg_entity_line_set_start(data, circle, 1.0, 2.0, 3.0) }, DWG_ETYPE);
    assert_last_error(DWG_ETYPE);
    assert_eq!(unsafe { read_vec3(dwg_entity_circle_get_center, circle) }, stable);
    assert_eq!(unsafe { dwg_entity_line_set_start(data, line, 21.0, 22.0, 23.0) }, DWG_OK);
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
    assert_eq!(unsafe { dwg_entity_ellipse_get_center(ell, &mut cx, &mut cy, &mut cz) }, DWG_OK);
    assert!((cx - 1.0).abs() < 1e-9 && (cy - 2.0).abs() < 1e-9);
    let mut ratio = 0.0;
    assert_eq!(unsafe { dwg_entity_ellipse_get_minor_axis_ratio(ell, &mut ratio) }, DWG_OK);
    assert!((ratio - 0.5).abs() < 1e-9);

    let mtx = find_object_of_type(data, acadrust::ffi::DwgObjectType::MText as i32);
    assert!(!mtx.is_null());
    let mut needed = 0usize;
    assert_eq!(unsafe { dwg_entity_mtext_get_value(mtx, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    assert_eq!(needed, 3);
    let mut h = 0.0;
    assert_eq!(unsafe { dwg_entity_mtext_get_height(mtx, &mut h) }, DWG_OK);
    assert!((h - 2.0).abs() < 1e-9);

    let sp = find_object_of_type(data, acadrust::ffi::DwgObjectType::Spline as i32);
    assert!(!sp.is_null());
    let mut degree = 0;
    assert_eq!(unsafe { dwg_entity_spline_get_degree(sp, &mut degree) }, DWG_OK);
    assert_eq!(degree, 3);
    let mut count = 0;
    assert_eq!(unsafe { dwg_entity_spline_get_control_point_count(sp, &mut count) }, DWG_OK);
    assert!(count >= 3);

    let lw = find_object_of_type(data, acadrust::ffi::DwgObjectType::LwPolyline as i32);
    assert!(!lw.is_null());
    let mut vcount = 0;
    assert_eq!(unsafe { dwg_entity_lwpolyline_get_vertex_count(lw, &mut vcount) }, DWG_OK);
    assert_eq!(vcount, 2);
    let mut vx = 0.0;
    let mut vy = 0.0;
    assert_eq!(unsafe { dwg_entity_lwpolyline_get_vertex(lw, 1, &mut vx, &mut vy) }, DWG_OK);
    assert!((vx - 10.0).abs() < 1e-9 && (vy - 5.0).abs() < 1e-9);

    // Common getters on the line-family sample from the first document.
    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(&path_buf);
}

fn write_batch2_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{Insert, Mesh, Polyline2D, Vertex2D, Viewport};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch2.dwg");
    let _ = std::fs::remove_file(&path);

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
    doc.add_entity(EntityType::Viewport(vp)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
}

#[test]
fn batch2_block_mesh_getters() {
    let (path, path_buf) = write_batch2_dwg("acadrust_ffi_p3b2");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);

    let ins = find_object_of_type(data, acadrust::ffi::DwgObjectType::Insert as i32);
    assert!(!ins.is_null());
    let mut needed = 0usize;
    assert_eq!(unsafe { dwg_entity_insert_get_block_name(ins, std::ptr::null_mut(), 0, &mut needed) }, DWG_OK);
    let mut name_buf = [0u8; 64];
    assert_eq!(
        unsafe { dwg_entity_insert_get_block_name(ins, name_buf.as_mut_ptr() as *mut _, name_buf.len(), &mut needed) },
        DWG_OK
    );
    eprintln!("block_name bytes: {:?}", &name_buf[..needed - 1]);
    assert!(needed >= 2, "block_name must be non-empty after round-trip");
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(unsafe { dwg_entity_insert_get_insert_point(ins, &mut x, &mut y, &mut z) }, DWG_OK);
    assert!((x - 1.0).abs() < 1e-9 && (y - 2.0).abs() < 1e-9 && (z - 3.0).abs() < 1e-9);

    let p2 = find_object_of_type(data, acadrust::ffi::DwgObjectType::Polyline2D as i32);
    assert!(!p2.is_null());
    let mut vcount = 0;
    assert_eq!(unsafe { dwg_entity_polyline2d_get_vertex_count(p2, &mut vcount) }, DWG_OK);
    assert_eq!(vcount, 2);
    assert_eq!(unsafe { dwg_entity_polyline2d_get_vertex(p2, 1, &mut x, &mut y, &mut z) }, DWG_OK);
    assert!((x - 5.0).abs() < 1e-9 && (y - 6.0).abs() < 1e-9);

    let mesh = find_object_of_type(data, acadrust::ffi::DwgObjectType::Mesh as i32);
    assert!(!mesh.is_null());
    let mut mcount = 0;
    assert_eq!(unsafe { dwg_entity_mesh_get_vertex_count(mesh, &mut mcount) }, DWG_OK);
    assert_eq!(mcount, 3);

    let vp = find_object_of_type(data, acadrust::ffi::DwgObjectType::Viewport as i32);
    assert!(!vp.is_null());
    let mut w = 0.0;
    let mut h = 0.0;
    assert_eq!(unsafe { dwg_entity_viewport_get_width(vp, &mut w) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_viewport_get_height(vp, &mut h) }, DWG_OK);
    assert!(w > 0.0 && h > 0.0);

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(&path_buf);
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
    doc.add_entity(EntityType::Hatch(h)).unwrap();

    let mut tol = Tolerance::new();
    tol.insertion_point = Vector3::new(1.0, 2.0, 0.0);
    tol.text = "TOL".to_string();
    doc.add_entity(EntityType::Tolerance(tol)).unwrap();

    let mut l = Leader::new();
    l.vertices = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 10.0, 0.0),
    ];
    doc.add_entity(EntityType::Leader(l)).unwrap();

    let mut a = AttributeDefinition::new("TAG1".to_string(), "Enter".to_string(), "D".to_string());
    a.insertion_point = Vector3::new(3.0, 4.0, 0.0);
    doc.add_entity(EntityType::AttributeDefinition(a)).unwrap();

    let mut s = Shape::new();
    s.insertion_point = Vector3::new(5.0, 6.0, 0.0);
    s.size = 2.0;
    s.rotation = 0.5;
    doc.add_entity(EntityType::Shape(s)).unwrap();

    let mut ml = MLine::from_points(&[
        Vector3::new(7.0, 8.0, 0.0),
        Vector3::new(9.0, 10.0, 0.0),
    ]);
    ml.style_name = "Standard".to_string();
    doc.add_entity(EntityType::MLine(ml)).unwrap();

    let mut hx = Helix::new();
    hx.axis_base_point = Vector3::new(0.0, 0.0, 0.0);
    hx.radius = 3.0;
    hx.turns = 4.0;
    doc.add_entity(EntityType::Helix(hx)).unwrap();

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
    assert_eq!(unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) }, DWG_OK);

    let mut needed = 0usize;
    let mut buf = [0u8; 64];

    let h = find_object_of_type(data, acadrust::ffi::DwgObjectType::Hatch as i32);
    assert!(!h.is_null());
    assert_eq!(
        unsafe { dwg_entity_hatch_get_pattern_name(h, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"SOLID");

    let tol = find_object_of_type(data, acadrust::ffi::DwgObjectType::Tolerance as i32);
    assert!(!tol.is_null());
    assert_eq!(
        unsafe { dwg_entity_tolerance_get_text(tol, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"TOL");

    let ldr = find_object_of_type(data, acadrust::ffi::DwgObjectType::Leader as i32);
    assert!(!ldr.is_null());
    let mut vcount = 0;
    assert_eq!(unsafe { dwg_entity_leader_get_vertex_count(ldr, &mut vcount) }, DWG_OK);
    assert_eq!(vcount, 2);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(unsafe { dwg_entity_leader_get_vertex(ldr, 1, &mut x, &mut y, &mut z) }, DWG_OK);
    assert!((x - 10.0).abs() < 1e-9 && (y - 10.0).abs() < 1e-9);

    let att = find_object_of_type(data, acadrust::ffi::DwgObjectType::AttributeDefinition as i32);
    assert!(!att.is_null());
    assert_eq!(
        unsafe { dwg_entity_attrdef_get_tag(att, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) },
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
    assert_eq!(unsafe { dwg_entity_helix_get_radius(hx, &mut radius) }, DWG_OK);
    assert!((radius - 3.0).abs() < 1e-9);
    let mut turns = 0.0;
    assert_eq!(unsafe { dwg_entity_helix_get_turns(hx, &mut turns) }, DWG_OK);
    assert!((turns - 4.0).abs() < 1e-9);

    unsafe { dwg_data_destroy(data) };
}

#[test]
fn phase3c_annotation_setters() {
    let fixture = write_batch3_dwg("acadrust_ffi_p3c2_annotation");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(fixture.c_path().as_ptr(), data) }, DWG_OK);

    let attrdef = find_object_of_type(data, DwgObjectType::AttributeDefinition as i32);
    let tolerance = find_object_of_type(data, DwgObjectType::Tolerance as i32);
    let mline = find_object_of_type(data, DwgObjectType::MLine as i32);
    let shape = find_object_of_type(data, DwgObjectType::Shape as i32);
    assert!(!attrdef.is_null() && !tolerance.is_null() && !mline.is_null() && !shape.is_null());

    let tag = CString::new("P3C_TAG").unwrap();
    let prompt = CString::new("P3C_PROMPT").unwrap();
    let default_value = CString::new("P3C_DEFAULT").unwrap();
    let text = CString::new("P3C_TOL").unwrap();
    let style = CString::new("P3C_STYLE").unwrap();

    assert_eq!(unsafe { dwg_entity_attrdef_set_tag(data, attrdef, tag.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_attrdef_set_prompt(data, attrdef, prompt.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_attrdef_set_default_value(data, attrdef, default_value.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_attrdef_set_insertion_point(data, attrdef, 10.0, 11.0, 12.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_attrdef_set_height(data, attrdef, 3.5) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_tolerance_set_text(data, tolerance, text.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_tolerance_set_insertion_point(data, tolerance, 20.0, 21.0, 22.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_mline_set_style_name(data, mline, style.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_mline_set_start_point(data, mline, 30.0, 31.0, 32.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_mline_set_scale_factor(data, mline, 1.75) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_shape_set_insertion_point(data, shape, 40.0, 41.0, 42.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_shape_set_size(data, shape, 2.5) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_shape_set_rotation(data, shape, 0.5) }, DWG_OK);

    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_tag, attrdef) }, "P3C_TAG");
    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_prompt, attrdef) }, "P3C_PROMPT");
    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_default_value, attrdef) }, "P3C_DEFAULT");
    assert_eq!(unsafe { read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef) }, (10.0, 11.0, 12.0));
    assert_eq!(unsafe { read_f64(dwg_entity_attrdef_get_height, attrdef) }, 3.5);
    assert_eq!(unsafe { read_string(dwg_entity_tolerance_get_text, tolerance) }, "P3C_TOL");
    assert_eq!(unsafe { read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance) }, (20.0, 21.0, 22.0));
    assert_eq!(unsafe { read_string(dwg_entity_mline_get_style_name, mline) }, "P3C_STYLE");
    assert_eq!(unsafe { read_vec3(dwg_entity_mline_get_start_point, mline) }, (30.0, 31.0, 32.0));
    assert_eq!(unsafe { read_f64(dwg_entity_mline_get_scale_factor, mline) }, 1.75);
    assert_eq!(unsafe { read_vec3(dwg_entity_shape_get_insertion_point, shape) }, (40.0, 41.0, 42.0));
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_rotation, shape) }, 0.5);

    let invalid_utf8 = [0xff_u8, 0];
    assert_eq!(unsafe { dwg_entity_attrdef_set_tag(std::ptr::null_mut(), attrdef, tag.as_ptr()) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_tag, attrdef) }, "P3C_TAG");
    assert_eq!(unsafe { dwg_entity_attrdef_set_prompt(data, std::ptr::null(), prompt.as_ptr()) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_prompt, attrdef) }, "P3C_PROMPT");
    assert_eq!(unsafe { dwg_entity_attrdef_set_default_value(data, attrdef, std::ptr::null()) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_attrdef_get_default_value, attrdef) }, "P3C_DEFAULT");
    assert_eq!(unsafe { dwg_entity_tolerance_set_text(data, tolerance, invalid_utf8.as_ptr().cast()) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_tolerance_get_text, tolerance) }, "P3C_TOL");
    assert_eq!(unsafe { dwg_entity_mline_set_style_name(data, mline, std::ptr::null()) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_string(dwg_entity_mline_get_style_name, mline) }, "P3C_STYLE");

    assert_eq!(unsafe { dwg_entity_attrdef_set_insertion_point(data, attrdef, f64::NAN, 11.0, 12.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_attrdef_get_insertion_point, attrdef) }, (10.0, 11.0, 12.0));
    assert_eq!(unsafe { dwg_entity_tolerance_set_insertion_point(std::ptr::null_mut(), tolerance, 20.0, 21.0, 22.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_tolerance_get_insertion_point, tolerance) }, (20.0, 21.0, 22.0));
    assert_eq!(unsafe { dwg_entity_mline_set_start_point(data, std::ptr::null(), 30.0, 31.0, 32.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_mline_get_start_point, mline) }, (30.0, 31.0, 32.0));
    assert_eq!(unsafe { dwg_entity_shape_set_insertion_point(data, shape, 40.0, f64::INFINITY, 42.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_vec3(dwg_entity_shape_get_insertion_point, shape) }, (40.0, 41.0, 42.0));

    assert_eq!(unsafe { dwg_entity_attrdef_set_height(data, attrdef, 0.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_attrdef_get_height, attrdef) }, 3.5);
    assert_eq!(unsafe { dwg_entity_mline_set_scale_factor(data, mline, f64::NAN) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_mline_get_scale_factor, mline) }, 1.75);
    assert_eq!(unsafe { dwg_entity_shape_set_size(data, shape, 0.0) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);
    assert_eq!(unsafe { dwg_entity_shape_set_rotation(std::ptr::null_mut(), shape, 0.5) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_rotation, shape) }, 0.5);
    assert_eq!(unsafe { dwg_entity_shape_set_rotation(data, shape, f64::INFINITY) }, DWG_EINVAL);
    assert_last_error(DWG_EINVAL);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_rotation, shape) }, 0.5);

    assert_eq!(unsafe { dwg_entity_shape_set_size(data, tolerance, 3.0) }, DWG_ETYPE);
    assert_last_error(DWG_ETYPE);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);

    let foreign_fixture = write_batch3_dwg("acadrust_ffi_p3c2_annotation_foreign");
    let mut foreign_data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut foreign_data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(foreign_fixture.c_path().as_ptr(), foreign_data) }, DWG_OK);
    let foreign_shape = find_object_of_type(foreign_data, DwgObjectType::Shape as i32);
    assert_eq!(unsafe { dwg_entity_shape_set_size(data, foreign_shape, 3.0) }, DWG_ENOTFOUND);
    assert_last_error(DWG_ENOTFOUND);
    assert_eq!(unsafe { read_f64(dwg_entity_shape_get_size, shape) }, 2.5);

    assert_eq!(unsafe { dwg_entity_attrdef_set_tag(data, attrdef, tag.as_ptr()) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_tolerance_set_insertion_point(data, tolerance, 20.0, 21.0, 22.0) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_mline_set_scale_factor(data, mline, 1.75) }, DWG_OK);
    assert_eq!(unsafe { dwg_entity_shape_set_rotation(data, shape, 0.5) }, DWG_OK);
    assert_eq!(unsafe { dwg_last_error_code() }, DWG_OK);

    unsafe { dwg_data_destroy(foreign_data) };
    unsafe { dwg_data_destroy(data) };
}

fn write_batch4_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{Dimension, DimensionLinear, RasterImage};
    use acadrust::types::{Vector2, Vector3};
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch4.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    let dim = DimensionLinear::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
    doc.add_entity(EntityType::Dimension(Dimension::Linear(dim))).unwrap();

    let mut img = RasterImage::new("img.png", Vector3::new(1.0, 1.0, 0.0), 640.0, 480.0);
    doc.add_entity(EntityType::RasterImage(img)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
}

#[test]
fn batch4_dimension_image_getters() {
    let (path, path_buf) = write_batch4_dwg("acadrust_ffi_p3b4");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);

    let dim = find_object_of_type(data, acadrust::ffi::DwgObjectType::Dimension as i32);
    assert!(!dim.is_null());
    let mut subtype = 0;
    assert_eq!(unsafe { dwg_entity_dimension_get_subtype(dim, &mut subtype) }, DWG_OK);
    assert_eq!(subtype, 2); // Linear
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    assert_eq!(unsafe { dwg_entity_dimension_get_definition_point(dim, &mut x, &mut y, &mut z) }, DWG_OK);
    assert!((x - 0.0).abs() < 1e-9 && (y - 0.0).abs() < 1e-9);
    let mut needed = 0usize;
    let mut buf = [0u8; 64];
    assert_eq!(
        unsafe { dwg_entity_dimension_get_style_name(dim, buf.as_mut_ptr() as *mut _, buf.len(), &mut needed) },
        DWG_OK
    );
    assert_eq!(&buf[..needed - 1], b"Standard");
    let mut m = 0.0;
    assert_eq!(unsafe { dwg_entity_dimension_get_measurement(dim, &mut m) }, DWG_OK);
    assert!(m > 0.0);

    let img = find_object_of_type(data, acadrust::ffi::DwgObjectType::RasterImage as i32);
    assert!(!img.is_null());
    let mut w = 0.0;
    let mut h = 0.0;
    assert_eq!(unsafe { dwg_entity_raster_image_get_size(img, &mut w, &mut h) }, DWG_OK);
    assert!((w - 640.0).abs() < 1e-9 && (h - 480.0).abs() < 1e-9);

    unsafe { dwg_data_destroy(data) };
    let _ = std::fs::remove_file(&path_buf);
}
