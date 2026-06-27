use proc_macro2::{Span, TokenStream};
use syn::DeriveInput;

use crate::gc_project::struct_::derive_gc_project_struct;

mod struct_;

pub fn derive_gc_project(item: DeriveInput) -> syn::Result<TokenStream> {
    match &item.data {
        syn::Data::Struct(data_struct) => derive_gc_project_struct(&item, data_struct),
        syn::Data::Enum(data_enum) => todo!(),
        syn::Data::Union(data_union) => Err(syn::Error::new(
            Span::call_site(),
            "#[derive(GCProject)] cannot be used with unions",
        )),
    }
}
