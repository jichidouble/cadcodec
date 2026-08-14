//! Object arena over `CadDocument`: stable `DwgObject*` references.
//!
//! The Rust core stores entities as `Vec<Arc<EntityType>>` and objects in a
//! `HashMap<Handle, ObjectType>`; neither gives stable pointers across
//! mutation. The FFI layer therefore owns a parallel arena of `FfiObject`
//! boxes whose heap addresses are stable for the document's lifetime.

use std::ptr;

use crate::document::CadDocument;
use crate::entities::{Entity, EntityType};
use crate::objects::ObjectType;
use crate::types::Handle;

/// Object type as seen by C consumers. Extended in later phases.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DwgObjectType {
    Unknown = 0,
    Line = 1,
    Circle = 2,
    Arc = 3,
    Text = 4,
    Point = 5,
    Ellipse = 6,
    MText = 7,
    Spline = 8,
    LwPolyline = 9,
    Ray = 10,
    XLine = 11,
    Solid = 12,
    Face3D = 13,
    Insert = 14,
    Polyline2D = 15,
    Polyline3D = 16,
    Mesh = 17,
    Viewport = 18,
    PolyfaceMesh = 19,
    Other = 1000,
}

/// One stable object reference in the arena.
pub struct FfiObject {
    pub kind: DwgObjectType,
    pub handle: Handle,
    pub entity: *const EntityType,
    pub object: *const ObjectType,
    pub next: *const FfiObject,
}

/// FFI wrapper around a document plus its object arena.
pub struct FfiDocument {
    pub doc: CadDocument,
    pub objects: Vec<Box<FfiObject>>,
}

fn entity_kind(e: &EntityType) -> DwgObjectType {
    match e {
        EntityType::Line(_) => DwgObjectType::Line,
        EntityType::Circle(_) => DwgObjectType::Circle,
        EntityType::Arc(_) => DwgObjectType::Arc,
        EntityType::Text(_) => DwgObjectType::Text,
        EntityType::Point(_) => DwgObjectType::Point,
        EntityType::Ellipse(_) => DwgObjectType::Ellipse,
        EntityType::MText(_) => DwgObjectType::MText,
        EntityType::Spline(_) => DwgObjectType::Spline,
        EntityType::LwPolyline(_) => DwgObjectType::LwPolyline,
        EntityType::Ray(_) => DwgObjectType::Ray,
        EntityType::XLine(_) => DwgObjectType::XLine,
        EntityType::Solid(_) => DwgObjectType::Solid,
        EntityType::Face3D(_) => DwgObjectType::Face3D,
        EntityType::Insert(_) => DwgObjectType::Insert,
        EntityType::Polyline2D(_) => DwgObjectType::Polyline2D,
        EntityType::Polyline3D(_) => DwgObjectType::Polyline3D,
        EntityType::Mesh(_) => DwgObjectType::Mesh,
        EntityType::Viewport(_) => DwgObjectType::Viewport,
        EntityType::PolyfaceMesh(_) => DwgObjectType::PolyfaceMesh,
        _ => DwgObjectType::Other,
    }
}

impl FfiDocument {
    /// Build the arena for `doc`. Entities first (document order, structural
    /// BLOCK/ENDBLK markers excluded by `entities()`), then non-entity
    /// objects sorted by handle.
    pub fn new(doc: CadDocument) -> Self {
        let mut objects: Vec<Box<FfiObject>> = Vec::new();
        for e in doc.entities() {
            objects.push(Box::new(FfiObject {
                kind: entity_kind(e),
                handle: e.common().handle,
                entity: e as *const EntityType,
                object: ptr::null(),
                next: ptr::null(),
            }));
        }
        let mut handles: Vec<Handle> = doc.objects.keys().copied().collect();
        handles.sort();
        for h in handles {
            objects.push(Box::new(FfiObject {
                kind: DwgObjectType::Other,
                handle: h,
                entity: ptr::null(),
                object: &doc.objects[&h] as *const ObjectType,
                next: ptr::null(),
            }));
        }
        // Link next pointers after all Boxes exist (heap addresses stable).
        for i in 0..objects.len() {
            let next = if i + 1 < objects.len() {
                objects[i + 1].as_ref() as *const FfiObject
            } else {
                ptr::null()
            };
            objects[i].next = next;
        }
        FfiDocument { doc, objects }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Circle, Line, Text};
    use crate::types::Vector3;
    use crate::{CadDocument, EntityType};

    fn sample_doc() -> CadDocument {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 2.0, 3.0))).unwrap();
        doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::new(5.0, 6.0, 0.0), 2.5))).unwrap();
        let mut t = Text::new();
        t.value = "hello".to_string();
        t.height = 1.0;
        doc.add_entity(EntityType::Text(t)).unwrap();
        doc
    }

    #[test]
    fn arena_builds_in_entity_order() {
        let ffi = FfiDocument::new(sample_doc());
        assert!(ffi.objects.len() > 3, "arena should contain entities plus default objects");
        assert_eq!(ffi.objects[0].kind, DwgObjectType::Line);
        assert_eq!(ffi.objects[1].kind, DwgObjectType::Circle);
        assert_eq!(ffi.objects[2].kind, DwgObjectType::Text);
        assert!(!ffi.objects[0].handle.is_null());
        assert!(!ffi.objects[0].next.is_null());
        assert!(ffi.objects.last().unwrap().next.is_null());
    }

    #[test]
    fn next_chain_walks_all_objects() {
        let ffi = FfiDocument::new(sample_doc());
        let mut cursor: *const FfiObject = ffi.objects[0].as_ref();
        let mut count = 0usize;
        while !cursor.is_null() {
            count += 1;
            cursor = unsafe { (*cursor).next };
        }
        assert!(count >= 3, "walk should cover at least the entities");
        assert_eq!(count, ffi.objects.len());
    }
}
