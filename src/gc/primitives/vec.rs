use std::{marker::PhantomData, num::NonZeroUsize};

use crate::gc::{
    GCPtr, GCSpace, GarbageCollector,
    util::{GCDebug, GCEq, GCGet, GCWrap},
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
pub struct GCVec<T: GCPtr> {
    ptr: NonZeroUsize,
    _phantomdata: PhantomData<Vec<T>>,
}

impl<T: GCPtr> GCVec<T> {
    pub unsafe fn new(gc: &mut GarbageCollector, elements: &[T]) -> Self {
        let vec = unsafe { Self::new_uninit_in_space(&mut gc.from_space, elements.len()) };
        let ptr = gc
            .from_space
            .block_ptr(vec.ptr)
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE)
            .cast::<T>();

        unsafe { std::ptr::copy_nonoverlapping(elements.as_ptr(), ptr, elements.len()) }

        vec
    }

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

    /// Allocates an unitialized garbage-collected dynamically-sized array
    /// # Safety
    /// The `GCVec` must be fully initialized be fully initialized before it is moved to from-space
    unsafe fn new_uninit_in_space(space: &mut GCSpace, len: usize) -> Self {
        let allocation_size =
            1 + (len * std::mem::size_of::<T>()).div_ceil(GarbageCollector::BLOCK_SIZE);

        let ptr = space.len;

        space.expand(space.len + allocation_size);
        space.len += allocation_size;

        unsafe {
            // write element length to first block
            space.block_ptr(ptr).cast::<usize>().write(len);
        }

        GCVec {
            ptr: unsafe { NonZeroUsize::new_unchecked(ptr) },
            _phantomdata: PhantomData,
        }
    }

    /// Gets a pointer to the element at `index` in a `GCVec`.
    /// # Safety
    /// `vec` must be a valid, non-frozen `GCVec` in `Self`
    unsafe fn element_ptr_in_space(self, space: &GCSpace, index: usize) -> *mut T {
        let base_ptr = space.block_ptr(self.ptr);

        let ptr = base_ptr
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE + index * std::mem::size_of::<T>())
            .cast::<T>();

        ptr
    }

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
    const MSB_RESERVED: bool = true;

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        if let Some(fwd) = unsafe { self.get_forwarded_value(gc) } {
            return fwd;
        }

        let from_base_ptr = gc.from_space.block_ptr(self.ptr);
        let len = unsafe { from_base_ptr.cast::<usize>().read() };

        // We must allocate the vec and write the forward pointer before copying the elements
        // because they may contain references to `self`
        let new_vec = unsafe { Self::new_uninit_in_space(&mut gc.to_space, len) };
        let discriminant = new_vec.ptr | 1usize.rotate_right(1);
        unsafe { from_base_ptr.cast::<usize>().write(discriminant.get()) };

        for i in 0..len {
            let old_element = unsafe { self.element_ptr_in_space(&gc.from_space, i).read() };
            let new_element = unsafe { old_element.gc_copy(gc) };

            unsafe {
                new_vec
                    .element_ptr_in_space(&gc.to_space, i)
                    .write(new_element)
            };
        }

        new_vec
    }
}

impl<T: GCDebug> GCDebug for GCVec<T> {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let mut debug_list = f.debug_list();

        for el in unsafe { self.as_slice(gc) } {
            let el = unsafe { el.wrap(gc) };
            debug_list.entry(&el);
        }

        debug_list.finish()
    }
}

impl<T: GCPtr> GCGet for GCVec<T> {
    type Borrowed = [T];

    unsafe fn get<'a>(&'a self, gc: &'a GarbageCollector) -> &'a Self::Borrowed {
        unsafe { self.as_slice(gc) }
    }
}

impl<T, Rhs> GCEq<[Rhs]> for GCVec<T>
where
    T: GCEq<Rhs>,
{
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &[Rhs]) -> bool {
        let slice = unsafe { self.as_slice(gc) };

        if slice.len() != rhs.len() {
            return false;
        }

        slice
            .iter()
            .zip(rhs.iter())
            .all(|(a, b)| unsafe { a.gc_eq(gc, b) })
    }
}

impl<'gc, T, Rhs> GCEq<GCWrap<'gc, GCVec<Rhs>>> for GCVec<T>
where
    T: GCEq<GCWrap<'gc, Rhs>>,
    Rhs: GCPtr,
{
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &GCWrap<'gc, GCVec<Rhs>>) -> bool {
        let slice_lhs = unsafe { self.get(gc) };
        let slice_rhs = rhs.get();

        if slice_lhs.len() != slice_rhs.len() {
            return false;
        }

        slice_lhs
            .iter()
            .zip(slice_rhs.iter())
            .all(|(a, b)| unsafe { a.gc_eq(gc, &b.wrap(rhs.gc_ref())) })
    }
}

impl<T, Rhs> GCEq<GCVec<Rhs>> for GCVec<T>
where
    T: GCEq<Rhs>,
    Rhs: GCPtr,
{
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &GCVec<Rhs>) -> bool {
        unsafe { self.gc_eq(gc, rhs.get(gc)) }
    }
}
