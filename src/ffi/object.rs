//! Object arena over `CadDocument`: stable `DwgObject*` references.
//!
//! The Rust core stores entities as `Vec<Arc<EntityType>>` and objects in a
//! `HashMap<Handle, ObjectType>`; neither gives stable pointers across
//! mutation. The FFI layer therefore owns a parallel arena of `FfiObject`
//! boxes whose heap addresses are stable for the document's lifetime.

use std::ptr;

use crate::document::CadDocument;
use crate::entities::EntityType;
use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::notification::DwgNotificationCallback;
use crate::ffi::types::{DwgObject, DWG_EINVAL, DWG_ENOMEM, DWG_ENOTFOUND, DWG_ETYPE, DWG_OK};
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
    Hatch = 20,
    Tolerance = 21,
    Leader = 22,
    MLine = 23,
    AttributeDefinition = 24,
    Shape = 25,
    Helix = 26,
    Dimension = 27,
    RasterImage = 28,
    Underlay = 29,
    Light = 30,
    Wipeout = 31,
    Solid3D = 32,
    Region = 33,
    Body = 34,
    Surface = 35,
    Other = 1000,
}

/// One stable object reference in the arena.
pub struct FfiObject {
    pub kind: DwgObjectType,
    pub handle: Handle,
    pub entity: *const EntityType,
    pub object: *const ObjectType,
    pub next: *const FfiObject,
    /// A removed record stays allocated until its document is destroyed so
    /// callers holding an old pointer never observe freed memory.
    pub alive: bool,
}

/// FFI wrapper around a document plus its object arena.
pub struct FfiDocument {
    pub doc: CadDocument,
    pub objects: Vec<Box<FfiObject>>,
    pub notification_callback: Option<DwgNotificationCallback>,
    pub notification_user_data: *mut std::ffi::c_void,
}

#[allow(dead_code)]
pub(crate) type MutationError = (i32, &'static str);

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
        EntityType::Hatch(_) => DwgObjectType::Hatch,
        EntityType::Tolerance(_) => DwgObjectType::Tolerance,
        EntityType::Leader(_) => DwgObjectType::Leader,
        EntityType::MLine(_) => DwgObjectType::MLine,
        EntityType::AttributeDefinition(_) => DwgObjectType::AttributeDefinition,
        EntityType::Shape(_) => DwgObjectType::Shape,
        EntityType::Helix(_) => DwgObjectType::Helix,
        EntityType::Dimension(_) => DwgObjectType::Dimension,
        EntityType::RasterImage(_) => DwgObjectType::RasterImage,
        EntityType::Underlay(_) => DwgObjectType::Underlay,
        EntityType::Light(_) => DwgObjectType::Light,
        EntityType::Wipeout(_) => DwgObjectType::Wipeout,
        EntityType::Solid3D(_) => DwgObjectType::Solid3D,
        EntityType::Region(_) => DwgObjectType::Region,
        EntityType::Body(_) => DwgObjectType::Body,
        EntityType::Surface(_) => DwgObjectType::Surface,
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
                alive: true,
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
                alive: true,
            }));
        }
        let mut ffi = FfiDocument {
            doc,
            objects,
            notification_callback: None,
            notification_user_data: std::ptr::null_mut(),
        };
        ffi.relink_objects();
        ffi
    }

    /// Rebuild the active-object chain without moving any boxed arena record.
    pub(crate) fn relink_objects(&mut self) {
        let mut next = ptr::null();
        for object in self.objects.iter_mut().rev() {
            object.next = next;
            if object.alive {
                next = object.as_ref() as *const FfiObject;
            }
        }
    }

    /// Add an entity to model space and append its stable FFI arena record.
    pub(crate) fn add_entity(&mut self, entity: EntityType) -> Result<*mut DwgObject, i32> {
        let handle = self.doc.add_entity(entity).map_err(|_| DWG_ENOMEM)?;
        let entity = self.doc.get_entity(handle).ok_or(DWG_ENOTFOUND)?;
        let mut object = Box::new(FfiObject {
            kind: entity_kind(entity),
            handle,
            entity: entity as *const EntityType,
            object: std::ptr::null(),
            next: std::ptr::null(),
            alive: true,
        });
        let object_ptr = object.as_mut() as *mut FfiObject as *mut DwgObject;
        self.objects.push(object);
        self.relink_objects();
        Ok(object_ptr)
    }

    pub(crate) fn object_index(&self, obj: *const DwgObject) -> Result<usize, i32> {
        if obj.is_null() {
            return Err(DWG_EINVAL);
        }
        self.objects
            .iter()
            .position(|candidate| {
                std::ptr::eq(
                    candidate.as_ref() as *const FfiObject as *const DwgObject,
                    obj,
                )
            })
            .ok_or(DWG_ENOTFOUND)
    }

    /// Mark an object and any core-owned companion objects as removed.
    pub(crate) fn remove_object(&mut self, obj: *const DwgObject) -> Result<(), i32> {
        let index = self.object_index(obj)?;
        if !self.objects[index].alive {
            return Err(DWG_ENOTFOUND);
        }

        let handle = self.objects[index].handle;
        let is_entity = !self.objects[index].entity.is_null();
        let mut removed_handles = Vec::new();
        if is_entity {
            // Delete the history graph while the entity is still present so
            // CadDocument can resolve its history root safely.
            removed_handles.extend(
                self.doc
                    .delete_solid_history(handle)
                    .into_iter()
                    .map(|(history_handle, _)| history_handle),
            );
            if self.doc.remove_entity_arc(handle).is_none() {
                return Err(DWG_ENOTFOUND);
            }
            // CadDocument::remove_entity_arc intentionally keeps this list
            // for undo. A real FFI deletion must remove the live owner link.
            for block in self.doc.block_records.iter_mut() {
                block
                    .entity_handles
                    .retain(|candidate| *candidate != handle);
            }
        } else if self.doc.objects.remove(&handle).is_none() {
            return Err(DWG_ENOTFOUND);
        }

        removed_handles.push(handle);
        for candidate in &mut self.objects {
            if removed_handles.contains(&candidate.handle) {
                candidate.alive = false;
                candidate.entity = ptr::null();
                candidate.object = ptr::null();
                candidate.next = ptr::null();
                candidate.kind = DwgObjectType::Unknown;
                candidate.handle = Handle::NULL;
            }
        }
        self.relink_objects();
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn mutate_entity<F>(
        &mut self,
        obj: *const DwgObject,
        expected: Option<DwgObjectType>,
        mutate: F,
    ) -> i32
    where
        F: FnOnce(&mut EntityType) -> Result<(), MutationError>,
    {
        if obj.is_null() {
            set_last_error(DWG_EINVAL, "object pointer is null");
            return DWG_EINVAL;
        }
        let Some(index) = self.objects.iter().position(|candidate| {
            let candidate = candidate.as_ref() as *const FfiObject as *const DwgObject;
            std::ptr::eq(candidate, obj)
        }) else {
            set_last_error(DWG_ENOTFOUND, "object does not belong to document");
            return DWG_ENOTFOUND;
        };
        if !self.objects[index].alive {
            set_last_error(DWG_ENOTFOUND, "object has been removed");
            return DWG_ENOTFOUND;
        }
        if self.objects[index].entity.is_null() {
            set_last_error(DWG_ETYPE, "object is not an entity");
            return DWG_ETYPE;
        }
        if expected.is_some_and(|kind| self.objects[index].kind != kind) {
            set_last_error(DWG_ETYPE, "entity type does not match setter");
            return DWG_ETYPE;
        }

        let handle = self.objects[index].handle;
        let result = match self.doc.get_entity_mut(handle) {
            Some(entity) => mutate(entity),
            None => Err((DWG_ENOTFOUND, "entity handle is missing from document")),
        };
        let Some(entity) = self.doc.get_entity(handle) else {
            set_last_error(DWG_ENOTFOUND, "entity disappeared during mutation");
            return DWG_ENOTFOUND;
        };
        self.objects[index].entity = entity as *const EntityType;

        match result {
            Ok(()) => {
                clear_last_error();
                DWG_OK
            }
            Err((code, message)) => {
                set_last_error(code, message);
                code
            }
        }
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
        doc.add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 1.0, 2.0, 3.0,
        )))
        .unwrap();
        doc.add_entity(EntityType::Circle(Circle::from_center_radius(
            Vector3::new(5.0, 6.0, 0.0),
            2.5,
        )))
        .unwrap();
        let mut t = Text::new();
        t.value = "hello".to_string();
        t.height = 1.0;
        doc.add_entity(EntityType::Text(t)).unwrap();
        doc
    }

    #[test]
    fn arena_builds_in_entity_order() {
        let ffi = FfiDocument::new(sample_doc());
        assert!(
            ffi.objects.len() > 3,
            "arena should contain entities plus default objects"
        );
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

    #[test]
    fn mutation_refreshes_entity_pointer_without_moving_object_handle() {
        let mut ffi = FfiDocument::new(sample_doc());
        let handle = ffi.objects[0].handle;
        let obj = ffi.objects[0].as_ref() as *const FfiObject as *const DwgObject;
        let old_entity = ffi.objects[0].entity;
        let held_old_arc = ffi.doc.get_entity_arc(handle).unwrap();

        let rc = ffi.mutate_entity(obj, Some(DwgObjectType::Line), |entity| {
            let EntityType::Line(line) = entity else {
                return Err((DWG_ETYPE, "expected LINE"));
            };
            line.start = Vector3::new(9.0, 8.0, 7.0);
            Ok(())
        });

        assert_eq!(rc, DWG_OK);
        assert_eq!(
            obj,
            ffi.objects[0].as_ref() as *const FfiObject as *const DwgObject
        );
        assert_ne!(old_entity, ffi.objects[0].entity);
        assert_eq!(held_old_arc.as_ref().common().handle, handle);
        let EntityType::Line(line) = (unsafe { &*ffi.objects[0].entity }) else {
            panic!()
        };
        assert_eq!(line.start, Vector3::new(9.0, 8.0, 7.0));
    }

    #[test]
    fn mutation_rejects_foreign_and_wrong_type_objects() {
        let mut left = FfiDocument::new(sample_doc());
        let right = FfiDocument::new(sample_doc());
        let foreign = right.objects[0].as_ref() as *const FfiObject as *const DwgObject;
        assert_eq!(left.mutate_entity(foreign, None, |_| Ok(())), DWG_ENOTFOUND);

        let local = left.objects[0].as_ref() as *const FfiObject as *const DwgObject;
        assert_eq!(
            left.mutate_entity(local, Some(DwgObjectType::Circle), |_| Ok(())),
            DWG_ETYPE
        );
    }
}
