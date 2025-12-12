use std::{marker::PhantomData, num::NonZeroUsize, ptr::NonNull};

use crate::gc::{
    GarbageCollector,
    gcspace::{GCObject, GCSpace},
};

/// A garbage collected dynamically-sized array.
///
/// # Forward pointer
/// A forward pointer is stored if `ptr` points to a `usize` with its most-significant-bit set. The
/// remaining bits of the usize indicate the new pointer in from-space.
///
/// # Memory layout
/// `ptr` points to a `usize` which contains the length of the string. An array of `T` elements
/// starts in the following block.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct GCVec<T: GCObject> {
    ptr: NonZeroUsize,
    _phantomdata: PhantomData<Vec<T>>,
}

impl GCSpace {
    /// Allocates and initializes a garbage-collected dynamically-sized array
    /// # Safety
    /// All `elements` must point to valid, non-frozen objects in the current GC space.
    pub unsafe fn alloc_vec<T>(&mut self, elements: &[T]) -> GCVec<T>
    where
        T: GCObject,
    {
        let vec = unsafe { self.alloc_uninit_vec(elements.len()) };
        let ptr = self
            .data
            .wrapping_byte_add((vec.ptr.get() + 1) * GarbageCollector::BLOCK_SIZE)
            .cast::<T>();

        unsafe { std::ptr::copy_nonoverlapping(elements.as_ptr(), ptr, elements.len()) }

        vec
    }

    /// Allocates an unitialized garbage-collected dynamically-sized array
    /// # Safety
    /// The `GCVec` must be fully initialized be fully initialized before it is moved to from-space
    pub unsafe fn alloc_uninit_vec<T>(&mut self, len: usize) -> GCVec<T>
    where
        T: GCObject,
    {
        let allocation_size =
            1 + (len * std::mem::size_of::<T>()).div_ceil(GarbageCollector::BLOCK_SIZE);

        let ptr = self.len;

        self.expand(self.len + allocation_size);
        self.len += allocation_size;

        unsafe {
            // write element length to first block
            self.ptr_at_mut(ptr).cast::<usize>().write(len);
        }

        GCVec {
            ptr: unsafe { NonZeroUsize::new_unchecked(ptr) },
            _phantomdata: PhantomData,
        }
    }

    /// Gets a pointer to the element at `index` in a `GCVec`. Returns `None` if `index` is out-
    /// of-range.
    /// # Safety
    /// `vec` must be a valid, non-frozen `GCVec` in `Self`
    pub unsafe fn element_ptr<T>(&self, vec: GCVec<T>, index: usize) -> Option<NonNull<T>>
    where
        T: GCObject,
    {
        let len_ptr = self
            .data
            .wrapping_byte_add(vec.ptr.get() * GarbageCollector::BLOCK_SIZE);

        let len = unsafe { len_ptr.cast::<usize>().read() };
        if index <= len {
            return None;
        }

        let ptr = len_ptr
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE + index * std::mem::size_of::<T>())
            .cast::<T>();

        NonNull::new(ptr)
    }
}
