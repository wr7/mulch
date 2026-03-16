use std::{marker::PhantomData, num::NonZeroUsize};

use crate::gc::{
    GCSpace, GarbageCollector,
    util::{GCDebug, GCWrap},
};

/// A temporary buffer in a `GCSpace`. This may be used for temporary allocations or as a part of
/// another datastructure allocated in the `GCSpace`.
///
/// The difference between this and `GCVec` is that `GCVec` stores its length in the `GCHeap` and it
/// implements `GCPtr`.
pub struct GCBuffer<T> {
    ptr: NonZeroUsize,
    len: usize,
    _phantomdata: PhantomData<*mut T>,
}

impl<T> Clone for GCBuffer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GCBuffer<T> {}

impl<T> GCBuffer<T> {
    pub fn from_raw_parts(ptr: NonZeroUsize, len: usize) -> Self {
        Self {
            ptr,
            len,
            _phantomdata: PhantomData,
        }
    }
    pub fn allocation_size_blocks(len: usize) -> usize {
        (len * std::mem::size_of::<T>()).div_ceil(GarbageCollector::BLOCK_SIZE)
    }

    pub fn new_uninit(gc: &GarbageCollector, len_items: usize) -> Self {
        Self::new_uninit_in_space(&gc.from_space, len_items)
    }

    pub fn as_mut_ptr(self, gc: &GarbageCollector) -> *mut T {
        gc.from_space.block_ptr(self.ptr).cast::<T>()
    }

    pub fn as_ptr(self, gc: &GarbageCollector) -> *const T {
        self.as_mut_ptr(gc).cast_const()
    }

    pub fn as_ptr_in_space(self, space: &GCSpace) -> *const T {
        space.block_ptr(self.ptr).cast::<T>().cast_const()
    }

    pub fn as_mut_ptr_in_space(self, space: &GCSpace) -> *mut T {
        space.block_ptr(self.ptr).cast::<T>()
    }

    pub fn gc_ptr(self) -> NonZeroUsize {
        self.ptr
    }

    pub fn len(self) -> usize {
        self.len
    }

    pub unsafe fn as_slice(self, gc: &GarbageCollector) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.as_mut_ptr(gc), self.len()) }
    }

    pub fn element_ptr(self, gc: &GarbageCollector, index: usize) -> *mut T {
        debug_assert!(index < self.len);

        self.as_mut_ptr(gc).wrapping_add(index)
    }

    /// Sets the length of the `GCVec` given that it was the last object allocated in the garbage collector.
    pub(crate) unsafe fn set_length_at_end(&mut self, gc: &GarbageCollector, new_length: usize) {
        let cur_length = self.len();

        let cur_allocation_size = Self::allocation_size_blocks(cur_length);

        debug_assert_eq!(
            self.ptr.get() + cur_allocation_size,
            gc.from_space.len(),
            "set_length_at_end can only be used when the allocation is the last allocation on the GC heap"
        );

        if cur_length == new_length {
            return;
        }

        let new_allocation_size = Self::allocation_size_blocks(new_length);

        gc.from_space
            .set_len(gc.from_space.len() - cur_allocation_size + new_allocation_size);

        self.len = new_length;
    }

    /// Removes the current object from the GC heap. Requires that this object is the most recently-allocated object.
    pub(crate) unsafe fn deallocate_from_end(self, gc: &GarbageCollector) {
        let length = unsafe { self.as_slice(gc).len() };
        let allocation_size = Self::allocation_size_blocks(length);

        debug_assert_eq!(
            self.ptr.get() + allocation_size,
            gc.from_space.len(),
            "freeze_from_end can only be used when the allocation is the last allocation on the GC heap"
        );

        gc.from_space.set_len(gc.from_space.len() - allocation_size);
    }

    pub fn new_uninit_in_space(gcspace: &GCSpace, len: usize) -> Self {
        let allocation_size = Self::allocation_size_blocks(len);

        let ptr = gcspace.len();

        gcspace.set_len(gcspace.len() + allocation_size);
        Self::from_raw_parts(unsafe { NonZeroUsize::new_unchecked(ptr) }, len)
    }
}

impl<T: GCDebug> GCDebug for GCBuffer<T> {
    unsafe fn gc_debug(
        self,
        gc: &crate::gc::GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let ptr = gc.from_space.block_ptr(self.ptr).cast::<T>();
        let slice = unsafe { std::slice::from_raw_parts(ptr, self.len) };

        let mut debug_list = f.debug_list();

        for item in slice {
            debug_list.entry(&unsafe { GCWrap::new(*item, gc) });
        }

        debug_list.finish()
    }
}
