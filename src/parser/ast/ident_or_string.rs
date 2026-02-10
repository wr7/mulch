use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::GCString,
    lexer::Token,
    parser::{self, Parse, ParseLeft, traits::impl_using_parse_left},
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
    ) -> crate::error::parse::PDResult<Option<PartialSpanned<Self>>> {
        let [
            PartialSpanned(Token::Identifier(str) | Token::StringLiteral(str), span),
            rem @ ..,
        ] = tokens
        else {
            return Ok(None);
        };

        *tokens = rem;

        Ok(Some(PartialSpanned(
            Self(GCString::new(parser.gc, str)),
            *span,
        )))
    }
}

impl Parse for IdentOrString {
    impl_using_parse_left!();
}
