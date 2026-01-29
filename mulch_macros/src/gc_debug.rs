use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{DeriveInput, Index, spanned::Spanned};

pub fn derive_gc_debug(input: DeriveInput) -> TokenStream {
    let fn_body = match &input.data {
        syn::Data::Struct(data_struct) => gcdebug_fn_body_struct(&input, data_struct),
        syn::Data::Enum(data_enum) => gcdebug_fn_body_enum(data_enum),
        syn::Data::Union(_) => panic!("derive(GCDebug) is not compatible with unions"),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;

    quote! {
        #[automatically_derived]
        impl #impl_generics ::mulch::gc::util::GCDebug for #type_name #ty_generics #where_clause {
            unsafe fn gc_debug(self, gc: &::mulch::gc::GarbageCollector, f: &mut ::core::fmt::Formatter) -> ::std::fmt::Result {#fn_body}
        }
    }
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
fn gcdebug_fn_body_struct(input: &DeriveInput, data_struct: &syn::DataStruct) -> TokenStream {
    let struct_name_stringified = input.ident.to_string();

    let is_tuple_struct = data_struct
        .fields
        .iter()
        .next()
        .is_none_or(|field| field.ident.is_none());

    let per_field_code = data_struct.fields.iter().enumerate().map(|(i, field)| {
        if is_tuple_struct {
            let i = Index::from(i);

            quote! {
                .field(&::mulch::gc::util::GCWrap::new(self.#i, gc))
            }
        } else {
            let Some(ident) = field.ident.as_ref() else {
                panic!("Field on non tuple struct is missing a name")
            };

            let ident_string = ident.to_string();

            quote! {
                .field(#ident_string, &::mulch::gc::util::GCWrap::new(self.#ident, gc))
            }
        }
    });

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
}
