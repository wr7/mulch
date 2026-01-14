use std::ops::{RangeFrom, RangeTo};

use copyspan::Span;

use crate::{
    error::parse::{PDResult, ParseDiagnostic},
    parser::{Parser, TokenStream},
};

/// Types that can be parsed from the left side with a remainder.
pub trait ParseLeft: Sized + Parse {
    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_left(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>>;
}

/// Types that can be parsed from the right side with a remainder.
pub trait ParseRight: Sized + Parse {
    /// Attempts to parse `Self` from the right of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_right(parser: &Parser, tokens: &mut &TokenStream) -> PDResult<Option<Self>>;
}

/// Types where you can search through a `TokenStream` and find the index of the leftmost occurance.
pub trait FindLeft: Sized {
    /// Returns a range containing the leftmost instance of `Self` and everything to the right of it.
    fn find_left(parser: &Parser, tokens: &TokenStream) -> PDResult<RangeFrom<usize>>;
}

/// Types where you can search through a `TokenStream` and find the index of the rightmost occurance.
pub trait FindRight: Sized {
    /// Returns a range containing the rightmost instance of `Self` and everything to the left of it.
    fn find_right(parser: &Parser, tokens: &TokenStream) -> PDResult<RangeTo<usize>>;
}

/// Types that can be parsed from a whole `TokenStream` with no remainder.
pub trait Parse: Sized {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic;

    /// Attempts to parse a whole [`TokenStream`] as `Self`.
    fn parse(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Self>>;

    fn parse_from_left_until<B: FindLeft>(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<Self>> {
        let range = B::find_left(parser, tokens)?;

        let res = Self::parse(parser, &tokens[..range.start]);

        if let Ok(Some(_)) = &res {
            *tokens = &tokens[range];
        }

        res
    }

    fn parse_from_right_until<B: FindRight>(
        parser: &Parser,
        tokens: &mut &TokenStream,
    ) -> PDResult<Option<Self>> {
        let range = B::find_right(parser, tokens)?;

        let res = Self::parse(parser, &tokens[range.end..]);

        if let Ok(Some(_)) = &res {
            *tokens = &tokens[range];
        }

        res
    }
}

macro_rules! impl_using_parse_left {
    () => {
        fn parse(
            parser: &$crate::parser::Parser,
            mut tokens: &$crate::parser::TokenStream,
        ) -> $crate::error::parse::PDResult<::core::option::Option<Self>> {
            let Some(val) = Self::parse_from_left(parser, &mut tokens)? else {
                return Ok(None);
            };

            if let Some(span) = $crate::error::span_of(tokens) {
                return Err($crate::parser::error::unexpected_tokens(span));
            }

            Ok(Some(val))
        }
    };
}

pub(super) use impl_using_parse_left;
