//! ABI-level tests calling the exported extern "C" symbols directly.

use acadrust::ffi::*;
use std::ffi::CString;

/// Build a temp DWG containing Line, Circle and Text via the Rust API, then
/// return its path (caller deletes it).
fn write_sample_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{Circle, Line, Text};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 2.0, 3.0))).unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
    let mut t = Text::new();
    t.value = "hello".to_string();
    t.height = 1.0;
    doc.add_entity(EntityType::Text(t)).unwrap();
    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
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
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
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
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
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
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
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
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
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
    let dll = std::path::Path::new(manifest).join("target/debug/acadrust.dll");
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
    let (path, path_buf) = write_sample_dwg("acadrust_ffi_p2_iter");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);

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
    let _ = std::fs::remove_file(&path_buf);
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

fn open_sample_document() -> (*mut DwgData, std::path::PathBuf) {
    let (path, path_buf) = write_sample_dwg("acadrust_ffi_p2_entity");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);
    (data, path_buf)
}

#[test]
fn line_and_circle_getters_return_values() {
    let (data, path_buf) = open_sample_document();
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
    let _ = std::fs::remove_file(&path_buf);
}

#[test]
fn text_getter_reads_value_with_buffer_protocol() {
    let (data, path_buf) = open_sample_document();
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
    let _ = std::fs::remove_file(&path_buf);
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
    use acadrust::entities::{Ellipse, LwPolyline, LwVertex, MText, Ray, Spline, XLine};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch1.dwg");
    let _ = std::fs::remove_file(&path);

    let mut doc = CadDocument::new();
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

fn write_batch3_dwg(dir_name: &str) -> (CString, std::path::PathBuf) {
    use acadrust::entities::{AttributeDefinition, Hatch, HatchPattern, Helix, Leader, Shape, Tolerance};
    use acadrust::types::Vector3;
    use acadrust::{CadDocument, DwgWriter, EntityType};

    let dir = std::env::temp_dir().join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("batch3.dwg");
    let _ = std::fs::remove_file(&path);

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

    let mut hx = Helix::new();
    hx.axis_base_point = Vector3::new(0.0, 0.0, 0.0);
    hx.radius = 3.0;
    hx.turns = 4.0;
    doc.add_entity(EntityType::Helix(hx)).unwrap();

    DwgWriter::write_to_file(&path, &doc).unwrap();
    (CString::new(path.to_str().unwrap()).unwrap(), path)
}

#[test]
fn batch3_annotation_text_getters() {
    let (path, path_buf) = write_batch3_dwg("acadrust_ffi_p3b3");
    let mut data: *mut DwgData = std::ptr::null_mut();
    assert_eq!(unsafe { dwg_data_create(&mut data) }, DWG_OK);
    assert_eq!(unsafe { dwg_read_file(path.as_ptr(), data) }, DWG_OK);

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
    let _ = std::fs::remove_file(&path_buf);
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
