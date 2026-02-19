use std::iter::Peekable;

use proc_macro2::TokenStream;
use quote::{IdentFragment, TokenStreamExt as _};

#[derive(Clone, Copy)]
pub enum FieldName<'a> {
    Name(&'a syn::Ident),
    Index(usize),
}

impl<'a> quote::ToTokens for FieldName<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldName::Name(ident) => ident.to_tokens(tokens),
            FieldName::Index(idx) => tokens.append(proc_macro2::Literal::usize_unsuffixed(*idx)),
        }
    }
}

impl<'a> IdentFragment for FieldName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldName::Name(ident) => write!(f, "{ident}"),
            FieldName::Index(idx) => write!(f, "{idx}"),
        }
    }
}

pub(crate) struct MapWithNext<I: Iterator, F> {
    inner: Peekable<I>,
    func: F,
}

impl<I, F, O> Iterator for MapWithNext<I, F>
where
    I: Iterator,
    F: FnMut(I::Item, Option<&I::Item>) -> O,
{
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.func)(self.inner.next()?, self.inner.peek()))
    }
}

/// Similar to `Iterator::map` except that it also supplies a reference to the next item
pub(crate) fn map_with_next<I, F, O>(iter: I, func: F) -> MapWithNext<I, F>
where
    I: Iterator,
    F: FnMut(I::Item, Option<&I::Item>) -> O,
{
    MapWithNext {
        inner: iter.peekable(),
        func,
    }
}
