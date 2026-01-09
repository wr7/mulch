use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

pub fn derive_gc_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let fn_body = match &input.data {
        syn::Data::Struct(data_struct) => gcdebug_fn_body_struct(&input, data_struct),
        syn::Data::Enum(data_enum) => gcdebug_fn_body_enum(data_enum),
        syn::Data::Union(_) => panic!("derive(GCDebug) is not compatible with unions"),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;

    quote! {
        #[automatically_derived]
        impl #impl_generics crate::gc::util::GCDebug for #type_name #ty_generics #where_clause {
            unsafe fn gc_debug(self, gc: &crate::gc::GarbageCollector, f: &mut ::core::fmt::Formatter) -> ::std::fmt::Result {#fn_body}
        }
    }
    .into()
}

/// Gets the derived GCDebug function body for an enum
fn gcdebug_fn_body_enum(data_enum: &syn::DataEnum) -> TokenStream {
    let variant_arms = data_enum.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_name = variant.ident.to_string();

        let code = match &variant.fields {
            syn::Fields::Named(fields_named) => {
                let per_field_in = fields_named
                    .named
                    .iter()
                    .map(|field| field.ident.clone().unwrap());

                let per_field_out = fields_named.named.iter().map(|field| {
                    let field_ident = field.ident.as_ref().unwrap();
                    let field_string = field_ident.to_string();

                    quote! {.field(#field_string, &crate::gc::util::GCWrap::new(#field_ident, gc))}
                });

                quote! {{#(#per_field_in),*} => f.debug_struct(#variant_name) #(#per_field_out)*}
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let per_field_in = (0..fields_unnamed.unnamed.len()).map(|i| format_ident!("v{i}"));
                let per_field_out = (0..fields_unnamed.unnamed.len()).map(|i| {
                    let field_name = format_ident!("v{i}");

                    quote! {.field(&crate::gc::util::GCWrap::new(#field_name, gc))}
                });
                quote! {(#(#per_field_in),*) => f.debug_tuple(#variant_name) #(#per_field_out)*}
            }
            syn::Fields::Unit => quote! {() => f.debug_tuple(#variant_name)},
        };

        quote!(Self::#variant_ident #code .finish())
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

    let per_field_code = data_struct.fields.iter().map(|field| {
        let Some(ident) = field.ident.as_ref() else {
            panic!("derive(GCDebug) does not have support for tuple structs")
        };

        let ident_string = ident.to_string();

        quote! {
            .field(#ident_string, &crate::gc::util::GCWrap::new(self.#ident, gc))
        }
    });

    quote! {
        unsafe {
            f.debug_struct(#struct_name_stringified)
                #(#per_field_code)*
                .finish()
        }
    }
}
