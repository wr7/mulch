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
