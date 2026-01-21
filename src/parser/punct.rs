use copyspan::Span;
use itertools::Itertools;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::{PartialSpanned, parse::ParseDiagnostic},
    lexer::{Symbol, Token},
    parser::{
        self, FindLeft, Parse, ParseLeft, Parser, traits::impl_using_parse_left,
        util::NonBracketedIter,
    },
    util::element_offset,
};

/// Parses a specific symbol token. This type should only be referred to using the [`punct`] macro.
/// The represented [`Symbol`] value can be accessed via the associated constant [`Self::SYMBOL`].
#[derive(GCDebug, GCPtr, Clone, Copy, Debug)]
pub struct Punct<const S: u8> {
    span: Span,
}

impl<const S: u8> Punct<S> {
    pub const SYMBOL: Symbol = if let Some(symbol) = Symbol::from_u8(S) {
        symbol
    } else {
        panic!("Invalid symbol id")
    };
}

impl<const S: u8> ParseLeft for Punct<S> {
    fn parse_from_left(
        _gc: &Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [PartialSpanned(Token::Symbol(sym), span), rem @ ..] = tokens else {
            return Ok(None);
        };

        if *sym != Self::SYMBOL {
            return Ok(None);
        }

        *tokens = rem;
        Ok(Some(Self { span: *span }))
    }
}

impl<const S: u8> FindLeft for Punct<S> {
    fn find_left<'a, 'src>(
        _: &Parser,
        tokens: &'a parser::TokenStream<'src>,
    ) -> crate::error::parse::PDResult<std::ops::RangeFrom<usize>> {
        let idx = NonBracketedIter::new(tokens)
            .process_results(|mut iter| {
                iter.find(|tok|
                    matches!(**tok, PartialSpanned(Token::Symbol(sym), _) if sym == Self::SYMBOL),
                )
            })?
            .and_then(|tok| element_offset(tokens, tok))
            .unwrap_or(tokens.len());

        Ok(idx..)
    }
}

impl<const S: u8> Parse for Punct<S> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> ParseDiagnostic =
        parser::error::expected_punctuation::<S>;

    impl_using_parse_left!();
}

#[macro_export]
macro_rules! punct {
    [$($sym:tt)+] => {
        $crate::parser::Punct::<{$crate::Sym!($($sym)+).to_u8()}>
    };
}

#[allow(unused)]
pub(crate) use punct;
