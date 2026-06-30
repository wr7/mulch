use itertools::Itertools as _;
use proc_macro2::{Span, TokenStream};
use syn::{DataStruct, DeriveInput, Lifetime, LifetimeParam};

use quote::{ToTokens, format_ident, quote};

use crate::util::FieldName;

pub fn derive_gc_project_struct(item: &DeriveInput, data: &DataStruct) -> syn::Result<TokenStream> {
    let struct_definition = derive_gc_project_struct_type(item, data)?;
    let from_impl = derive_gc_project_struct_from_impl(item, data)?;
    let trait_impl = derive_gc_project_struct_trait_impl(item, data)?;

    Ok(quote! {
        #[automatically_derived]
        const _: () = {
            #struct_definition
            #from_impl
            #trait_impl
        };
    })
}

/// Generates:
/// ```
/// struct MyStructProj<'a> {
///     /* Omitted */
/// }
/// ```
fn derive_gc_project_struct_type(
    item: &DeriveInput,
    data: &DataStruct,
) -> syn::Result<TokenStream> {
    let vis = &item.vis;

    let fields = data.fields.iter().filter_map(|field| {
        let ty = &field.ty;

        if let Some(attr) = field.attrs.iter().find(|attr| attr.path().is_ident("zst")) {
            if let Err(e) = attr.meta.require_path_only() {
                return Some(Err(e));
            }

            return None;
        }

        let vis = &field.vis;

        Some(Ok(match field.ident.as_ref() {
            Some(ident) => quote!(#vis #ident: ::mulch::gc::safety::GC<'a, #ty>),
            None => quote!(#vis ::mulch::gc::safety::GC<'a, #ty>),
        }))
    });

    let doc_comment = format!(
        "GC projection of [{}]. See [GCProject](::mulch::gc::GCProject) for more information.",
        &item.ident
    );

    let struct_body = fields.process_results(|fields| match &data.fields {
        syn::Fields::Named(_) => quote! {
            {#(#fields),*}
        },
        syn::Fields::Unnamed(_) => quote! {
            (#(#fields),*);
        },
        syn::Fields::Unit => quote!(;),
    })?;

    let projection_name = format_ident!("{}Proj", item.ident);

    let generics =
        std::iter::once(quote!('a)).chain(item.generics.params.iter().map(|g| g.to_token_stream()));

    let where_clause = &item.generics.where_clause;

    Ok(quote! {
        #[doc = #doc_comment]
        #vis struct #projection_name<#(#generics),*> #where_clause #struct_body
    })
}

/// Generates:
/// ```
/// impl<'a> From<MyStructProj<'a>> for GC<'a, MyStruct> {
///  /* Omitted */
/// }
/// ```
fn derive_gc_project_struct_from_impl(
    item: &DeriveInput,
    data: &DataStruct,
) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let projection_name = format_ident!("{name}Proj");

    if !data
        .fields
        .iter()
        .any(|f| !f.attrs.iter().any(|a| a.path().is_ident("zst")))
    {
        return Err(syn::Error::new(
            Span::call_site(),
            "Cannot derive GCProject on fieldless struct",
        ));
    }

    let non_zst_fields = data
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| !field.attrs.iter().any(|a| a.path().is_ident("zst")));

    let initial_field_gc_check = non_zst_fields.clone().next().map(|(i, field)| {
        let field_name = field
            .ident
            .as_ref()
            .map_or(FieldName::Index(i), |n| FieldName::Name(n));

        quote! {
            let gc_ref = ::mulch::gc::safety::GC::gc(&val.#field_name);
        }
    });

    let fields_gc_check = non_zst_fields.skip(1).map(|(i, field)| {
        let field_name = field
            .ident
            .as_ref()
            .map_or(FieldName::Index(i), |id| FieldName::Name(id));

        quote! {
            ::core::assert_eq!(::core::ptr::from_ref(gc_ref), ::core::ptr::from_ref(::mulch::gc::safety::GC::gc(&val.#field_name)));
        }
    });

    let per_field = data.fields.iter().enumerate().map(|(i, field)| {
        let old_name = field
            .ident
            .as_ref()
            .map_or(FieldName::Index(i), |n| FieldName::Name(n));

        let value = if field.attrs.iter().any(|a| a.path().is_ident("zst")) {
            quote! {::core::default::Default::default()}
        } else {
            quote! {::mulch::gc::safety::GC::raw(val.#old_name)}
        };

        Some(if let Some(name) = field.ident.as_ref() {
            quote! (#name: #value)
        } else {
            value
        })
    });

    let struct_instantiation = match &data.fields {
        syn::Fields::Named(_) => quote! ({#(#per_field),*}),
        syn::Fields::Unnamed(_) => quote! ((#(#per_field),*)),
        syn::Fields::Unit => unreachable!(),
    };

    let mut generics = item.generics.clone();

    generics.params.insert(
        0,
        syn::GenericParam::Lifetime(LifetimeParam::new(Lifetime::new("'a", Span::call_site()))),
    );

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (_, base_ty_generics, _) = item.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::core::convert::From<#projection_name #ty_generics> for ::mulch::gc::safety::GC<'a, #name #base_ty_generics> #where_clause {
            fn from(val: #projection_name #ty_generics) -> Self {
                #initial_field_gc_check
                #(#fields_gc_check)*

                unsafe {
                    ::mulch::gc::safety::GC::from_raw_parts(gc_ref, #name #struct_instantiation)
                }
            }
        }
    })
}

/// Generates:
/// ```
/// impl<'a> GCProject<'a> for MyStruct {
///     /* Omitted */
/// }
/// ```
fn derive_gc_project_struct_trait_impl(
    item: &DeriveInput,
    data: &DataStruct,
) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let projected_name = format_ident!("{name}Proj");

    if !data
        .fields
        .iter()
        .any(|f| !f.attrs.iter().any(|a| a.path().is_ident("zst")))
    {
        return Err(syn::Error::new(
            Span::call_site(),
            "Cannot derive GCProject on fieldless struct",
        ));
    }

    let per_field = data
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.attrs.iter().any(|a| a.path().is_ident("zst")))
        .map(|(i, field)| {
            let field_name = field
                .ident
                .as_ref()
                .map_or(FieldName::Index(i), |n| FieldName::Name(n));

            let value = quote! {
                unsafe {::mulch::gc::safety::GC::from_raw_parts(gc, raw.#field_name)}
            };

            if let FieldName::Name(field_name) = field_name {
                quote!(#field_name: #value)
            } else {
                value
            }
        });

    let instantiation = match &data.fields {
        syn::Fields::Named(_) => quote! ({#(#per_field),*}),
        syn::Fields::Unnamed(_) => quote! ((#(#per_field),*)),
        syn::Fields::Unit => unreachable!(),
    };

    let mut generics = item.generics.clone();

    generics.params.insert(
        0,
        syn::GenericParam::Lifetime(LifetimeParam::new(Lifetime::new("'a", Span::call_site()))),
    );

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (_, base_ty_generics, _) = item.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::mulch::gc::GCProject<'a> for #name #base_ty_generics #where_clause {
            type Projected = #projected_name #ty_generics;

            fn project(value: ::mulch::gc::safety::GC<'a, Self>) -> Self::Projected {
                let gc = ::mulch::gc::safety::GC::gc(&value);
                let raw = ::mulch::gc::safety::GC::raw(value);

                #projected_name #instantiation
            }
        }
    })
}
