use std::{
    fmt::Debug,
    num::{IntErrorKind, NonZeroUsize},
};

use crate::{
    error::{PartialSpanned, parse::PDResult},
    gc::{
        GCPtr, GarbageCollector,
        math::rational::GCRational,
        util::{GCDebug, GCEq, GCWrap},
    },
};

/// A garbage collected, infinite precision rational number.
///
/// This type contains an optimization for small integers
/// # Layout
/// If the most-significant-bit is set, the remaining bits are the signed value.
///
/// If the MSB is not set, the remaining bits should be interpereted as a `GCRational`
#[derive(Clone, Copy)]
pub struct GCNumber {
    value: NonZeroUsize,
}

enum GetGCNumber {
    Inline(usize),
    Rational(GCRational),
}

impl GCNumber {
    pub fn parse_from_literal(
        gc: &GarbageCollector,
        decimal: PartialSpanned<&str>,
    ) -> PDResult<Self> {
        let rational = GCRational::parse_from_literal(gc, decimal)?;

        if let Some(num) = unsafe { rational.as_usize(gc) }
            && let Some(num) = Self::from_usize(num)
        {
            unsafe { rational.deallocate_from_end(gc) };
            return Ok(num);
        }

        Ok(rational.into())
    }

    /// Parses a `GCNumer` from a numerator and denominator. Panics on failiure. This is solely
    /// intended for writing tests.
    #[allow(unused)]
    pub(crate) fn parse_from_numerator_and_denominator_panicking(
        gc: &GarbageCollector,
        numerator: &str,
        denominator: Option<&str>,
    ) -> Self {
        if denominator.is_none() {
            match str::parse::<usize>(numerator).map(|val| Self::from_usize(val)) {
                Ok(Some(val)) => return val,
                Ok(None) => {}
                Err(err) => {
                    if *err.kind() != IntErrorKind::PosOverflow {
                        panic!("{err:?}")
                    }
                }
            }
        }

        let rational =
            GCRational::parse_from_numerator_and_denominator_panicking(gc, numerator, denominator);

        rational.into()
    }

    pub fn from_usize(usize: usize) -> Option<Self> {
        if usize & 1usize.rotate_right(1) != 0 {
            Some(Self {
                value: unsafe { NonZeroUsize::new_unchecked(usize | 1usize.rotate_right(1)) },
            })
        } else {
            None
        }
    }

    fn get(self) -> GetGCNumber {
        if self.value.get() & 1usize.rotate_right(1) != 0 {
            GetGCNumber::Inline(self.value.get() & !1usize.rotate_right(1))
        } else {
            unsafe { GetGCNumber::Rational(GCRational::from_raw(self.value)) }
        }
    }
}

impl From<GCRational> for GCNumber {
    fn from(value: GCRational) -> Self {
        Self {
            value: value.gc_ptr(),
        }
    }
}

unsafe impl GCPtr for GCNumber {
    const MSB_RESERVED: bool = false;

    unsafe fn gc_copy(self, gc: &crate::gc::GarbageCollector) -> Self {
        match self.get() {
            GetGCNumber::Inline(_) => self,
            GetGCNumber::Rational(rat) => unsafe { rat.gc_copy(gc) }.into(),
        }
    }
}

impl GCDebug for GCNumber {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match self.get() {
            GetGCNumber::Inline(int) => Debug::fmt(&int, f),
            GetGCNumber::Rational(rat) => unsafe { rat.gc_debug(gc, f) },
        }
    }
}

impl GCEq<GCNumber> for GCNumber {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &GCNumber) -> bool {
        unsafe {
            match (self.get(), rhs.get()) {
                (GetGCNumber::Inline(inline1), GetGCNumber::Inline(inline2)) => inline1 == inline2,

                (GetGCNumber::Inline(inline), GetGCNumber::Rational(rational))
                | (GetGCNumber::Rational(rational), GetGCNumber::Inline(inline)) => {
                    GCWrap::new(rational, gc) == inline
                }

                (GetGCNumber::Rational(rational1), GetGCNumber::Rational(rational2)) => {
                    GCWrap::new(rational1, gc) == GCWrap::new(rational2, gc)
                }
            }
        }
    }
}
