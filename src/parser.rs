use mulch_macros::{GCDebug, GCEq, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::{GCString, GarbageCollector},
    lexer::Token,
    parser::{self, traits::single_token_parse_type},
};

pub mod ast;
mod bracketed;
pub mod error;
mod keyword;
mod punct;
mod separatedlist;
mod traits;
pub mod util;

// Macro helper function used for better error messages
#[doc(hidden)]
pub use util::run_parse_hook;

// Macro helper function used for better error messages
#[doc(hidden)]
pub use util::run_directional_parse_hook;

pub use bracketed::CurlyBracketed;
pub use bracketed::Parenthesized;
pub use bracketed::SquareBracketed;

pub use bracketed::Bracketed;
pub use punct::Punct;

/// The [`Punct`] type. Takes a string literal as input.
///
/// The string must be a valid symbol.
pub use mulch_macros::punct;
pub use separatedlist::SeparatedList;
pub use separatedlist::SeparatedListIter;

pub use traits::FindLeft;
pub use traits::FindRight;
pub use traits::Parse;
pub use traits::ParseLeft;
pub use traits::ParseRight;

pub use keyword::Keyword;

/// The [`Keyword`] type. Takes a string literal as input.
///
/// The string must be less than 16 characters long.
pub use mulch_macros::keyword;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

/// Contains parser state. Currently only contains a reference to the garbage collector, but that
/// will change if or when custom parsers are added.
pub struct Parser<'a> {
    #[allow(unused)]
    gc: &'a GarbageCollector,
}

impl<'a> Parser<'a> {
    pub fn new_default(gc: &'a GarbageCollector) -> Self {
        Self { gc }
    }
}

single_token_parse_type! {
    error_function = parser::error::expected_identifier;

    #[derive(Clone, Copy, GCPtr, GCDebug, GCEq)]
    #[debug_direct]
    pub struct Ident(pub GCString);

    |parser| {
        PartialSpanned(Token::Identifier(ident), _) => Self(GCString::new(parser.gc, ident))
    }
}

single_token_parse_type! {
    error_function = parser::error::expected_ident_or_string;

    #[derive(Clone, Copy, GCPtr, GCDebug, GCEq)]
    #[debug_direct]
    pub struct IdentOrString(pub GCString);

    |parser| {
        PartialSpanned(Token::Identifier(ident), _) => Self(GCString::new(parser.gc, ident)),
        PartialSpanned(Token::StringLiteral(lit), _) => Self(GCString::new(parser.gc, lit)),
    }
}
