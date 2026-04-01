mod collection;
mod gcspace;
mod primitives;

pub mod util;
use std::marker::PhantomData;

pub use collection::{GCRoot, GCValue, GCValueEnum};
pub use gcspace::GCPtr;
pub use gcspace::GCSpace;
use gmp_mpfr_sys::gmp;
pub use primitives::math;
pub use primitives::*;

use crate::error::PartialSpanned;
use crate::gc::util::GCDebug;
use crate::gc::util::GCEq;
use crate::gc::util::GCWrap;
use crate::gc::util::NonGC;

#[cfg(test)]
mod test;

/// The garbage collector.
///
/// New objects may be allocated through an immutable reference. A garbage-collection cycle can only
/// be triggered by methods that take a mutable reference.
pub struct GarbageCollector {
    from_space: GCSpace,
    to_space: GCSpace,
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl GarbageCollector {
    pub const BLOCK_SIZE: usize = crate::util::ceil_power_two(crate::util::max!(
        std::mem::align_of::<crate::parser::ast::Expression>(),
        std::mem::align_of::<crate::eval::MValue>(),
        std::mem::align_of::<usize>(),
        std::mem::align_of::<u64>(),
        std::mem::align_of::<copyspan::Span>(),
        std::mem::align_of::<gmp::limb_t>(),
        std::mem::size_of::<usize>(),
    ));

    pub fn new() -> Self {
        GarbageCollector {
            from_space: GCSpace::new(),
            to_space: GCSpace::new(),
        }
    }
}

/// Implements gc traits for copyable, non-garbage-collected types
macro_rules! gc_trivial_impl {
    {
        $(
        $ty:ty
        ),* $(,)?
    } => {
        $(
            unsafe impl ::mulch::gc::GCPtr for $ty {
                const MSB_RESERVED: bool = false;

                unsafe fn gc_copy(self, _gc: &mut ::mulch::gc::GarbageCollector) -> Self {
                    self
                }
            }

            unsafe impl ::mulch::gc::util::NonGC for $ty {}

            impl ::mulch::gc::util::GCDebug for $ty {
                unsafe fn gc_debug(
                    self,
                    _gc: &::mulch::gc::GarbageCollector,
                    f: &mut ::std::fmt::Formatter,
                ) -> ::std::fmt::Result {
                    ::std::fmt::Debug::fmt(&self, f)
                }
            }

            impl ::mulch::gc::util::GCEq<$ty> for $ty {
                unsafe fn gc_eq(&self, _gc: &::mulch::gc::GarbageCollector, rhs: &$ty) -> bool {
                    self == rhs
                }
            }
        )*
    };
}

gc_trivial_impl! {
    (),
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    copyspan::Span,
}

unsafe impl<T: ?Sized> GCPtr for PhantomData<T> {
    const MSB_RESERVED: bool = false;

    unsafe fn gc_copy(self, _gc: &mut GarbageCollector) -> Self {
        self
    }
}

impl<T: ?Sized> GCDebug for PhantomData<T> {
    unsafe fn gc_debug(
        self,
        _gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl<T> GCEq<PhantomData<T>> for PhantomData<T> {
    unsafe fn gc_eq(&self, _: &GarbageCollector, _: &Self) -> bool {
        true
    }
}

unsafe impl<T> NonGC for PhantomData<T> {}

unsafe impl<T: GCPtr> GCPtr for PartialSpanned<T> {
    const MSB_RESERVED: bool = T::MSB_RESERVED;

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        let inner_copy = unsafe { self.0.gc_copy(gc) };

        PartialSpanned(inner_copy, self.1)
    }
}

unsafe impl<T: NonGC> NonGC for PartialSpanned<T> {}

impl<T: GCDebug> GCDebug for PartialSpanned<T> {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_tuple("PartialSpanned")
            .field(&unsafe { GCWrap::new(self.0, gc) })
            .field(&self.1)
            .finish()
    }
}

impl<T: GCEq<T>> GCEq<PartialSpanned<T>> for PartialSpanned<T> {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &PartialSpanned<T>) -> bool {
        unsafe { self.0.gc_eq(gc, &rhs.0) && self.1 == rhs.1 }
    }
}

unsafe impl<T: GCPtr> GCPtr for Option<T> {
    const MSB_RESERVED: bool = false;

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        self.map(|val| unsafe { val.gc_copy(gc) })
    }
}

unsafe impl<T: NonGC> NonGC for Option<T> {}

impl<T: GCDebug> GCDebug for Option<T> {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match self {
            Some(val) => unsafe { f.debug_tuple("Some").field(&GCWrap::new(val, gc)).finish() },
            None => write!(f, "None"),
        }
    }
}

impl<T: GCEq<T>> GCEq<Option<T>> for Option<T> {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &Option<T>) -> bool {
        match (self, rhs) {
            (None, None) => true,
            (Some(a), Some(b)) => unsafe { a.gc_eq(gc, b) },
            _ => false,
        }
    }
}
