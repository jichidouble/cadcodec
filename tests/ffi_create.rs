use acadrust::ffi::*;

struct OwnedData(*mut DwgData);

impl OwnedData {
    fn new() -> Self {
        let mut data = std::ptr::null_mut();
        assert_eq!(dwg_data_create(&mut data), DWG_OK);
        assert!(!data.is_null());
        Self(data)
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

fn entity_count(data: *const DwgData) -> i32 {
    let mut count = -1;
    assert_eq!(unsafe { dwg_data_entity_count(data, &mut count) }, DWG_OK);
    count
}

fn object_count(data: *const DwgData) -> i32 {
    let mut count = -1;
    assert_eq!(unsafe { dwg_data_object_count(data, &mut count) }, DWG_OK);
    count
}

#[test]
fn create_basic_entities_and_validate_ffi_contract() {
    let first = OwnedData::new();
    let data = first.as_ptr();
    let initial_entity_count = entity_count(data);
    let initial_object_count = object_count(data);

    let mut line: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe { dwg_entity_line_create(data, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, &mut line) },
        DWG_OK
    );
    assert!(!line.is_null());
    assert_eq!(
        unsafe { dwg_object_get_type(line) },
        DwgObjectType::Line as i32
    );
    assert_ne!(unsafe { dwg_object_get_handle(line) }, 0);

    let mut circle: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe { dwg_entity_circle_create(data, 6.0, 7.0, 8.0, 9.0, &mut circle) },
        DWG_OK
    );
    assert!(!circle.is_null());
    assert_eq!(
        unsafe { dwg_object_get_type(circle) },
        DwgObjectType::Circle as i32
    );
    assert_ne!(unsafe { dwg_object_get_handle(circle) }, 0);

    let mut arc: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe { dwg_entity_arc_create(data, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, &mut arc) },
        DWG_OK
    );
    assert!(!arc.is_null());
    assert_eq!(
        unsafe { dwg_object_get_type(arc) },
        DwgObjectType::Arc as i32
    );
    assert_ne!(unsafe { dwg_object_get_handle(arc) }, 0);

    let mut point: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe { dwg_entity_point_create(data, 16.0, 17.0, 18.0, &mut point) },
        DWG_OK
    );
    assert!(!point.is_null());
    assert_eq!(
        unsafe { dwg_object_get_type(point) },
        DwgObjectType::Point as i32
    );
    assert_ne!(unsafe { dwg_object_get_handle(point) }, 0);

    assert_eq!(entity_count(data), initial_entity_count + 4);
    assert_eq!(object_count(data), initial_object_count + 4);

    let created = [line, circle, arc, point];
    for object in created {
        let handle = unsafe { dwg_object_get_handle(object) };
        let mut error = -1;
        let resolved = unsafe { dwg_object_by_handle(data, handle, &mut error) };
        assert_eq!(error, DWG_OK);
        assert_eq!(resolved, object as *const DwgObject);
    }

    let mut error = -1;
    let mut cursor = unsafe { dwg_data_first_object(data, &mut error) };
    assert_eq!(error, DWG_OK);
    let mut reachable = Vec::new();
    while !cursor.is_null() {
        reachable.push(cursor);
        cursor = unsafe { dwg_object_next(cursor) };
    }
    for object in created {
        assert!(reachable.contains(&(object as *const DwgObject)));
    }

    let second = OwnedData::new();
    let second_data = second.as_ptr();
    let second_initial_object_count = object_count(second_data);
    let mut second_point: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe { dwg_entity_point_create(second_data, 19.0, 20.0, 21.0, &mut second_point) },
        DWG_OK
    );
    assert!(!second_point.is_null());
    assert_eq!(entity_count(second_data), 1);
    assert_eq!(object_count(second_data), second_initial_object_count + 1);
    assert_ne!(second_point, point);
    let mut second_error = -1;
    assert_eq!(
        unsafe {
            dwg_object_by_handle(
                second_data,
                unsafe { dwg_object_get_handle(point) },
                &mut second_error,
            )
        },
        std::ptr::null()
    );
    assert_eq!(second_error, DWG_ENOTFOUND);

    let unchanged_count = entity_count(data);
    let mut invalid_output: *mut DwgObject = std::ptr::null_mut();
    assert_eq!(
        unsafe {
            dwg_entity_line_create(
                std::ptr::null_mut(),
                0.0,
                1.0,
                2.0,
                3.0,
                4.0,
                5.0,
                &mut invalid_output,
            )
        },
        DWG_EINVAL
    );
    assert!(invalid_output.is_null());
    assert_eq!(
        unsafe { dwg_entity_line_create(data, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, std::ptr::null_mut()) },
        DWG_EINVAL
    );

    invalid_output = std::ptr::null_mut();
    assert_eq!(
        unsafe {
            dwg_entity_circle_create(
                std::ptr::null_mut(),
                0.0,
                1.0,
                2.0,
                3.0,
                &mut invalid_output,
            )
        },
        DWG_EINVAL
    );
    assert!(invalid_output.is_null());
    assert_eq!(
        unsafe { dwg_entity_circle_create(data, 0.0, 1.0, 2.0, 3.0, std::ptr::null_mut()) },
        DWG_EINVAL
    );

    invalid_output = std::ptr::null_mut();
    assert_eq!(
        unsafe {
            dwg_entity_arc_create(
                std::ptr::null_mut(),
                0.0,
                1.0,
                2.0,
                3.0,
                4.0,
                5.0,
                &mut invalid_output,
            )
        },
        DWG_EINVAL
    );
    assert!(invalid_output.is_null());
    assert_eq!(
        unsafe { dwg_entity_arc_create(data, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, std::ptr::null_mut()) },
        DWG_EINVAL
    );

    invalid_output = std::ptr::null_mut();
    assert_eq!(
        unsafe {
            dwg_entity_point_create(std::ptr::null_mut(), 0.0, 1.0, 2.0, &mut invalid_output)
        },
        DWG_EINVAL
    );
    assert!(invalid_output.is_null());
    assert_eq!(
        unsafe { dwg_entity_point_create(data, 0.0, 1.0, 2.0, std::ptr::null_mut()) },
        DWG_EINVAL
    );

    assert_eq!(entity_count(data), unchanged_count);
}
