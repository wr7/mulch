use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
};

use crate::gc::{GCPtr, GarbageCollector};

#[derive(Clone, Copy)]
pub struct GCWrap<'gc, T> {
    inner: T,
    gc: &'gc GarbageCollector,
}

impl<'gc, T> GCWrap<'gc, T> {
    /// Wraps a GC object with a reference to the GarbageCollector.
    ///
    /// # Safety
    /// - `inner` must be valid and alive in `gc`
    pub unsafe fn new(inner: T, gc: &'gc GarbageCollector) -> Self {
        Self { inner, gc }
    }

    pub fn gc_ref<'a>(&'a self) -> &'gc GarbageCollector {
        self.gc
    }
}

pub trait GCDebug: GCPtr {
    /// `Debug::fmt` method for garbage-collected objects.
    ///
    /// # Safety
    /// `self` must be a valid, non-frozen object in `gc`
    unsafe fn gc_debug(self, gc: &GarbageCollector, f: &mut Formatter) -> std::fmt::Result;
}

pub trait GCEq<Rhs: ?Sized>: GCPtr {
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

pub trait GCGet: GCPtr {
    type Borrowed: ?Sized;
    /// Gets the data pointed to by `self`
    ///
    /// # Safety
    /// - `self` must be valid and alive in `gc`
    unsafe fn get<'a>(&'a self, gc: &'a GarbageCollector) -> &'a Self::Borrowed;
}

impl<'gc, T> Debug for GCWrap<'gc, T>
where
    T: GCDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { self.inner.gc_debug(self.gc, f) }
    }
}

impl<'gc, T, Rhs: ?Sized> PartialEq<Rhs> for GCWrap<'gc, T>
where
    T: GCEq<Rhs>,
{
    fn eq(&self, other: &Rhs) -> bool {
        unsafe { self.inner.gc_eq(self.gc, other) }
    }
}

impl<'gc, T: GCGet> Deref for GCWrap<'gc, T> {
    type Target = T::Borrowed;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'gc, T: GCGet> GCWrap<'gc, T> {
    pub fn get(&self) -> &T::Borrowed {
        unsafe { self.inner.get(self.gc) }
    }
}
