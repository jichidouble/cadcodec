//! Dictionary access for the C ABI.

use crate::ffi::error::{clear_last_error, set_last_error};
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::ffi::util::read_c_string;
use crate::objects::{Dictionary, ObjectType};
use crate::types::Handle;

fn fail(code: i32, message: &'static str) -> i32 {
    set_last_error(code, message);
    code
}

unsafe fn document<'a>(data: *const DwgData) -> Result<&'a FfiDocument, i32> {
    if data.is_null() {
        return Err(fail(DWG_EINVAL, "document pointer is null"));
    }
    Ok(&*(data as *const FfiDocument))
}

unsafe fn document_mut<'a>(data: *mut DwgData) -> Result<&'a mut FfiDocument, i32> {
    if data.is_null() {
        return Err(fail(DWG_EINVAL, "document pointer is null"));
    }
    Ok(&mut *(data as *mut FfiDocument))
}

fn dictionary<'a>(document: &'a FfiDocument, handle: u64) -> Result<&'a Dictionary, i32> {
    match document.doc.objects.get(&Handle::new(handle)) {
        Some(ObjectType::Dictionary(dictionary)) => Ok(dictionary),
        Some(_) => Err(fail(DWG_ETYPE, "object is not a dictionary")),
        None => Err(fail(
            DWG_ENOTFOUND,
            "dictionary handle is missing from document",
        )),
    }
}

fn dictionary_mut<'a>(
    document: &'a mut FfiDocument,
    handle: u64,
) -> Result<&'a mut Dictionary, i32> {
    match document.doc.objects.get_mut(&Handle::new(handle)) {
        Some(ObjectType::Dictionary(dictionary)) => Ok(dictionary),
        Some(_) => Err(fail(DWG_ETYPE, "object is not a dictionary")),
        None => Err(fail(
            DWG_ENOTFOUND,
            "dictionary handle is missing from document",
        )),
    }
}

unsafe fn dictionary_key(key: *const i8) -> Result<String, i32> {
    if key.is_null() {
        return Err(fail(DWG_EINVAL, "dictionary key is null"));
    }
    read_c_string(key).map_err(|_| fail(DWG_EINVAL, "dictionary key is not valid UTF-8"))
}

/// Get the handle of the named-objects root dictionary.
#[no_mangle]
pub unsafe extern "C" fn dwg_data_named_objects_dictionary(
    data: *const DwgData,
    dictionary_handle: *mut u64,
) -> i32 {
    if dictionary_handle.is_null() {
        return fail(DWG_EINVAL, "dictionary handle output is null");
    }
    let ffi = match document(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    *dictionary_handle = ffi.doc.header.named_objects_dict_handle.value();
    clear_last_error();
    DWG_OK
}

/// Get the number of entries in a dictionary.
#[no_mangle]
pub unsafe extern "C" fn dwg_dictionary_count(
    data: *const DwgData,
    dictionary_handle: u64,
    count: *mut usize,
) -> i32 {
    if count.is_null() {
        return fail(DWG_EINVAL, "dictionary count output is null");
    }
    let ffi = match document(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    let dictionary = match dictionary(ffi, dictionary_handle) {
        Ok(dictionary) => dictionary,
        Err(code) => return code,
    };
    *count = dictionary.len();
    clear_last_error();
    DWG_OK
}

/// Look up an entry in a dictionary by case-insensitive key.
#[no_mangle]
pub unsafe extern "C" fn dwg_dictionary_get(
    data: *const DwgData,
    dictionary_handle: u64,
    key: *const i8,
    value: *mut u64,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "dictionary value output is null");
    }
    let ffi = match document(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    let key = match dictionary_key(key) {
        Ok(key) => key,
        Err(code) => return code,
    };
    let dictionary = match dictionary(ffi, dictionary_handle) {
        Ok(dictionary) => dictionary,
        Err(code) => return code,
    };
    let Some(handle) = dictionary.get(&key) else {
        return fail(DWG_ENOTFOUND, "dictionary entry is missing");
    };
    *value = handle.value();
    clear_last_error();
    DWG_OK
}

/// Read a dictionary entry by index into caller-owned output storage.
#[no_mangle]
pub unsafe extern "C" fn dwg_dictionary_entry(
    data: *const DwgData,
    dictionary_handle: u64,
    index: usize,
    key: *mut i8,
    key_capacity: usize,
    needed: *mut usize,
    value: *mut u64,
) -> i32 {
    if needed.is_null() || value.is_null() || (!key.is_null() && key_capacity == 0) {
        return fail(DWG_EINVAL, "dictionary entry output is invalid");
    }
    let ffi = match document(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    let dictionary = match dictionary(ffi, dictionary_handle) {
        Ok(dictionary) => dictionary,
        Err(code) => return code,
    };
    let Some((entry_key, entry_handle)) = dictionary.entries.get(index) else {
        return fail(DWG_ENOTFOUND, "dictionary entry index is out of range");
    };

    let bytes = entry_key.as_bytes();
    let total = bytes.len() + 1;
    *needed = total;
    if key.is_null() {
        *value = entry_handle.value();
        clear_last_error();
        return DWG_OK;
    }
    if key_capacity < total {
        return fail(DWG_EBUFFER, "dictionary key buffer is too small");
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), key.cast::<u8>(), bytes.len());
    *key.add(bytes.len()) = 0;
    *value = entry_handle.value();
    clear_last_error();
    DWG_OK
}

/// Set a dictionary entry, replacing a case-insensitive key when it exists.
#[no_mangle]
pub unsafe extern "C" fn dwg_dictionary_set(
    data: *mut DwgData,
    dictionary_handle: u64,
    key: *const i8,
    value: u64,
) -> i32 {
    let ffi = match document_mut(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    let key = match dictionary_key(key) {
        Ok(key) => key,
        Err(code) => return code,
    };
    if let Err(code) = dictionary(ffi, dictionary_handle) {
        return code;
    }
    if !ffi.doc.objects.contains_key(&Handle::new(value)) {
        return fail(
            DWG_ENOTFOUND,
            "dictionary value handle is missing from document",
        );
    }
    let dictionary = match dictionary_mut(ffi, dictionary_handle) {
        Ok(dictionary) => dictionary,
        Err(code) => return code,
    };
    if let Some((_, handle)) = dictionary
        .entries
        .iter_mut()
        .find(|(entry_key, _)| entry_key.eq_ignore_ascii_case(&key))
    {
        *handle = Handle::new(value);
    } else {
        dictionary.add_entry(key, Handle::new(value));
    }
    clear_last_error();
    DWG_OK
}

/// Remove one dictionary entry by case-insensitive key.
#[no_mangle]
pub unsafe extern "C" fn dwg_dictionary_remove(
    data: *mut DwgData,
    dictionary_handle: u64,
    key: *const i8,
    value: *mut u64,
) -> i32 {
    if value.is_null() {
        return fail(DWG_EINVAL, "dictionary value output is null");
    }
    let ffi = match document_mut(data) {
        Ok(ffi) => ffi,
        Err(code) => return code,
    };
    let key = match dictionary_key(key) {
        Ok(key) => key,
        Err(code) => return code,
    };
    let dictionary = match dictionary_mut(ffi, dictionary_handle) {
        Ok(dictionary) => dictionary,
        Err(code) => return code,
    };
    let Some(index) = dictionary
        .entries
        .iter()
        .position(|(entry_key, _)| entry_key.eq_ignore_ascii_case(&key))
    else {
        return fail(DWG_ENOTFOUND, "dictionary entry is missing");
    };
    let (removed_key, handle) = dictionary.entries.remove(index);
    dictionary.set_entry_hard_owner(&removed_key, false);
    *value = handle.value();
    clear_last_error();
    DWG_OK
}
