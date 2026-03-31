use mulch_macros::{GCDebug, GCEq, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::{GCNumber, GCString},
    lexer::Token,
    parser::single_token_parse_type,
};

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug, GCEq)]
    #[debug_direct]
    pub struct StringLiteral(pub GCString);

    |parser| {
        PartialSpanned(Token::StringLiteral(lit), _) => Self(GCString::new(parser.gc, lit))
    }
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug, GCEq)]
    #[debug_direct]
    pub struct NumberLiteral(pub GCNumber);

    |parser| {
        PartialSpanned(Token::Number(lit), span) => Self(GCNumber::parse_from_literal(parser.gc, PartialSpanned(lit, *span))?)
    }
}
