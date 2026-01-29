use copyspan::Span;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::GCString,
    lexer::Token,
    parser::{self, Parse, ParseLeft, traits::impl_using_parse_left},
};

#[derive(Clone, Copy, GCPtr, GCDebug)]
pub struct IdentOrString(GCString, Span);

impl ParseLeft for IdentOrString {
    fn parse_from_left(
        parser: &crate::parser::Parser,
        tokens: &mut &crate::parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [
            PartialSpanned(Token::Identifier(str) | Token::StringLiteral(str), span),
            rem @ ..,
        ] = tokens
        else {
            return Ok(None);
        };

        *tokens = rem;

        Ok(Some(Self(GCString::new(parser.gc, str), *span)))
    }
}

impl Parse for IdentOrString {
    const EXPECTED_ERROR_FUNCTION: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_ident_or_string;

    impl_using_parse_left!();
}
