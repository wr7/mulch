use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    DataEnum, DataStruct, DeriveInput, Field, Fields, Token, punctuated::Punctuated,
    spanned::Spanned,
};

pub fn derive_gc_ptr(item: DeriveInput) -> TokenStream {
    let body = match &item.data {
        syn::Data::Struct(data_struct) => gcptr_fn_body_struct(data_struct),
        syn::Data::Enum(data_enum) => gcptr_fn_body_enum(data_enum),
        syn::Data::Union(_) => {
            quote! {compile_error!("`derive(GCPtr)` is not compatible with unions")}
        }
    };

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let type_name = &item.ident;

    let msb_reserved = match calculate_msb_reserved(&item) {
        Ok(value) => value,
        Err(value) => return value,
    };

    quote! {
        #[automatically_derived]
        unsafe impl #impl_generics ::mulch::gc::GCPtr for #type_name #ty_generics #where_clause {
            const MSB_RESERVED: bool = #msb_reserved;

            unsafe fn gc_copy(self, gc: &mut ::mulch::gc::GarbageCollector) -> Self {
                #body
            }
        }
    }
}

fn gcptr_fn_body_struct(data_struct: &DataStruct) -> TokenStream {
    match &data_struct.fields {
        Fields::Named(fields_named) => {
            let per_field = fields_named.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap();

                quote! {
                    #field_name: ::mulch::gc::GCPtr::gc_copy(self.#field_name, gc)
                }
            });
            quote! {
                Self {#(#per_field),*}
            }
        }
        Fields::Unnamed(fields_unnamed) => {
            let per_field = (0..fields_unnamed.unnamed.len())
                .map(|i| quote! {::mulch::gc::GCPtr::gc_copy(self.#i, gc)});

            quote! {
                Self(#(#per_field),*)
            }
        }
        Fields::Unit => quote! {},
    }
}

fn gcptr_fn_body_enum(data_enum: &DataEnum) -> TokenStream {
    let per_variant = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Named(fields_named) => {
                let per_field_input = fields_named.named.iter().map(|field| field.ident.as_ref().unwrap());
                let per_field_output = fields_named.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();

                    quote! {#field_name: ::mulch::gc::GCPtr::gc_copy(#field_name, gc)}
                });

                quote!{Self::#variant_name {#(#per_field_input),*} => Self::#variant_name{#(#per_field_output),*}}
            },
            Fields::Unnamed(fields_unnamed) => {
                let per_field_input = (0..fields_unnamed.unnamed.len()).map(|i| format_ident!("v{i}"));
                let per_field_output = per_field_input.clone().map(|field_name| quote! {::mulch::gc::GCPtr::gc_copy(#field_name, gc)});

                quote!{Self::#variant_name(#(#per_field_input),*) => Self::#variant_name(#(#per_field_output),*)}
            },
            Fields::Unit => quote! { Self::#variant_name() => self},
        }
    });

    quote! {match self {
        #(#per_variant),*
    }}
}

/// Calculates `GCPtr::MSB_RESERVED` for a type
fn calculate_msb_reserved(item: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let msb_reserved_attr = item.attrs.iter().find(|a| {
        a.meta
            .path()
            .get_ident()
            .is_some_and(|i| i == "msb_reserved")
    });

    let msb_reserved = if let Some(msb_reserved_attr) = msb_reserved_attr {
        let syn::Data::Enum(_) = &item.data else {
            return Err(quote_spanned! {
                msb_reserved_attr.span() => compile_error!{"#[msb_reserved] can only be used on enums. The most-significant-bit will be automatically reserved on applicable `repr(C)` structs"}
            });
        };

        let is_repr_usize = item.attrs.iter().any(|a| {
            a.meta.require_list().is_ok_and(|meta_list| {
                meta_list
                    .path
                    .get_ident()
                    .is_some_and(|ident| ident == "repr")
                    && meta_list
                        .tokens
                        .clone()
                        .into_iter()
                        .next()
                        .is_some_and(|t| t.to_string() == "usize")
            })
        });

        if !is_repr_usize {
            return Err(
                quote_spanned! { msb_reserved_attr.span() => compile_error!{"#[msb_reserved] can only be used on enums with #[repr(usize)]"}},
            );
        }

        quote! {true}
    } else {
        let is_repr_c = item.attrs.iter().any(|a| {
            a.meta.require_list().is_ok_and(|meta_list| {
                meta_list
                    .path
                    .get_ident()
                    .is_some_and(|ident| ident == "repr")
                    && meta_list
                        .tokens
                        .clone()
                        .into_iter()
                        .next()
                        .is_some_and(|t| t.to_string() == "C")
            })
        });

        if let syn::Data::Struct(struct_data) = &item.data
            && let Some(first_field) =
                get_struct_fields(&struct_data.fields).and_then(|f| f.first())
            && is_repr_c
        {
            let first_field_type = &first_field.ty;

            quote! { <#first_field_type as ::mulch::gc::GCPtr>::MSB_RESERVED }
        } else {
            quote! { false }
        }
    };
    Ok(msb_reserved)
}

fn get_struct_fields(fields: &Fields) -> Option<&Punctuated<Field, Token![,]>> {
    match fields {
        Fields::Named(fields_named) => Some(&fields_named.named),
        Fields::Unnamed(fields_unnamed) => Some(&fields_unnamed.unnamed),
        Fields::Unit => None,
    }
}
