use std::fmt::Display;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{DeriveInput, Expr};

mod enum_;
mod struct_;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseTrait {
    Parse,
    ParseLeft,
}

pub fn derive_parse(input: DeriveInput, trait_: ParseTrait) -> syn::Result<TokenStream> {
    let body = match &input.data {
        syn::Data::Struct(data_struct) => {
            struct_::derive_struct_fn_body(&input, data_struct, trait_)
        }
        syn::Data::Enum(data_enum) => {
            if trait_ == ParseTrait::ParseLeft {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "#[derive(ParseLeft)] has not been implemented for enums",
                ));
            }

            enum_::derive_enum_fn_body(data_enum)
        }
        syn::Data::Union(_) => Err(syn::Error::new(
            Span::call_site(),
            format!("#[derive({trait_})] is not supported for unions"),
        )),
    }?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;

    let error_function = get_error_function(&input, trait_)?;

    let parse_function = match trait_ {
        ParseTrait::Parse => quote! {
            fn parse(parser: &::mulch::parser::Parser, tokens: &::mulch::parser::TokenStream) -> ::mulch::error::parse::PDResult<Option<Self>> {
                #body
            }
        },
        ParseTrait::ParseLeft => quote! {
            fn parse_from_left(parser: &::mulch::parser::Parser, tokens_input: &mut &::mulch::parser::TokenStream) -> ::mulch::error::parse::PDResult<Option<Self>> {
                #body
            }
        },
    };

    let additional_parse_impl = match trait_ {
        ParseTrait::Parse => quote! {},
        ParseTrait::ParseLeft => quote! {
            #[automatically_derived]
            impl #impl_generics ::mulch::parser::Parse for #type_name #ty_generics #where_clause {
                const EXPECTED_ERROR_FUNCTION: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
                    <Self as ::mulch::parser::ParseLeft>::EXPECTED_ERROR_FUNCTION_LEFT;

                fn parse(
                    parser: &::mulch::parser::Parser,
                    mut tokens: &::mulch::parser::TokenStream,
                ) -> ::mulch::error::parse::PDResult<::core::option::Option<Self>> {
                    let Some(val) = <Self as ::mulch::parser::ParseLeft>::parse_from_left(parser, &mut tokens)? else {
                        return Ok(None);
                    };

                    if !tokens.is_empty() {
                        return Ok(None);
                    }

                    Ok(Some(val))
                }
            }
        },
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::parser::#trait_ for #type_name #ty_generics #where_clause {
            #error_function

            #parse_function
        }

        #additional_parse_impl
    })
}

fn get_error_function(input: &DeriveInput, trait_: ParseTrait) -> syn::Result<TokenStream> {
    for attr in &input.attrs {
        if attr.path().is_ident("mulch_parse_error") {
            let error_fn = attr.parse_args::<Expr>()?;

            let error_fn_name = trait_.error_fn_name();

            return Ok(quote! {
                const #error_fn_name: fn(::copyspan::Span) -> ::mulch::error::parse::ParseDiagnostic = #error_fn;
            });
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "No `mulch_parse_error` attribute found for `#[derive(Parse)]`",
    ))
}

impl Display for ParseTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseTrait::Parse => write!(f, "Parse"),
            ParseTrait::ParseLeft => write!(f, "ParseLeft"),
        }
    }
}

impl ToTokens for ParseTrait {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let str = match self {
            ParseTrait::Parse => "Parse",
            ParseTrait::ParseLeft => "ParseLeft",
        };

        tokens.append(proc_macro2::Ident::new(str, Span::call_site()));
    }
}

impl ParseTrait {
    fn error_fn_name(self) -> proc_macro2::Ident {
        let str = match self {
            ParseTrait::Parse => "EXPECTED_ERROR_FUNCTION",
            ParseTrait::ParseLeft => "EXPECTED_ERROR_FUNCTION_LEFT",
        };

        proc_macro2::Ident::new(str, Span::call_site())
    }
}
