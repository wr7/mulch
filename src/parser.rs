use crate::{error::PartialSpanned, lexer::Token};

pub mod ast;
pub mod error;
mod keyword;
mod traits;
mod util;

pub use traits::FindLeft;
pub use traits::FindRight;
pub use traits::Parse;
pub use traits::ParseLeft;
pub use traits::ParseRight;

pub use keyword::Keyword;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];
