use copyspan::Span;
use itertools::Itertools;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::{PartialSpanned, parse::ParseDiagnostic},
    lexer::{Symbol, Token},
    parser::{
        self, FindLeft, FindRight, Parse, ParseLeft, ParseRight, Parser,
        traits::impl_using_parse_left, util::NonBracketedIter,
    },
    util::element_offset,
};

/// Parses a specific symbol token. This type should only be referred to using the [`punct`](super::punct!) macro.
/// The represented [`Symbol`] value can be accessed via the associated constant [`Self::SYMBOL`].
#[derive(GCDebug, GCPtr, Clone, Copy, Debug)]
pub struct Punct<const S: u128>();

impl<const S: u128> Punct<S> {
    pub const SYMBOL: Symbol = Symbol::from_u128_str(S);
    /// The string literal contained in the `Punct` type
    pub const STRING: &'static str = crate::util::str_from_u128(&Self::RAW_BYTES);

    const RAW_BYTES: [u8; 16] = S.to_be_bytes();
}

impl<const S: u128> ParseLeft for Punct<S> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> ParseDiagnostic =
        parser::error::expected_punctuation::<S>;

    fn parse_from_left(
        _gc: &Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [PartialSpanned(Token::Symbol(sym), _), rem @ ..] = tokens else {
            return Ok(None);
        };

        if *sym != Self::SYMBOL {
            return Ok(None);
        }

        *tokens = rem;
        Ok(Some(Self()))
    }
}

impl<const S: u128> FindLeft for Punct<S> {
    fn find_left<'a, 'src>(
        _: &Parser,
        tokens: &'a parser::TokenStream<'src>,
    ) -> crate::error::parse::PDResult<Option<std::ops::RangeFrom<usize>>> {
        Ok(
            NonBracketedIter::new(tokens)
                .process_results(|mut iter| {
                    iter.find(|tok|
                        matches!(**tok, PartialSpanned(Token::Symbol(sym), _) if sym == Self::SYMBOL),
                    )
                })?
                .and_then(|tok| element_offset(tokens, tok))
                .map(|idx| idx..)
        )
    }
}

impl<const S: u128> ParseRight for Punct<S> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> ParseDiagnostic =
        parser::error::expected_punctuation::<S>;

    fn parse_from_right(
        _gc: &Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [rem @ .., PartialSpanned(Token::Symbol(sym), _)] = tokens else {
            return Ok(None);
        };

        if *sym != Self::SYMBOL {
            return Ok(None);
        }

        *tokens = rem;
        Ok(Some(Self()))
    }
}

impl<const S: u128> FindRight for Punct<S> {
    fn find_right<'a, 'src>(
        _: &Parser,
        tokens: &'a parser::TokenStream<'src>,
    ) -> crate::error::parse::PDResult<Option<std::ops::RangeTo<usize>>> {
        Ok(
            NonBracketedIter::new(tokens)
                .rev()
                .process_results(|mut iter| {
                    iter.find(|tok|
                        matches!(**tok, PartialSpanned(Token::Symbol(sym), _) if sym == Self::SYMBOL),
                    )
                })?
                .and_then(|tok| element_offset(tokens, tok))
                .map(|idx| ..idx + 1)
        )
    }
}

impl<const S: u128> Parse for Punct<S> {
    impl_using_parse_left!();
}
