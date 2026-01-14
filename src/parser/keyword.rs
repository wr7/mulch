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
/// NOTE: Rust currently doesn't have a way to use string literals as const generics. To get around
/// this, we're storing the `0xff`-terminated bytes in a big-endian u128. The actual string can be
/// accessed through the associated constant `KEYWORD`.
#[derive(GCDebug, GCPtr, Clone, Copy, Debug)]
pub struct Keyword<const K: u128> {
    span: Span,
}

impl<const K: u128> Keyword<K> {
    /// The string literal contained in the `Keyword` type
    pub const KEYWORD: &'static str = Self::get();

    const RAW_BYTES: [u8; 16] = K.to_be_bytes();
    const fn get() -> &'static str {
        let mut ret: Option<&'static [u8]> = None;
        let mut remaining = Self::RAW_BYTES.as_slice();

        while let Some((byte, r)) = remaining.split_last() {
            remaining = r;

            if *byte == 0xff {
                ret = Some(remaining);
            }
        }

        if let Some(ret) = ret
            && let Ok(ret) = std::str::from_utf8(ret)
        {
            return ret;
        }

        panic!("Invalid string generic for `Keyword`")
    }
}

impl<const K: u128> ParseLeft for Keyword<K> {
    fn parse_from_left(
        _: &Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [
            PartialSpanned(Token::Identifier(ident), span),
            remainder @ ..,
        ] = tokens
        else {
            return Ok(None);
        };

        if ident != Self::KEYWORD {
            return Ok(None);
        }

        *tokens = remainder;
        Ok(Some(Self { span: *span }))
    }
}

impl<const K: u128> FindLeft for Keyword<K> {
    fn find_left<'a, 'src>(
        _: &Parser,
        tokens: &'a super::TokenStream<'src>,
    ) -> crate::error::parse::PDResult<std::ops::RangeFrom<usize>> {
        let idx = NonBracketedIter::new(tokens)
            .process_results(|mut it|
                it.find(|tok|
                    matches!(tok, PartialSpanned(Token::Identifier(ident), _) if ident == Self::KEYWORD)
                )
            )?
            .and_then(|tok| element_offset(tokens, tok))
            .unwrap_or(tokens.len());

        Ok(idx..)
    }
}

impl<const K: u128> Parse for Keyword<K> {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> crate::error::parse::ParseDiagnostic =
        error::expected_keyword::<K>;

    impl_using_parse_left! {}
}
