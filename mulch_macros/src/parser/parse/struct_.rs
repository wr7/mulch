use itertools::{Either, Itertools as _};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::{
    ParseTrait,
    parser::parse::{DeriveParseParameters, ParseDirection},
    util::{FieldName, map_with_next},
};

pub fn derive_struct_fn_body(
    input: &DeriveInput,
    data: &syn::DataStruct,
    params: &DeriveParseParameters,
) -> syn::Result<TokenStream> {
    let trait_ = params.trait_;

    let per_field = data
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| parse_struct_field(i, f));

    let per_field = if params.direction == ParseDirection::Left {
        Either::Left(per_field)
    } else {
        Either::Right(per_field.rev())
    };

    let per_field = map_with_next(per_field, |field, next_field| {
        generate_field_parsing_code(params, trait_, field, next_field)
    });

    let field_names = data
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| parse_struct_field(i, f))
        .map(|field| match field.name {
            FieldName::Name(ident) => {
                let var_name = format_ident!("field_{}", ident);
                let field_name = field.name;

                quote! {#field_name: #var_name}
            }
            FieldName::Index(idx) => {
                let var_name = format_ident!("field_{}", idx);
                quote! {#var_name}
            }
        });

    let is_tuple_struct = matches!(data.fields, syn::Fields::Unnamed(_) | syn::Fields::Unit);
    let struct_initializer = if is_tuple_struct {
        quote! { Self ( #(#field_names),* ) }
    } else {
        quote! { Self { #(#field_names),* } }
    };

    let hooks = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("parse_hook"))
        .map(|attr| attr.parse_args::<syn::Expr>());

    let per_hook = hooks.map(|hook| -> syn::Result<TokenStream> {
        let hook = hook?;

        Ok(match trait_ {
            ParseTrait::Parse => quote! {
                if let Some(res) = ::mulch::parser::run_parse_hook::<Self>(parser, tokens, #hook)? {
                    return Ok(Some(res));
                }
            },
            ParseTrait::ParseLeft | ParseTrait::ParseRight => quote! {
                if let Some(res) = ::mulch::parser::run_directional_parse_hook::<Self>(parser, &mut tokens, #hook)? {
                    return Ok(Some(res));
                }
            },
        })
    });

    per_field.process_results(|per_field| {
        per_hook.process_results(|per_hook| match trait_ {
            ParseTrait::Parse => quote! {
                let mut tokens = tokens;
                let prev_span: Option<::copyspan::Span> = None;

                #(#per_hook)*

                #(#per_field)*

                Ok(Some(#struct_initializer))
            },
            ParseTrait::ParseLeft | ParseTrait::ParseRight => quote! {
                let mut tokens = *tokens_input;
                let prev_span: Option<::copyspan::Span> = None;

                #(#per_hook)*

                #(#per_field)*

                *tokens_input = tokens;

                Ok(Some(#struct_initializer))
            },
        })
    })?
}

/// Generates the code to parse a single field
fn generate_field_parsing_code(
    params: &DeriveParseParameters,
    trait_: ParseTrait,
    field: StructParseField<'_>,
    next_field: Option<&StructParseField<'_>>,
) -> Result<TokenStream, syn::Error> {
    let field_var_name = format_ident!("field_{}", field.name);
    let field_type = field.ty;

    let else_body = if field.error_if_not_found {
        match params.direction {
            ParseDirection::Left => quote! {
                let Some(span) = tokens.first().map(|t| t.1.span_at()).or_else(|| prev_span.map(|span| span.span_after())) else {
                    return Ok(None);
                };

                return Err(<#field_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            },
            ParseDirection::Right => quote! {
                let Some(span) = tokens.last().map(|t| t.1.span_after()).or_else(|| prev_span.map(|span| span.span_at())) else {
                    return Ok(None);
                };

                return Err(<#field_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            },
        }
    } else {
        quote! {
            return Ok(None);
        }
    };

    Ok(if next_field.is_some() || trait_ != ParseTrait::Parse {
        let parse = if field.parse_until_next {
            let Some(&next_field) = next_field else {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "#[parse_until_next] cannot be applied to the last field of a struct",
                ));
            };

            parse_until_next_expr(field, next_field, params)
        } else {
            let dir_trait = params.direction.parse_trait_name();
            let fn_name = params.direction.parse_with_span_fn_name();

            quote! {
                <#field_type as ::mulch::parser::#dir_trait>::#fn_name(parser, &mut tokens)?
            }
        };

        quote! {
            let Some((#field_var_name, span)) = #parse else {
                #else_body
            };

            let prev_span = span.or(prev_span);
        }
    } else {
        quote! {
            let Some(#field_var_name) = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens)? else {
                #else_body
            };
        }
    })
}

/// Generates an expression that parses a field with the attribute `#[parse_until_next]`
fn parse_until_next_expr(
    field: StructParseField<'_>,
    next_field: StructParseField<'_>,
    params: &DeriveParseParameters,
) -> TokenStream {
    let next_type = &next_field.ty;
    let field_type = &field.ty;

    let find_else_branch = if next_field.error_if_not_found {
        if params.direction == ParseDirection::Left {
            quote! {
                let Some(span) = tokens.last().map(|t| t.1.span_after()).or_else(|| prev_span.map(|span| span.span_after())) else {
                    return Ok(None);
                };

                return Err(<#next_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            }
        } else {
            quote! {
                let Some(span) = tokens.first().map(|t| t.1.span_at()).or_else(|| prev_span.map(|span| span.span_before())) else {
                    return Ok(None);
                };

                return Err(<#next_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            }
        }
    } else {
        quote! { return Ok(None) }
    };

    let find_trait = params.direction.find_trait_name();
    let find_fn = params.direction.find_fn_name();

    let split_code = match params.direction {
        ParseDirection::Left => quote! {
            let (tokens_to_parse, remaining) = tokens.split_at(range.start);
        },
        ParseDirection::Right => quote! {
            let (remaining, tokens_to_parse) = tokens.split_at(range.end);
        },
    };

    quote! {
        ({
            let Some(range) = <#next_type as ::mulch::parser::#find_trait>::#find_fn(parser, tokens)? else {
                #find_else_branch
            };

            #split_code

            let val = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens_to_parse)?;

            if val.is_some() {
                tokens = remaining;
            }

            val.map(|val| (val, ::mulch::error::span_of(tokens_to_parse)))
        })
    }
}

#[derive(Clone, Copy)]
struct StructParseField<'a> {
    /// Whether or not the field has `#[error_if_not_found]`
    error_if_not_found: bool,
    /// Whether or not the field has `#[parse_until_next]`
    parse_until_next: bool,
    ty: &'a syn::Type,
    name: FieldName<'a>,
}

fn parse_struct_field<'a>(idx: usize, field: &'a syn::Field) -> StructParseField<'a> {
    let name = field
        .ident
        .as_ref()
        .map_or_else(|| FieldName::Index(idx), |ident| FieldName::Name(ident));

    let error_if_not_found = field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("error_if_not_found"));

    let parse_until_next = field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("parse_until_next"));

    StructParseField {
        error_if_not_found,
        parse_until_next,
        ty: &field.ty,
        name,
    }
}
