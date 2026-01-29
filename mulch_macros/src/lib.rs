use syn::{DeriveInput, LitStr, parse_macro_input};

mod from_to_u8;
mod gc_debug;
mod gc_ptr;
mod parser;

#[proc_macro_derive(GCDebug, attributes(debug_direct))]
pub fn derive_gc_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_debug::derive_gc_debug(parse_macro_input!(item as DeriveInput)).into()
}

#[proc_macro_derive(GCPtr, attributes(msb_reserved))]
pub fn derive_gc_ptr(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_ptr::derive_gc_ptr(parse_macro_input!(item as DeriveInput)).into()
}

/// The [`Keyword`](::mulch::parser::Keyword) type. Takes a string literal as input.
///
/// The string must be less than 16 characters long.
#[proc_macro]
pub fn keyword(lit: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::keyword(parse_macro_input!(lit as LitStr)).into()
}

#[proc_macro_derive(Parse, attributes(mulch_parse_error, error_if_not_found))]
pub fn derive_parse(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::parse::derive_parse(parse_macro_input!(item as DeriveInput))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

#[proc_macro_derive(FromToU8)]
pub fn derive_from_to_u8(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_to_u8::derive_from_to_u8(parse_macro_input!(item as DeriveInput)).into()
}
