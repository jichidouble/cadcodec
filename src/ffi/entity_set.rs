//! Entity setter entry points.
//!
//! Concrete setters are added here as the FFI mutation surface expands.

use crate::entities::{EntityType, LwVertex, PolyfaceVertex, Spline, Vertex2D, Vertex3DPolyline};
use crate::ffi::error::set_last_error;
use crate::ffi::object::{DwgObjectType, FfiDocument};
use crate::ffi::types::*;
use crate::ffi::util::{color_from_c, finite, finite3, nonzero3, read_c_string};
use crate::types::{Vector2, Vector3};

macro_rules! vec3_setter {
    ($name:ident, $kind:ident, $entity:ident, $field:ident) => {
        #[doc = concat!("Set the ", stringify!($field), " vector of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer and `obj` must be a valid entity object from that document."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            data: *mut DwgData,
            obj: *const DwgObject,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = Vector3::new(x, y, z);
                    Ok(())
                },
            )
        }
    };
}

macro_rules! f64_setter {
    ($name:ident, Ellipse, Ellipse, minor_axis_ratio, positive) => {
        f64_setter!(
            @impl $name,
            Ellipse,
            Ellipse,
            minor_axis_ratio,
            "minor axis ratio must be finite and within (0, 1]",
            valid_minor_axis_ratio
        );
    };
    ($name:ident, $kind:ident, $entity:ident, $field:ident, positive) => {
        f64_setter!(
            @impl $name,
            $kind,
            $entity,
            $field,
            "value must be finite and positive",
            valid_positive
        );
    };
    ($name:ident, $kind:ident, $entity:ident, $field:ident, finite_only) => {
        f64_setter!(
            @impl $name,
            $kind,
            $entity,
            $field,
            "value must be finite",
            finite
        );
    };
    (@impl $name:ident, $kind:ident, $entity:ident, $field:ident, $message:literal, $validator:ident) => {
        #[doc = concat!("Set the ", stringify!($field), " scalar of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer and `obj` must be a valid entity object from that document."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(data: *mut DwgData, obj: *const DwgObject, value: f64) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !$validator(value) {
                set_last_error(DWG_EINVAL, $message);
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = value;
                    Ok(())
                },
            )
        }
    };
}

macro_rules! string_setter {
    ($name:ident, $kind:ident, $entity:ident, $field:ident, allow_empty) => {
        #[doc = concat!("Set the ", stringify!($field), " of a ", stringify!($kind), " entity.")]
        #[doc = "\n# Safety\n\n`data` must be a valid document pointer, `obj` must be a valid entity object from that document, and `value` must point to a valid NUL-terminated string."]
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            data: *mut DwgData,
            obj: *const DwgObject,
            value: *const i8,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            let Ok(value) = read_c_string(value) else {
                set_last_error(DWG_EINVAL, "value is not valid UTF-8");
                return DWG_EINVAL;
            };
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.$field = value;
                    Ok(())
                },
            )
        }
    };
}

fn valid_positive(value: f64) -> bool {
    finite(value) && value > 0.0
}

fn valid_minor_axis_ratio(value: f64) -> bool {
    finite(value) && value > 0.0 && value <= 1.0
}

fn normalize_direction(x: f64, y: f64, z: f64) -> Vector3 {
    let scale = x.abs().max(y.abs()).max(z.abs());
    let x = x / scale;
    let y = y / scale;
    let z = z / scale;
    let length = (x * x + y * y + z * z).sqrt();
    Vector3::new(x / length, y / length, z / length)
}

fn valid_index(index: i32, len: usize) -> Option<usize> {
    let index = usize::try_from(index).ok()?;
    (index < len).then_some(index)
}

macro_rules! vec3_collection_setters {
    (
        $set_name:ident,
        $append_name:ident,
        $remove_name:ident,
        $clear_name:ident,
        $kind:ident,
        $entity:ident,
        $constructor:path,
        $field:ident
    ) => {
        #[doc = concat!("Set one ", stringify!($kind), " vertex.")]
        #[no_mangle]
        pub unsafe extern "C" fn $set_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            index: i32,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    let Some(index) = valid_index(index, entity.vertices.len()) else {
                        return Err((DWG_EINVAL, "vertex index is out of range"));
                    };
                    entity.vertices[index].$field = Vector3::new(x, y, z);
                    Ok(())
                },
            )
        }

        #[doc = concat!("Append one ", stringify!($kind), " vertex.")]
        #[no_mangle]
        pub unsafe extern "C" fn $append_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.vertices.push($constructor(Vector3::new(x, y, z)));
                    Ok(())
                },
            )
        }

        #[doc = concat!("Remove one ", stringify!($kind), " vertex.")]
        #[no_mangle]
        pub unsafe extern "C" fn $remove_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            index: i32,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    let Some(index) = valid_index(index, entity.vertices.len()) else {
                        return Err((DWG_EINVAL, "vertex index is out of range"));
                    };
                    entity.vertices.remove(index);
                    Ok(())
                },
            )
        }

        #[doc = concat!("Clear all ", stringify!($kind), " vertices.")]
        #[no_mangle]
        pub unsafe extern "C" fn $clear_name(data: *mut DwgData, obj: *const DwgObject) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                |entity| {
                    let EntityType::$entity(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.vertices.clear();
                    Ok(())
                },
            )
        }
    };
}

/// Set one LWPOLYLINE vertex location while preserving its segment metadata.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_set_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
    x: f64,
    y: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite(x) || !finite(y) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::LwPolyline),
        move |entity| {
            let EntityType::LwPolyline(polyline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, polyline.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            polyline.vertices[index].location = Vector2::new(x, y);
            Ok(())
        },
    )
}

/// Append one LWPOLYLINE vertex.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_append_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite(x) || !finite(y) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::LwPolyline),
        move |entity| {
            let EntityType::LwPolyline(polyline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            polyline.vertices.push(LwVertex::new(Vector2::new(x, y)));
            Ok(())
        },
    )
}

/// Remove one LWPOLYLINE vertex.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_remove_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::LwPolyline),
        move |entity| {
            let EntityType::LwPolyline(polyline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, polyline.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            polyline.vertices.remove(index);
            Ok(())
        },
    )
}

/// Clear all LWPOLYLINE vertices.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_lwpolyline_clear_vertices(
    data: *mut DwgData,
    obj: *const DwgObject,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::LwPolyline),
        |entity| {
            let EntityType::LwPolyline(polyline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            polyline.vertices.clear();
            Ok(())
        },
    )
}

vec3_collection_setters!(
    dwg_entity_polyline2d_set_vertex,
    dwg_entity_polyline2d_append_vertex,
    dwg_entity_polyline2d_remove_vertex,
    dwg_entity_polyline2d_clear_vertices,
    Polyline2D,
    Polyline2D,
    Vertex2D::new,
    location
);
vec3_collection_setters!(
    dwg_entity_polyline3d_set_vertex,
    dwg_entity_polyline3d_append_vertex,
    dwg_entity_polyline3d_remove_vertex,
    dwg_entity_polyline3d_clear_vertices,
    Polyline3D,
    Polyline3D,
    Vertex3DPolyline::new,
    position
);

fn polyface_face_uses_vertex(face: &crate::entities::PolyfaceFace, vertex: u16) -> bool {
    [face.index1, face.index2, face.index3, face.index4]
        .into_iter()
        .any(|index| index.unsigned_abs() == vertex)
}

fn remap_polyface_face_index(index: &mut i16, removed_vertex: u16) {
    let absolute = index.unsigned_abs();
    if absolute > removed_vertex {
        let invisible = *index < 0;
        let remapped = (absolute - 1) as i16;
        *index = if invisible { -remapped } else { remapped };
    }
}

/// Set one POLYFACE MESH vertex location while preserving its metadata.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_polyface_mesh_set_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::PolyfaceMesh),
        move |entity| {
            let EntityType::PolyfaceMesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, mesh.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            mesh.vertices[index].location = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

/// Append one POLYFACE MESH vertex.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_polyface_mesh_append_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::PolyfaceMesh),
        move |entity| {
            let EntityType::PolyfaceMesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if mesh.vertices.len() >= i16::MAX as usize {
                return Err((DWG_EINVAL, "polyface vertex count is too large"));
            }
            mesh.vertices
                .push(PolyfaceVertex::new(Vector3::new(x, y, z)));
            Ok(())
        },
    )
}

/// Remove one POLYFACE MESH vertex and remap subsequent face references.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_polyface_mesh_remove_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::PolyfaceMesh),
        move |entity| {
            let EntityType::PolyfaceMesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, mesh.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            let removed_vertex = u16::try_from(index + 1)
                .map_err(|_| (DWG_EINVAL, "polyface vertex index is too large"))?;
            if mesh
                .faces
                .iter()
                .any(|face| polyface_face_uses_vertex(face, removed_vertex))
            {
                return Err((
                    DWG_EINVAL,
                    "cannot remove a polyface vertex referenced by a face",
                ));
            }
            mesh.vertices.remove(index);
            for face in &mut mesh.faces {
                remap_polyface_face_index(&mut face.index1, removed_vertex);
                remap_polyface_face_index(&mut face.index2, removed_vertex);
                remap_polyface_face_index(&mut face.index3, removed_vertex);
                remap_polyface_face_index(&mut face.index4, removed_vertex);
            }
            Ok(())
        },
    )
}

/// Clear all POLYFACE MESH vertices when no face records reference them.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_polyface_mesh_clear_vertices(
    data: *mut DwgData,
    obj: *const DwgObject,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::PolyfaceMesh),
        |entity| {
            let EntityType::PolyfaceMesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !mesh.faces.is_empty() {
                return Err((
                    DWG_EINVAL,
                    "cannot clear polyface vertices while faces exist",
                ));
            }
            mesh.vertices.clear();
            Ok(())
        },
    )
}

macro_rules! vec3_entity_collection_setters {
    ($set_name:ident, $append_name:ident, $remove_name:ident, $clear_name:ident, $kind:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $set_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            index: i32,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$kind(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    let Some(index) = valid_index(index, entity.vertices.len()) else {
                        return Err((DWG_EINVAL, "vertex index is out of range"));
                    };
                    entity.vertices[index] = Vector3::new(x, y, z);
                    Ok(())
                },
            )
        }

        #[no_mangle]
        pub unsafe extern "C" fn $append_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            x: f64,
            y: f64,
            z: f64,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            if !finite3(x, y, z) {
                set_last_error(DWG_EINVAL, "coordinates must be finite");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$kind(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.vertices.push(Vector3::new(x, y, z));
                    Ok(())
                },
            )
        }

        #[no_mangle]
        pub unsafe extern "C" fn $remove_name(
            data: *mut DwgData,
            obj: *const DwgObject,
            index: i32,
        ) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                move |entity| {
                    let EntityType::$kind(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    let Some(index) = valid_index(index, entity.vertices.len()) else {
                        return Err((DWG_EINVAL, "vertex index is out of range"));
                    };
                    entity.vertices.remove(index);
                    Ok(())
                },
            )
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear_name(data: *mut DwgData, obj: *const DwgObject) -> i32 {
            if data.is_null() {
                set_last_error(DWG_EINVAL, "document pointer is null");
                return DWG_EINVAL;
            }
            (&mut *(data as *mut FfiDocument)).mutate_entity(
                obj,
                Some(DwgObjectType::$kind),
                |entity| {
                    let EntityType::$kind(entity) = entity else {
                        return Err((DWG_ETYPE, "entity type does not match setter"));
                    };
                    entity.vertices.clear();
                    Ok(())
                },
            )
        }
    };
}

vec3_entity_collection_setters!(
    dwg_entity_leader_set_vertex,
    dwg_entity_leader_append_vertex,
    dwg_entity_leader_remove_vertex,
    dwg_entity_leader_clear_vertices,
    Leader
);

/// Set one MESH vertex location.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_set_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, mesh.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            mesh.vertices[index] = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

/// Append one MESH vertex.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_append_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            mesh.vertices.push(Vector3::new(x, y, z));
            Ok(())
        },
    )
}

/// Remove an unreferenced MESH vertex and remap subsequent topology indices.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_remove_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let Some(index) = valid_index(index, mesh.vertices.len()) else {
                return Err((DWG_EINVAL, "vertex index is out of range"));
            };
            if mesh.faces.iter().any(|face| face.vertices.contains(&index))
                || mesh
                    .edges
                    .iter()
                    .any(|edge| edge.start == index || edge.end == index)
            {
                return Err((
                    DWG_EINVAL,
                    "cannot remove a mesh vertex referenced by topology",
                ));
            }
            mesh.vertices.remove(index);
            for face in &mut mesh.faces {
                for vertex in &mut face.vertices {
                    if *vertex > index {
                        *vertex -= 1;
                    }
                }
            }
            for edge in &mut mesh.edges {
                if edge.start > index {
                    edge.start -= 1;
                }
                if edge.end > index {
                    edge.end -= 1;
                }
            }
            Ok(())
        },
    )
}

/// Clear all MESH vertices and edges when no faces remain.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_clear_vertices(
    data: *mut DwgData,
    obj: *const DwgObject,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, Some(DwgObjectType::Mesh), |entity| {
        let EntityType::Mesh(mesh) = entity else {
            return Err((DWG_ETYPE, "entity type does not match setter"));
        };
        if !mesh.faces.is_empty() {
            return Err((DWG_EINVAL, "cannot clear mesh vertices while faces exist"));
        }
        mesh.vertices.clear();
        mesh.edges.clear();
        Ok(())
    })
}

unsafe fn read_mesh_face_indices(
    indices: *const u32,
    count: usize,
    vertex_count: usize,
) -> Result<Vec<usize>, (i32, &'static str)> {
    if indices.is_null() || count < 3 {
        return Err((
            DWG_EINVAL,
            "mesh faces require at least three vertex indices",
        ));
    }
    let mut vertices = Vec::with_capacity(count);
    for offset in 0..count {
        let value = *indices.add(offset) as usize;
        if value >= vertex_count {
            return Err((DWG_EINVAL, "mesh vertex index is out of range"));
        }
        vertices.push(value);
    }
    Ok(vertices)
}

/// Replace one MESH face vertex index and recompute edges.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_set_face_vertex(
    data: *mut DwgData,
    obj: *const DwgObject,
    face: i32,
    slot: i32,
    vertex: u32,
) -> i32 {
    if data.is_null() || face < 0 || slot < 0 {
        set_last_error(DWG_EINVAL, "mesh face or slot index is invalid");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let face = face as usize;
            let slot = slot as usize;
            let vertex = vertex as usize;
            if vertex >= mesh.vertices.len() {
                return Err((DWG_EINVAL, "mesh vertex index is out of range"));
            }
            let Some(face) = mesh.faces.get_mut(face) else {
                return Err((DWG_EINVAL, "mesh face index is out of range"));
            };
            let Some(slot) = face.vertices.get_mut(slot) else {
                return Err((DWG_EINVAL, "mesh face slot is out of range"));
            };
            *slot = vertex;
            mesh.compute_edges();
            Ok(())
        },
    )
}

/// Replace a MESH face with indices supplied by the caller.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_set_face(
    data: *mut DwgData,
    obj: *const DwgObject,
    face: i32,
    indices: *const u32,
    count: usize,
) -> i32 {
    if data.is_null() || face < 0 {
        set_last_error(DWG_EINVAL, "document pointer or mesh face index is invalid");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let vertices = read_mesh_face_indices(indices, count, mesh.vertices.len())?;
            let face = face as usize;
            let Some(target) = mesh.faces.get_mut(face) else {
                return Err((DWG_EINVAL, "mesh face index is out of range"));
            };
            target.vertices = vertices;
            mesh.compute_edges();
            Ok(())
        },
    )
}

/// Append a MESH face with indices supplied by the caller.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_append_face(
    data: *mut DwgData,
    obj: *const DwgObject,
    indices: *const u32,
    count: usize,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let vertices = read_mesh_face_indices(indices, count, mesh.vertices.len())?;
            mesh.faces.push(crate::entities::MeshFace::new(vertices));
            mesh.compute_edges();
            Ok(())
        },
    )
}

/// Remove one MESH face and recompute edges.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_remove_face(
    data: *mut DwgData,
    obj: *const DwgObject,
    face: i32,
) -> i32 {
    if data.is_null() || face < 0 {
        set_last_error(DWG_EINVAL, "document pointer or mesh face index is invalid");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Mesh),
        move |entity| {
            let EntityType::Mesh(mesh) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            let face = face as usize;
            if face >= mesh.faces.len() {
                return Err((DWG_EINVAL, "mesh face index is out of range"));
            }
            mesh.faces.remove(face);
            mesh.compute_edges();
            Ok(())
        },
    )
}

/// Clear all MESH faces and derived edges.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_mesh_clear_faces(
    data: *mut DwgData,
    obj: *const DwgObject,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, Some(DwgObjectType::Mesh), |entity| {
        let EntityType::Mesh(mesh) = entity else {
            return Err((DWG_ETYPE, "entity type does not match setter"));
        };
        mesh.faces.clear();
        mesh.edges.clear();
        Ok(())
    })
}

/// Set an entity's layer name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid entity object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_layer(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "layer is not valid UTF-8");
        return DWG_EINVAL;
    };
    if value.is_empty() {
        set_last_error(DWG_EINVAL, "layer must not be empty");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().layer = value;
        Ok(())
    })
}

/// Set an entity's color using the C ABI color encoding.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid entity object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_color(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().color = color_from_c(value);
        Ok(())
    })
}

/// Set an entity's linetype name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid entity object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_linetype(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "linetype is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().linetype = value;
        Ok(())
    })
}

/// Set an entity's positive, finite linetype scale.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid entity object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_set_linetype_scale(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite(value) || value <= 0.0 {
        set_last_error(DWG_EINVAL, "linetype scale must be finite and positive");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, None, move |entity| {
        entity.common_mut().linetype_scale = value;
        Ok(())
    })
}

vec3_setter!(dwg_entity_line_set_start, Line, Line, start);
vec3_setter!(dwg_entity_line_set_end, Line, Line, end);
vec3_setter!(dwg_entity_circle_set_center, Circle, Circle, center);
f64_setter!(
    dwg_entity_circle_set_radius,
    Circle,
    Circle,
    radius,
    positive
);
vec3_setter!(dwg_entity_arc_set_center, Arc, Arc, center);
f64_setter!(dwg_entity_arc_set_radius, Arc, Arc, radius, positive);
f64_setter!(
    dwg_entity_arc_set_start_angle,
    Arc,
    Arc,
    start_angle,
    finite_only
);
f64_setter!(
    dwg_entity_arc_set_end_angle,
    Arc,
    Arc,
    end_angle,
    finite_only
);
vec3_setter!(dwg_entity_point_set_location, Point, Point, location);
vec3_setter!(dwg_entity_ellipse_set_center, Ellipse, Ellipse, center);
vec3_setter!(
    dwg_entity_ellipse_set_major_axis,
    Ellipse,
    Ellipse,
    major_axis
);
f64_setter!(
    dwg_entity_ellipse_set_minor_axis_ratio,
    Ellipse,
    Ellipse,
    minor_axis_ratio,
    positive
);
string_setter!(dwg_entity_text_set_value, Text, Text, value, allow_empty);
vec3_setter!(
    dwg_entity_text_set_insertion_point,
    Text,
    Text,
    insertion_point
);
f64_setter!(dwg_entity_text_set_height, Text, Text, height, positive);
string_setter!(dwg_entity_mtext_set_value, MText, MText, value, allow_empty);
vec3_setter!(
    dwg_entity_mtext_set_insertion_point,
    MText,
    MText,
    insertion_point
);
f64_setter!(dwg_entity_mtext_set_height, MText, MText, height, positive);
vec3_setter!(dwg_entity_ray_set_base_point, Ray, Ray, base_point);
vec3_setter!(dwg_entity_xline_set_base_point, XLine, XLine, base_point);
string_setter!(
    dwg_entity_attrdef_set_tag,
    AttributeDefinition,
    AttributeDefinition,
    tag,
    allow_empty
);
string_setter!(
    dwg_entity_attrdef_set_prompt,
    AttributeDefinition,
    AttributeDefinition,
    prompt,
    allow_empty
);
string_setter!(
    dwg_entity_attrdef_set_default_value,
    AttributeDefinition,
    AttributeDefinition,
    default_value,
    allow_empty
);
vec3_setter!(
    dwg_entity_attrdef_set_insertion_point,
    AttributeDefinition,
    AttributeDefinition,
    insertion_point
);
f64_setter!(
    dwg_entity_attrdef_set_height,
    AttributeDefinition,
    AttributeDefinition,
    height,
    positive
);
string_setter!(
    dwg_entity_tolerance_set_text,
    Tolerance,
    Tolerance,
    text,
    allow_empty
);
vec3_setter!(
    dwg_entity_tolerance_set_insertion_point,
    Tolerance,
    Tolerance,
    insertion_point
);
string_setter!(
    dwg_entity_mline_set_style_name,
    MLine,
    MLine,
    style_name,
    allow_empty
);
vec3_setter!(dwg_entity_mline_set_start_point, MLine, MLine, start_point);
f64_setter!(
    dwg_entity_mline_set_scale_factor,
    MLine,
    MLine,
    scale_factor,
    positive
);
vec3_setter!(
    dwg_entity_shape_set_insertion_point,
    Shape,
    Shape,
    insertion_point
);
f64_setter!(dwg_entity_shape_set_size, Shape, Shape, size, positive);
f64_setter!(
    dwg_entity_shape_set_rotation,
    Shape,
    Shape,
    rotation,
    finite_only
);

/// Set an INSERT entity's block name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid INSERT object from that
/// document, and `value` must point to a valid non-empty NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_block_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    if value.is_empty() {
        set_last_error(DWG_EINVAL, "block name must not be empty");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.block_name = value;
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_insert_set_insert_point,
    Insert,
    Insert,
    insert_point
);
f64_setter!(
    dwg_entity_insert_set_rotation,
    Insert,
    Insert,
    rotation,
    finite_only
);

/// Set an INSERT entity's normalized normal vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_normal(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "normal must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.normal = normalize_direction(x, y, z);
            Ok(())
        },
    )
}

/// Set an INSERT entity's array dimensions and spacing.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid INSERT object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_insert_set_array(
    data: *mut DwgData,
    obj: *const DwgObject,
    columns: u16,
    rows: u16,
    column_spacing: f64,
    row_spacing: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if columns == 0 || rows == 0 {
        set_last_error(DWG_EINVAL, "array dimensions must be at least 1");
        return DWG_EINVAL;
    }
    if !finite(column_spacing) || !finite(row_spacing) {
        set_last_error(DWG_EINVAL, "array spacing must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Insert),
        move |entity| {
            let EntityType::Insert(insert) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            insert.column_count = columns;
            insert.row_count = rows;
            insert.column_spacing = column_spacing;
            insert.row_spacing = row_spacing;
            if columns > 1 || rows > 1 {
                insert.mark_as_minsert();
            }
            Ok(())
        },
    )
}

vec3_setter!(dwg_entity_viewport_set_center, Viewport, Viewport, center);
f64_setter!(
    dwg_entity_viewport_set_width,
    Viewport,
    Viewport,
    width,
    positive
);
f64_setter!(
    dwg_entity_viewport_set_height,
    Viewport,
    Viewport,
    height,
    positive
);

f64_setter!(
    dwg_entity_hatch_set_elevation,
    Hatch,
    Hatch,
    elevation,
    finite_only
);

/// Set a HATCH entity's pattern name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid HATCH object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_hatch_set_pattern_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Hatch),
        move |entity| {
            let EntityType::Hatch(hatch) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            hatch.pattern.name = value;
            Ok(())
        },
    )
}

f64_setter!(
    dwg_entity_hatch_set_pattern_angle,
    Hatch,
    Hatch,
    pattern_angle,
    finite_only
);
f64_setter!(
    dwg_entity_hatch_set_pattern_scale,
    Hatch,
    Hatch,
    pattern_scale,
    positive
);

/// Set a RAY entity's normalized direction vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid RAY object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_ray_set_direction(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "direction must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(obj, Some(DwgObjectType::Ray), move |entity| {
        let EntityType::Ray(ray) = entity else {
            return Err((DWG_ETYPE, "entity type does not match setter"));
        };
        ray.direction = normalize_direction(x, y, z);
        Ok(())
    })
}

/// Set an XLINE entity's normalized direction vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid XLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_xline_set_direction(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !nonzero3(x, y, z) {
        set_last_error(DWG_EINVAL, "direction must be finite and nonzero");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::XLine),
        move |entity| {
            let EntityType::XLine(xline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            xline.direction = normalize_direction(x, y, z);
            Ok(())
        },
    )
}

/// Set one of a SOLID entity's four corners (`corner` 1..=4).
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SOLID object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_solid_set_corner(
    data: *mut DwgData,
    obj: *const DwgObject,
    corner: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !matches!(corner, 1..=4) {
        set_last_error(DWG_EINVAL, "corner must be between 1 and 4");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    let value = Vector3::new(x, y, z);
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Solid),
        move |entity| {
            let EntityType::Solid(solid) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            match corner {
                1 => solid.first_corner = value,
                2 => solid.second_corner = value,
                3 => solid.third_corner = value,
                4 => solid.fourth_corner = value,
                _ => unreachable!(),
            }
            Ok(())
        },
    )
}

/// Set one of a 3DFACE entity's four corners (`corner` 1..=4).
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid 3DFACE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_face3d_set_corner(
    data: *mut DwgData,
    obj: *const DwgObject,
    corner: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !matches!(corner, 1..=4) {
        set_last_error(DWG_EINVAL, "corner must be between 1 and 4");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    let value = Vector3::new(x, y, z);
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Face3D),
        move |entity| {
            let EntityType::Face3D(face3d) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            match corner {
                1 => face3d.first_corner = value,
                2 => face3d.second_corner = value,
                3 => face3d.third_corner = value,
                4 => face3d.fourth_corner = value,
                _ => unreachable!(),
            }
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_helix_set_axis_base_point,
    Helix,
    Helix,
    axis_base_point
);
f64_setter!(dwg_entity_helix_set_radius, Helix, Helix, radius, positive);
f64_setter!(dwg_entity_helix_set_turns, Helix, Helix, turns, positive);
f64_setter!(
    dwg_entity_helix_set_turn_height,
    Helix,
    Helix,
    turn_height,
    positive
);

/// Set a DIMENSION entity's style name.
///
/// # Safety
///
/// `data` must be a valid document pointer, `obj` must be a valid DIMENSION object from that
/// document, and `value` must point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_style_name(
    data: *mut DwgData,
    obj: *const DwgObject,
    value: *const i8,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    let Ok(value) = read_c_string(value) else {
        set_last_error(DWG_EINVAL, "value is not valid UTF-8");
        return DWG_EINVAL;
    };
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().style_name = value;
            Ok(())
        },
    )
}

/// Set a DIMENSION entity's definition point.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_definition_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().definition_point = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

/// Set a DIMENSION entity's text middle point.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid DIMENSION object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_dimension_set_text_middle_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Dimension),
        move |entity| {
            let EntityType::Dimension(dimension) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            dimension.base_mut().text_middle_point = Vector3::new(x, y, z);
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_raster_image_set_insertion_point,
    RasterImage,
    RasterImage,
    insertion_point
);

/// Set a RASTERIMAGE entity's pixel width and height atomically.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid RASTERIMAGE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_raster_image_set_size(
    data: *mut DwgData,
    obj: *const DwgObject,
    width: f64,
    height: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !valid_positive(width) || !valid_positive(height) {
        set_last_error(DWG_EINVAL, "width and height must be finite and positive");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::RasterImage),
        move |entity| {
            let EntityType::RasterImage(raster) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            raster.size = Vector2::new(width, height);
            Ok(())
        },
    )
}

vec3_setter!(
    dwg_entity_underlay_set_insertion_point,
    Underlay,
    Underlay,
    insertion_point
);
vec3_setter!(dwg_entity_light_set_position, Light, Light, position);
vec3_setter!(
    dwg_entity_wipeout_set_insertion_point,
    Wipeout,
    Wipeout,
    insertion_point
);

/// Set a SPLINE entity's degree and regenerate its clamped knot vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_set_degree(
    data: *mut DwgData,
    obj: *const DwgObject,
    degree: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !(1..=25).contains(&degree) {
        set_last_error(DWG_EINVAL, "spline degree must be between 1 and 25");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Spline),
        move |entity| {
            let EntityType::Spline(spline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !spline.control_points.is_empty() && degree as usize >= spline.control_points.len() {
                return Err((
                    DWG_EINVAL,
                    "spline degree must be less than the control point count",
                ));
            }
            spline.degree = degree;
            spline.knots =
                Spline::generate_clamped_knots(degree as usize, spline.control_points.len());
            Ok(())
        },
    )
}

/// Replace one zero-based SPLINE control point and regenerate its clamped knot vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_set_control_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Spline),
        move |entity| {
            let EntityType::Spline(spline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !spline.fit_points.is_empty() {
                return Err((
                    DWG_EINVAL,
                    "spline control points cannot be changed when fit points are present",
                ));
            }
            let Some(index) = valid_index(index, spline.control_points.len()) else {
                return Err((DWG_EINVAL, "control point index is out of range"));
            };
            spline.control_points[index] = Vector3::new(x, y, z);
            spline.knots =
                Spline::generate_clamped_knots(spline.degree as usize, spline.control_points.len());
            Ok(())
        },
    )
}

/// Append a SPLINE control point and regenerate its clamped knot vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_append_control_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    x: f64,
    y: f64,
    z: f64,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    if !finite3(x, y, z) {
        set_last_error(DWG_EINVAL, "coordinates must be finite");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Spline),
        move |entity| {
            let EntityType::Spline(spline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !spline.fit_points.is_empty() {
                return Err((
                    DWG_EINVAL,
                    "spline control points cannot be changed when fit points are present",
                ));
            }
            let Some(new_len) = spline.control_points.len().checked_add(1) else {
                return Err((DWG_EINVAL, "control point count is too large"));
            };
            if !(1..=25).contains(&spline.degree) || new_len <= spline.degree as usize {
                return Err((
                    DWG_EINVAL,
                    "control point count must be greater than the spline degree",
                ));
            }
            spline.control_points.push(Vector3::new(x, y, z));
            if !spline.weights.is_empty() {
                spline.weights.push(1.0);
            }
            spline.knots = Spline::generate_clamped_knots(spline.degree as usize, new_len);
            Ok(())
        },
    )
}

/// Remove one zero-based SPLINE control point and regenerate its clamped knot vector.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_remove_control_point(
    data: *mut DwgData,
    obj: *const DwgObject,
    index: i32,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Spline),
        move |entity| {
            let EntityType::Spline(spline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !spline.fit_points.is_empty() {
                return Err((
                    DWG_EINVAL,
                    "spline control points cannot be changed when fit points are present",
                ));
            }
            let Some(index) = valid_index(index, spline.control_points.len()) else {
                return Err((DWG_EINVAL, "control point index is out of range"));
            };
            let new_len = spline.control_points.len() - 1;
            if new_len != 0
                && (!(1..=25).contains(&spline.degree) || new_len <= spline.degree as usize)
            {
                return Err((
                    DWG_EINVAL,
                    "control point count must be greater than the spline degree",
                ));
            }
            if !spline.weights.is_empty() && index >= spline.weights.len() {
                return Err((
                    DWG_EINVAL,
                    "spline weights do not match the control point collection",
                ));
            }
            spline.control_points.remove(index);
            if !spline.weights.is_empty() {
                spline.weights.remove(index);
            }
            spline.knots = Spline::generate_clamped_knots(spline.degree as usize, new_len);
            Ok(())
        },
    )
}

/// Clear a SPLINE entity's control points, knots, and weights.
///
/// # Safety
///
/// `data` must be a valid document pointer and `obj` must be a valid SPLINE object from that
/// document.
#[no_mangle]
pub unsafe extern "C" fn dwg_entity_spline_clear_control_points(
    data: *mut DwgData,
    obj: *const DwgObject,
) -> i32 {
    if data.is_null() {
        set_last_error(DWG_EINVAL, "document pointer is null");
        return DWG_EINVAL;
    }
    (&mut *(data as *mut FfiDocument)).mutate_entity(
        obj,
        Some(DwgObjectType::Spline),
        move |entity| {
            let EntityType::Spline(spline) = entity else {
                return Err((DWG_ETYPE, "entity type does not match setter"));
            };
            if !spline.fit_points.is_empty() {
                return Err((
                    DWG_EINVAL,
                    "spline control points cannot be changed when fit points are present",
                ));
            }
            spline.control_points.clear();
            spline.knots.clear();
            spline.weights.clear();
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{
        Leader, Mesh, MeshEdge, MeshFace, PolyfaceFace, PolyfaceMesh, PolyfaceVertex, Spline,
    };
    use crate::ffi::entity::dwg_entity_mesh_get_face_vertex;
    use crate::types::Handle;
    use crate::{CadDocument, EntityType};

    fn spline_fixture(spline: Spline) -> (FfiDocument, *const DwgObject, Handle) {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Spline(spline)).unwrap();
        let ffi = FfiDocument::new(doc);
        let (obj, handle) = {
            let object = ffi
                .objects
                .iter()
                .find(|object| object.kind == DwgObjectType::Spline)
                .unwrap();
            (
                object.as_ref() as *const _ as *const DwgObject,
                object.handle,
            )
        };
        (ffi, obj, handle)
    }

    fn spline_entity(ffi: &FfiDocument, handle: Handle) -> &Spline {
        let EntityType::Spline(spline) = ffi.doc.get_entity(handle).unwrap() else {
            panic!("fixture entity is not a spline");
        };
        spline
    }

    fn polyface_fixture(mesh: PolyfaceMesh) -> (FfiDocument, *const DwgObject, Handle) {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::PolyfaceMesh(mesh)).unwrap();
        let ffi = FfiDocument::new(doc);
        let (obj, handle) = {
            let object = ffi
                .objects
                .iter()
                .find(|object| object.kind == DwgObjectType::PolyfaceMesh)
                .unwrap();
            (
                object.as_ref() as *const _ as *const DwgObject,
                object.handle,
            )
        };
        (ffi, obj, handle)
    }

    fn polyface_entity(ffi: &FfiDocument, handle: Handle) -> &PolyfaceMesh {
        let EntityType::PolyfaceMesh(mesh) = ffi.doc.get_entity(handle).unwrap() else {
            panic!("fixture entity is not a polyface mesh");
        };
        mesh
    }

    fn leader_fixture(leader: Leader) -> (FfiDocument, *const DwgObject, Handle) {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Leader(leader)).unwrap();
        let ffi = FfiDocument::new(doc);
        let (obj, handle) = {
            let object = ffi
                .objects
                .iter()
                .find(|object| object.kind == DwgObjectType::Leader)
                .unwrap();
            (
                object.as_ref() as *const _ as *const DwgObject,
                object.handle,
            )
        };
        (ffi, obj, handle)
    }

    fn mesh_fixture(mesh: Mesh) -> (FfiDocument, *const DwgObject, Handle) {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Mesh(mesh)).unwrap();
        let ffi = FfiDocument::new(doc);
        let (obj, handle) = {
            let object = ffi
                .objects
                .iter()
                .find(|object| object.kind == DwgObjectType::Mesh)
                .unwrap();
            (
                object.as_ref() as *const _ as *const DwgObject,
                object.handle,
            )
        };
        (ffi, obj, handle)
    }

    fn mesh_entity(ffi: &FfiDocument, handle: Handle) -> &Mesh {
        let EntityType::Mesh(mesh) = ffi.doc.get_entity(handle).unwrap() else {
            panic!("fixture entity is not a mesh");
        };
        mesh
    }

    #[test]
    fn phase3c_spline_collection_setters_keep_internal_collections_synchronized() {
        let original_points = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(3.0, 1.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
        ];
        let mut spline = Spline::from_control_points(3, original_points.clone());
        spline.knots = vec![42.0];
        spline.weights = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        spline.knot_tolerance = 0.125;
        let (mut ffi, obj, handle) = spline_fixture(spline);
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();

        assert_eq!(
            unsafe { dwg_entity_spline_set_control_point(data, obj, 1, 11.0, 12.0, 13.0) },
            DWG_OK
        );
        let spline = spline_entity(&ffi, handle);
        assert_eq!(spline.control_points[0], original_points[0]);
        assert_eq!(spline.control_points[1], Vector3::new(11.0, 12.0, 13.0));
        assert_eq!(&spline.control_points[2..], &original_points[2..]);
        assert_eq!(spline.weights, vec![2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(
            spline.knots,
            Spline::generate_clamped_knots(3, original_points.len())
        );
        assert_eq!(spline.knot_tolerance, 0.125);

        assert_eq!(
            unsafe { dwg_entity_spline_append_control_point(data, obj, 21.0, 22.0, 23.0) },
            DWG_OK
        );
        let spline = spline_entity(&ffi, handle);
        assert_eq!(
            spline.control_points.last(),
            Some(&Vector3::new(21.0, 22.0, 23.0))
        );
        assert_eq!(spline.weights, vec![2.0, 3.0, 4.0, 5.0, 6.0, 1.0]);
        assert_eq!(spline.knots, Spline::generate_clamped_knots(3, 6));

        assert_eq!(
            unsafe { dwg_entity_spline_remove_control_point(data, obj, 1) },
            DWG_OK
        );
        let spline = spline_entity(&ffi, handle);
        assert_eq!(
            spline.control_points,
            vec![
                original_points[0],
                original_points[2],
                original_points[3],
                original_points[4],
                Vector3::new(21.0, 22.0, 23.0),
            ]
        );
        assert_eq!(spline.weights, vec![2.0, 4.0, 5.0, 6.0, 1.0]);
        assert_eq!(spline.knots, Spline::generate_clamped_knots(3, 5));

        assert_eq!(
            unsafe { dwg_entity_spline_set_degree(data, obj, 2) },
            DWG_OK
        );
        let spline = spline_entity(&ffi, handle);
        assert_eq!(spline.degree, 2);
        assert_eq!(spline.knots, Spline::generate_clamped_knots(2, 5));
        assert_eq!(spline.weights, vec![2.0, 4.0, 5.0, 6.0, 1.0]);

        assert_eq!(
            unsafe { dwg_entity_spline_clear_control_points(data, obj) },
            DWG_OK
        );
        let spline = spline_entity(&ffi, handle);
        assert!(spline.control_points.is_empty());
        assert!(spline.knots.is_empty());
        assert!(spline.weights.is_empty());
        assert_eq!(spline.degree, 2);
        assert_eq!(spline.knot_tolerance, 0.125);
    }

    #[test]
    fn phase3c_spline_collection_setters_preserve_fit_point_spline_state() {
        let mut spline = Spline::from_fit_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 2.0, 0.0),
            Vector3::new(2.0, 1.0, 0.0),
            Vector3::new(3.0, 3.0, 0.0),
        ]);
        spline.control_points = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(3.0, 1.0, 0.0),
        ];
        spline.knots = Spline::generate_clamped_knots(3, 4);
        spline.weights = vec![1.0, 1.5, 2.0, 2.5];
        let (mut ffi, obj, handle) = spline_fixture(spline);
        let expected = spline_entity(&ffi, handle).clone();
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();

        assert_eq!(
            unsafe { dwg_entity_spline_set_control_point(data, obj, 0, 9.0, 8.0, 7.0) },
            DWG_EINVAL
        );
        assert_eq!(spline_entity(&ffi, handle), &expected);
        assert_eq!(
            unsafe { dwg_entity_spline_append_control_point(data, obj, 9.0, 8.0, 7.0) },
            DWG_EINVAL
        );
        assert_eq!(spline_entity(&ffi, handle), &expected);
        assert_eq!(
            unsafe { dwg_entity_spline_remove_control_point(data, obj, 0) },
            DWG_EINVAL
        );
        assert_eq!(spline_entity(&ffi, handle), &expected);
        assert_eq!(
            unsafe { dwg_entity_spline_clear_control_points(data, obj) },
            DWG_EINVAL
        );
        assert_eq!(spline_entity(&ffi, handle), &expected);
    }

    #[test]
    fn phase3c_polyface_collection_setters_preserve_and_remap_face_references() {
        let mut mesh = PolyfaceMesh::new();
        for vertex in [
            PolyfaceVertex::from_xyz(0.0, 0.0, 0.0),
            PolyfaceVertex::from_xyz(1.0, 0.0, 0.0),
            PolyfaceVertex::from_xyz(2.0, 0.0, 0.0),
            PolyfaceVertex::from_xyz(3.0, 0.0, 0.0),
        ] {
            mesh.add_vertex(vertex);
        }
        mesh.add_face(PolyfaceFace::triangle_with_visibility(
            1, 3, 4, true, false, false,
        ));
        mesh.add_face(PolyfaceFace::triangle(-3, 4, 1));
        let (mut ffi, obj, handle) = polyface_fixture(mesh);
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();
        let original = polyface_entity(&ffi, handle).clone();

        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_remove_vertex(data, obj, 0) },
            DWG_EINVAL
        );
        assert_eq!(polyface_entity(&ffi, handle), &original);
        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_clear_vertices(data, obj) },
            DWG_EINVAL
        );
        assert_eq!(polyface_entity(&ffi, handle), &original);

        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_set_vertex(data, obj, 0, 10.0, 11.0, 12.0) },
            DWG_OK
        );
        assert_eq!(
            polyface_entity(&ffi, handle).vertices[0].location,
            Vector3::new(10.0, 11.0, 12.0)
        );
        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_append_vertex(data, obj, 4.0, 0.0, 0.0) },
            DWG_OK
        );
        assert_eq!(polyface_entity(&ffi, handle).vertices.len(), 5);

        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_remove_vertex(data, obj, 1) },
            DWG_OK
        );
        let mesh = polyface_entity(&ffi, handle);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.faces[0].index1, -1);
        assert_eq!(mesh.faces[0].index2, 2);
        assert_eq!(mesh.faces[0].index3, 3);
        assert_eq!(mesh.faces[0].index4, 0);
        assert_eq!(mesh.faces[1].index1, -2);
        assert_eq!(mesh.faces[1].index2, 3);
        assert_eq!(mesh.faces[1].index3, 1);
        assert_eq!(mesh.faces[1].index4, 0);

        let EntityType::PolyfaceMesh(mesh) = ffi.doc.get_entity_mut(handle).unwrap() else {
            panic!("fixture entity is not a polyface mesh");
        };
        mesh.faces.clear();
        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_clear_vertices(data, obj) },
            DWG_OK
        );
        assert!(polyface_entity(&ffi, handle).vertices.is_empty());
    }

    #[test]
    fn phase3c_polyface_append_rejects_i16_vertex_limit() {
        let mut mesh = PolyfaceMesh::new();
        mesh.vertices
            .resize(i16::MAX as usize, PolyfaceVertex::default());
        let (mut ffi, obj, handle) = polyface_fixture(mesh);
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();

        assert_eq!(
            unsafe { dwg_entity_polyface_mesh_append_vertex(data, obj, 1.0, 2.0, 3.0) },
            DWG_EINVAL
        );
        assert_eq!(
            polyface_entity(&ffi, handle).vertices.len(),
            i16::MAX as usize
        );
    }

    #[test]
    fn phase3c_leader_and_mesh_vertex_setters_preserve_topology() {
        let leader = Leader::from_vertices(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        ]);
        let (mut leader_ffi, leader_obj, leader_handle) = leader_fixture(leader);
        let leader_data = (&mut leader_ffi as *mut FfiDocument).cast::<DwgData>();
        assert_eq!(
            unsafe { dwg_entity_leader_set_vertex(leader_data, leader_obj, 0, 10.0, 11.0, 12.0) },
            DWG_OK
        );
        assert_eq!(
            unsafe { dwg_entity_leader_append_vertex(leader_data, leader_obj, 2.0, 3.0, 4.0) },
            DWG_OK
        );
        assert_eq!(
            unsafe { dwg_entity_leader_remove_vertex(leader_data, leader_obj, 2) },
            DWG_OK
        );
        assert_eq!(
            unsafe { dwg_entity_leader_set_vertex(leader_data, leader_obj, -1, 1.0, 2.0, 3.0) },
            DWG_EINVAL
        );
        {
            let EntityType::Leader(leader) = leader_ffi.doc.get_entity(leader_handle).unwrap()
            else {
                panic!("fixture entity is not a leader");
            };
            assert_eq!(leader.vertices[0], Vector3::new(10.0, 11.0, 12.0));
            assert_eq!(leader.vertices.len(), 2);
        }
        assert_eq!(
            unsafe { dwg_entity_leader_clear_vertices(leader_data, leader_obj) },
            DWG_OK
        );
        let EntityType::Leader(leader) = leader_ffi.doc.get_entity(leader_handle).unwrap() else {
            panic!("fixture entity is not a leader");
        };
        assert!(leader.vertices.is_empty());

        let mut mesh = Mesh::new();
        mesh.vertices = (0..5).map(|x| Vector3::new(x as f64, 0.0, 0.0)).collect();
        mesh.faces.push(MeshFace::triangle(0, 2, 3));
        mesh.edges.push(MeshEdge::new(2, 3));
        mesh.edges.push(MeshEdge::new(3, 4));
        let (mut mesh_ffi, mesh_obj, mesh_handle) = mesh_fixture(mesh);
        let mesh_data = (&mut mesh_ffi as *mut FfiDocument).cast::<DwgData>();
        assert_eq!(
            unsafe { dwg_entity_mesh_set_vertex(mesh_data, mesh_obj, 0, 20.0, 21.0, 22.0) },
            DWG_OK
        );
        assert_eq!(
            unsafe { dwg_entity_mesh_append_vertex(mesh_data, mesh_obj, 5.0, 0.0, 0.0) },
            DWG_OK
        );
        let before = mesh_entity(&mesh_ffi, mesh_handle).clone();
        assert_eq!(
            unsafe { dwg_entity_mesh_remove_vertex(mesh_data, mesh_obj, 0) },
            DWG_EINVAL
        );
        assert_eq!(mesh_entity(&mesh_ffi, mesh_handle), &before);
        assert_eq!(
            unsafe { dwg_entity_mesh_remove_vertex(mesh_data, mesh_obj, 4) },
            DWG_EINVAL
        );
        assert_eq!(mesh_entity(&mesh_ffi, mesh_handle), &before);
        assert_eq!(
            unsafe { dwg_entity_mesh_remove_vertex(mesh_data, mesh_obj, 1) },
            DWG_OK
        );
        let mesh = mesh_entity(&mesh_ffi, mesh_handle);
        assert_eq!(mesh.faces[0].vertices, vec![0, 1, 2]);
        assert_eq!(mesh.edges[0].start, 1);
        assert_eq!(mesh.edges[0].end, 2);
        assert_eq!(mesh.edges[1].start, 2);
        assert_eq!(mesh.edges[1].end, 3);
        assert_eq!(
            unsafe { dwg_entity_mesh_clear_vertices(mesh_data, mesh_obj) },
            DWG_EINVAL
        );
        let EntityType::Mesh(mesh) = mesh_ffi.doc.get_entity_mut(mesh_handle).unwrap() else {
            panic!("fixture entity is not a mesh");
        };
        mesh.faces.clear();
        assert_eq!(
            unsafe { dwg_entity_mesh_clear_vertices(mesh_data, mesh_obj) },
            DWG_OK
        );
        assert!(mesh_entity(&mesh_ffi, mesh_handle).vertices.is_empty());
        assert!(mesh_entity(&mesh_ffi, mesh_handle).edges.is_empty());
    }

    #[test]
    fn phase3c_mesh_face_setters_recompute_edges() {
        let mut mesh = Mesh::new();
        mesh.vertices = (0..4).map(|x| Vector3::new(x as f64, 0.0, 0.0)).collect();
        mesh.add_face(MeshFace::triangle(0, 1, 2));
        mesh.compute_edges();
        let (mut ffi, obj, handle) = mesh_fixture(mesh);
        let data = (&mut ffi as *mut FfiDocument).cast::<DwgData>();

        let mut vertex = u32::MAX;
        assert_eq!(
            unsafe { dwg_entity_mesh_get_face_vertex(obj, 0, 1, &mut vertex) },
            DWG_OK
        );
        assert_eq!(vertex, 1);
        assert_eq!(
            unsafe { dwg_entity_mesh_set_face_vertex(data, obj, 0, 1, 3) },
            DWG_OK
        );
        assert_eq!(mesh_entity(&ffi, handle).faces[0].vertices, vec![0, 3, 2]);

        let replacement = [0_u32, 2, 3];
        assert_eq!(
            unsafe {
                dwg_entity_mesh_set_face(data, obj, 0, replacement.as_ptr(), replacement.len())
            },
            DWG_OK
        );
        let appended = [1_u32, 2, 3];
        assert_eq!(
            unsafe { dwg_entity_mesh_append_face(data, obj, appended.as_ptr(), appended.len()) },
            DWG_OK
        );
        assert_eq!(mesh_entity(&ffi, handle).faces.len(), 2);
        let mut actual = mesh_entity(&ffi, handle).edges.clone();
        actual.sort_by_key(|edge| (edge.start, edge.end));
        let expected = {
            let mut expected = mesh_entity(&ffi, handle).clone();
            expected.compute_edges();
            expected.edges.sort_by_key(|edge| (edge.start, edge.end));
            expected.edges
        };
        assert_eq!(actual, expected);

        assert_eq!(unsafe { dwg_entity_mesh_remove_face(data, obj, 0) }, DWG_OK);
        assert_eq!(mesh_entity(&ffi, handle).faces.len(), 1);
        assert_eq!(unsafe { dwg_entity_mesh_clear_faces(data, obj) }, DWG_OK);
        assert!(mesh_entity(&ffi, handle).faces.is_empty());
        assert!(mesh_entity(&ffi, handle).edges.is_empty());
    }
}
