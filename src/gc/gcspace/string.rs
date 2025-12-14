use std::{num::NonZeroUsize, ptr::addr_of_mut};

use crate::gc::{
    GarbageCollector,
    gcspace::{GCObject, GCSpace},
};

/// A garbage collected string.
///
/// We cannot use regular heap-allocated strings because the fields of garbage collected objects are
/// never dropped.
///
/// # Forward pointer
/// A forward pointer is stored if `ptr` points to a `usize` with a most-significant-byte of `0xFF`.
/// The remaining bytes of the `usize` refer to its new location in to-space, and a `usize` stored
/// after it in memory contain its length in bytes.
///
/// # Inline strings
/// If the most significant bit of `ptr` is set, it is an "inline string". The next seven bits
/// indicate the length. The remainder of the `GCString` contains the object. On little endian
/// systems, the string data starts on the 0th byte, and on big endian platforms, it starts on the
/// 1th byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GCString {
    /// length (in bytes)
    #[cfg(target_endian = "little")]
    len: usize,
    /// pointer (in blocks)
    ptr: NonZeroUsize,
    /// length (in bytes)
    #[cfg(target_endian = "big")]
    len: usize,
}

impl GCString {
    /// Gets the string if it is less than 2 * sizeof(usize) bytes long. Otherwise returns `None`
    pub fn get_inline(&self) -> Option<&str> {
        if self.ptr.get() & (0b1 << usize::BITS - 1) == 0 {
            return None;
        }

        let len = (self.ptr.get() >> (usize::BITS - 8)) & 0b0111_1111;

        let ptr = std::ptr::from_ref(self).cast::<u8>();

        #[cfg(target_endian = "big")]
        let ptr = unsafe { ptr.byte_offset(1) };

        let string = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
        Some(string)
    }
}

impl GCSpace {
    /// Creates a garbage collected string without invoking a GC pass and WITHOUT CREATING A ROOT
    pub fn alloc_string(&mut self, string: &str) -> GCString {
        // If the string is small enough, it can be stored inline rather than on the GC Heap

        if string.len() < std::mem::size_of::<GCString>() && string.len() <= 127 {
            let discriminant: usize = (0b1000_0000 | string.len()) << (usize::BITS - 8); // The MSB being set signifies that the string is stored inline

            let mut retval = GCString {
                ptr: NonZeroUsize::new(discriminant).unwrap(),
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

        let data_ptr = self.len;
        self.len += num_blocks;

        unsafe {
            std::ptr::copy_nonoverlapping(string.as_ptr(), self.block_ptr(data_ptr), string.len())
        };

        GCString {
            ptr: unsafe { NonZeroUsize::new_unchecked(data_ptr) },
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

        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.block_ptr(string.ptr),
                string.len,
            ))
        }
    }
}

unsafe impl GCObject for GCString {
    unsafe fn get_forwarded_value(self, gc: &mut GarbageCollector) -> Option<Self> {
        if self.get_inline().is_some() {
            return Some(self);
        }

        let ptr = gc.from_space.block_ptr(self.ptr);
        let discriminant = unsafe { ptr.cast::<usize>().read() };

        // If the most-significant-byte is not `0xFF`, it is not a forwarded value.
        if discriminant >> (usize::BITS - 8) != 0xFF {
            return None;
        }

        let mask = (!0usize) >> 8;
        let forward = discriminant & mask;

        Some(Self {
            len: self.len,
            ptr: NonZeroUsize::new(forward).unwrap(),
        })
    }

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        if let Some(forward) = unsafe { self.get_forwarded_value(gc) } {
            return forward;
        }

        let to_value = gc
            .to_space
            .alloc_string(unsafe { gc.from_space.get_string(&self) });

        let forward = to_value.ptr.get() | 0xFF;
        unsafe {
            gc.from_space
                .block_ptr(self.ptr)
                .cast::<usize>()
                .write(forward);
        }

        to_value
    }
}
