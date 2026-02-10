use std::iter::Peekable;

use itertools::Itertools as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, spanned::Spanned as _};

pub fn derive_enum_fn_body(data: &DataEnum) -> syn::Result<TokenStream> {
    let per_rule = EnumParseRuleIterator::new(&data.variants).map(|rule| {
        let rule = rule?;

        Ok(
            match rule {
                EnumParseRule::Hook(hook) => quote! {
                    if let Some(val) = ::mulch::parser::run_parse_hook::<Self>(parser, tokens, #hook)? {
                        return Ok(Some(val))
                    }
                },
                EnumParseRule::Variant(variant) => {
                    let field_ty = variant.ty;
                    let variant_name = variant.name;

                    quote! {
                        if let Some(val) = <#field_ty as ::mulch::parser::Parse>::parse(parser, tokens)? {
                            return Ok(Some(Self::#variant_name(val)))
                        }
                    }
                },
            }
        )
    });

    per_rule.process_results(|per_variant| {
        quote! {
            #(#per_variant)*

            Ok(None)
        }
    })
}

enum EnumParseRule<'a> {
    Hook(syn::Expr),
    Variant(EnumParseVariant<'a>),
}

struct EnumParseVariant<'a> {
    ty: &'a syn::Type,
    name: &'a syn::Ident,
}

struct EnumParseRuleIterator<'a> {
    variants: Peekable<syn::punctuated::Iter<'a, syn::Variant>>,
    attr_idx: usize,
}

impl<'a> EnumParseRuleIterator<'a> {
    pub fn new<P>(variants: &'a syn::punctuated::Punctuated<syn::Variant, P>) -> Self {
        Self {
            variants: variants.iter().peekable(),
            attr_idx: 0,
        }
    }
}

impl<'a> Iterator for EnumParseRuleIterator<'a> {
    type Item = syn::Result<EnumParseRule<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let variant = *self.variants.peek()?;

            if let Some(attr) = variant.attrs.get(self.attr_idx) {
                self.attr_idx += 1;

                if attr.path().is_ident("parse_hook") {
                    match attr.parse_args::<syn::Expr>() {
                        Ok(hook) => return Some(Ok(EnumParseRule::Hook(hook))),
                        Err(err) => return Some(Err(err)),
                    }
                } else {
                    continue;
                }
            }

            let _ = self.variants.next();

            return if let syn::Fields::Unnamed(fields) = &variant.fields
                && let Some(field) = fields.unnamed.first()
                && fields.unnamed.len() == 1
            {
                Some(Ok(EnumParseRule::Variant(EnumParseVariant {
                    ty: &field.ty,
                    name: &variant.ident,
                })))
            } else {
                Some(Err(syn::Error::new(
                    variant.span(),
                    "Error deriving `Parse`: enum variants must have exactly one tuple-style field",
                )))
            };
        }
    }
}
