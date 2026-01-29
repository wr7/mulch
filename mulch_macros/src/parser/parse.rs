use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DataEnum, DataStruct, DeriveInput, ExprPath};

pub fn derive_parse(input: DeriveInput) -> syn::Result<TokenStream> {
    let body = match &input.data {
        syn::Data::Struct(data_struct) => derive_struct_fn_body(data_struct),
        syn::Data::Enum(data_enum) => derive_enum_fn_body(data_enum),
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
            let error_fn = attr.parse_args::<ExprPath>()?;

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

fn derive_enum_fn_body(data: &DataEnum) -> syn::Result<TokenStream> {
    let per_variant = data.variants.iter().map(|variant| {
        let mut fields = variant.fields.iter();
        let Some(field) = fields
            .next()
            .and_then(|field| (field.ident.is_none() && fields.next().is_none()).then_some(field))
        else {
            return Err(syn::Error::new(
                Span::call_site(),
                "Error deriving `Parse`: enum variants must have exactly one tuple-style field",
            ));
        };

        let field_ty = &field.ty;
        let variant_name = &variant.ident;

        Ok(quote! {
            if let Some(val) = <#field_ty as ::mulch::parser::Parse>::parse(parser, tokens)? {
                return Ok(Some(Self::#variant_name(val)))
            }
        })
    });

    per_variant.process_results(|per_variant| {
        quote! {
            #(#per_variant)*

            Ok(None)
        }
    })
}

fn derive_struct_fn_body(data: &DataStruct) -> syn::Result<TokenStream> {
    let per_field = data.fields.iter().enumerate().map(|(i, field)| {
        let field_name = field.ident.as_ref().ok_or_else(||
            syn::Error::new(
                Span::call_site(),
                "#[derive(Parse)] is not supported on tuple structs",
            )
        )?;

        let field_type = &field.ty;
        let error_if_not_found = field.attrs.iter().any(|attr| attr.path().is_ident("error_if_not_found"));

        let else_body = if error_if_not_found {
            quote! {
                let Some(span) = __mulch_prev_span.map(|span| span.span_after()).or_else(|| tokens.first().map(|t| t.1)) else {
                    return Ok(None);
                };

                return Err(<#field_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            }
        } else {
            quote! {
                return Ok(None);
            }
        };

        Ok(if i + 1 == data.fields.len() {
            quote! {
                let Some(#field_name) = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens)? else {
                    #else_body
                };
            }
        } else {
            quote! {
                let Some(PartialSpanned(#field_name, __mulch_prev_span)) = <#field_type as ::mulch::parser::ParseLeft>::parse_from_left(parser, &mut tokens)? else {
                    #else_body
                };

                let __mulch_prev_span = Some(__mulch_prev_span);
            }
        })
    });

    let field_names = data.fields.iter().map(|field| {
        field.ident.as_ref().ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "#[derive(Parse)] is not supported on tuple structs",
            )
        })
    });

    field_names
        .process_results(|field_names| {
            per_field.process_results(|per_field| {
                quote! {
                    let mut tokens = tokens;
                    let __mulch_prev_span: Option<::copyspan::Span> = None;

                    #(#per_field)*

                    Ok(Some(Self {#(#field_names),*}))
                }
            })
        })
        .flatten()
}
