use copyspan::Span;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::GCString,
    lexer::Token,
    parser::{self, Parse, ParseLeft, traits::impl_using_parse_left},
};

#[derive(Clone, Copy, GCPtr, GCDebug)]
#[debug_direct]
pub struct Ident(pub GCString);

impl ParseLeft for Ident {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_identifier;

    fn parse_from_left(
        parser: &super::Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<PartialSpanned<Self>>> {
        let [
            PartialSpanned(Token::Identifier(ident), span),
            remainder @ ..,
        ] = &tokens
        else {
            return Ok(None);
        };

        *tokens = remainder;
        Ok(Some(PartialSpanned(
            Self(GCString::new(parser.gc, &ident)),
            *span,
        )))
    }
}

impl Parse for Ident {
    impl_using_parse_left!();
}
