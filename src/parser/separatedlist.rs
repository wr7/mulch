use std::marker::PhantomData;

use mulch_macros::GCPtr;

use crate::{
    error::{PartialSpanned, parse::PDResult, span_of},
    gc::{GCPtr, GCVec, util::GCDebug},
    parser::{FindLeft, Parse, ParseLeft, Parser, TokenStream},
};

#[derive(GCPtr)]
pub struct SeparatedList<T: GCPtr, S> {
    pub values: GCVec<T>,
    _phantomdata: PhantomData<*const S>,
}

impl<T: GCPtr + Parse, S: FindLeft + ParseLeft> Parse for SeparatedList<T, S> {
    const EXPECTED_ERROR_FUNCTION: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
        |_| unreachable!();

    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>> {
        let mut items = Vec::new();

        for val in SeparatedListIter::<T, S>::new(parser, tokens) {
            let (item, _sep) = val?;
            items.push(item);
        }

        let gc_vec = unsafe { GCVec::new(parser.gc, &items) };

        Ok(Some(Self {
            values: gc_vec,
            _phantomdata: PhantomData,
        }))
    }
}

impl<T: GCPtr + GCDebug, S> GCDebug for SeparatedList<T, S> {
    unsafe fn gc_debug(
        self,
        gc: &crate::gc::GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        unsafe { self.values.gc_debug(gc, f) }
    }
}

// These derives don't work for whatever reason
impl<T: GCPtr, S> Copy for SeparatedList<T, S> {}
impl<T: GCPtr, S> Clone for SeparatedList<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct SeparatedListIter<'a, 'p, 't, T: Parse, S: FindLeft + ParseLeft> {
    parser: &'a Parser<'p>,
    remaining: &'t TokenStream<'t>,
    _phantomdata: PhantomData<Box<(T, S)>>,
}

impl<'a, 'p, 't, T: Parse, S: FindLeft + ParseLeft> SeparatedListIter<'a, 'p, 't, T, S> {
    pub fn new(parser: &'a Parser<'p>, tokens: &'t TokenStream<'t>) -> Self {
        Self {
            parser,
            remaining: tokens,
            _phantomdata: PhantomData,
        }
    }
}

impl<'a, 'p, 't, T: Parse, S: FindLeft + ParseLeft> Iterator
    for SeparatedListIter<'a, 'p, 't, T, S>
{
    type Item = PDResult<(T, Option<PartialSpanned<S>>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = match S::find_left(self.parser, self.remaining) {
            Ok(r) => r.map_or(self.remaining.len(), |r| r.start),
            Err(e) => return Some(Err(e)),
        };

        let item = &self.remaining[..idx];
        let item = match T::parse(self.parser, item) {
            Ok(Some(item)) => item,
            Ok(None) => {
                let Some(tok) = self.remaining.first() else {
                    return None;
                };

                return Some(Err(T::EXPECTED_ERROR_FUNCTION(
                    span_of(item).unwrap_or_else(|| tok.1),
                )));
            }
            Err(e) => return Some(Err(e)),
        };

        self.remaining = &self.remaining[idx..];
        let separator = if self.remaining.is_empty() {
            None
        } else {
            match S::parse_from_left(self.parser, &mut self.remaining) {
                Ok(Some(s)) => Some(s),
                Ok(None) => unreachable!(),
                Err(e) => return Some(Err(e)),
            }
        };

        Some(Ok((item, separator)))
    }
}
