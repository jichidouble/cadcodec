//! TDD regression tests for Header-variable round-trips.
//!
//! Bugs under test:
//! 1. `$DIMAZIN` (angular zero suppression) was read from DXF but never
//!    written back — the value was silently lost on a DXF save.
//! 2. The DWG Header `INTERFERECOLOR` field (spec §D.5 item 97) was written
//!    with the `INTERSECTIONCOLOR` value and discarded on read; the two
//!    variables are independent and must round-trip independently.

use std::io::Cursor;

use acadrust::types::{Color, DxfVersion};
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

#[test]
fn dxf_roundtrip_preserves_dimazin() {
    // §D.4 item 64: R2000+ DIMTAD, DIMZIN, DIMAZIN as BS. The DXF writer must
    // emit $DIMAZIN (group 70) or the value is lost on save.
    let mut doc = CadDocument::with_version(DxfVersion::AC1024);
    doc.header.dim_alt_zero_suppression = 3; // DIMAZIN: suppress leading zeros

    let rt = dxf_roundtrip(doc);
    assert_eq!(
        rt.header.dim_alt_zero_suppression, 3,
        "DIMAZIN must survive a DXF round-trip"
    );
}

#[test]
fn dxf_roundtrip_preserves_interference_color() {
    // $INTERFERECOLOR (group 62) is a distinct R2007+ header variable and must
    // round-trip independently of $INTERSECTIONCOLOR.
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.header.interference_color = Color::from_index(5);
    doc.header.intersection_color = 7;

    let rt = dxf_roundtrip(doc);
    assert_eq!(
        rt.header.interference_color.approximate_index(),
        5,
        "INTERFERECOLOR must survive a DXF round-trip"
    );
    assert_eq!(
        rt.header.intersection_color, 7,
        "INTERSECTIONCOLOR must remain independent"
    );
}

#[test]
fn dwg_roundtrip_preserves_interference_color() {
    // §D.5 item 97: R2007+ B, CMC(INTERFERECOLOR), H*, H*, H*, RC, BD. The CMC
    // slot carries INTERFERECOLOR, not INTERSECTIONCOLOR.
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.header.interference_color = Color::from_index(5);
    doc.header.intersection_color = 7;

    let rt = dwg_roundtrip(&doc);
    assert_eq!(
        rt.header.interference_color.approximate_index(),
        5,
        "INTERFERECOLOR must survive a DWG round-trip"
    );
    assert_eq!(
        rt.header.intersection_color, 7,
        "INTERSECTIONCOLOR must remain independent"
    );
}

#[test]
fn dxf_dwg_dxf_preserves_both() {
    // Cross-format: DXF -> DWG -> DXF must keep INTERFERECOLOR and DIMAZIN.
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.header.interference_color = Color::from_index(9);
    doc.header.dim_alt_zero_suppression = 1;

    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write failed");
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    let from_dwg = reader.read().expect("DWG read failed");

    let rt = dxf_roundtrip(from_dwg);
    assert_eq!(
        rt.header.interference_color.approximate_index(),
        9,
        "INTERFERECOLOR must survive DXF -> DWG -> DXF"
    );
    assert_eq!(
        rt.header.dim_alt_zero_suppression, 1,
        "DIMAZIN must survive DXF -> DWG -> DXF"
    );
}
