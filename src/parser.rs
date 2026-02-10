use crate::gc::GarbageCollector;
use crate::{error::PartialSpanned, lexer::Token};

pub mod ast;
mod bracketed;
pub mod error;
mod ident;
mod keyword;
mod punct;
mod separatedlist;
mod traits;
mod util;

// Macro helper function used for better error messages
#[doc(hidden)]
pub use util::run_parse_hook;

// Macro helper function used for better error messages
#[doc(hidden)]
pub use util::run_left_parse_hook;

pub use bracketed::CurlyBracketed;
pub use bracketed::Parenthesized;
pub use bracketed::SquareBracketed;

pub use bracketed::Bracketed;
pub use ident::Ident;
pub use punct::Punct;

/// The [`Punct`] type. Takes a string literal as input.
///
/// The string must be a valid symbol.
pub use mulch_macros::punct;
pub use separatedlist::SeparatedList;
pub use separatedlist::SeparatedListIter;

pub use traits::FindLeft;
pub use traits::Parse;
pub use traits::ParseLeft;

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
