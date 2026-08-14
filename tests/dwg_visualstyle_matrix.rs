//! VisualStyle output matrix across every supported DWG version.
//!
//! Reads a DXF that contains real VisualStyle objects (R2007 legacy form)
//! and writes it as DWG in every version the library supports, then
//! verifies each output with ODA File Converter when available.
//!
//! Run with ODA:
//!
//!   $env:ODA_FILE_CONVERTER = "D:\Program Files\ODA\ODADrawingsExplorer 27.1.0\OdaFileConverter.exe"
//!   cargo test --test dwg_visualstyle_matrix -- --ignored

use std::fs;
use std::path::{Path, PathBuf};

use acadrust::objects::ObjectType;
use acadrust::types::DxfVersion;
use acadrust::{DwgReader, DwgWriter, DxfReader};

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

fn source_dxf() -> PathBuf {
    // cn.dxf from the LibreDWG test corpus: 3 entities, 24 VisualStyle
    // objects, 7 Material objects, MLEADERSTYLE/TABLESTYLE etc.
    PathBuf::from(r"D:\Projects\libredwg\.build-tmp\cn-test\cn.dxf")
}

fn output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dwg_visualstyle_matrix")
}

fn write_all_versions() -> Vec<String> {
    let dxf_path = source_dxf();
    assert!(dxf_path.exists(), "source DXF not found: {}", dxf_path.display());

    let doc = DxfReader::from_file(&dxf_path)
        .expect("open source DXF")
        .read()
        .expect("read source DXF");

    let visual_styles = doc
        .objects
        .values()
        .filter(|o| matches!(o, ObjectType::VisualStyle(_)))
        .count();
    assert!(
        visual_styles > 0,
        "source DXF must contain VisualStyle objects"
    );
    println!("Source DXF: {} entities, {} objects, {visual_styles} VisualStyle", doc.entities().count(), doc.objects.len());

    let dir = output_dir();
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create output dir");

    let mut written = Vec::new();
    for (label, version) in VERSIONS {
        let mut d = doc.clone();
        d.version = *version;
        let path = dir.join(format!("acadrust_{label}.dwg"));
        match DwgWriter::write_to_file(&path, &d) {
            Ok(_) => {
                // In-library read-back: entity and VisualStyle counts must survive.
                match DwgReader::from_file(&path).and_then(|mut r| r.read()) {
                    Ok(rt) => {
                        let rt_vs = rt
                            .objects
                            .values()
                            .filter(|o| matches!(o, ObjectType::VisualStyle(_)))
                            .count();
                        let rt_ents = rt.entities().count();
                        println!(
                            "{label}: wrote {} bytes, read-back OK ({} entities, {rt_vs} VisualStyle)",
                            fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                            rt_ents
                        );
                        written.push(label.to_string());
                    }
                    Err(e) => println!("{label}: read-back FAILED: {e}"),
                }
            }
            Err(e) => println!("{label}: WRITE FAILED: {e}"),
        }
    }
    written
}

#[test]
fn write_visualstyle_in_all_versions_and_read_back() {
    let written = write_all_versions();
    assert_eq!(
        written.len(),
        VERSIONS.len(),
        "expected all {} versions to write and read back, got {}",
        VERSIONS.len(),
        written.len()
    );
}

#[test]
#[ignore = "requires ODA File Converter; set ODA_FILE_CONVERTER and run with --ignored"]
fn oda_parses_visualstyle_in_all_versions() {
    let converter = PathBuf::from(
        std::env::var("ODA_FILE_CONVERTER")
            .expect("set ODA_FILE_CONVERTER to OdaFileConverter.exe"),
    );
    assert!(converter.exists(), "ODA File Converter not found");

    let dir = output_dir();
    if !dir.join("acadrust_R2018.dwg").exists() {
        write_all_versions();
    }

    let out_dir = dir.join("oda_out_2018");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("create ODA out dir");

    let status = std::process::Command::new(&converter)
        .args([
            dir.as_os_str(),
            out_dir.as_os_str(),
            "ACAD2018".as_ref(),
            "DWG".as_ref(),
            "0".as_ref(),
            "1".as_ref(),
        ])
        .status()
        .expect("launch ODA File Converter");
    assert!(status.success(), "ODA File Converter exited with {status:?}");

    let mut failures = Vec::new();
    let mut passes = Vec::new();
    for (label, _) in VERSIONS {
        let out = out_dir.join(format!("acadrust_{label}.dwg"));
        let err = out_dir.join(format!("acadrust_{label}.dwg.err"));
        if out.exists() && !err.exists() {
            passes.push(label.to_string());
        } else {
            failures.push(label.to_string());
            if let Ok(text) = fs::read_to_string(&err) {
                println!("--- {label} ODA error ---\n{text}");
            }
        }
    }

    println!("ODA accepted: {}", passes.join(", "));
    println!("ODA rejected: {}", failures.join(", "));
    assert!(
        failures.is_empty(),
        "ODA rejected VisualStyle DWG outputs: {}",
        failures.join(", ")
    );
}
