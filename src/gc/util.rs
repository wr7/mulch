use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
};

use crate::gc::{GCDebug, GCEq, GCGet, GarbageCollector, NonGC};

#[derive(Clone, Copy)]
pub struct GCWrap<'a, T> {
    inner: T,
    gc: &'a GarbageCollector,
}

impl<'a, T: Clone> GCWrap<'a, T> {
    /// Wraps a GC object with a reference to the GarbageCollector.
    ///
    /// # Safety
    /// - `inner` must be valid and alive in `gc`
    pub unsafe fn new(inner: &'a T, gc: &'a GarbageCollector) -> Self {
        Self {
            inner: inner.clone(),
            gc,
        }
    }
}

impl<'a, T> GCWrap<'a, T> {
    /// Wraps a GC object with a reference to the GarbageCollector.
    ///
    /// # Safety
    /// - `inner` must be valid and alive in `gc`
    pub unsafe fn from_value(inner: T, gc: &'a GarbageCollector) -> Self {
        Self { inner, gc }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn gc<'b>(&'b self) -> &'a GarbageCollector {
        self.gc
    }
}

impl<'gc, T> Debug for GCWrap<'gc, T>
where
    T: GCDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { self.inner.clone().gc_debug(self.gc, f) }
    }
}

impl<'gc, T, Rhs> PartialEq<GCWrap<'gc, Rhs>> for GCWrap<'gc, T>
where
    T: GCEq<Rhs>,
{
    fn eq(&self, rhs: &GCWrap<'gc, Rhs>) -> bool {
        assert!(std::ptr::eq(self.gc, rhs.gc));

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
