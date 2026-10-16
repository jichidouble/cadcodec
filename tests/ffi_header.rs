use std::{env, fs, path::PathBuf};

fn generated_header() -> String {
    let header_path = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set by Cargo when locating include/dwg_api.h"),
    )
    .join("include")
    .join("dwg_api.h");

    fs::read_to_string(&header_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", header_path.display()))
}

#[test]
fn generated_header_preserves_public_ffi_contract() {
    let header = generated_header();

    for required in [
        "typedef struct DwgData",
        "typedef struct DwgObject",
        "dwg_data_create",
        "dwg_data_entity_count",
        "dwg_data_object_count",
        "dwg_object_by_handle",
        "dwg_object_remove",
        "dwg_entity_line_create",
        "dwg_entity_circle_create",
        "dwg_entity_arc_create",
        "dwg_entity_point_create",
        "dwg_set_notification_callback",
        "dwg_entity_acis_set_sat",
        "DWG_OK",
        "DWG_XDATA_STRING",
        "DWG_XRECORD_STRING",
    ] {
        assert!(
            header.contains(required),
            "required FFI token missing: {required}"
        );
    }

    let normalized_header: String = header.split_whitespace().collect();

    assert!(
        normalized_header.contains("typedefvoid(*DwgNotificationCallback)"),
        "notification callback must remain a plain function-pointer typedef"
    );
    assert!(
        normalized_header.contains("DwgNotificationCallbackcallback"),
        "notification setter must accept the callback typedef"
    );
    assert!(
        !header.contains("Option_DwgNotificationCallback"),
        "Option-wrapped callback leaked into the public header"
    );

    for forbidden in [
        "FfiDocument",
        "FfiObject",
        "CadDocument",
        "EntityType",
        "CHARACTER",
    ] {
        assert!(
            !header.contains(forbidden),
            "internal type leaked: {forbidden}"
        );
    }
}
