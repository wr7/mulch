use proc_macro2::{Span, TokenStream};
use syn::DeriveInput;

use crate::gc_project::{enum_::derive_gc_project_enum, struct_::derive_gc_project_struct};

mod enum_;
mod struct_;

pub fn derive_gc_project(item: DeriveInput) -> syn::Result<TokenStream> {
    if let Some(lifetime) = item.generics.lifetimes().next() {
        return Err(syn::Error::new_spanned(
            &lifetime,
            "#[derive(GCProject)] does not support lifetime generics",
        ));
    }

    match &item.data {
        syn::Data::Struct(data_struct) => derive_gc_project_struct(&item, data_struct),
        syn::Data::Enum(data_enum) => derive_gc_project_enum(&item, data_enum),
        syn::Data::Union(_) => Err(syn::Error::new(
            Span::call_site(),
            "#[derive(GCProject)] cannot be used with unions",
        )),
    }
}
