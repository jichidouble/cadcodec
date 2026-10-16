//! Compile-time and linker-level inventory for the FFI entity getter/setter ABI.

const GETTERS: [&str; 84] = [
    "dwg_entity_arc_get_center",
    "dwg_entity_arc_get_end_angle",
    "dwg_entity_arc_get_radius",
    "dwg_entity_arc_get_start_angle",
    "dwg_entity_attrdef_get_default_value",
    "dwg_entity_attrdef_get_height",
    "dwg_entity_attrdef_get_insertion_point",
    "dwg_entity_attrdef_get_prompt",
    "dwg_entity_attrdef_get_tag",
    "dwg_entity_circle_get_center",
    "dwg_entity_circle_get_radius",
    "dwg_entity_dimension_get_definition_point",
    "dwg_entity_dimension_get_measurement",
    "dwg_entity_dimension_get_style_name",
    "dwg_entity_dimension_get_subtype",
    "dwg_entity_dimension_get_text_middle_point",
    "dwg_entity_ellipse_get_center",
    "dwg_entity_ellipse_get_major_axis",
    "dwg_entity_ellipse_get_minor_axis_ratio",
    "dwg_entity_face3d_get_corner",
    "dwg_entity_get_color",
    "dwg_entity_get_layer",
    "dwg_entity_get_linetype",
    "dwg_entity_get_linetype_scale",
    "dwg_entity_hatch_get_elevation",
    "dwg_entity_hatch_get_pattern_angle",
    "dwg_entity_hatch_get_pattern_name",
    "dwg_entity_hatch_get_pattern_scale",
    "dwg_entity_helix_get_axis_base_point",
    "dwg_entity_helix_get_radius",
    "dwg_entity_helix_get_turn_height",
    "dwg_entity_helix_get_turns",
    "dwg_entity_insert_get_array",
    "dwg_entity_insert_get_block_name",
    "dwg_entity_insert_get_insert_point",
    "dwg_entity_insert_get_normal",
    "dwg_entity_insert_get_rotation",
    "dwg_entity_leader_get_vertex",
    "dwg_entity_leader_get_vertex_count",
    "dwg_entity_light_get_position",
    "dwg_entity_line_get_end",
    "dwg_entity_line_get_start",
    "dwg_entity_lwpolyline_get_vertex",
    "dwg_entity_lwpolyline_get_vertex_count",
    "dwg_entity_mesh_get_face_count",
    "dwg_entity_mesh_get_face_vertex_count",
    "dwg_entity_mesh_get_vertex",
    "dwg_entity_mesh_get_vertex_count",
    "dwg_entity_mline_get_scale_factor",
    "dwg_entity_mline_get_start_point",
    "dwg_entity_mline_get_style_name",
    "dwg_entity_mtext_get_height",
    "dwg_entity_mtext_get_insertion_point",
    "dwg_entity_mtext_get_value",
    "dwg_entity_point_get_location",
    "dwg_entity_polyface_mesh_get_vertex",
    "dwg_entity_polyface_mesh_get_vertex_count",
    "dwg_entity_polyline2d_get_vertex",
    "dwg_entity_polyline2d_get_vertex_count",
    "dwg_entity_polyline3d_get_vertex",
    "dwg_entity_polyline3d_get_vertex_count",
    "dwg_entity_raster_image_get_insertion_point",
    "dwg_entity_raster_image_get_size",
    "dwg_entity_ray_get_base_point",
    "dwg_entity_ray_get_direction",
    "dwg_entity_shape_get_insertion_point",
    "dwg_entity_shape_get_rotation",
    "dwg_entity_shape_get_size",
    "dwg_entity_solid_get_corner",
    "dwg_entity_spline_get_control_point",
    "dwg_entity_spline_get_control_point_count",
    "dwg_entity_spline_get_degree",
    "dwg_entity_text_get_height",
    "dwg_entity_text_get_insertion_point",
    "dwg_entity_text_get_value",
    "dwg_entity_tolerance_get_insertion_point",
    "dwg_entity_tolerance_get_text",
    "dwg_entity_underlay_get_insertion_point",
    "dwg_entity_viewport_get_center",
    "dwg_entity_viewport_get_height",
    "dwg_entity_viewport_get_width",
    "dwg_entity_wipeout_get_insertion_point",
    "dwg_entity_xline_get_base_point",
    "dwg_entity_xline_get_direction",
];

const READ_ONLY: [&str; 11] = [
    "dwg_entity_spline_get_control_point_count",
    "dwg_entity_lwpolyline_get_vertex_count",
    "dwg_entity_polyline2d_get_vertex_count",
    "dwg_entity_polyline3d_get_vertex_count",
    "dwg_entity_polyface_mesh_get_vertex_count",
    "dwg_entity_leader_get_vertex_count",
    "dwg_entity_mesh_get_vertex_count",
    "dwg_entity_mesh_get_face_count",
    "dwg_entity_mesh_get_face_vertex_count",
    "dwg_entity_dimension_get_subtype",
    "dwg_entity_dimension_get_measurement",
];

const SETTERS: [&str; 74] = [
    "dwg_entity_arc_set_center",
    "dwg_entity_arc_set_end_angle",
    "dwg_entity_arc_set_radius",
    "dwg_entity_arc_set_start_angle",
    "dwg_entity_attrdef_set_default_value",
    "dwg_entity_attrdef_set_height",
    "dwg_entity_attrdef_set_insertion_point",
    "dwg_entity_attrdef_set_prompt",
    "dwg_entity_attrdef_set_tag",
    "dwg_entity_circle_set_center",
    "dwg_entity_circle_set_radius",
    "dwg_entity_dimension_set_definition_point",
    "dwg_entity_dimension_set_style_name",
    "dwg_entity_dimension_set_text_middle_point",
    "dwg_entity_ellipse_set_center",
    "dwg_entity_ellipse_set_major_axis",
    "dwg_entity_ellipse_set_minor_axis_ratio",
    "dwg_entity_face3d_set_corner",
    "dwg_entity_set_color",
    "dwg_entity_set_layer",
    "dwg_entity_set_linetype",
    "dwg_entity_set_linetype_scale",
    "dwg_entity_hatch_set_elevation",
    "dwg_entity_hatch_set_pattern_angle",
    "dwg_entity_hatch_set_pattern_name",
    "dwg_entity_hatch_set_pattern_scale",
    "dwg_entity_helix_set_axis_base_point",
    "dwg_entity_helix_set_radius",
    "dwg_entity_helix_set_turn_height",
    "dwg_entity_helix_set_turns",
    "dwg_entity_insert_set_array",
    "dwg_entity_insert_set_block_name",
    "dwg_entity_insert_set_insert_point",
    "dwg_entity_insert_set_normal",
    "dwg_entity_insert_set_rotation",
    "dwg_entity_leader_set_vertex",
    "dwg_entity_light_set_position",
    "dwg_entity_line_set_end",
    "dwg_entity_line_set_start",
    "dwg_entity_lwpolyline_set_vertex",
    "dwg_entity_mesh_set_vertex",
    "dwg_entity_mline_set_scale_factor",
    "dwg_entity_mline_set_start_point",
    "dwg_entity_mline_set_style_name",
    "dwg_entity_mtext_set_height",
    "dwg_entity_mtext_set_insertion_point",
    "dwg_entity_mtext_set_value",
    "dwg_entity_point_set_location",
    "dwg_entity_polyface_mesh_set_vertex",
    "dwg_entity_polyline2d_set_vertex",
    "dwg_entity_polyline3d_set_vertex",
    "dwg_entity_raster_image_set_insertion_point",
    "dwg_entity_raster_image_set_size",
    "dwg_entity_ray_set_base_point",
    "dwg_entity_ray_set_direction",
    "dwg_entity_shape_set_insertion_point",
    "dwg_entity_shape_set_rotation",
    "dwg_entity_shape_set_size",
    "dwg_entity_solid_set_corner",
    "dwg_entity_spline_set_control_point",
    "dwg_entity_spline_set_degree",
    "dwg_entity_text_set_height",
    "dwg_entity_text_set_insertion_point",
    "dwg_entity_text_set_value",
    "dwg_entity_tolerance_set_insertion_point",
    "dwg_entity_tolerance_set_text",
    "dwg_entity_underlay_set_insertion_point",
    "dwg_entity_viewport_set_center",
    "dwg_entity_viewport_set_height",
    "dwg_entity_viewport_set_width",
    "dwg_entity_wipeout_set_insertion_point",
    "dwg_entity_xline_set_base_point",
    "dwg_entity_xline_set_direction",
    "dwg_entity_mesh_set_face_vertex",
];

fn paired_setter(getter: &str) -> String {
    if let Some(rest) = getter.strip_prefix("dwg_entity_get_") {
        format!("dwg_entity_set_{rest}")
    } else {
        getter.replacen("_get_", "_set_", 1)
    }
}

#[test]
fn abi_manifest_pairs_every_writable_getter() {
    assert_eq!(GETTERS.len(), 84);
    assert_eq!(READ_ONLY.len(), 11);
    assert_eq!(SETTERS.len(), 74);

    for getter in GETTERS {
        if READ_ONLY.contains(&getter) {
            continue;
        }
        assert!(
            SETTERS.contains(&paired_setter(getter).as_str()),
            "missing setter pair for {getter}"
        );
    }
    assert!(SETTERS.contains(&"dwg_entity_mesh_set_face_vertex"));
    assert!(!SETTERS.contains(&"dwg_entity_dimension_set_subtype"));
    assert!(!SETTERS.contains(&"dwg_entity_dimension_set_measurement"));
}

fn ffi_dll() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("ACADRUST_DLL") {
        let path = std::path::PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/x86_64-pc-windows-gnu/release/acadrust.dll");
    assert!(
        path.exists(),
        "acadrust.dll not found at {}",
        path.display()
    );
    path
}

fn c_runtime_command(executable: &std::path::Path, dll: &std::path::Path) -> std::process::Command {
    let dll_dir = dll.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut search_path = vec![dll_dir.as_os_str().to_os_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        search_path.push(existing);
    }
    let mut command = std::process::Command::new(executable);
    command.env("PATH", std::env::join_paths(search_path).unwrap());
    command
}

#[test]
fn abi_manifest_symbols_export_from_cdylib() {
    let dll = ffi_dll();
    let cc = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let out_dir = std::env::temp_dir().join(format!(
        "acadrust_ffi_inventory_{}_{}",
        std::process::id(),
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();
    let source = out_dir.join("inventory.c");
    let executable = out_dir.join("inventory.exe");

    let mut names = GETTERS.to_vec();
    names.push("dwg_entity_mesh_get_face_vertex");
    names.extend(SETTERS);
    let mut c = String::from("#include <stddef.h>\n\n");
    for name in &names {
        c.push_str("extern void ");
        c.push_str(name);
        c.push_str("(void);\n");
    }
    c.push_str("typedef void (*symbol_fn)(void);\nstatic symbol_fn symbols[] = {\n");
    for name in &names {
        c.push_str("    ");
        c.push_str(name);
        c.push_str(",\n");
    }
    c.push_str("};\nint main(void) { return symbols[0] == NULL; }\n");
    std::fs::write(&source, c).unwrap();

    let compile = std::process::Command::new(cc)
        .arg(&source)
        .arg(&dll)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("failed to launch C compiler");
    assert!(
        compile.status.success(),
        "C ABI inventory link failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = c_runtime_command(&executable, &dll)
        .output()
        .expect("failed to run C ABI inventory");
    assert!(
        run.status.success(),
        "C ABI inventory exited with {:?}",
        run.status.code()
    );

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&executable);
    let _ = std::fs::remove_dir(&out_dir);
}
