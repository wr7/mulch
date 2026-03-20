use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{DataEnum, DataStruct, DeriveInput};

use crate::util::FieldName;

pub fn derive_gc_eq(item: DeriveInput) -> syn::Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let body = match &item.data {
        syn::Data::Struct(data_struct) => gc_eq_struct_body(data_struct)?,
        syn::Data::Enum(data_enum) => gc_eq_enum_body(data_enum)?,
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site(),
                "#[derive(GCEq)] is not compatible with unions",
            ));
        }
    };

    let type_name = &item.ident;

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::gc::util::GCEq<#type_name #ty_generics> for #type_name #ty_generics #where_clause {
            unsafe fn gc_eq(&self, gc: &::mulch::gc::GarbageCollector, rhs: &Self) -> bool {
                #body
            }
        }
    })
}

fn gc_eq_struct_body(data: &DataStruct) -> syn::Result<TokenStream> {
    let per_field = data.fields.iter().enumerate().map(|(i, field)| {
        let field_name = field
            .ident
            .as_ref()
            .map_or(FieldName::Index(i), |id| FieldName::Name(id));

        quote! {
            if unsafe {::mulch::gc::util::GCEq::gc_ne(&self.#field_name, gc, &rhs.#field_name)} {
                return false;
            }
        }
    });

    Ok(quote! {
        #(#per_field)*

        true
    })
}

fn gc_eq_enum_body(data: &DataEnum) -> syn::Result<TokenStream> {
    let per_variant = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        let (lhs_fields, rhs_fields) = match &variant.fields {
            syn::Fields::Named(fields_named) => {
                let lhs_fields = fields_named
                    .named
                    .iter()
                    .map(|f| format_ident!("lhs_{}", f.ident.as_ref().unwrap()));

                let rhs_fields = fields_named
                    .named
                    .iter()
                    .map(|f| format_ident!("rhs_{}", f.ident.as_ref().unwrap()));

                (
                    Some(quote!({#(#lhs_fields),*})),
                    Some(quote!({#(#rhs_fields),*})),
                )
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let lhs_fields = fields_unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format_ident!("lhs_{i}"));

                let rhs_fields = fields_unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format_ident!("rhs_{i}"));

                (
                    Some(quote!((#(#lhs_fields),*))),
                    Some(quote!((#(#rhs_fields),*))),
                )
            }
            syn::Fields::Unit => (None, None),
        };

        let per_field_check = variant.fields.iter().enumerate().map(|(i, field)| {
            let lhs_name = field.ident.as_ref().map_or_else(
                || format_ident!("lhs_{i}"),
                |name| format_ident!("lhs_{name}"),
            );
            let rhs_name = field.ident.as_ref().map_or_else(
                || format_ident!("rhs_{i}"),
                |name| format_ident!("rhs_{name}"),
            );

            quote! {
                if unsafe {::mulch::gc::util::GCEq::gc_ne(#lhs_name, gc, #rhs_name)} {
                    return false;
                }
            }
        });

        quote! {
            (Self::#variant_name #lhs_fields, Self::#variant_name #rhs_fields) => {
                #(#per_field_check)*
                true
            }
        }
    });

    Ok(quote! {
        match (self, rhs) {
            #(#per_variant)*
            _ => false
        }
    })
}
