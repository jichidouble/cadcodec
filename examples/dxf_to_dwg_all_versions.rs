//! Read a DXF and write it as DWG in every supported version.
//!
//! Usage:
//!   cargo run --example dxf_to_dwg_all_versions -- <input.dxf> <output_dir>

use std::env;
use std::fs;
use std::path::Path;

use acadrust::types::DxfVersion;
use acadrust::{DwgWriter, DxfReader};

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

fn main() {
    let args: Vec<String> = env::args().collect();
    let dxf_path = args.get(1).expect("usage: dxf_to_dwg_all_versions <input.dxf> <output_dir>");
    let out_dir = args.get(2).expect("usage: dxf_to_dwg_all_versions <input.dxf> <output_dir>");
    fs::create_dir_all(out_dir).expect("create output dir");

    let reader = DxfReader::from_file(dxf_path).expect("open dxf");
    let mut doc = reader.read().expect("read dxf");
    println!("Read {} ({} entities, {} objects)", dxf_path, doc.entities().count(), doc.objects.len());

    for (label, version) in VERSIONS {
        let mut d = doc.clone();
        d.version = *version;
        let path = Path::new(out_dir).join(format!("acadrust_{label}.dwg"));
        match DwgWriter::write_to_file(&path, &d) {
            Ok(_) => println!("{label}: wrote {} bytes", fs::metadata(&path).map(|m| m.len()).unwrap_or(0)),
            Err(e) => println!("{label}: FAILED {e}"),
        }
    }
}
