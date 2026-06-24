use std::{cell::UnsafeCell, marker::PhantomData, mem::ManuallyDrop, num::NonZeroUsize};

use crate::gc::{GCPtr, GarbageCollector};

pub use rootlist::GCRootList;

/// A smart reference to a garbage-collection root. When this is dropped, it will remove the
/// garbage-collector root associated with it. However, all garbage-collector roots must be removed
/// in the opposite order in which they were created. Otherwise, undefined behavior may occur.
///
/// This is created with [`GarbageCollector::push_root`].
pub struct UnsafeRootGuard<'gc, T: GCPtr> {
    gc: &'gc GarbageCollector,
    raw_ref: ManuallyDrop<GCRootRef<T>>,
}

impl<'gc, T: GCPtr> UnsafeRootGuard<'gc, T> {
    pub unsafe fn new(gc: &'gc GarbageCollector, value: T) -> Self {
        Self::from_raw(gc, unsafe { gc.push_root(value) })
    }

    pub fn from_raw(gc: &'gc GarbageCollector, raw: GCRootRef<T>) -> Self {
        Self {
            gc,
            raw_ref: ManuallyDrop::new(raw),
        }
    }

    pub fn into_raw(self) -> GCRootRef<T> {
        let retval = ManuallyDrop::into_inner(self.raw_ref.clone());

        std::mem::forget(self);

        retval
    }

    pub fn as_raw(&self) -> &GCRootRef<T> {
        &self.raw_ref
    }

    /// Gets the value of a GC root without removing it.
    pub unsafe fn get(&self) -> T {
        unsafe { self.raw_ref.get(self.gc) }
    }
}

impl<'gc, T: GCPtr> Drop for UnsafeRootGuard<'gc, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::<GCRootRef<T>>::take(&mut self.raw_ref).pop(self.gc);
        }
    }
}

mod rootlist {
    use super::*;

    pub struct GCRootList {
        roots: UnsafeCell<Vec<GCRootEntry>>,
    }

    impl GCRootList {
        pub fn new() -> Self {
            Self {
                roots: UnsafeCell::new(Vec::new()),
            }
        }

        pub fn get(&self, index: usize) -> GCRootEntry {
            let vec = unsafe { self.roots.get().as_ref_unchecked() };

            assert!(index < vec.len());

            unsafe { vec.as_ptr().add(index).read() }
        }

        pub unsafe fn get_unchecked(&self, index: usize) -> GCRootEntry {
            let vec = unsafe { self.roots.get().as_ref_unchecked() };

            debug_assert!(index < vec.len());

            unsafe { vec.as_ptr().add(index).read() }
        }

        // NOTE: all of this interior mutability is safe because we don't provide a safe way to get
        // a reference to any element.

        pub fn set(&self, index: usize, value: GCRootEntry) {
            unsafe { self.roots.get().as_mut_unchecked()[index] = value }
        }

        pub fn len(&self) -> usize {
            unsafe { self.roots.get().as_ref_unchecked().len() }
        }

        pub(super) fn push(&self, entry: GCRootEntry) {
            unsafe { self.roots.get().as_mut_unchecked().push(entry) };
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
pub(super) struct GCRootEntry {
    /// The function that the garbage collector calls to copy this entry. Its first argument is `data_ptr`.
    pub copy_fn: unsafe fn(NonZeroUsize, &GarbageCollector) -> NonZeroUsize,

    /// For most types, this is a `GCBox<Self>`. This data is determined by a type's `GCPtr`
    /// implementation.
    pub data_ptr: NonZeroUsize,

    /// The name of the type stored. This is only used for debug assertions.
    #[cfg(debug_assertions)]
    pub type_name: &'static str,
}

/// A raw reference to a GC root. The difference between this and a `GCRootGuard` is that a
/// `GCRootGuard` contains a reference to the garbage-collector and will automatically remove the GC
/// root when it falls out of scope.
///
/// This type, however, will panic when it's destructor is called and instead requires that its
/// [`pop`](GCRootRef::pop) or [`forget`](GCRootRef::forget) methods are called.
pub struct GCRootRef<T> {
    index: usize,
    _phantomdata: PhantomData<T>,
}

impl<T> Clone for GCRootRef<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            _phantomdata: PhantomData,
        }
    }
}

impl<T: GCPtr> GCRootRef<T> {
    /// Gets the value of a GC root without removing it.
    pub unsafe fn get(&self, gc: &GarbageCollector) -> T {
        debug_assert!(self.index < gc.roots.len());

        let entry = unsafe { gc.roots.get_unchecked(self.index) };

        #[cfg(debug_assertions)]
        assert_eq!(entry.type_name, core::any::type_name::<T>());

        unsafe { <T as GCPtr>::from_gc_root_entry(gc, entry) }
    }

    /// Removes a GC root and gets its value. GC roots should always be removed in the opposite
    /// order in which they were created.
    pub unsafe fn pop(self, gc: &GarbageCollector) -> T {
        let val = unsafe { self.get(gc) };

        debug_assert_eq!(gc.roots.len(), self.index + 1);
        gc.roots.remove_last_root();

        self.forget();

        val
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
        let entry = unsafe { root.to_gc_root_entry(self) };
        let index = self.roots.len();

        self.roots.push(entry);

        GCRootRef {
            index,
            _phantomdata: PhantomData,
        }
    }

    /// Removes a GC root along with all roots after it.
    pub fn truncate_roots<T: GCPtr>(&self, root: GCRootRef<T>) {
        #[cfg(debug_assertions)]
        assert_eq!(
            self.roots.get(root.index).type_name,
            core::any::type_name::<T>()
        );

        self.roots.truncate(root.index);
        root.forget();
    }
}
