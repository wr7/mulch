use itertools::Itertools as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::util::FieldName;

pub fn derive_struct_fn_body(data: &syn::DataStruct) -> syn::Result<TokenStream> {
    let per_field = StructParseFieldIterator::new(data.fields.iter()).enumerate().map(|(i, field)| {
        let field_var_name = format_ident!("field_{}", field.name);
        let field_type = field.ty;

        let else_body = if field.error_if_not_found {
            quote! {
                let Some(span) = tokens.first().map(|t| t.1.span_at()).or_else(|| prev_span.map(|span| span.span_after())) else {
                    return Ok(None);
                };

                return Err(<#field_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
            }
        } else {
            quote! {
                return Ok(None);
            }
        };

        let next_field = StructParseFieldIterator::new(data.fields.iter()).nth(i + 1);

        Ok(if let Some(next_field) = next_field {
            let parse = if field.parse_until_next {
                parse_until_next_expr(field, next_field)
            } else {
                quote! {
                    <#field_type as ::mulch::parser::ParseLeft>::parse_from_left(parser, &mut tokens)?
                }
            };

            quote! {
                let Some(::mulch::error::PartialSpanned(#field_var_name, prev_span)) = #parse else {
                    #else_body
                };

                let prev_span = Some(prev_span);
            }
        } else {
            quote! {
                let Some(#field_var_name) = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens)? else {
                    #else_body
                };
            }
        })
    });

    let field_names =
        StructParseFieldIterator::new(data.fields.iter()).map(|field| match field.name {
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

    per_field.process_results(|per_field| {
        quote! {
            let mut tokens = tokens;
            let prev_span: Option<::copyspan::Span> = None;

            #(#per_field)*

            Ok(Some(#struct_initializer))
        }
    })
}

/// Generates an expression that parses a field with the attribute `#[parse_until_next_expr]`
fn parse_until_next_expr(
    field: StructParseField<'_>,
    next_field: StructParseField<'_>,
) -> TokenStream {
    let next_type = &next_field.ty;
    let field_type = &field.ty;

    let find_left_else_branch = if field.error_if_not_found {
        quote! {
            let Some(span) = tokens.last().map(|t| t.1.span_after()).or_else(|| prev_span.map(|span| span.span_after())) else {
                return Ok(None);
            };

            return Err(<#next_type as ::mulch::parser::Parse>::EXPECTED_ERROR_FUNCTION(span));
        }
    } else {
        quote! { return Ok(None) }
    };

    quote! {
        ({
            let Some(range) = <#next_type as ::mulch::parser::FindLeft>::find_left(parser, tokens)? else {
                #find_left_else_branch
            };

            let (tokens_to_parse, remaining) = tokens.split_at(range.start);

            let val = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens_to_parse)?
                .and_then(|val| {
                    Some(PartialSpanned(val, ::mulch::error::span_of(tokens_to_parse).or(prev_span.map(|span| span.span_after()))?))
                });

            if val.is_some() {
                tokens = remaining;
            }

            val
        })
    }
}

struct StructParseField<'a> {
    /// Whether or not the field has `#[error_if_not_found]`
    error_if_not_found: bool,
    /// Whether or not the field has `#[parse_until_next]`
    parse_until_next: bool,
    ty: &'a syn::Type,
    name: FieldName<'a>,
}

struct StructParseFieldIterator<'a> {
    fields: std::iter::Enumerate<syn::punctuated::Iter<'a, syn::Field>>,
}

impl<'a> StructParseFieldIterator<'a> {
    pub fn new(fields: syn::punctuated::Iter<'a, syn::Field>) -> Self {
        Self {
            fields: fields.enumerate(),
        }
    }
}

impl<'a> Iterator for StructParseFieldIterator<'a> {
    type Item = StructParseField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }

    fn nth(&mut self, idx: usize) -> Option<Self::Item> {
        let (i, field) = self.fields.nth(idx)?;

        let name = field
            .ident
            .as_ref()
            .map_or_else(|| FieldName::Index(i), |ident| FieldName::Name(ident));

        let error_if_not_found = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("error_if_not_found"));

        let parse_until_next = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("parse_until_next"));

        Some(StructParseField {
            error_if_not_found,
            parse_until_next,
            ty: &field.ty,
            name,
        })
    }
}
