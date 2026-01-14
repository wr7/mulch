use crate::gc::GarbageCollector;
use crate::{error::PartialSpanned, lexer::Token};

pub mod ast;
pub mod error;
mod keyword;
mod punct;
mod traits;
mod util;

pub use punct::Punct;

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
