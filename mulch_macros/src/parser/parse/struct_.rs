use itertools::Itertools as _;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::{ParseTrait, util::FieldName};

pub fn derive_struct_fn_body(
    input: &DeriveInput,
    data: &syn::DataStruct,
    trait_: ParseTrait,
) -> syn::Result<TokenStream> {
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

        Ok(if next_field.is_some() || trait_ == ParseTrait::ParseLeft {
            let parse = if field.parse_until_next {
                let Some(next_field) = next_field else {
                    return Err(
                        syn::Error::new(
                            Span::call_site(),
                            "#[parse_until_next] cannot be applied to the last field of a struct"
                        )
                    );
                };

                parse_until_next_expr(field, next_field)
            } else {
                quote! {
                    <#field_type as ::mulch::parser::ParseLeft>::parse_from_left_with_span(parser, &mut tokens)?
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
            ParseTrait::ParseLeft => quote! {
                if let Some(res) = ::mulch::parser::run_left_parse_hook::<Self>(parser, &mut tokens, #hook)? {
                    return Ok(Some(res));
                }
            },
        })
    });

    per_field
        .process_results(|per_field| {
            per_hook.process_results(|per_hook| match trait_ {
                ParseTrait::Parse => quote! {
                    let mut tokens = tokens;
                    let prev_span: Option<::copyspan::Span> = None;

                    #(#per_hook)*

                    #(#per_field)*

                    Ok(Some(#struct_initializer))
                },
                ParseTrait::ParseLeft => quote! {
                    let mut tokens = *tokens_input;
                    let prev_span: Option<::copyspan::Span> = None;

                    #(#per_hook)*

                    #(#per_field)*

                    *tokens_input = tokens;

                    Ok(Some(#struct_initializer))
                },
            })
        })
        .flatten()
}

/// Generates an expression that parses a field with the attribute `#[parse_until_next]`
fn parse_until_next_expr(
    field: StructParseField<'_>,
    next_field: StructParseField<'_>,
) -> TokenStream {
    let next_type = &next_field.ty;
    let field_type = &field.ty;

    let find_left_else_branch = if next_field.error_if_not_found {
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

            let val = <#field_type as ::mulch::parser::Parse>::parse(parser, tokens_to_parse)?;

            if val.is_some() {
                tokens = remaining;
            }

            val.map(|val| (val, ::mulch::error::span_of(tokens_to_parse)))
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
