use std::{marker::PhantomData, mem, num::NonZeroUsize};

use crate::gc::{
    GCPtr, GCSpace, GarbageCollector,
    util::{GCDebug, GCEq, GCGet},
};

/// Analogous to `std::boxed::Box`. This can be useful for recursively-defined datastructures.
/// # Memory layout in GCSpace
/// An instance of `T` is stored at `ptr`.
///
/// If `T::MSB_RESERVED` is set, `ptr` can be reinterpereted as a `usize` where the MSB being set
/// indicates a forward. In such case, the remaining bits store the forward pointer for the `GCBox`.
///
/// Otherwise, the first block after the instance of `T` should be interpereted as a `usize` with
/// the same properties as the one above.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct GCBox<T: GCPtr> {
    ptr: NonZeroUsize,
    _phantomdata: PhantomData<Box<T>>,
}

impl<T: GCPtr> GCBox<T> {
    /// Allocates a new `GCBox`
    ///
    /// This will never trigger a garbage-collection cycle.
    ///
    /// # Safety
    /// - `value` must point to a valid, non-frozen object in `gc`
    pub unsafe fn new(gc: &mut GarbageCollector, value: T) -> Self {
        let ptr = Self::alloc_uninit_in_space(&mut gc.from_space);
        unsafe { ptr.ptr_in_space(&gc.from_space).write(value) };

        ptr
    }

    /// Reads the value stored in the `GCBox`
    ///
    /// # Safety
    /// - `self` must point to a valid, non-frozen `GCBox<T>` in `gc`
    /// - This cannot be used during a GC cycle
    pub unsafe fn get(self, gc: &GarbageCollector) -> T {
        unsafe { self.ptr_in_space(&gc.from_space).read() }
    }

    /// Allocates an uninitialized `GCBox` in a given `GCSpace`
    fn alloc_uninit_in_space(space: &mut GCSpace) -> Self {
        let ptr = space.len;

        let mut size_blocks = mem::size_of::<T>().div_ceil(GarbageCollector::BLOCK_SIZE);

        if !T::MSB_RESERVED {
            size_blocks += 1; // An additional block must be used to keep track of whether the value is a forward pointer.
        }

        space.expand(space.len + size_blocks);
        space.len += size_blocks;

        if !T::MSB_RESERVED {
            // Zero the forward pointer of the newly allocated object
            unsafe {
                space
                    .block_ptr(ptr + mem::size_of::<T>().div_ceil(GarbageCollector::BLOCK_SIZE))
                    .cast::<usize>()
                    .write(0usize)
            };
        }

        Self {
            ptr: unsafe { NonZeroUsize::new_unchecked(ptr) },
            _phantomdata: PhantomData,
        }
    }

    fn ptr_in_space(self, space: &GCSpace) -> *mut T {
        space.block_ptr(self.ptr).cast::<T>()
    }

    unsafe fn get_forwarded_value(self, space: &GCSpace) -> Option<Self> {
        let fwd = if T::MSB_RESERVED {
            self.ptr.get()
        } else {
            self.ptr.get() + mem::size_of::<T>().div_ceil(GarbageCollector::BLOCK_SIZE)
        };

        let fwd = unsafe { space.block_ptr(fwd).cast::<usize>().read() };

        if fwd & 1usize.rotate_right(1) == 0 {
            return None;
        }

        let fwd = fwd & !1usize.rotate_right(1);

        Some(Self {
            ptr: NonZeroUsize::new(fwd).unwrap(),
            _phantomdata: PhantomData,
        })
    }
}

unsafe impl<T: GCPtr> GCPtr for GCBox<T> {
    const MSB_RESERVED: bool = true;

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        if let Some(fwd) = unsafe { self.get_forwarded_value(&gc.from_space) } {
            return fwd;
        }

        let old_value = unsafe { self.ptr_in_space(&gc.from_space).read() };

        let new_box = Self::alloc_uninit_in_space(&mut gc.to_space);

        let fwd_storage_ptr = if T::MSB_RESERVED {
            self.ptr.get()
        } else {
            self.ptr.get() + mem::size_of::<T>().div_ceil(GarbageCollector::BLOCK_SIZE)
        };

        unsafe {
            gc.from_space
                .block_ptr(fwd_storage_ptr)
                .cast::<usize>()
                .write(new_box.ptr.get() | 1usize.rotate_right(1))
        };

        let new_value = unsafe { old_value.gc_copy(gc) };
        unsafe { new_box.ptr_in_space(&gc.to_space).write(new_value) };

        new_box
    }
}

impl<T: GCDebug> GCDebug for GCBox<T> {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        unsafe { self.get(gc).gc_debug(gc, f) }
    }
}

impl<T: GCPtr> GCGet for GCBox<T> {
    type Borrowed = T;

    unsafe fn get<'a>(&'a self, gc: &'a GarbageCollector) -> &'a Self::Borrowed {
        unsafe { &*self.ptr_in_space(&gc.from_space) }
    }
}

impl<T, Rhs> GCEq<Rhs> for GCBox<T>
where
    T: GCEq<Rhs>,
{
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &Rhs) -> bool {
        unsafe { self.get(gc).gc_eq(gc, rhs) }
    }
}
