#![allow(unused)]

use std::{
    alloc::Layout,
    marker::PhantomData,
    num::NonZeroUsize,
    ptr::{NonNull, addr_of_mut},
};

mod string;
mod vec;

use crate::gc::GarbageCollector;

pub use string::GCString;
pub use vec::GCVec;

pub struct GCSpace {
    data: *mut u8,
    /// Currently occupied space (in blocks)
    len: usize,
    /// Capacity (in blocks)
    capacity: usize,
}

/// Represents a garbage-collectable object.
///
/// # Safety
/// - The alignment of `Self` must be less than or equal to `GarbageCollector::BLOCK_SIZE`
pub unsafe trait GCObject: Sized + Clone + Copy {
    unsafe fn get_forwarded_value(gc: &GarbageCollector, ptr: Self) -> Option<Self>;
    unsafe fn gc_copy(gc: &GarbageCollector, ptr: Self) -> Self;
}

impl GCSpace {
    const STARTING_BLOCKS: usize = 64;

    fn ptr_at_mut(&self, block_idx: usize) -> *mut u8 {
        self.data
            .wrapping_byte_add(block_idx * GarbageCollector::BLOCK_SIZE)
    }

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

    /// Clears the GCSpace. All objects in the space are "forgotten".
    pub fn clear(&mut self) {
        self.len = 1; // We must reserve the first block
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
            len: 1, // We reserve the first block. This allows us to use `NonZeroUsize` for many of our datastructures.
            capacity: Self::STARTING_BLOCKS,
        }
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
