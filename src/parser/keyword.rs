use copyspan::Span;
use itertools::Itertools;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    lexer::Token,
    parser::{
        FindLeft, Parse, ParseLeft, Parser, error, traits::impl_using_parse_left,
        util::NonBracketedIter,
    },
    util::element_offset,
};

/// Parses a literal that matches a specific keyword. This type should only be referred to using the
/// [`keyword`](mulch_macros::keyword) macro, and the value of `K` should only be accessed through
/// the `KEYWORD` associated constant.
///
/// The `mulch` programming language does not make a distinction between "keywords" and
/// "identifiers", so any string can be used as a keyword.
///
/// NOTE: Rust currently doesn't have a way to use string literals as const generics. To get around
/// this, we're storing the `0xff`-terminated bytes in a big-endian u128. The actual string can be
/// accessed through the associated constant `KEYWORD`.
#[derive(GCDebug, GCPtr, Clone, Copy, Debug)]
pub struct Keyword<const K: u128>();

impl<const K: u128> Keyword<K> {
    /// The string literal contained in the `Keyword` type
    pub const KEYWORD: &'static str = crate::util::str_from_u128(&Self::RAW_BYTES);

    const RAW_BYTES: [u8; 16] = K.to_be_bytes();
}

impl<const K: u128> ParseLeft for Keyword<K> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> crate::error::parse::ParseDiagnostic =
        error::expected_keyword::<K>;

    fn parse_from_left(
        _: &Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [PartialSpanned(Token::Identifier(ident), _), remainder @ ..] = tokens else {
            return Ok(None);
        };

        if ident != Self::KEYWORD {
            return Ok(None);
        }

        *tokens = remainder;
        Ok(Some(Self()))
    }
}

impl<const K: u128> FindLeft for Keyword<K> {
    fn find_left<'a, 'src>(
        _: &Parser,
        tokens: &'a super::TokenStream<'src>,
    ) -> crate::error::parse::PDResult<Option<std::ops::RangeFrom<usize>>> {
        Ok(
            NonBracketedIter::new(tokens)
                .process_results(|mut it|
                    it.find(|tok|
                        matches!(tok, PartialSpanned(Token::Identifier(ident), _) if ident == Self::KEYWORD)
                    )
                )?
                .and_then(|tok| element_offset(tokens, tok))
                .map(|idx| idx..)
        )
    }
}

impl<const K: u128> Parse for Keyword<K> {
    impl_using_parse_left! {}
}
