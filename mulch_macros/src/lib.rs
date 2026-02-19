use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

use crate::parser::parse::ParseTrait;

mod from_to_u8;
mod gc_debug;
mod gc_ptr;
mod parser;

mod util;

#[proc_macro_derive(GCDebug, attributes(debug_direct, debug_hidden))]
pub fn derive_gc_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_debug::derive_gc_debug(parse_macro_input!(item as DeriveInput))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro_derive(GCPtr, attributes(msb_reserved))]
pub fn derive_gc_ptr(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_ptr::derive_gc_ptr(parse_macro_input!(item as DeriveInput)).into()
}

#[proc_macro]
pub fn keyword(lit: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::keyword(parse_macro_input!(lit as LitStr))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro]
pub fn punct(lit: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::punct(parse_macro_input!(lit as LitStr))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Creates a u128 from a string. This is the same algorithm used for the [`punct`] and [`keyword`] types.
#[proc_macro]
pub fn u128_string(lit: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::u128_from_string(parse_macro_input!(lit as LitStr))
        .map_or_else(|err| err.into_compile_error(), |ok| quote! {#ok})
        .into()
}

#[proc_macro_derive(
    Parse,
    attributes(
        mulch_parse_error,
        error_if_not_found,
        parse_until_next,
        parse_hook,
        parse_direction
    )
)]
pub fn derive_parse(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::parse::derive_parse(parse_macro_input!(item as DeriveInput), ParseTrait::Parse)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro_derive(
    ParseLeft,
    attributes(mulch_parse_error, error_if_not_found, parse_until_next, parse_hook)
)]
pub fn derive_parse_left(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::parse::derive_parse(
        parse_macro_input!(item as DeriveInput),
        ParseTrait::ParseLeft,
    )
    .unwrap_or_else(|err| err.into_compile_error())
    .into()
}

#[proc_macro_derive(
    ParseRight,
    attributes(mulch_parse_error, error_if_not_found, parse_until_next, parse_hook)
)]
pub fn derive_parse_right(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::parse::derive_parse(
        parse_macro_input!(item as DeriveInput),
        ParseTrait::ParseRight,
    )
    .unwrap_or_else(|err| err.into_compile_error())
    .into()
}

#[proc_macro_derive(FromToU8)]
pub fn derive_from_to_u8(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_to_u8::derive_from_to_u8(parse_macro_input!(item as DeriveInput)).into()
}
