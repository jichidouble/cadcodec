//! DXF round-trip of a LAYER record's true color (group 420).
//!
//! Regression guard for the LAYER DXF reader's true-colour handling: an RGB
//! layer must come back as the exact RGB value, not fall back to the ACI
//! index (7/white) carried by group 62.
use std::io::Cursor;

use acadrust::tables::Layer;
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DxfReader, DxfWriter};

#[test]
fn dxf_roundtrip_layer_true_color() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut layer = Layer::new("RGBLayer");
    layer.color = Color::Rgb {
        r: 0x12,
        g: 0x34,
        b: 0x56,
    };
    doc.layers.add(layer).expect("add layer");

    let writer = DxfWriter::new(&doc);
    let bytes = writer.write_to_vec().expect("DXF write");

    let reader = DxfReader::from_reader(Cursor::new(bytes)).expect("reader init");
    let rt = reader.read().expect("DXF read");

    let back = rt.layers.get("RGBLayer").expect("layer present");
    assert_eq!(
        back.color,
        Color::Rgb {
            r: 0x12,
            g: 0x34,
            b: 0x56
        }
    );
}

#[test]
fn dxf_layer_true_color_with_negative_aci_index() {
    // A negative code-62 index marks an OFF layer; the 420 true color must
    // still be honoured on top of it (both match arms in the reader).
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    let mut layer = Layer::new("RGBOffLayer");
    layer.color = Color::Rgb {
        r: 0xAA,
        g: 0xBB,
        b: 0xCC,
    };
    doc.layers.add(layer).expect("add layer");

    let writer = DxfWriter::new(&doc);
    let bytes = writer.write_to_vec().expect("DXF write");
    let reader = DxfReader::from_reader(Cursor::new(bytes)).expect("reader init");
    let rt = reader.read().expect("DXF read");
    let back = rt.layers.get("RGBOffLayer").expect("layer present");
    assert_eq!(
        back.color,
        Color::Rgb {
            r: 0xAA,
            g: 0xBB,
            b: 0xCC
        }
    );
}
