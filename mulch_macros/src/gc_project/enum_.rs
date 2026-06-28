use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, DeriveInput};

pub fn derive_gc_project_enum(item: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let definition = derive_gc_project_enum_definition(item, data)?;
    let from_impl = derive_gc_project_enum_from_impl(item, data)?;
    let trait_impl = derive_gc_project_enum_trait(item, data)?;

    Ok(quote! {
        #[automatically_derived]
        const _: () = {
            #definition
            #from_impl
            #trait_impl
        };
    })
}

/// Generates:
/// ```
/// enum MyEnumProj<'a> {
///     /* Omitted */
/// }
/// ```
pub fn derive_gc_project_enum_definition(
    item: &DeriveInput,
    data: &DataEnum,
) -> syn::Result<TokenStream> {
    let vis = &item.vis;
    let projected_name = format_ident!("{}Proj", item.ident);

    let variants = data.variants.iter().map(|variant| {
        match &variant.fields {
            syn::Fields::Named(_) => return Err(
                syn::Error::new_spanned(
                    variant,
                    "#[derive(GCProject)] has not been implemented for enum variants with named fields"
                )
            ),
            _ => {}
        }

        let field_type = if let Some(field) = variant.fields.iter().next() {
            if variant.fields.len() > 1 {
                return Err(
                    syn::Error::new_spanned(
                        &variant,
                        "#[derive(GCProject)] has not been implemented for enum variants with multiple fields"
                    )
                );
            }

            let ty = &field.ty;
            quote!(::mulch::gc::safety::GC<'a, #ty>)
        } else {
            quote!(&'a ::mulch::gc::GarbageCollector)
        };

        let name = &variant.ident;

        Ok(quote! {
            #name(#field_type)
        })
    });

    let doc_comment = format!(
        "GC projection of [{}]. See [GCProject](::mulch::gc::GCProject) for more information.",
        &item.ident
    );

    variants.process_results(|variants| {
        quote! {
            #[doc = #doc_comment]
            #vis enum #projected_name<'a> {
                #(#variants),*
            }
        }
    })
}

/// Generates:
/// ```
/// impl<'a> From<MyEnumProj<'a>> for GC<'a, MyEnum> {
///     /* Omitted */
/// }
/// ```
pub fn derive_gc_project_enum_from_impl(
    item: &DeriveInput,
    data: &DataEnum,
) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let projected_name = format_ident!("{name}Proj");

    let match_arms = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        let rhs = if variant.fields.len() == 1 {
            quote! {
                let gc = ::mulch::gc::safety::GC::gc(&value);

                ::mulch::gc::safety::GC::from_raw_parts(
                    gc,
                    #name::#variant_name(::mulch::gc::safety::GC::raw(value))
                )
            }
        } else {
            if matches!(&variant.fields, syn::Fields::Unit) {
                quote!(::mulch::gc::safety::GC::from_raw_parts(value, #name::#variant_name))
            } else {
                quote!(::mulch::gc::safety::GC::from_raw_parts(value, #name::#variant_name()))
            }
        };

        quote! {
            #projected_name::#variant_name(value) => unsafe {#rhs}
        }
    });

    Ok(quote! {
        impl<'a> ::core::convert::From<#projected_name<'a>> for ::mulch::gc::safety::GC<'a, #name> {
            fn from(value: #projected_name<'a>) -> Self {
                match value {
                    #(#match_arms),*
                }
            }
        }
    })
}

pub fn derive_gc_project_enum_trait(
    item: &DeriveInput,
    data: &DataEnum,
) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let projected_name = format_ident!("{name}Proj");

    let arms = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        if variant.fields.is_empty() {
            let lhs_arg = (!matches!(&variant.fields, syn::Fields::Unit)).then(|| quote![()]);

            quote! {
                #name::#variant_name #lhs_arg => #projected_name::#variant_name(gc)
            }
        } else {
            quote! {
                #name::#variant_name(value) => unsafe {
                    #projected_name::#variant_name(::mulch::gc::safety::GC::from_raw_parts(gc, value))
                }
            }
        }
    });

    Ok(quote! {
        impl<'a> ::mulch::gc::GCProject<'a> for #name {
            type Projected = #projected_name<'a>;

            fn project(value: ::mulch::gc::safety::GC<'a, Self>) -> #projected_name<'a> {
                let gc = ::mulch::gc::safety::GC::gc(&value);

                match ::mulch::gc::safety::GC::raw(value) {
                    #(#arms),*
                }
            }
        }
    })
}
