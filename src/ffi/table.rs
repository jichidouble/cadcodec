//! Symbol-table access: Layer, Block, Style, DimStyle.

use crate::ffi::error::clear_last_error;
use crate::ffi::object::FfiDocument;
use crate::ffi::types::*;
use crate::ffi::util::{color_from_c, color_to_c, copy_c_string};
use crate::tables::{DimStyle, Layer, TableEntry, TextStyle};

/// Which symbol table a generic call targets.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DwgTableKind {
    Layer = 0,
    Block = 1,
    Style = 2,
    DimStyle = 3,
}

fn layers<'a>(data: *const DwgData) -> Result<&'a crate::tables::Table<Layer>, i32> {
    if data.is_null() {
        return Err(DWG_EINVAL);
    }
    let ffi = unsafe { &*(data as *const FfiDocument) };
    Ok(&ffi.doc.layers)
}

/// Number of layers in the document.
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_table_count(data: *const DwgData, count: *mut i32) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    match layers(data) {
        Ok(t) => {
            *count = t.len() as i32;
            DWG_OK
        }
        Err(code) => code,
    }
}

/// Copy the name of the layer at `index` (iteration order).
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_get_name(
    data: *const DwgData,
    index: i32,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    match layers(data) {
        Ok(t) => match t.iter().nth(index as usize) {
            Some(layer) => copy_c_string(layer.name(), buf, cap, needed),
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Read the layer color at `index` as an ARGB int32.
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_get_color(
    data: *const DwgData,
    index: i32,
    color: *mut i32,
) -> i32 {
    if color.is_null() {
        return DWG_EINVAL;
    }
    match layers(data) {
        Ok(t) => match t.iter().nth(index as usize) {
            Some(layer) => {
                *color = color_to_c(&layer.color);
                DWG_OK
            }
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Read the layer handle at `index`.
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_get_handle(
    data: *const DwgData,
    index: i32,
    handle: *mut u64,
) -> i32 {
    if handle.is_null() {
        return DWG_EINVAL;
    }
    match layers(data) {
        Ok(t) => match t.iter().nth(index as usize) {
            Some(layer) => {
                *handle = layer.handle().value();
                DWG_OK
            }
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Set the layer color at `index`.
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_set_color(
    data: *mut DwgData,
    index: i32,
    color: i32,
) -> i32 {
    if data.is_null() {
        return DWG_EINVAL;
    }
    let ffi = &mut *(data as *mut FfiDocument);
    let Some(layer) = ffi.doc.layers.iter_mut().nth(index as usize) else {
        return DWG_ENOTFOUND;
    };
    layer.color = color_from_c(color);
    clear_last_error();
    DWG_OK
}

/// Locate a layer by name; `*index` receives its iteration index.
#[no_mangle]
pub unsafe extern "C" fn dwg_layer_index_by_name(
    data: *const DwgData,
    name: *const i8,
    index: *mut i32,
) -> i32 {
    if name.is_null() || index.is_null() {
        return DWG_EINVAL;
    }
    let cname = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DWG_EINVAL,
    };
    match layers(data) {
        Ok(t) => match t.iter().position(|l| l.name() == cname) {
            Some(i) => {
                *index = i as i32;
                DWG_OK
            }
            None => DWG_ENOTFOUND,
        },
        Err(code) => code,
    }
}

/// Number of entries in a symbol table (`DwgTableKind`).
#[no_mangle]
pub unsafe extern "C" fn dwg_table_entry_count(
    data: *const DwgData,
    table: i32,
    count: *mut i32,
) -> i32 {
    if count.is_null() {
        return DWG_EINVAL;
    }
    if data.is_null() {
        return DWG_EINVAL;
    }
    let ffi = &*(data as *const FfiDocument);
    let len = match table {
        t if t == DwgTableKind::Layer as i32 => ffi.doc.layers.len(),
        t if t == DwgTableKind::Block as i32 => ffi.doc.block_records.len(),
        t if t == DwgTableKind::Style as i32 => ffi.doc.text_styles.len(),
        t if t == DwgTableKind::DimStyle as i32 => ffi.doc.dim_styles.len(),
        _ => return DWG_EINVAL,
    };
    *count = len as i32;
    DWG_OK
}

fn table_name<'a>(ffi: &'a FfiDocument, table: i32, index: i32) -> Option<&'a str> {
    let i = index as usize;
    match table {
        t if t == DwgTableKind::Layer as i32 => ffi.doc.layers.iter().nth(i).map(|e| e.name()),
        t if t == DwgTableKind::Block as i32 => ffi.doc.block_records.iter().nth(i).map(|e| e.name()),
        t if t == DwgTableKind::Style as i32 => ffi.doc.text_styles.iter().nth(i).map(|e| e.name()),
        t if t == DwgTableKind::DimStyle as i32 => ffi.doc.dim_styles.iter().nth(i).map(|e| e.name()),
        _ => None,
    }
}

/// Copy the name of table entry `index` in the given `DwgTableKind`.
#[no_mangle]
pub unsafe extern "C" fn dwg_table_entry_get_name(
    data: *const DwgData,
    table: i32,
    index: i32,
    buf: *mut i8,
    cap: usize,
    needed: *mut usize,
) -> i32 {
    if data.is_null() || needed.is_null() || (!buf.is_null() && cap == 0) {
        return DWG_EINVAL;
    }
    let ffi = &*(data as *const FfiDocument);
    match table_name(ffi, table, index) {
        Some(name) => copy_c_string(name, buf, cap, needed),
        None => DWG_ENOTFOUND,
    }
}

// Referenced to keep the trait bounds exercised for both table families.
fn _assert_table_types(_: &crate::tables::Table<DimStyle>, _: &crate::tables::Table<TextStyle>) {}
