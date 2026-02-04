use proc_macro2::TokenStream;
use quote::quote;
use syn::LitInt;

pub mod parse;

pub fn keyword(lit_ast: syn::LitStr) -> syn::Result<TokenStream> {
    let literal = u128_from_string(lit_ast)?;

    Ok(quote! { ::mulch::parser::Keyword::<#literal> })
}

pub fn punct(lit_ast: syn::LitStr) -> syn::Result<TokenStream> {
    let literal = u128_from_string(lit_ast)?;

    Ok(quote! { ::mulch::parser::Punct::<#literal> })
}

pub fn u128_from_string(lit_ast: syn::LitStr) -> syn::Result<LitInt> {
    let lit = lit_ast.value();

    if lit.len() > 15 {
        return Err(syn::Error::new(
            lit_ast.span(),
            "String literal cannot be more than `15` characters",
        ));
    }

    let mut ret = 0u128;

    for byte in lit.bytes().chain(std::iter::once(0xff)) {
        ret <<= 8;
        ret |= byte as u128;
    }

    ret <<= 8 * (16 - lit.len() - 1);

    Ok(syn::LitInt::new(&format!("0x{ret:x}u128"), lit_ast.span()))
}
