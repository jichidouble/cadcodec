//! Multi-version DWG output matrix.
//!
//! Writes a representative document in every DWG version the library
//! supports (R13 .. R2018), then re-reads each output with the library
//! itself to confirm the writer's own round-trip is intact.
//!
//! Optional third-party validation with ODA File Converter (run manually):
//!
//!   $env:ODA_FILE_CONVERTER = "D:\Program Files\ODA\ODADrawingsExplorer 27.1.0\OdaFileConverter.exe"
//!   cargo test --test dwg_output_matrix -- --ignored
//!
//! The ODA check converts every generated file to ACAD2018; a version is
//! only counted as passing when ODA produces an output file with no `.err`.
//!
//! Known limitation (as of 0.4.1): `CadDocument::with_version` always
//! installs the R2007+ `MultiLeaderStyle`/`TableStyle` objects via
//! `initialize_defaults`, and the DWG writer does not filter objects by
//! target version. ODA therefore rejects the R13/R14 outputs with
//! `Object improperly read: <AcDbMLeaderStyle>` while the library's own
//! read-back succeeds. R2000+ outputs (including the previously broken
//! R2010+ VisualStyle serialization) pass ODA validation.

use std::fs;
use std::path::{Path, PathBuf};

use acadrust::entities::*;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};

const VERSIONS: &[(&str, DxfVersion)] = &[
    ("R13", DxfVersion::AC1012),
    ("R14", DxfVersion::AC1014),
    ("R2000", DxfVersion::AC1015),
    ("R2004", DxfVersion::AC1018),
    ("R2007", DxfVersion::AC1021),
    ("R2010", DxfVersion::AC1024),
    ("R2013", DxfVersion::AC1027),
    ("R2018", DxfVersion::AC1032),
];

fn output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dwg_output_matrix")
}

fn build_document() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);

    let mut wall = acadrust::tables::Layer::new("A-WALL");
    wall.color = acadrust::types::Color::from_index(1);
    doc.layers.add(wall);

    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 100.0, 50.0, 0.0))).unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(10.0, 20.0, 0.0), 5.0))).unwrap();
    doc.add_entity(EntityType::Arc(Arc::from_center_radius_angles(Vector3::ZERO, 10.0, 0.0, 1.5707963))).unwrap();

    let mut lw = LwPolyline::new();
    lw.add_vertex(LwVertex::from_coords(0.0, 0.0));
    lw.add_vertex(LwVertex::from_coords(10.0, 0.0));
    lw.add_vertex(LwVertex::from_coords(10.0, 10.0));
    doc.add_entity(EntityType::LwPolyline(lw)).unwrap();

    let mut spline = Spline::new();
    spline.control_points = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(5.0, 10.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
        Vector3::new(15.0, 10.0, 0.0),
    ];
    doc.add_entity(EntityType::Spline(spline)).unwrap();

    let mut txt = Text::new();
    txt.value = "Hello acadrust".to_string();
    txt.insertion_point = Vector3::ZERO;
    txt.height = 2.5;
    doc.add_entity(EntityType::Text(txt)).unwrap();

    let mut mtxt = MText::new();
    mtxt.insertion_point = Vector3::new(20.0, 20.0, 0.0);
    mtxt.height = 2.5;
    mtxt.value = "Multi\\Pline text".to_string();
    doc.add_entity(EntityType::MText(mtxt)).unwrap();

    let dim = DimensionLinear::new(Vector3::ZERO, Vector3::new(10.0, 0.0, 0.0));
    doc.add_entity(EntityType::Dimension(Dimension::Linear(dim))).unwrap();

    let insert = Insert::new("WALL", Vector3::new(5.0, 5.0, 0.0));
    doc.add_entity(EntityType::Insert(insert)).unwrap();

    doc.add_entity(EntityType::Point(Point::new())).unwrap();
    doc.add_entity(EntityType::Hatch(Hatch::new())).unwrap();

    let mut vp = Viewport::new();
    vp.center = Vector3::new(50.0, 50.0, 0.0);
    vp.width = 100.0;
    vp.height = 50.0;
    doc.add_entity(EntityType::Viewport(vp)).unwrap();

    doc
}

fn oda_converter() -> Option<PathBuf> {
    let exe = std::env::var_os("ODA_FILE_CONVERTER")?;
    let p = PathBuf::from(exe);
    p.exists().then_some(p)
}

/// Convert every DWG in `in_dir` to ACAD2018 via ODA File Converter and
/// return the list of input stems that failed (missing output or `.err`).
fn run_oda_conversion(converter: &Path, in_dir: &Path) -> Vec<String> {
    let out_dir = in_dir.join("oda_out_2018");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("create ODA out dir");

    let result = std::process::Command::new(converter)
        .args([
            in_dir.as_os_str(),
            out_dir.as_os_str(),
            "ACAD2018".as_ref(),
            "DWG".as_ref(),
            "0".as_ref(),
            "1".as_ref(),
        ])
        .status();

    let mut failures = Vec::new();
    match result {
        Ok(status) if status.success() => {
            for entry in fs::read_dir(in_dir).expect("read in dir") {
                let p = entry.expect("entry").path();
                if p.extension().and_then(|e| e.to_str()) != Some("dwg") {
                    continue;
                }
                let stem = p.file_stem().unwrap().to_string_lossy().to_string();
                let out = out_dir.join(format!("{stem}.dwg"));
                let err = out_dir.join(format!("{stem}.dwg.err"));
                if !out.exists() || err.exists() {
                    failures.push(stem);
                }
            }
        }
        Ok(_) => failures.push("ODA exit status non-zero".to_string()),
        Err(e) => failures.push(format!("ODA failed to launch: {e}")),
    }
    failures
}

#[test]
fn write_all_supported_dwg_versions_and_verify() {
    let dir = output_dir();
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create output dir");

    let doc = build_document();
    let mut failures: Vec<String> = Vec::new();

    for (label, version) in VERSIONS {
        let mut d = doc.clone();
        d.version = *version;
        let path = dir.join(format!("acadrust_{label}.dwg"));
        match DwgWriter::write_to_file(&path, &d) {
            Ok(_) => match DwgReader::from_file(&path).and_then(|mut r| r.read()) {
                Ok(read) => {
                    let expected = doc.entities().count();
                    let actual = read.entities().count();
                    println!("{label}: write OK, read-back OK ({actual} entities, expected {expected})");
                    if actual != expected {
                        failures.push(format!("{label}: entity count {actual} != {expected}"));
                    }
                }
                Err(e) => failures.push(format!("{label}: read-back failed: {e}")),
            },
            Err(e) => failures.push(format!("{label}: write failed: {e}")),
        }
    }

    assert!(failures.is_empty(), "DWG output failures:\n{}", failures.join("\n"));
    println!(
        "Wrote {} versioned DWGs to {}",
        VERSIONS.len(),
        dir.display()
    );
}

#[test]
#[ignore = "requires ODA File Converter; set ODA_FILE_CONVERTER and run with --ignored"]
fn oda_accepts_all_written_versions() {
    let converter = oda_converter().expect("set ODA_FILE_CONVERTER to OdaFileConverter.exe");
    let dir = output_dir();
    if !dir.join("acadrust_R2018.dwg").exists() {
        // Ensure outputs exist even if the write test was skipped.
        let doc = build_document();
        for (label, version) in VERSIONS {
            let mut d = doc.clone();
            d.version = *version;
            DwgWriter::write_to_file(dir.join(format!("acadrust_{label}.dwg")), &d).unwrap();
        }
    }
    let failures = run_oda_conversion(&converter, &dir);
    assert!(
        failures.is_empty(),
        "ODA rejected or failed on these DWG outputs: {}",
        failures.join(", ")
    );
    println!("ODA accepted all {} versions", VERSIONS.len());
}
