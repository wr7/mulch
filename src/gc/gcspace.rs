use std::{alloc::Layout, cell::Cell};

use crate::gc::GarbageCollector;

pub struct GCSpace {
    data: Cell<*mut u8>,
    /// Currently occupied space (in blocks)
    len: Cell<usize>,
    /// Capacity (in blocks)
    capacity: Cell<usize>,
}

impl Default for GCSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl GCSpace {
    const STARTING_BLOCKS: usize = 128;

    pub(super) fn ptr(&self) -> *mut u8 {
        self.data.get()
    }
    pub(super) fn capacity(&self) -> usize {
        self.capacity.get()
    }
    pub(super) fn len(&self) -> usize {
        self.len.get()
    }

    /// Sets the length and increases the capacity if needed.
    pub(super) fn set_len(&self, len: usize) {
        self.expand_capacity_to(len);
        self.len.set(len);
    }

    fn set(&self, ptr: *mut u8, capacity: usize) {
        self.data.set(ptr);
        self.capacity.set(capacity);
    }

    /// Grows the allocation to be exactly `new_size_blocks` blocks. [`GCSpace::expand`] should be
    /// used instead whenever possible.
    pub fn expand_capacity_to_exact(&self, new_size_blocks: usize) {
        if new_size_blocks <= self.capacity() {
            return;
        }

        self.set(
            unsafe {
                std::alloc::realloc(
                    self.ptr(),
                    Layout::from_size_align_unchecked(
                        self.capacity() * GarbageCollector::BLOCK_SIZE,
                        GarbageCollector::BLOCK_SIZE,
                    ),
                    new_size_blocks * GarbageCollector::BLOCK_SIZE,
                )
            },
            new_size_blocks,
        );
    }

    /// Grows the allocation to be at least `new_size_blocks` blocks. NOTE: this will not increase
    /// its length.
    pub fn expand_capacity_to(&self, new_size_blocks: usize) {
        let mut new_exact_size_blocks = self.capacity();

        while new_exact_size_blocks < new_size_blocks {
            new_exact_size_blocks *= 2;
        }

        if new_exact_size_blocks == self.capacity() {
            return;
        }

        self.expand_capacity_to_exact(new_exact_size_blocks);
    }

    /// Clears the GCSpace. All objects in the space are "forgotten".
    pub fn clear(&mut self) {
        self.len.set(1);
    }

    pub fn new() -> Self {
        let data = unsafe {
            std::alloc::alloc(Layout::from_size_align_unchecked(
                Self::STARTING_BLOCKS * GarbageCollector::BLOCK_SIZE,
                GarbageCollector::BLOCK_SIZE,
            ))
        };

        Self {
            data: Cell::new(data),
            len: Cell::new(1), // We reserve the first block. This allows us to use `NonZeroUsize` for many of our datastructures.
            capacity: Cell::new(Self::STARTING_BLOCKS),
        }
    }

    /// Gets a pointer to the block at `idx`
    pub(super) fn block_ptr(&self, idx: impl Into<usize>) -> *mut u8 {
        self.ptr()
            .wrapping_byte_add(idx.into() * GarbageCollector::BLOCK_SIZE)
    }

    /// Swaps this `GCSpace` with another `GCSpace`. This should only be done as a part of a
    /// garbage-collection cycle.
    #[allow(unreachable_code)]
    pub(super) unsafe fn swap(&self, other: &GCSpace) {
        self.data.swap(&other.data);
        self.len.swap(&other.len);
        self.capacity.swap(&other.capacity);

        return;

        // This code is only here to prevent this function from compiling if new fields are added in
        // the future.
        #[allow(clippy::diverging_sub_expression)]
        let _ = Self {
            data: unreachable!(),
            len: unreachable!(),
            capacity: unreachable!(),
        };
    }
}

impl Drop for GCSpace {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.ptr(),
                std::alloc::Layout::from_size_align_unchecked(
                    self.capacity() * GarbageCollector::BLOCK_SIZE,
                    GarbageCollector::BLOCK_SIZE,
                ),
            )
        };
    }
}
