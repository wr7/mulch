use copyspan::Span;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::GCString,
    lexer::Token,
    parser::{self, Parse, ParseLeft, traits::impl_using_parse_left},
};

#[derive(Clone, Copy, GCPtr, GCDebug)]
pub struct Ident {
    string: GCString,
    span: Span,
}

impl ParseLeft for Ident {
    fn parse_from_left(
        parser: &super::Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let [
            PartialSpanned(Token::Identifier(ident), span),
            remainder @ ..,
        ] = &tokens
        else {
            return Ok(None);
        };

        *tokens = remainder;
        Ok(Some(Self {
            string: GCString::new(parser.gc, &ident),
            span: *span,
        }))
    }
}

impl Parse for Ident {
    const EXPECTED_ERROR_FUNCTION: fn(Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_identifier;

    impl_using_parse_left!();
}
