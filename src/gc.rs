mod collection;
mod gcspace;
mod primitives;

pub mod util;
use std::marker::PhantomData;

pub use collection::{GCRoot, GCValue, GCValueEnum};
pub use gcspace::GCPtr;
pub use gcspace::GCSpace;
pub use primitives::*;

use crate::gc::util::GCDebug;

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
    const BLOCK_SIZE: usize = crate::util::ceil_power_two(crate::util::max!(
        std::mem::align_of::<crate::parser_old::Expression>(),
        std::mem::align_of::<crate::eval::MValue>(),
        std::mem::align_of::<usize>(),
        std::mem::size_of::<usize>()
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

            impl ::mulch::gc::util::GCDebug for $ty {
                unsafe fn gc_debug(
                    self,
                    _gc: &::mulch::gc::GarbageCollector,
                    f: &mut ::std::fmt::Formatter,
                ) -> ::std::fmt::Result {
                    ::std::fmt::Debug::fmt(&self, f)
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
