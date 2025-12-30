use std::fmt::{self, Debug, Formatter};

use crate::gc::{GCPtr, GarbageCollector};

pub struct GCWrap<'gc, T> {
    inner: T,
    gc: &'gc GarbageCollector,
}

pub trait GCDebug: GCPtr {
    /// `Debug::fmt` method for garbage-collected objects.
    ///
    /// # Safety
    /// `self` must be a valid, non-frozen object in `gc`
    unsafe fn gc_debug(self, gc: &GarbageCollector, f: &mut Formatter) -> std::fmt::Result;
}

impl<'gc, T> Debug for GCWrap<'gc, T>
where
    T: GCDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { self.inner.gc_debug(self.gc, f) }
    }
}

impl<'gc, T> GCWrap<'gc, T> {
    pub unsafe fn new(inner: T, gc: &'gc GarbageCollector) -> Self {
        Self { inner, gc }
    }
}
