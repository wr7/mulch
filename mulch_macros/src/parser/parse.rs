use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Expr};

mod enum_;
mod struct_;

pub fn derive_parse(input: DeriveInput) -> syn::Result<TokenStream> {
    let body = match &input.data {
        syn::Data::Struct(data_struct) => struct_::derive_struct_fn_body(data_struct),
        syn::Data::Enum(data_enum) => enum_::derive_enum_fn_body(data_enum),
        syn::Data::Union(_) => Err(syn::Error::new(
            Span::call_site(),
            "#[derive(Parse)] is not supported for unions",
        )),
    }?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;

    let error_function = get_error_function(&input)?;

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::parser::Parse for #type_name #ty_generics #where_clause {
            #error_function

            fn parse(parser: &::mulch::parser::Parser, tokens: &::mulch::parser::TokenStream) -> ::mulch::error::parse::PDResult<Option<Self>> {
                #body
            }
        }
    })
}

fn get_error_function(input: &DeriveInput) -> syn::Result<TokenStream> {
    for attr in &input.attrs {
        if attr.path().is_ident("mulch_parse_error") {
            let error_fn = attr.parse_args::<Expr>()?;

            return Ok(quote! {
                const EXPECTED_ERROR_FUNCTION: fn(::copyspan::Span) -> ::mulch::error::parse::ParseDiagnostic = #error_fn;
            });
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "No `mulch_parse_error` attribute found for `#[derive(Parse)]`",
    ))
}
