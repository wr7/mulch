use std::{marker::PhantomData, num::NonZeroUsize};

use crate::gc::{GCPtr, GCSpace, GarbageCollector};

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
pub struct GCVec<T: GCPtr> {
    ptr: NonZeroUsize,
    _phantomdata: PhantomData<Vec<T>>,
}

impl<T: GCPtr> GCVec<T> {
    pub fn ptr(self) -> usize {
        self.ptr.get()
    }

    pub unsafe fn as_slice(self, gc: &GarbageCollector) -> &[T] {
        let base_ptr = gc.from_space.block_ptr(self.ptr);

        let len = unsafe { base_ptr.cast::<usize>().read() };
        let ptr = base_ptr
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE)
            .cast::<T>();

        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl GCSpace {
    /// Allocates and initializes a garbage-collected dynamically-sized array
    /// # Safety
    /// All `elements` must point to valid, non-frozen objects in the current GC space.
    pub unsafe fn alloc_vec<T>(&mut self, elements: &[T]) -> GCVec<T>
    where
        T: GCPtr,
    {
        let vec = unsafe { self.alloc_uninit_vec(elements.len()) };
        let ptr = self
            .block_ptr(vec.ptr)
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE)
            .cast::<T>();

        unsafe { std::ptr::copy_nonoverlapping(elements.as_ptr(), ptr, elements.len()) }

        vec
    }

    /// Allocates an unitialized garbage-collected dynamically-sized array
    /// # Safety
    /// The `GCVec` must be fully initialized be fully initialized before it is moved to from-space
    pub unsafe fn alloc_uninit_vec<T>(&mut self, len: usize) -> GCVec<T>
    where
        T: GCPtr,
    {
        let allocation_size =
            1 + (len * std::mem::size_of::<T>()).div_ceil(GarbageCollector::BLOCK_SIZE);

        let ptr = self.len;

        self.expand(self.len + allocation_size);
        self.len += allocation_size;

        unsafe {
            // write element length to first block
            self.block_ptr(ptr).cast::<usize>().write(len);
        }

        GCVec {
            ptr: unsafe { NonZeroUsize::new_unchecked(ptr) },
            _phantomdata: PhantomData,
        }
    }

    /// Gets a pointer to the element at `index` in a `GCVec`.
    /// # Safety
    /// `vec` must be a valid, non-frozen `GCVec` in `Self`
    pub fn element_ptr_unchecked<T>(&self, vec: GCVec<T>, index: usize) -> *mut T
    where
        T: GCPtr,
    {
        let base_ptr = self.block_ptr(vec.ptr);

        let ptr = base_ptr
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE + index * std::mem::size_of::<T>())
            .cast::<T>();

        ptr
    }
}

impl<T> GCVec<T>
where
    T: GCPtr,
{
    unsafe fn get_forwarded_value(self, gc: &mut GarbageCollector) -> Option<Self> {
        let discriminant = unsafe { gc.from_space.block_ptr(self.ptr).cast::<usize>().read() };
        if discriminant & 1usize.rotate_right(1) == 0 {
            return None;
        }

        let ptr = discriminant & ((!0usize) >> 1);
        Some(Self {
            ptr: NonZeroUsize::new(ptr).unwrap(),
            _phantomdata: PhantomData,
        })
    }
}

unsafe impl<T> GCPtr for GCVec<T>
where
    T: GCPtr,
{
    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        if let Some(fwd) = unsafe { self.get_forwarded_value(gc) } {
            return fwd;
        }

        let from_base_ptr = gc.from_space.block_ptr(self.ptr);
        let len = unsafe { from_base_ptr.cast::<usize>().read() };

        // We must allocate the vec and write the forward pointer before copying the elements
        // because they may contain references to `self`
        let new_vec = unsafe { gc.to_space.alloc_uninit_vec::<T>(len) };
        let discriminant = new_vec.ptr | 1usize.rotate_right(1);
        unsafe { from_base_ptr.cast::<usize>().write(discriminant.get()) };

        for i in 0..len {
            let old_element = unsafe { gc.from_space.element_ptr_unchecked(self, i).read() };
            let new_element = unsafe { old_element.gc_copy(gc) };

            unsafe {
                gc.to_space
                    .element_ptr_unchecked(new_vec, i)
                    .write(new_element)
            };
        }

        new_vec
    }
}
