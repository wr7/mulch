use proc_macro2::{Span, TokenStream};
use syn::{FnArg, Ident, ItemFn, Lifetime, Signature, Token, Type, punctuated::Punctuated};

use quote::quote;

pub fn gc_fn_impl(attr: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "No arguments expected for #[gc_fn] attribute",
        ));
    }

    let input = syn::parse2::<ItemFn>(input)?;

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    let Signature {
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token: _,
        ident,
        generics,
        paren_token: _,
        inputs,
        variadic,
        output,
    } = sig;

    if let Some(constness) = constness {
        return Err(syn::Error::new_spanned(
            constness,
            "#[gc_fn] does not support const functions",
        ));
    }

    if let Some(asyncness) = asyncness {
        return Err(syn::Error::new_spanned(
            asyncness,
            "#[gc_fn] does not support async functions",
        ));
    }

    if let Some(variadic) = variadic {
        return Err(syn::Error::new_spanned(
            variadic,
            "#[gc_fn] does not support variadic functions",
        ));
    }

    const GC_FN_ARG_ERROR: &'static str = "The first argument of a #[gc_fn] must be of the form `context_name: &mut gc!(...)` or `context_name: gc!(...)`";

    let Some(FnArg::Typed(gc_arg)) = inputs.first() else {
        return Err(syn::Error::new(Span::call_site(), GC_FN_ARG_ERROR));
    };

    let syn::Pat::Ident(ctx_name) = &*gc_arg.pat else {
        return Err(syn::Error::new_spanned(&*gc_arg.pat, GC_FN_ARG_ERROR));
    };

    if ctx_name.by_ref.is_some()
        || ctx_name.mutability.is_some()
        || ctx_name.subpat.is_some()
        || !ctx_name.attrs.is_empty()
    {
        return Err(syn::Error::new_spanned(&*gc_arg.pat, GC_FN_ARG_ERROR));
    }

    let (ctx_lifetime, gc_macro) = match &*gc_arg.ty {
        Type::Macro(type_macro) => (None, type_macro),
        Type::Reference(type_reference) => match &*type_reference.elem {
            Type::Macro(type_macro) => {
                type_reference
                    .mutability
                    .ok_or_else(|| syn::Error::new_spanned(&*gc_arg.ty, GC_FN_ARG_ERROR))?;

                (type_reference.lifetime.as_ref(), type_macro)
            }
            _ => return Err(syn::Error::new_spanned(&*gc_arg.ty, GC_FN_ARG_ERROR)),
        },
        _ => {
            return Err(syn::Error::new_spanned(&*gc_arg.ty, GC_FN_ARG_ERROR));
        }
    };

    let ctx_name = &ctx_name.ident;

    if !gc_macro.mac.path.is_ident("gc") {
        return Err(syn::Error::new_spanned(&gc_macro.mac, GC_FN_ARG_ERROR));
    }

    let GCMacroArgs {
        gc_lifetime,
        gc_inputs,
    } = syn::parse2(gc_macro.mac.tokens.clone())?;

    let where_clause = &generics.where_clause;

    let bundle_lifetime_args = match (gc_lifetime, ctx_lifetime) {
        (None, None) => None,
        (Some(gc), None) => Some(quote!(#gc,)),
        (None, Some(ctx)) => Some(quote!('_, #ctx,)),
        (Some(gc), Some(ctx)) => Some(quote!(#gc, #ctx,)),
    };

    let bundle_types = gc_inputs.iter().map(|arg| &arg.ty);

    let statements = block.stmts;

    let gc_arg_names = gc_inputs.iter().map(|arg| &arg.ident);
    let gc_arg_names2 = gc_inputs.iter().map(|arg| &arg.ident);

    let non_gc_args = inputs.iter().skip(1);

    Ok(quote! {
        #(
            #attrs
        )*
        #vis

        #unsafety #abi fn #ident #generics(
            #ctx_name: ::mulch::gc::safety::GCArgs<#bundle_lifetime_args (#(#bundle_types,)*)>,
            #(#non_gc_args,)*
        ) #output #where_clause

        {
            let (#ctx_name, (#(#gc_arg_names,)*)) = #ctx_name.split();

            #(
                let #gc_arg_names2 = unsafe {::mulch::gc::safety::GC::new(#ctx_name, #gc_arg_names2)};
            )*

            #(
                #statements
            )*
        }
    })
}

struct GCMacroArgs {
    pub gc_lifetime: Option<Lifetime>,
    pub gc_inputs: Punctuated<GCFnArg, Token![,]>,
}

impl syn::parse::Parse for GCMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let gc_lifetime = input
            .peek(Lifetime)
            .then(|| -> syn::Result<_> {
                let lifetime = input.parse::<Lifetime>()?;
                input.parse::<Token![,]>()?;
                Ok(lifetime)
            })
            .transpose()?;

        Ok(Self {
            gc_lifetime,
            gc_inputs: input.parse_terminated(GCFnArg::parse, Token![,])?,
        })
    }
}

struct GCFnArg {
    pub ident: Ident,
    pub _colon_tok: Token![:],
    pub ty: Type,
}

impl syn::parse::Parse for GCFnArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _colon_tok: input.parse()?,
            ty: input.parse()?,
        })
    }
}
