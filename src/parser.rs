use crate::{
    error::{DResult, PartialSpanned},
    gc::GarbageCollector,
    lexer::Token,
};

pub mod ast;
mod keyword;

pub use crate::parser_old::error;

pub use keyword::Keyword;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

pub trait ParseLeft: Sized {
    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_left_mut(
        gc: &mut GarbageCollector,
        tokens: &mut &TokenStream,
    ) -> DResult<Option<Self>> {
        match Self::parse_from_left_imm(gc, *tokens) {
            Ok(Some((self_, remaining))) => {
                *tokens = remaining;
                Ok(Some(self_))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Attempts to parse `Self` from the left of a [`TokenStream`]. Returns `Self` and the
    /// remaining [`TokenStream`] upon success. The other method is typically more convenient for
    /// callers.
    fn parse_from_left_imm<'gc, 'a, 'src>(
        gc: &'gc mut GarbageCollector,
        tokens: &'a TokenStream<'src>,
    ) -> DResult<Option<(Self, &'a TokenStream<'src>)>>;
}

pub trait ParseRight: Sized + Parse {
    /// Attempts to parse `Self` from the right of a [`TokenStream`]. Writes the remaining
    /// `TokenStream` to `tokens` upon success.
    fn parse_from_right_mut(
        gc: &mut GarbageCollector,
        tokens: &mut &TokenStream,
    ) -> DResult<Option<Self>> {
        match Self::parse_from_right_imm(gc, *tokens) {
            Ok(Some((self_, remaining))) => {
                *tokens = remaining;
                Ok(Some(self_))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Attempts to parse `Self` from the right of a [`TokenStream`]. Returns `Self` and the
    /// remaining [`TokenStream`] upon success. The other method is typically more convenient for
    /// callers.
    fn parse_from_right_imm<'gc, 'a, 'src>(
        gc: &'gc mut GarbageCollector,
        tokens: &'a TokenStream<'src>,
    ) -> DResult<Option<(Self, &'a TokenStream<'src>)>>;
}

pub trait SplitLeft: Sized {
    /// Splits a [`TokenStream`] at the start of the leftmost occurance of `Self`. The first return
    /// value is everything until `Self`, and the second return value is `Self` and everything
    /// after.
    fn split_left<'a, 'src>(
        tokens: &'a TokenStream<'src>,
    ) -> DResult<(&'a TokenStream<'src>, &'a TokenStream<'src>)>;
}

pub trait SplitRight: Sized {
    /// Splits a [`TokenStream`] at the start of the rightmost occurance of `Self`. The first return
    /// value is everything until `Self`, and the second return value is `Self` and everything
    /// after.
    fn split_right<'a, 'src>(
        tokens: &'a TokenStream<'src>,
    ) -> DResult<(&'a TokenStream<'src>, &'a TokenStream<'src>)>;
}

pub trait Parse: Sized {
    /// Attempts to parse a whole [`TokenStream`] as `Self`.
    fn parse(gc: &mut GarbageCollector, tokens: &TokenStream) -> DResult<Option<Self>>;
}
