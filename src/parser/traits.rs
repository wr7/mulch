use std::{marker::PhantomData, ops::RangeFrom};

use copyspan::Span;

use crate::{
    error::{
        PartialSpanned,
        parse::{PDResult, ParseDiagnostic},
        span_of,
    },
    gc::{GCBox, GCPtr},
    parser::{self, Parser, TokenStream},
};

/// Types that can be parsed from the left side with a remainder.
pub trait ParseLeft: Sized {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic;

    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_left(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<PartialSpanned<Self>>>;
}

/// Types where you can search through a `TokenStream` and find the index of the leftmost occurance.
pub trait FindLeft: Sized {
    /// Returns a range containing the leftmost instance of `Self` and everything to the right of
    /// it.
    ///
    /// NOTE: if `Some(_)` is returned, `parse_left` must return `Ok(Some(_))`
    fn find_left(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeFrom<usize>>>;
}

/// Types that can be parsed from a whole `TokenStream` with no remainder.
pub trait Parse: Sized {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic;

    /// Attempts to parse a whole [`TokenStream`] as `Self`.
    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>>;
}

impl<T: Parse> Parse for Option<T> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic = parser::error::unexpected_tokens;

    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>> {
        Ok(match T::parse(parser, tokens)? {
            Some(val) => Some(Some(val)),
            None if tokens.is_empty() => Some(None),
            None => None,
        })
    }
}

impl<T: ParseLeft> ParseLeft for Option<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic = |_| unreachable!();

    fn parse_from_left(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<PartialSpanned<Self>>> {
        Ok(match T::parse_from_left(parser, tokens)? {
            Some(PartialSpanned(val, span)) => Some(PartialSpanned(Some(val), span)),
            None => Some(PartialSpanned(
                None,
                tokens
                    .first()
                    .map_or(Span::from(0..0), |val| val.1.span_at()), // The fallback value of 0..0 is incredibly janky, but it will work for now
            )),
        })
    }
}

impl<T: Parse + GCPtr> Parse for GCBox<T> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic = T::EXPECTED_ERROR_FUNCTION;

    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse(parser, tokens)?.map(|val| unsafe { GCBox::new(parser.gc, val) }))
    }
}

impl<T: ParseLeft> ParseLeft for PhantomData<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_LEFT;

    fn parse_from_left(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<PartialSpanned<Self>>> {
        Ok(T::parse_from_left(parser, tokens)?.map(|val| val.map(|_| PhantomData)))
    }
}

impl<T: ParseLeft + GCPtr> ParseLeft for GCBox<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_LEFT;

    fn parse_from_left(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<PartialSpanned<Self>>> {
        Ok(T::parse_from_left(parser, tokens)?
            .map(|val| val.map(|val| unsafe { GCBox::new(parser.gc, val) })))
    }
}

impl<T: ParseLeft> ParseLeft for PartialSpanned<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_LEFT;

    fn parse_from_left(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<PartialSpanned<Self>>> {
        Ok(T::parse_from_left(parser, tokens)?
            .map(|PartialSpanned(val, span)| PartialSpanned(PartialSpanned(val, span), span)))
    }
}

impl<T: Parse> Parse for PartialSpanned<T> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic = T::EXPECTED_ERROR_FUNCTION;

    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>> {
        let Some(span) = span_of(tokens) else {
            return Ok(None);
        };

        Ok(T::parse(parser, tokens)?.map(|val| PartialSpanned(val, span)))
    }
}

impl<T: FindLeft> FindLeft for PhantomData<T> {
    fn find_left(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeFrom<usize>>> {
        T::find_left(parser, tokens)
    }
}

impl<T: FindLeft + GCPtr> FindLeft for GCBox<T> {
    fn find_left(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeFrom<usize>>> {
        T::find_left(parser, tokens)
    }
}

macro_rules! impl_using_parse_left {
    () => {
        const EXPECTED_ERROR_FUNCTION: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
            Self::EXPECTED_ERROR_FUNCTION_LEFT;

        fn parse(
            parser: &$crate::parser::Parser,
            mut tokens: &$crate::parser::TokenStream,
        ) -> $crate::error::parse::PDResult<::core::option::Option<Self>> {
            let Some(val) = Self::parse_from_left(parser, &mut tokens)? else {
                return Ok(None);
            };

            if !tokens.is_empty() {
                return Ok(None);
            }

            Ok(Some(val.0))
        }
    };
}

pub(super) use impl_using_parse_left;
