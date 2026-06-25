use std::{fmt::Formatter, num::NonZeroUsize};

use crate::gc::{GCBox, GCRootEntry, GarbageCollector, util::GCWrap};

/// Represents a pointer to a garbage-collectable object.
///
/// # Safety
/// - The alignment of `Self` must be less than or equal to `GarbageCollector::BLOCK_SIZE`
pub unsafe trait GCPtr: Sized + Clone {
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
    unsafe fn gc_copy(self, gc: &GarbageCollector) -> Self;

    /// Wraps `self` with a reference to the garbage collector. This wrapper may implement `Debug`,
    /// `PartialEq`, and similar traits.
    ///
    /// # Safety
    /// - This object must not be used if (or when) `self` is frozen or invalid.
    unsafe fn wrap<'a>(&'a self, gc: &'a GarbageCollector) -> GCWrap<'a, Self> {
        unsafe { GCWrap::new(self, gc) }
    }

    /// Creates a [`GCRootEntry`] object. This method is only overwritten for certain gc primitives.
    ///
    /// This method should not be called directly. [`root`](crate::gc::safety::root) or
    /// [`GCRootGuard::new`](crate::gc::safety::GCRootGuard::new) should be used instead.
    unsafe fn to_gc_root_entry(self, gc: &GarbageCollector) -> GCRootEntry {
        unsafe fn copy_fn<Self_: GCPtr>(data: NonZeroUsize, gc: &GarbageCollector) -> NonZeroUsize {
            let old_box = GCBox::<Self_>::from_ptr(data);
            let new_box = unsafe { GCPtr::gc_copy(old_box, gc) };

            new_box.ptr()
        }

        let data = unsafe { GCBox::<Self>::new(gc, self) };

        GCRootEntry {
            copy_fn: copy_fn::<Self>,
            data_ptr: data.ptr(),
            #[cfg(debug_assertions)]
            type_name: std::any::type_name::<Self>(),
        }
    }

    /// Retrieves an instance of `Self` from a [`GCRootEntry`]. This method is only overwritten for certain gc primitives.
    ///
    /// This method should not be called directly.
    /// [`GCRootGuard::get`](crate::gc::safety::GCRootGuard::get) should be used instead.
    unsafe fn from_gc_root_entry(gc: &GarbageCollector, entry: GCRootEntry) -> Self {
        #[cfg(debug_assertions)]
        assert_eq!(entry.type_name, std::any::type_name::<Self>());

        let box_ = GCBox::<Self>::from_ptr(entry.data_ptr);
        unsafe { box_.get(gc) }
    }
}

pub trait GCDebug: Clone {
    /// `Debug::fmt` method for garbage-collected objects.
    ///
    /// # Safety
    /// `self` must be a valid, non-frozen object in `gc`
    unsafe fn gc_debug(&self, gc: &GarbageCollector, f: &mut Formatter) -> std::fmt::Result;
}

pub trait GCEq<Rhs: ?Sized = Self> {
    /// Compares a garbage-collected value with a non-garbage collected value or a wrapped value.
    ///
    /// # Safety
    /// - `inner` must be valid and alive in `gc`
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &Rhs) -> bool;

    /// Compares a garbage-collected value with a non-garbage collected value or a wrapped value.
    ///
    /// # Safety
    /// - `inner` must be valid and alive in `gc`
    unsafe fn gc_ne(&self, gc: &GarbageCollector, rhs: &Rhs) -> bool {
        !unsafe { self.gc_eq(gc, rhs) }
    }
}

pub trait GCGet {
    type Borrowed: ?Sized;
    /// Gets the data pointed to by `self`
    ///
    /// # Safety
    /// - `self` must be valid and alive in `gc`
    unsafe fn get<'a>(&'a self, gc: &'a GarbageCollector) -> &'a Self::Borrowed;
}

/// An object that does not contain a garbage-collected object.
pub unsafe trait NonGC {}
