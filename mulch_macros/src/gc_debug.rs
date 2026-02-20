use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{DeriveInput, spanned::Spanned};

use crate::util::FieldName;

pub fn derive_gc_debug(input: DeriveInput) -> syn::Result<TokenStream> {
    let fn_body = match &input.data {
        syn::Data::Struct(data_struct) => gcdebug_fn_body_struct(&input, data_struct)?,
        syn::Data::Enum(data_enum) => gcdebug_fn_body_enum(data_enum),
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "derive(GCDebug) is not compatible with unions",
            ));
        }
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::gc::util::GCDebug for #type_name #ty_generics #where_clause {
            unsafe fn gc_debug(self, gc: &::mulch::gc::GarbageCollector, f: &mut ::core::fmt::Formatter) -> ::std::fmt::Result {#fn_body}
        }
    })
}

/// Gets the derived GCDebug function body for an enum
fn gcdebug_fn_body_enum(data_enum: &syn::DataEnum) -> TokenStream {
    let variant_arms = data_enum.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_name = variant.ident.to_string();

        let debug_direct = variant
            .attrs
            .iter()
            .find(|a| {
                a.meta
                    .path()
                    .get_ident()
                    .is_some_and(|i| i == "debug_direct")
            })
            ;

        let code = if let Some(debug_direct) = debug_direct{
            if
                let syn::Fields::Unnamed(fields) = &variant.fields
                && let Some(_field) = fields.unnamed.first()
                && fields.unnamed.len() == 1
            {
                quote! { (v0) => ::mulch::gc::util::GCDebug::gc_debug(v0, gc, f) }
            } else {
                return quote_spanned! { debug_direct.span() => compile_error!("#[debug_direct] is only supported on single-value tuple variants")}
            }
        } else {match &variant.fields {
            syn::Fields::Named(fields_named) => {
                let per_field_in = fields_named
                    .named
                    .iter()
                    .map(|field| field.ident.clone().unwrap());

                let per_field_out = fields_named.named.iter().map(|field| {
                    let field_ident = field.ident.as_ref().unwrap();
                    let field_string = field_ident.to_string();

                    quote! {.field(#field_string, &::mulch::gc::util::GCWrap::new(#field_ident, gc))}
                });

                quote! {{#(#per_field_in),*} => f.debug_struct(#variant_name) #(#per_field_out)*.finish()}
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let per_field_in = (0..fields_unnamed.unnamed.len()).map(|i| format_ident!("v{i}"));
                let per_field_out = (0..fields_unnamed.unnamed.len()).map(|i| {
                    let field_name = format_ident!("v{i}");

                    quote! {.field(&::mulch::gc::util::GCWrap::new(#field_name, gc))}
                });
                quote! {(#(#per_field_in),*) => f.debug_tuple(#variant_name) #(#per_field_out)*.finish()}
            }
            syn::Fields::Unit => quote! {() => f.debug_tuple(#variant_name).finish()},
        }};

        quote!(Self::#variant_ident #code)
    });

    quote! {
        unsafe {
            match self {
                #(#variant_arms),*
            }
        }
    }
}

/// Gets the derived GCDebug function body for a struct
fn gcdebug_fn_body_struct(
    input: &DeriveInput,
    data_struct: &syn::DataStruct,
) -> syn::Result<TokenStream> {
    let struct_name_stringified = input.ident.to_string();

    let is_tuple_struct = data_struct
        .fields
        .iter()
        .next()
        .is_none_or(|field| field.ident.is_none());

    let debug_direct_with_name = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("debug_direct_with_name"));

    let debug_direct = debug_direct_with_name.or_else(|| {
        input
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("debug_direct"))
    });

    if let Some(debug_direct) = debug_direct {
        let mut iter = DebugFieldIter::new(&data_struct.fields);

        if let Some(field) = iter.next()
            && iter.next().is_none()
        {
            let name = field.name;
            let field_access = quote! { self.#name };

            let with_name = debug_direct_with_name.map(|_| {
                let mut struct_name = input.ident.to_string();
                struct_name.push(' ');

                quote! {::std::write!(f, #struct_name)?;}
            });

            return Ok(quote! {
                #with_name
                ::mulch::gc::util::GCDebug::gc_debug(#field_access, gc, f)
            });
        } else {
            return Err(syn::Error::new(
                debug_direct.path().span(),
                "#[debug_direct] is only supported on structs/variants with one field",
            ));
        }
    }

    let per_field_code = DebugFieldIter::new(&data_struct.fields).map(|field| {
        let name = field.name;

        if is_tuple_struct {
            Ok(quote! {
                .field(&::mulch::gc::util::GCWrap::new(self.#name, gc))
            })
        } else {
            let FieldName::Name(ident) = field.name else {
                return Err(syn::Error::new(
                    field.span,
                    "Field on non tuple struct is missing a name",
                ));
            };

            let ident_string = ident.to_string();

            Ok(quote! {
                .field(#ident_string, &::mulch::gc::util::GCWrap::new(self.#ident, gc))
            })
        }
    });

    per_field_code.process_results(|per_field_code| {
        if is_tuple_struct {
            quote! {
                unsafe {
                    f.debug_tuple(#struct_name_stringified)
                        #(#per_field_code)*
                        .finish()
                }
            }
        } else {
            quote! {
                unsafe {
                    f.debug_struct(#struct_name_stringified)
                        #(#per_field_code)*
                        .finish()
                }
            }
        }
    })
}

struct DebugFieldIter<'a> {
    fields: std::iter::Enumerate<syn::punctuated::Iter<'a, syn::Field>>,
}

struct DebugField<'a> {
    name: FieldName<'a>,
    span: proc_macro2::Span,
}

impl<'a> DebugFieldIter<'a> {
    pub fn new(fields: &'a syn::Fields) -> Self {
        Self {
            fields: fields.iter().enumerate(),
        }
    }
}

impl<'a> Iterator for DebugFieldIter<'a> {
    type Item = DebugField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, field) = self.fields.next()?;

            if field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("debug_hidden"))
            {
                continue;
            }

            let name = if let Some(name) = field.ident.as_ref() {
                FieldName::Name(name)
            } else {
                FieldName::Index(i)
            };

            return Some(DebugField {
                name,
                span: field.span(),
            });
        }
    }
}
