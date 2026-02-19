use std::fmt::Display;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::DeriveInput;

mod enum_;
mod struct_;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseTrait {
    Parse,
    ParseLeft,
    ParseRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseDirection {
    Left,
    Right,
}

struct DeriveParseParameters {
    trait_: ParseTrait,
    direction: ParseDirection,
    error_fn: syn::Expr,
}

pub fn derive_parse(input: DeriveInput, trait_: ParseTrait) -> syn::Result<TokenStream> {
    let params = DeriveParseParameters::get(&input, trait_)?;
    let error_fn_name = trait_.error_fn_name();
    let error_fn = &params.error_fn;

    let body = match &input.data {
        syn::Data::Struct(data_struct) => {
            struct_::derive_struct_fn_body(&input, data_struct, &params)
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
        ParseTrait::ParseRight => quote! {
            fn parse_from_right(parser: &::mulch::parser::Parser, tokens_input: &mut &::mulch::parser::TokenStream) -> ::mulch::error::parse::PDResult<Option<Self>> {
                #body
            }
        },
    };

    let additional_parse_impl = match trait_ {
        ParseTrait::Parse => quote! {},
        ParseTrait::ParseLeft | ParseTrait::ParseRight => {
            let parse_fn_name = trait_.parse_fn_name();

            quote! {
                #[automatically_derived]
                impl #impl_generics ::mulch::parser::Parse for #type_name #ty_generics #where_clause {
                    const EXPECTED_ERROR_FUNCTION: fn(copyspan::Span) -> crate::error::parse::ParseDiagnostic =
                        <Self as ::mulch::parser::#trait_>::#error_fn_name;

                    fn parse(
                        parser: &::mulch::parser::Parser,
                        mut tokens: &::mulch::parser::TokenStream,
                    ) -> ::mulch::error::parse::PDResult<::core::option::Option<Self>> {
                        let Some(val) = <Self as ::mulch::parser::#trait_>::#parse_fn_name(parser, &mut tokens)? else {
                            return Ok(None);
                        };

                        if !tokens.is_empty() {
                            return Ok(None);
                        }

                        Ok(Some(val))
                    }
                }
            }
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::parser::#trait_ for #type_name #ty_generics #where_clause {
            const #error_fn_name: fn(::copyspan::Span) -> ::mulch::error::parse::ParseDiagnostic = #error_fn;

            #parse_function
        }

        #additional_parse_impl
    })
}

impl DeriveParseParameters {
    fn get(input: &DeriveInput, trait_: ParseTrait) -> syn::Result<Self> {
        let direction = input
            .attrs
            .iter()
            .find(|a| a.path().is_ident("parse_direction"))
            .map(|a| a.parse_args::<Ident>())
            .transpose()?
            .map(|d| match &*d.to_string() {
                "left" | "Left" => Ok(ParseDirection::Left),
                "right" | "Right" => Ok(ParseDirection::Right),
                _ => Err(syn::Error::new(d.span(), "Invalid parse direction {d}")),
            })
            .transpose()?
            .unwrap_or(ParseDirection::Left);

        let direction = match trait_ {
            ParseTrait::Parse => direction,
            ParseTrait::ParseLeft => ParseDirection::Left,
            ParseTrait::ParseRight => ParseDirection::Right,
        };

        let error_fn = input
            .attrs
            .iter()
            .find(|a| a.path().is_ident("mulch_parse_error"))
            .map(|a| a.parse_args::<syn::Expr>())
            .ok_or_else(|| {
                syn::Error::new(
                    Span::call_site(),
                    "No `mulch_parse_error` attribute found for `#[derive(Parse)]`",
                )
            })??;

        Ok(Self {
            trait_,
            direction,
            error_fn,
        })
    }
}

impl Display for ParseTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseTrait::Parse => write!(f, "Parse"),
            ParseTrait::ParseLeft => write!(f, "ParseLeft"),
            ParseTrait::ParseRight => write!(f, "ParseRight"),
        }
    }
}

impl ToTokens for ParseTrait {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let str = match self {
            ParseTrait::Parse => "Parse",
            ParseTrait::ParseLeft => "ParseLeft",
            ParseTrait::ParseRight => "ParseRight",
        };

        tokens.append(Ident::new(str, Span::call_site()));
    }
}

impl ParseTrait {
    fn error_fn_name(self) -> Ident {
        let str = match self {
            ParseTrait::Parse => "EXPECTED_ERROR_FUNCTION",
            ParseTrait::ParseLeft => "EXPECTED_ERROR_FUNCTION_LEFT",
            ParseTrait::ParseRight => "EXPECTED_ERROR_FUNCTION_RIGHT",
        };

        Ident::new(str, Span::call_site())
    }

    fn parse_fn_name(self) -> Ident {
        let str = match self {
            ParseTrait::Parse => "parse",
            ParseTrait::ParseLeft => "parse_from_left",
            ParseTrait::ParseRight => "parse_from_right",
        };

        Ident::new(str, Span::call_site())
    }
}

impl ParseDirection {
    fn parse_trait_name(self) -> Ident {
        let str = match self {
            ParseDirection::Left => "ParseLeft",
            ParseDirection::Right => "ParseRight",
        };

        Ident::new(str, Span::call_site())
    }

    fn find_trait_name(self) -> Ident {
        let str = match self {
            ParseDirection::Left => "FindLeft",
            ParseDirection::Right => "FindRight",
        };

        Ident::new(str, Span::call_site())
    }

    fn find_fn_name(self) -> Ident {
        let str = match self {
            ParseDirection::Left => "find_left",
            ParseDirection::Right => "find_right",
        };

        Ident::new(str, Span::call_site())
    }

    fn parse_with_span_fn_name(self) -> Ident {
        let str = match self {
            ParseDirection::Left => "parse_from_left_with_span",
            ParseDirection::Right => "parse_from_right_with_span",
        };

        Ident::new(str, Span::call_site())
    }
}
