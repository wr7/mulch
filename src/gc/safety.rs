//! Provides a safe API for interfacing with the garbage collector

use core::fmt;
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomPinned,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
};

use crate::gc::{GCDebug, GCGet, GCProject, GCPtr, GCRootRef, GarbageCollector};

/// Creates a garbage-collector object and a context object.
///
/// This is the only safe way to create a context.
#[allow(unused)]
macro_rules! let_gc_and_context {
    ($gc_name:ident, $ctx_name:ident) => {
        let $gc_name = $crate::gc::GarbageCollector::new();

        #[allow(unused_mut)]
        let mut $ctx_name = unsafe { $crate::gc::safety::GCCtx::new(&$gc_name) };

        let $ctx_name = &mut $ctx_name;
    };
}

#[allow(unused)]
pub(crate) use let_gc_and_context;

/// The garbage collection context. This is used for the safe garbage collection API. If a function
/// takes in a mutable reference to the context, it can trigger a garbage-collection cycle. If a
/// safe function takes in an immutable reference, it cannot.
pub struct GCCtx<'gc> {
    gc: &'gc GarbageCollector,
}

impl<'gc> GCCtx<'gc> {
    /// Creates a garbage collection context for the safe garbage collection API.
    ///
    /// # Safety
    /// - Only one context should exist for a given garbage collector at any time.
    pub unsafe fn new(gc: &'gc GarbageCollector) -> Self {
        Self { gc }
    }

    /// Forcefully performs a garbage collection cycle.
    ///
    /// This only makes sense to use for testing purposes.
    pub fn force_collect(&mut self) {
        unsafe {
            self.gc.force_collect();
        }
    }

    /// Performs a garbage collection cycle if it makes sense to at the given moment.
    pub fn collect(&mut self) {
        unsafe {
            self.gc.collect();
        }
    }
}

impl<'gc> Deref for GCCtx<'gc> {
    type Target = &'gc GarbageCollector;

    fn deref(&self) -> &Self::Target {
        &self.gc
    }
}

/// Represents a valid garbage-collected reference under the safe garbage collection API.
#[derive(Clone, Copy)]
pub struct GC<'a, T: GCPtr> {
    gc: &'a GarbageCollector,
    inner: T,
}

impl<'a, T: GCPtr> GC<'a, T> {
    /// Wraps a GC ptr using a GC context.
    ///
    /// # Safety
    /// - `T` must be valid, alive, and initialized
    pub unsafe fn new(ctx: &'a GCCtx, value: T) -> Self {
        unsafe { Self::from_raw_parts(ctx, value) }
    }

    /// Wraps a GC ptr using a reference to the garbage collector.
    ///
    /// Whenever possible, [`GC::new`] should be used instead. Unlike [`GC::new`], this method does
    /// not return a fully bound lifetime. If the caller is not careful, safe code can use this
    /// object after a garbage collection cycle is triggered.
    ///
    /// # Safety
    /// - `T` must be valid, alive, and initialized.
    /// - `T` must be alive for the lifetime of the returned object.
    pub unsafe fn from_raw_parts(gc: &'a GarbageCollector, value: T) -> Self {
        Self { gc, inner: value }
    }

    pub fn raw(self) -> T {
        self.inner
    }

    pub fn gc(&self) -> &'a GarbageCollector {
        self.gc
    }
}

impl<'a, T: GCProject<'a>> GC<'a, T> {
    /// Allows you to access fields of a struct or to match an enum.
    pub fn project(self) -> T::Projected {
        T::project(self)
    }
}

/// Represents a garbage collection root under the safe garbage collection API. These can be created
/// using the [`root`] macro.
///
/// Instances of this type are locally pinned in order to ensure that they are dropped in the
/// correct order.
pub type Root<'r, T> = Pin<&'r mut GCRootGuard<'r, T>>;

/// Represents a garbage collection root. When this object is dropped, its corresponding root is
/// freed. Safe code cannot create this directly because garbage collection roots must be freed in
/// the reverse order in which they're created.
///
/// The [`root`] macro defines a safe interface for the creation of this type.
pub struct GCRootGuard<'gc, T: GCPtr> {
    gc: &'gc GarbageCollector,
    raw_ref: ManuallyDrop<GCRootRef<T>>,
    _pin: PhantomPinned,
}

impl<'gc, T: GCPtr> GCRootGuard<'gc, T> {
    /// Creates a new garbage collection root.
    ///
    /// # Safety
    /// - The caller must ensure that all garbage collection roots are dropped in the reverse order
    ///   in which they're created.
    pub unsafe fn new<'b>(gc: &'gc GarbageCollector, value: GC<'b, T>) -> Self {
        unsafe {
            assert_eq!(
                gc as *const GarbageCollector,
                value.gc as *const GarbageCollector
            );

            Self {
                gc,
                raw_ref: ManuallyDrop::new(gc.push_root(value.inner)),
                _pin: PhantomPinned,
            }
        }
    }
}

impl<'gc, T: GCPtr> GCRootGuard<'gc, T> {
    pub fn get<'val>(&'_ self, ctx: &'val GCCtx<'gc>) -> GC<'val, T> {
        assert_eq!(
            self.gc as *const GarbageCollector,
            ctx.gc as *const GarbageCollector
        );

        unsafe { GC::new(&ctx, self.raw_ref.get(self.gc)) }
    }
}

impl<'gc, T: GCPtr> Drop for GCRootGuard<'gc, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::take(&mut self.raw_ref).pop(self.gc);
        }
    }
}

/// Safely defines a safe garbage collector root. Returns a [Root]. This root is locally pinned, so
/// its lifetime will be exactly equal to the lifetime of the scope where the macro is called in.
///
/// This macro will ensure at compile time that any roots defined with it are dropped in the
/// required order.
#[allow(unused)]
macro_rules! root {
    ($gc:expr, $val:expr) => {{
        let v = $val;

        ::core::pin::pin!(unsafe { $crate::gc::safety::GCRootGuard::new($gc, v) })
    }};
}

/// Immutably re-borrows a `GC<'_, _>` object.
///
/// When a function consumes a `&mut GCCtx` and returns a `GC<'_, _>`, the returned object will
/// mutably borrows from the context. This means that the context cannot be used until the returned
/// object is dropped.
///
/// This macro will fix this issue and will allow you to immutably borrow from the context for the
/// lifetime of the object.
#[allow(unused)]
macro_rules! rebind {
    ($ctx:ident, $val:expr) => {{
        let val = $val;

        let associated_gc_ptr = ::core::ptr::from_ref(::mulch::gc::safety::GC::gc(&val));
        let raw = ::mulch::gc::safety::GC::raw(val);

        ::core::assert_eq!(
            associated_gc_ptr,
            ::core::ptr::from_ref(*<::mulch::gc::safety::GCCtx as ::core::ops::Deref>::deref(
                $ctx
            ))
        );

        unsafe { ::mulch::gc::safety::GC::new(&$ctx, raw) }
    }};
}

/// The projected version of a type.
#[allow(type_alias_bounds)]
pub type Projected<'a, T: GCProject<'a>> = T::Projected;

#[allow(unused)]
pub(crate) use root;

#[allow(unused)]
pub(crate) use rebind;

impl<'gc, T: GCPtr> Debug for GC<'gc, T>
where
    T: GCDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe { self.inner.clone().gc_debug(self.gc, f) }
    }
}

impl<'gc, T: GCGet + GCPtr> GC<'gc, T> {
    pub fn read(&self) -> &T::Borrowed {
        unsafe { self.inner.get(self.gc) }
    }
}
