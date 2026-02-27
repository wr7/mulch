use std::marker::PhantomData;

use copyspan::Span;

use crate::{
    error::parse::{PDResult, ParseDiagnostic},
    gc::{GCPtr, GarbageCollector, util::GCDebug},
    parser::{FindRight, ParseRight, Parser, TokenStream},
};

/// Matches its first type parameter when parsed. The second type parameter is used for the
/// `FindRight` trait. `FindRight` will only match occurances of `T` that are not preceded by `P`.
#[repr(transparent)]
pub struct NotPrecededBy<T, P> {
    pub value: T,
    _phantomdata: PhantomData<P>,
}

impl<T: FindRight, P: FindRight> FindRight for NotPrecededBy<T, P> {
    fn find_right(
        parser: &Parser,
        mut tokens: &TokenStream,
    ) -> PDResult<Option<std::ops::Range<usize>>> {
        loop {
            let Some(range) = T::find_right(parser, tokens)? else {
                return Ok(None);
            };

            let prefix = P::find_right(parser, &tokens[..range.start])?;

            if let Some(prefix) = prefix
                && prefix.end == range.start
            {
                tokens = &tokens[..range.start];
                continue;
            }

            return Ok(Some(range));
        }
    }
}

impl<T: ParseRight, P> ParseRight for NotPrecededBy<T, P> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic = |_| unimplemented!();

    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_right(parser, tokens)?.map(|value| Self {
            value,
            _phantomdata: PhantomData,
        }))
    }
}

impl<T: Clone, P> Clone for NotPrecededBy<T, P> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            _phantomdata: PhantomData,
        }
    }
}

impl<T: Copy, P> Copy for NotPrecededBy<T, P> {}

unsafe impl<T: GCPtr, P> GCPtr for NotPrecededBy<T, P> {
    const MSB_RESERVED: bool = T::MSB_RESERVED;

    unsafe fn gc_copy(self, gc: &mut GarbageCollector) -> Self {
        Self {
            value: unsafe { self.value.gc_copy(gc) },
            _phantomdata: PhantomData,
        }
    }
}

impl<T: GCDebug, P> GCDebug for NotPrecededBy<T, P> {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        unsafe { self.value.gc_debug(gc, f) }
    }
}
