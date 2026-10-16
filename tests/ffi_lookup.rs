use acadrust::entities::{Circle, Line};
use acadrust::ffi::object::FfiDocument;
use acadrust::ffi::*;
use acadrust::types::Vector3;
use acadrust::{CadDocument, EntityType};

struct OwnedData(*mut DwgData);

impl OwnedData {
    fn new(doc: CadDocument) -> Self {
        Self(Box::into_raw(Box::new(FfiDocument::new(doc))) as *mut DwgData)
    }

    fn as_ptr(&self) -> *mut DwgData {
        self.0
    }
}

impl Drop for OwnedData {
    fn drop(&mut self) {
        unsafe { dwg_data_destroy(self.0) };
    }
}

fn populated_document() -> OwnedData {
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
    OwnedData::new(doc)
}

fn active_object_count(data: *const DwgData) -> usize {
    let mut error = DWG_EINVAL;
    let mut object = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    let mut count = 0;
    while !object.is_null() {
        count += 1;
        object = unsafe { dwg_object_next(object) };
    }
    count
}

#[test]
fn document_counts_and_lookup_follow_active_arena() {
    let data = populated_document();
    let other = populated_document();

    let mut entity_count = -1;
    assert_eq!(
        unsafe { dwg_data_entity_count(data.as_ptr(), &mut entity_count) },
        DWG_OK
    );
    assert_eq!(entity_count, 2);

    let mut object_count: i32 = -1;
    assert_eq!(
        unsafe { dwg_data_object_count(data.as_ptr(), &mut object_count) },
        DWG_OK
    );
    assert_eq!(object_count as usize, active_object_count(data.as_ptr()));
    let object_count_before_removal = object_count;
    let other_before = active_object_count(other.as_ptr());

    let mut error = DWG_EINVAL;
    let first = unsafe { dwg_data_first_object(data.as_ptr(), &mut error) };
    assert_eq!(error, DWG_OK);
    assert!(!first.is_null());
    let first_handle = unsafe { dwg_object_get_handle(first) };
    assert_ne!(first_handle, 0);

    let found = unsafe { dwg_object_by_handle(data.as_ptr(), first_handle, &mut error) };
    assert_eq!(error, DWG_OK);
    assert_eq!(found, first);

    assert_eq!(unsafe { dwg_object_remove(data.as_ptr(), first) }, DWG_OK);
    assert_eq!(
        unsafe { dwg_data_entity_count(data.as_ptr(), &mut entity_count) },
        DWG_OK
    );
    assert_eq!(entity_count, 1);
    assert_eq!(
        unsafe { dwg_data_object_count(data.as_ptr(), &mut object_count) },
        DWG_OK
    );
    assert_eq!(object_count, object_count_before_removal - 1);
    assert_eq!(object_count as usize, active_object_count(data.as_ptr()));
    assert_eq!(
        unsafe { dwg_object_by_handle(data.as_ptr(), first_handle, &mut error) },
        std::ptr::null()
    );
    assert_eq!(error, DWG_ENOTFOUND);
    assert_eq!(active_object_count(other.as_ptr()), other_before);
}

#[test]
fn counts_and_lookup_report_empty_and_invalid_inputs() {
    let empty = OwnedData::new(CadDocument::new());
    let mut count = -1;
    assert_eq!(
        unsafe { dwg_data_entity_count(empty.as_ptr(), &mut count) },
        DWG_OK
    );
    assert_eq!(count, 0);
    assert_eq!(
        unsafe { dwg_data_entity_count(empty.as_ptr(), std::ptr::null_mut()) },
        DWG_EINVAL
    );
    assert_eq!(
        unsafe { dwg_data_object_count(empty.as_ptr(), &mut count) },
        DWG_OK
    );
    assert_eq!(count as usize, active_object_count(empty.as_ptr()));
    assert_eq!(
        unsafe { dwg_data_object_count(empty.as_ptr(), std::ptr::null_mut()) },
        DWG_EINVAL
    );

    let mut error = DWG_OK;
    assert_eq!(
        unsafe { dwg_object_by_handle(empty.as_ptr(), 0, &mut error) },
        std::ptr::null()
    );
    assert_eq!(error, DWG_ENOTFOUND);
    assert_eq!(
        unsafe { dwg_object_by_handle(empty.as_ptr(), u64::MAX, &mut error) },
        std::ptr::null()
    );
    assert_eq!(error, DWG_ENOTFOUND);
    assert_eq!(
        unsafe { dwg_object_by_handle(std::ptr::null(), 1, &mut error) },
        std::ptr::null()
    );
    assert_eq!(error, DWG_EINVAL);
    assert_eq!(
        unsafe { dwg_object_by_handle(empty.as_ptr(), 1, std::ptr::null_mut()) },
        std::ptr::null()
    );
}
