use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
};

use crate::gc::GarbageCollector;

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

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub unsafe fn map<U>(self, func: impl FnOnce(T) -> U) -> GCWrap<'gc, U> {
        GCWrap {
            inner: func(self.inner),
            gc: self.gc,
        }
    }

    pub fn gc<'a>(&'a self) -> &'gc GarbageCollector {
        self.gc
    }
}

pub trait GCDebug: Copy {
    /// `Debug::fmt` method for garbage-collected objects.
    ///
    /// # Safety
    /// `self` must be a valid, non-frozen object in `gc`
    unsafe fn gc_debug(self, gc: &GarbageCollector, f: &mut Formatter) -> std::fmt::Result;
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

impl<'gc, T> Debug for GCWrap<'gc, T>
where
    T: GCDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { self.inner.gc_debug(self.gc, f) }
    }
}

impl<'gc, T, Rhs> PartialEq<GCWrap<'gc, Rhs>> for GCWrap<'gc, T>
where
    T: GCEq<Rhs>,
{
    fn eq(&self, rhs: &GCWrap<'gc, Rhs>) -> bool {
        assert!(self.gc as *const GarbageCollector == rhs.gc as *const GarbageCollector);

        unsafe { self.inner.gc_eq(self.gc(), &rhs.inner) }
    }
}

impl<'gc, T, Rhs> PartialEq<Rhs> for GCWrap<'gc, T>
where
    T: GCEq<Rhs>,
    Rhs: NonGC + ?Sized,
{
    fn eq(&self, rhs: &Rhs) -> bool {
        unsafe { self.inner.gc_eq(self.gc, rhs) }
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
