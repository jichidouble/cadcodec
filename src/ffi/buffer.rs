use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_void;
use std::ptr;

use crate::ffi::types::DWG_ENOMEM;

const BUFFER_MAGIC: u64 = 0x4341_4443_4f44_4543;

#[repr(C)]
#[derive(Clone, Copy)]
struct BufferHeader {
    magic: u64,
    len: usize,
}

fn allocation_layout(len: usize) -> Option<(Layout, usize)> {
    Layout::new::<BufferHeader>()
        .extend(Layout::array::<u8>(len).ok()?)
        .ok()
        .map(|(layout, offset)| (layout.pad_to_align(), offset))
}

pub(crate) fn into_ffi_buffer(bytes: Vec<u8>) -> Result<(*mut u8, usize), i32> {
    let len = bytes.len();
    let (layout, payload_offset) = allocation_layout(len).ok_or(DWG_ENOMEM)?;
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        return Err(DWG_ENOMEM);
    }

    unsafe {
        ptr::write(
            base.cast::<BufferHeader>(),
            BufferHeader {
                magic: BUFFER_MAGIC,
                len,
            },
        );
        ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(payload_offset), len);
    }
    Ok((unsafe { base.add(payload_offset) }, len))
}

pub(crate) unsafe fn free_ffi_buffer(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let payload = ptr.cast::<u8>();
    let header_offset = std::mem::size_of::<BufferHeader>();
    let base = payload.sub(header_offset);
    let header = ptr::read(base.cast::<BufferHeader>());
    if header.magic != BUFFER_MAGIC {
        return;
    }

    let Some((layout, payload_offset)) = allocation_layout(header.len) else {
        return;
    };
    debug_assert_eq!(payload_offset, header_offset);
    dealloc(base, layout);
}

#[cfg(test)]
mod tests {
    use super::{free_ffi_buffer, into_ffi_buffer};

    #[test]
    fn allocates_and_releases_non_empty_payload() {
        let payload = b"phase4a-bytes".to_vec();
        let (ptr, len) = into_ffi_buffer(payload.clone()).expect("allocation should succeed");

        assert_eq!(len, payload.len());
        let actual = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert_eq!(actual, payload.as_slice());

        unsafe { free_ffi_buffer(ptr.cast()) };
    }

    #[test]
    fn allocates_and_releases_empty_payload() {
        let (ptr, len) = into_ffi_buffer(Vec::new()).expect("empty allocation should succeed");

        assert_eq!(len, 0);
        assert!(!ptr.is_null());

        unsafe { free_ffi_buffer(ptr.cast()) };
    }

    #[test]
    fn releasing_null_is_a_no_op() {
        unsafe { free_ffi_buffer(std::ptr::null_mut()) };
    }
}
