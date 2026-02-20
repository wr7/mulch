use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::GCString,
    lexer::Token,
    parser::{self, Parse, ParseLeft, ParseRight, traits::impl_using_parse_left},
};

#[derive(Clone, Copy, GCPtr, GCDebug)]
#[debug_direct]
pub struct IdentOrString(pub GCString);

impl ParseLeft for IdentOrString {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_ident_or_string;

    fn parse_from_left(
        parser: &crate::parser::Parser,
        tokens: &mut &crate::parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [
            PartialSpanned(Token::Identifier(str) | Token::StringLiteral(str), _),
            rem @ ..,
        ] = tokens
        else {
            return Ok(None);
        };

        *tokens = rem;

        Ok(Some(Self(GCString::new(parser.gc, str))))
    }
}

impl ParseRight for IdentOrString {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(
        copyspan::Span,
    ) -> crate::error::parse::ParseDiagnostic = parser::error::expected_ident_or_string;

    fn parse_from_right(
        parser: &crate::parser::Parser,
        tokens: &mut &crate::parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [
            rem @ ..,
            PartialSpanned(Token::Identifier(str) | Token::StringLiteral(str), _),
        ] = tokens
        else {
            return Ok(None);
        };

        *tokens = rem;

        Ok(Some(Self(GCString::new(parser.gc, str))))
    }
}

impl Parse for IdentOrString {
    impl_using_parse_left!();
}
