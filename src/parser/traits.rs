use std::{
    marker::PhantomData,
    ops::{RangeFrom, RangeTo},
};

use copyspan::Span;

use crate::{
    error::{
        PartialSpanned,
        parse::{PDResult, ParseDiagnostic},
        span_of,
    },
    gc::{GCBox, GCPtr},
    parser::{self, Parser, TokenStream},
    util::subslice_range,
};

/// Types that can be parsed from the left side with a remainder.
pub trait ParseLeft: Sized {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic;

    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>>;

    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success. Also returns the `Span` of the parsed tokens (or
    /// `None` if `tokens` is empty).
    fn parse_from_left_with_span(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<(Self, Option<Span>)>> {
        let mut tokens_copy = *tokens;
        let Some(res) = Self::parse_from_left(parser, &mut tokens_copy)? else {
            return Ok(None);
        };

        let end_idx = subslice_range(tokens, tokens_copy).unwrap().start;
        let span = span_of(&tokens[..end_idx]).or_else(|| tokens.first().map(|t| t.1.span_at()));

        *tokens = tokens_copy;

        Ok(Some((res, span)))
    }
}

/// Types that can be parsed from the right side with a remainder.
pub trait ParseRight: Sized {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic;

    /// Attempts to parse `Self` from the right of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>>;

    /// Attempts to parse `Self` from the right of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success. Also returns the `Span` of the parsed tokens (or
    /// `None` if `tokens` is empty).
    fn parse_from_right_with_span(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<(Self, Option<Span>)>> {
        let mut tokens_copy = *tokens;
        let Some(res) = Self::parse_from_right(parser, &mut tokens_copy)? else {
            return Ok(None);
        };

        let start_idx = subslice_range(tokens, tokens_copy).unwrap().end;
        let span =
            span_of(&tokens[start_idx..]).or_else(|| tokens.last().map(|t| t.1.span_after()));

        *tokens = tokens_copy;

        Ok(Some((res, span)))
    }
}

/// Types where you can search through a `TokenStream` and find the index of the leftmost occurance.
pub trait FindLeft: Sized {
    /// Returns a range containing the leftmost instance of `Self` and everything to the right of
    /// it.
    ///
    /// NOTE: if `Some(_)` is returned, `parse_left` must return `Ok(Some(_))`
    fn find_left(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeFrom<usize>>>;
}

/// Types where you can search through a `TokenStream` and find the index of the rightmost occurance.
pub trait FindRight: Sized {
    /// Returns a range containing the rightmost instance of `Self` and everything to the left of
    /// it.
    ///
    /// NOTE: if `Some(_)` is returned, `parse_right` must return `Ok(Some(_))`
    fn find_right(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeTo<usize>>>;
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

    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        T::parse_from_left(parser, tokens).map(Some)
    }
}

impl<T: ParseRight> ParseRight for Option<T> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic = |_| unreachable!();

    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        T::parse_from_right(parser, tokens).map(Some)
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

    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_left(parser, tokens)?.map(|_| PhantomData))
    }
}

impl<T: ParseRight> ParseRight for PhantomData<T> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_RIGHT;

    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_right(parser, tokens)?.map(|_| PhantomData))
    }
}

impl<T: ParseLeft + GCPtr> ParseLeft for GCBox<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_LEFT;

    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_left(parser, tokens)?.map(|val| unsafe { GCBox::new(parser.gc, val) }))
    }
}

impl<T: ParseRight + GCPtr> ParseRight for GCBox<T> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_RIGHT;

    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_right(parser, tokens)?.map(|val| unsafe { GCBox::new(parser.gc, val) }))
    }
}

impl<T: ParseLeft> ParseLeft for PartialSpanned<T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_LEFT;

    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_left_with_span(parser, tokens)?
            .map(|(val, span)|
                PartialSpanned(
                    val,
                    span.unwrap_or_else(||
                        panic!("The parse trait implementations for `PartialSpanned<{T}>` should not be used because `{T}` can be parsed from an empty tokenstream", T=::core::any::type_name::<T>())
                    )
                )
            ))
    }
}

impl<T: ParseRight> ParseRight for PartialSpanned<T> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic =
        T::EXPECTED_ERROR_FUNCTION_RIGHT;

    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>> {
        Ok(T::parse_from_right_with_span(parser, tokens)?
            .map(|(val, span)|
                PartialSpanned(
                    val,
                    span.unwrap_or_else(||
                        panic!("The parse trait implementations for `PartialSpanned<{T}>` should not be used because `{T}` can be parsed from an empty tokenstream", T=::core::any::type_name::<T>())
                    )
                )
            ))
    }
}

impl<T: Parse> Parse for PartialSpanned<T> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic = T::EXPECTED_ERROR_FUNCTION;

    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>> {
        let Some(span) = span_of(tokens) else {
            panic!(
                "The parse trait implementations for `PartialSpanned<{T}>` should not be used because `{T}` can be parsed from an empty tokenstream",
                T = ::core::any::type_name::<T>()
            );
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

impl<T: FindRight> FindRight for PhantomData<T> {
    fn find_right(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeTo<usize>>> {
        T::find_right(parser, tokens)
    }
}

impl<T: FindRight + GCPtr> FindRight for GCBox<T> {
    fn find_right(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<RangeTo<usize>>> {
        T::find_right(parser, tokens)
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

            Ok(Some(val))
        }
    };
}

pub(super) use impl_using_parse_left;
