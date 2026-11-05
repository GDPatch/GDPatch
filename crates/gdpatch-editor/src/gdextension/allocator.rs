//! [GlobalAlloc] implementation using the GDExtension allocator API.
use crate::gdextension::api::GDExtensionApi;
use std::alloc::{GlobalAlloc, Layout};
use std::ffi::c_void;
use std::ptr;

#[derive(Debug)]
pub struct GDExtensionAllocator;

const ALIGNMENT_METADATA_SIZE: usize = size_of::<usize>();
const GUARANTEED_ALIGNMENT: usize = align_of::<*const ()>();

const _: () = {
    if GUARANTEED_ALIGNMENT < align_of::<usize>() {
        panic!("guaranteed alignment must be more than the metadata alignment");
    }
};

unsafe fn required_additional_size_for_alignment(layout: Layout) -> usize {
    layout.align() + ALIGNMENT_METADATA_SIZE
}

unsafe fn align_ptr_with_metadata(ptr: *mut u8, layout: Layout) -> *mut u8 {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let align = layout.align();

    // SAFETY: Memory block must have been allocated large enough for this.
    let required_offset = unsafe { ptr.byte_add(ALIGNMENT_METADATA_SIZE).align_offset(align) };

    debug_assert!(required_offset * size_of::<u8>() <= align);

    // SAFETY: we allocated the memory block large enough to store this
    let aligned_ptr = unsafe { ptr.add(required_offset) };

    // SAFETY: address is valid for writes and aligned to at least pointer
    unsafe {
        aligned_ptr
            .cast::<usize>()
            .byte_sub(ALIGNMENT_METADATA_SIZE)
            .write(required_offset);
    }

    aligned_ptr
}

unsafe fn unalign_ptr(ptr: *mut u8, layout: Layout) -> *mut u8 {
    if layout.align() > GUARANTEED_ALIGNMENT {
        // SAFETY: `alloc` stores the offset just before the returned pointer
        let original_offset =
            unsafe { ptr.cast::<usize>().byte_sub(ALIGNMENT_METADATA_SIZE).read() };

        // SAFETY: Pointer was allocated with align_ptr_with_metadata
        unsafe { ptr.byte_sub(original_offset + ALIGNMENT_METADATA_SIZE) }
    } else {
        ptr
    }
}

unsafe impl GlobalAlloc for GDExtensionAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(api) = GDExtensionApi::try_instance() else {
            return ptr::null_mut();
        };

        let align = layout.align();

        let aligned_ptr = unsafe {
            if align > GUARANTEED_ALIGNMENT {
                // alignment is more than guaranteed alignment
                let additional_size = required_additional_size_for_alignment(layout);
                let ptr = (api.interface.mem_alloc)(layout.size() + additional_size) as *mut u8;
                align_ptr_with_metadata(ptr, layout)
            } else {
                (api.interface.mem_alloc)(layout.size()) as *mut u8
            }
        };

        if aligned_ptr.is_null() {
            return ptr::null_mut();
        }

        debug_assert_eq!(aligned_ptr.addr() & (align - 1), 0);
        aligned_ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let Some(api) = GDExtensionApi::try_instance() else {
            return;
        };

        unsafe {
            let original_ptr = unalign_ptr(ptr, layout);
            (api.interface.mem_free)(original_ptr as *mut c_void);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            unsafe {
                let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
                return self.alloc(new_layout);
            }
        }

        let Some(api) = GDExtensionApi::try_instance() else {
            return ptr::null_mut();
        };

        let align = layout.align();
        let aligned_ptr = unsafe {
            if align > GUARANTEED_ALIGNMENT {
                let unaligned_ptr = unalign_ptr(ptr, layout);
                let additional_size = required_additional_size_for_alignment(layout);
                let new_ptr = (api.interface.mem_realloc)(
                    unaligned_ptr as *mut c_void,
                    new_size + additional_size,
                ) as *mut u8;
                align_ptr_with_metadata(new_ptr, layout)
            } else {
                (api.interface.mem_realloc)(ptr as *mut c_void, new_size) as *mut u8
            }
        };

        if aligned_ptr.is_null() {
            return ptr::null_mut();
        }

        debug_assert_eq!(aligned_ptr.addr() & (align - 1), 0);
        aligned_ptr
    }
}
