use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

pub mod parse;

pub fn keyword(lit_ast: syn::LitStr) -> TokenStream {
    let lit = lit_ast.value();

    if lit.len() > 15 {
        return quote_spanned! { lit_ast.span() => compile_error!("String literal cannot be more than `15` characters")};
    }

    let mut ret = 0u128;

    for byte in lit.bytes().chain(std::iter::once(0xff)) {
        ret <<= 8;
        ret |= byte as u128;
    }

    ret <<= 8 * (16 - lit.len() - 1);

    let literal = syn::LitInt::new(&format!("0x{ret:x}u128"), lit_ast.span());

    quote! { ::mulch::parser::Keyword::<#literal> }
}
