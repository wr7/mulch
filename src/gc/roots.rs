use std::{cell::UnsafeCell, marker::PhantomData, num::NonZeroUsize};

use crate::gc::{GCPtr, GarbageCollector};

pub use rootlist::GCRootList;

mod rootlist {
    use super::*;

    pub struct GCRootList {
        roots: UnsafeCell<Vec<Option<GCRootEntry>>>,
    }

    impl GCRootList {
        pub fn new() -> Self {
            Self {
                roots: UnsafeCell::new(Vec::new()),
            }
        }

        pub fn get(&self, index: usize) -> Option<GCRootEntry> {
            let vec = unsafe { self.roots.get().as_ref_unchecked() };

            assert!(index < vec.len());

            unsafe { vec.as_ptr().add(index).read() }
        }

        pub unsafe fn get_unchecked(&self, index: usize) -> Option<GCRootEntry> {
            let vec = unsafe { self.roots.get().as_ref_unchecked() };

            debug_assert!(index < vec.len());

            unsafe { vec.as_ptr().add(index).read() }
        }

        // NOTE: all of this interior mutability is safe because we don't provide a safe way to get
        // a reference to any element.

        pub fn set(&self, index: usize, value: Option<GCRootEntry>) {
            unsafe { self.roots.get().as_mut_unchecked()[index] = value }
        }

        pub fn len(&self) -> usize {
            unsafe { self.roots.get().as_ref_unchecked().len() }
        }

        pub(super) fn push(&self, entry: GCRootEntry) {
            unsafe { self.roots.get().as_mut_unchecked().push(Some(entry)) };
        }

        pub(super) fn remove_last_root(&self) {
            unsafe { self.roots.get().as_mut_unchecked().pop() };
        }

        pub(super) fn truncate(&self, val: usize) {
            debug_assert!(val < self.len());
            unsafe { self.roots.get().as_mut_unchecked().truncate(val) };
        }
    }
}

#[derive(Clone, Copy)]
pub struct GCRootInfo {
    /// The function that the garbage collector calls to copy this entry. Its first argument is `data_ptr`.
    pub(crate) copy_fn: unsafe fn(NonZeroUsize, &GarbageCollector) -> NonZeroUsize,

    /// For most types, this is a `GCBox<Self>`. This data is determined by a type's `GCPtr`
    /// implementation.
    pub(crate) data_ptr: NonZeroUsize,
}

#[derive(Clone, Copy)]
pub(super) struct GCRootEntry {
    /// The function that the garbage collector calls to copy this entry. Its first argument is `data_ptr`.
    pub(crate) copy_fn: unsafe fn(NonZeroUsize, &GarbageCollector) -> NonZeroUsize,

    /// For most types, this is a `GCBox<Self>`. This data is determined by a type's `GCPtr`
    /// implementation.
    pub(crate) data_ptr: NonZeroUsize,

    /// The name of the type stored. This is only used for debug assertions.
    #[cfg(debug_assertions)]
    pub(crate) type_name: &'static str,
}

/// A raw reference to a GC root.
///
/// This type, however, will panic when it's destructor is called and instead requires that its
/// [`free`](GCRootRef::free) or [`forget`](GCRootRef::forget) methods are called.
pub struct GCRootRef<T> {
    index: usize,
    _phantomdata: PhantomData<T>,
}

impl<T: GCPtr> GCRootRef<T> {
    /// Gets the value of a GC root without removing it.
    ///
    /// # Safety
    /// - `self` must point to a valid root in `gc`
    pub unsafe fn get(&self, gc: &GarbageCollector) -> Option<T> {
        debug_assert!(self.index < gc.roots.len());

        let entry = unsafe { gc.roots.get_unchecked(self.index) }?;

        #[cfg(debug_assertions)]
        assert_eq!(entry.type_name, core::any::type_name::<T>());

        Some(unsafe { <T as GCPtr>::from_gc_root_entry(gc, entry.data_ptr) })
    }

    /// Frees a GC root. GC roots should always be removed in the opposite
    /// order in which they were created.
    ///
    /// # Safety
    /// - `self` must point to a valid root in `gc`
    pub unsafe fn free(self, gc: &GarbageCollector) {
        assert_eq!(gc.roots.len(), self.index + 1);
        gc.roots.remove_last_root();

        self.forget();
    }

    /// Removes a GC root without actually removing it from the GC root stack. When
    /// [`GCRootRef::get`] is called on it, `None` is returned.
    ///
    /// This is useful for removing GC roots in a non stack-like order. Notably, this does
    /// not "free" the root. The root must still be freed using [`GCRootRef::free`] or
    /// [`GarbageCollector::truncate_roots`].
    ///
    /// # Safety
    /// - `self` must point to a valid root in `gc`
    pub unsafe fn remove(&self, gc: &GarbageCollector) {
        gc.roots.set(self.index, None);
    }

    /// Destroys a `GCRootRef` without removing the root that it points to. This is useful for
    /// if you need to clone a `GCRootRef` (which would cause UB if you `pop`ped both of the
    /// references).
    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl<T> Drop for GCRootRef<T> {
    fn drop(&mut self) {
        panic!(
            "GC root references should not be dropped. You should either call `GCRootRef::pop` or (rarely) `GCRoot::forget`."
        )
    }
}

impl GarbageCollector {
    /// Adds an object as a garbage-collection root. This may create another allocation on the GC
    /// heap.
    ///
    /// # Safety
    /// All garbage-collector roots must be removed in the opposite order in which they were
    /// created.
    #[must_use = "The GC root should be used and released at some point"]
    pub unsafe fn push_root<T: GCPtr>(&self, root: T) -> GCRootRef<T> {
        let info = unsafe { root.to_gc_root_entry(self) };
        let index = self.roots.len();

        let entry = GCRootEntry {
            copy_fn: info.copy_fn,
            data_ptr: info.data_ptr,

            #[cfg(debug_assertions)]
            type_name: ::core::any::type_name::<T>(),
        };

        self.roots.push(entry);

        GCRootRef {
            index,
            _phantomdata: PhantomData,
        }
    }

    /// Removes a GC root along with all roots after it.
    ///
    /// # Safety
    /// - The caller must ensure that none of the roots that were created after this one are in use.
    pub unsafe fn truncate_roots<T: GCPtr>(&self, root: GCRootRef<T>) {
        #[cfg(debug_assertions)]
        if let Some(entry) = self.roots.get(root.index) {
            assert_eq!(entry.type_name, core::any::type_name::<T>());
        }

        self.roots.truncate(root.index);
        root.forget();
    }
}
