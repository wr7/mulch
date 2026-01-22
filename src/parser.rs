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

pub use bracketed::CurlyBracketed;
pub use bracketed::Parenthesized;
pub use bracketed::SquareBracketed;

pub use bracketed::Bracketed;
pub use ident::Ident;
pub use punct::Punct;
pub use separatedlist::SeparatedList;
pub use separatedlist::SeparatedListIter;

pub use traits::FindLeft;
pub use traits::FindRight;
pub use traits::Parse;
pub use traits::ParseLeft;
pub use traits::ParseRight;

pub use keyword::Keyword;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

/// Contains parser state. Currently only contains a reference to the garbage collector, but that
/// will change if or when custom parsers are added.
pub struct Parser<'a> {
    #[allow(unused)]
    gc: &'a GarbageCollector,
}
