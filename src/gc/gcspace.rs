use std::{alloc::Layout, ptr::addr_of_mut};

use crate::gc::GarbageCollector;

pub struct GCSpace {
    data: *mut u8,
    /// Currently occupied space (in blocks)
    len: usize,
    /// Capacity (in blocks)
    capacity: usize,
}

/// A garbage collected string.
///
/// We cannot use regular heap-allocated strings because the fields of garbage collected objects are
/// never dropped.
///
/// # Inline strings
/// If the most significant bit of `ptr` is set, it is an "inline string". The next seven bits
/// indicate the length. The remainder of the `GCString` contains the object. On little endian
/// systems, the string data starts on the 0th byte, and on big endian platforms, it starts on the
/// 1th byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GCString {
    #[cfg(target_endian = "little")]
    len: usize,
    ptr: usize,
    #[cfg(target_endian = "big")]
    len: usize,
}

impl GCString {
    /// Gets the string if it is less than 16 bytes long. Otherwise returns `None`
    pub fn get_inline(&self) -> Option<&str> {
        if self.ptr & (0b1 << usize::BITS - 1) == 0 {
            return None;
        }

        let len = (self.ptr >> (usize::BITS - 8)) & 0b0111_1111;

        let ptr = std::ptr::from_ref(self).cast::<u8>();

        #[cfg(target_endian = "big")]
        let ptr = unsafe { ptr.byte_offset(1) };

        let string = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
        Some(string)
    }
}

impl GCSpace {
    const STARTING_BLOCKS: usize = 64;

    /// Grows the allocation to be at least `new_size_blocks` blocks
    pub fn expand(&mut self, new_size_blocks: usize) {
        let mut new_size = self.capacity;

        while new_size < new_size_blocks {
            new_size *= 2;
        }

        if new_size == self.capacity {
            return;
        }

        self.data = unsafe {
            std::alloc::realloc(
                self.data,
                Layout::from_size_align_unchecked(
                    self.capacity * GarbageCollector::BLOCK_SIZE,
                    GarbageCollector::BLOCK_SIZE,
                ),
                new_size,
            )
        };

        self.capacity = new_size;
    }

    pub fn new() -> Self {
        let data = unsafe {
            std::alloc::alloc(Layout::from_size_align_unchecked(
                Self::STARTING_BLOCKS * GarbageCollector::BLOCK_SIZE,
                GarbageCollector::BLOCK_SIZE,
            ))
        };

        Self {
            data,
            len: 0,
            capacity: Self::STARTING_BLOCKS,
        }
    }

    /// Creates a garbace collected string without invoking a GC pass and WITHOUT CREATING A ROOT
    pub fn alloc_string(&mut self, string: &str) -> GCString {
        // If the string is small enough, it can be stored inline rather than on the GC Heap

        if string.len() < std::mem::size_of::<GCString>() && string.len() <= 127 {
            let discriminant: usize = (0b1000_0000 | string.len()) << (usize::BITS - 8); // The MSB being set signifies that the string is stored inline

            let mut retval = GCString {
                ptr: discriminant,
                len: 0,
            };

            let ptr = addr_of_mut!(retval).cast::<u8>();

            #[cfg(target_endian = "big")]
            let ptr = unsafe { ptr.byte_offset(1) };

            unsafe {
                std::ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
            }

            return retval;
        }

        let num_blocks = string.len().div_ceil(GarbageCollector::BLOCK_SIZE);
        self.expand(self.len + num_blocks);

        let data_ptr = self.len * GarbageCollector::BLOCK_SIZE;
        self.len += num_blocks;

        unsafe {
            std::ptr::copy_nonoverlapping(
                string.as_ptr(),
                self.data.wrapping_byte_add(data_ptr),
                string.len(),
            )
        };

        GCString {
            ptr: data_ptr,
            len: string.len(),
        }
    }

    /// Gets string data from the GC heap.
    ///
    /// # Safety
    /// - `string` must point to a valid, currently alive string that was obtained from this GC store.
    /// - The object pointed to by `string` cannot be destroyed before the returned reference is dropped.
    pub unsafe fn get_string<'a>(&'a self, string: &'a GCString) -> &'a str {
        if let Some(string) = string.get_inline() {
            return string;
        }

        let ptr = self.data.wrapping_byte_add(string.ptr);
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, string.len)) }
    }
}

impl Drop for GCSpace {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.data,
                std::alloc::Layout::from_size_align_unchecked(
                    self.capacity * GarbageCollector::BLOCK_SIZE,
                    GarbageCollector::BLOCK_SIZE,
                ),
            )
        };
    }
}
