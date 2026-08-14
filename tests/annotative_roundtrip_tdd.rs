//! TDD regression tests for annotative-style round-trips.
//!
//! Bug under test: the DXF round-trip loses the annotative flag on
//! MLEADERSTYLE / TABLESTYLE objects (observed to be order/run dependent,
//! sometimes "mleader lost", sometimes "table style lost").

use std::io::Cursor;

use acadrust::objects::{MultiLeaderStyle, ObjectType, TableStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn dxf_roundtrip(doc: CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(&doc).write_to_vec().expect("DXF write failed");
    let reader = DxfReader::from_reader(Cursor::new(bytes)).expect("DXF reader init failed");
    reader.read().expect("DXF read failed")
}

fn dwg_roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("DWG write failed");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    reader.read().expect("DWG read failed")
}

fn inventory(doc: &CadDocument) -> String {
    let mut lines = Vec::new();
    for (handle, obj) in &doc.objects {
        let desc = match obj {
            ObjectType::MultiLeaderStyle(s) => {
                format!("MultiLeaderStyle(name={}, annotative={})", s.name, s.is_annotative)
            }
            ObjectType::TableStyle(s) => {
                format!("TableStyle(name={}, annotative={})", s.name, s.annotative)
            }
            ObjectType::Dictionary(d) => format!("Dictionary({} entries)", d.entries.len()),
            other => format!("{other:?}"),
        };
        lines.push(format!("  {handle:?} -> {desc}"));
    }
    lines.sort();
    lines.join("\n")
}

fn build_doc() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut mls = MultiLeaderStyle::new("AnnoML");
    mls.handle = doc.allocate_handle();
    mls.is_annotative = true;
    doc.objects.insert(mls.handle, ObjectType::MultiLeaderStyle(mls));

    let mut tbs = TableStyle::new("AnnoTS");
    tbs.handle = doc.allocate_handle();
    tbs.annotative = true;
    doc.objects.insert(tbs.handle, ObjectType::TableStyle(tbs));
    doc
}

fn mleader_annotative(doc: &CadDocument) -> bool {
    doc.objects.values().any(|o| matches!(o, ObjectType::MultiLeaderStyle(s) if s.is_annotative))
}

fn table_annotative(doc: &CadDocument) -> bool {
    doc.objects.values().any(|o| matches!(o, ObjectType::TableStyle(s) if s.annotative))
}

#[test]
fn dxf_roundtrip_mleader_annotative_only() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut mls = MultiLeaderStyle::new("AnnoML");
    mls.handle = doc.allocate_handle();
    mls.is_annotative = true;
    doc.objects.insert(mls.handle, ObjectType::MultiLeaderStyle(mls));

    let rt = dxf_roundtrip(doc);
    assert!(
        mleader_annotative(&rt),
        "DXF: mleader style annotative lost.\n{}",
        inventory(&rt)
    );
}

#[test]
fn dxf_roundtrip_tablestyle_annotative_only() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut tbs = TableStyle::new("AnnoTS");
    tbs.handle = doc.allocate_handle();
    tbs.annotative = true;
    doc.objects.insert(tbs.handle, ObjectType::TableStyle(tbs));

    let rt = dxf_roundtrip(doc);
    assert!(
        table_annotative(&rt),
        "DXF: table style annotative lost.\n{}",
        inventory(&rt)
    );
}

#[test]
fn dxf_roundtrip_both_annotative() {
    let rt = dxf_roundtrip(build_doc());
    assert!(
        mleader_annotative(&rt),
        "DXF: mleader style annotative lost.\n{}",
        inventory(&rt)
    );
    assert!(
        table_annotative(&rt),
        "DXF: table style annotative lost.\n{}",
        inventory(&rt)
    );
}

#[test]
fn dwg_roundtrip_both_annotative() {
    let rt = dwg_roundtrip(&build_doc());
    assert!(
        mleader_annotative(&rt),
        "DWG: mleader style annotative lost.\n{}",
        inventory(&rt)
    );
    assert!(
        table_annotative(&rt),
        "DWG: table style annotative lost.\n{}",
        inventory(&rt)
    );
}
