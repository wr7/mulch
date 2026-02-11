#![allow(unused)]

use std::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    num::NonZeroUsize,
    ptr::{NonNull, addr_of_mut},
};

use crate::gc::{GarbageCollector, util::GCWrap};

pub struct GCSpace {
    data: Cell<*mut u8>,
    /// Currently occupied space (in blocks)
    len: Cell<usize>,
    /// Capacity (in blocks)
    capacity: Cell<usize>,
}

/// Represents a pointer to a garbage-collectable object.
///
/// # Safety
/// - The alignment of `Self` must be less than or equal to `GarbageCollector::BLOCK_SIZE`
pub unsafe trait GCPtr: Sized + Clone + Copy {
    /// Can be set to true if the following conditions are met:
    /// - The most-significant-bit of the first `usize` in `self` is reserved and always `0`.
    /// - `align_of::<Self>() >= align_of::<usize>()`
    ///
    /// These properties can be used to save some memory with `GCBox<Self>`. Otherwise, an
    /// additional block is used to keep track of whether or not the current value is a forward.
    const MSB_RESERVED: bool;

    /// Copies `self` into `to-space` and leaves behind a forward pointer. Returns a pointer to the
    /// new object in `to-space`.
    ///
    /// When implementing this method, make sure that any references contained in the object are
    /// copied AFTER a forward for the current object is created. Otherwise, there will be issues if
    /// the subreferences directly or indirectly refer back to `Self`.
    ///
    /// # Safety
    /// `self` must be a valid, currently-alive value in `from-space`.
    #[must_use]
    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self;

    /// Wraps `self` with a reference to the garbage collector. This wrapper may implement `Debug`,
    /// `PartialEq`, and similar traits.
    ///
    /// # Safety
    /// - This object must not be used if (or when) `self` is frozen or invalid.
    unsafe fn wrap<'gc>(self, gc: &'gc GarbageCollector) -> GCWrap<'gc, Self> {
        unsafe { GCWrap::new(self, gc) }
    }
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
        self.expand_to(len);
        self.len.set(len);
    }

    fn set(&self, ptr: *mut u8, capacity: usize) {
        self.data.set(ptr);
        self.capacity.set(capacity);
    }

    /// Grows the allocation to be exactly `new_size_blocks` blocks. [`GCSpace::expand`] should be
    /// used instead whenever possible.
    pub fn expand_exact(&self, new_size_blocks: usize) {
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
    pub fn expand_to(&self, new_size_blocks: usize) {
        let mut new_size = self.capacity();

        while new_size < new_size_blocks {
            new_size *= 2;
        }

        if new_size == self.capacity() {
            return;
        }

        self.expand_exact(new_size);
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
