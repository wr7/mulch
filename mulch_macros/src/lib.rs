use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

use crate::parser::parse::ParseTrait;

mod from_to_u8;
mod gc_debug;
mod gc_eq;
mod gc_ptr;
mod parser;

mod util;

/// Derives the `GCDebug` trait. This is the equivalent to the standard library `Debug` trait except
/// for garbage-collected objects.
///
/// # Attributes
/// - `debug_direct`
///   - Can either be used with single field enum variants or single field structs.
///   - This will directly use the field's debug implementation without wrapping it in parenthesis
///     or adding the struct/variant name.
/// - `debug_direct_with_name`
///   - Can either be used with single field enum variants or single field structs.
///   - This will directly use the field's debug implementation without wrapping it in parenthesis
///     but will prepend the struct/variant name.
/// - `debug_hidden`
///   - Can be used on struct fields.
///   - This will prevent a field from being outputted in the debug string. This may be useful for
///     zero-sized fields.
#[proc_macro_derive(
    GCDebug,
    attributes(debug_direct, debug_direct_with_name, debug_hidden)
)]
pub fn derive_gc_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_debug::derive_gc_debug(parse_macro_input!(item as DeriveInput))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Derives the `GCPtr` trait. This allows a type to be garbage-collected.
///
/// # Attributes
/// - `msb_reserved`
///   - Can be used on enums if `#[repr(usize)]` is also used.
///   - This sets `<Self as GCPtr>::MSB_RESERVED` to be true which allows for a space-
///     saving optimization on `GCBox<Self>` (see `GCPtr::MSB_RESERVED` for more information).
///     `MSB_RESERVED` is automatically calculated on structs but is `false` on all enums without
///     this attribute.
#[proc_macro_derive(GCPtr, attributes(msb_reserved))]
pub fn derive_gc_ptr(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_ptr::derive_gc_ptr(parse_macro_input!(item as DeriveInput)).into()
}

/// Derives the `GCEq` trait. This is the equivalent to the standard library `PartialEq` trait
/// except for garbage-collected objects.
#[proc_macro_derive(GCEq)]
pub fn derive_gc_eq(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_eq::derive_gc_eq(parse_macro_input!(item as DeriveInput))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Refers to the AST type for a specific keyword.
///
/// For example, `keyword!["in"]` would correspond to the type for the "in" keyword. See
/// `mulch::parser::Keyword` for more information.
///
/// Note: mulch has no "reserved" keywords but they must be 16 characters or less and are case
/// sensitive, so something like `keyword!["iLOVEbeans"]` is 100% valid.
#[proc_macro]
pub fn keyword(lit: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::keyword(parse_macro_input!(lit as LitStr))
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Refers to the AST type for a specific symbol.
///
/// For example, `punct!["->"]` refers to the right arrow symbol. These should correspond to a
/// symbol defined in `mulch::lexer::Symbol`. See `mulch::parser::Punct` for more information.
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

/// Derives the `Parse` trait.
///
/// # Attributes
/// - `mulch_parse_error(error_fn)` (required)
///   - Must be used exactly once on all struct and enum definitions.
///   - Takes in a function which generates an error message for when the given type is expected in
///     a parsed string but some other type or nothing was found instead. Said function should have
///     the following signature:
///   - `fn(Span) -> ParseDiagnostic`
/// - `error_if_not_found`
///   - Can be defined on an enum definition or struct field.
///   - Normally, if an enum has no variants that match the input token stream or a struct field
///     cannot be matched from the input token stream, `Ok(None)` is returned. This attribute will
///     instead emit an error when this happens. This can result in better error messages but, if
///     used incorrectly, can prevent other alternative parse rules from running (which may match to
///     the input that was rejected by this enum/field).
/// - `parse_until_next`
///   - Can be defined on a struct field.
///   - Normally, when parsing struct fields, the parser implementation will use `parse_from_left`
///     or `parse_from_right` on all of the fields except for the last/first field. However,
///     many types (such as `ast::Expression`) do not and should not implement either function. This
///     attribute will make the parser implementation instead call `find_left` or `find_right` on
///     the next field and then use that information to parse this field using the `Parse::parse`
///     function.
/// - `parse_hook(hook)`
///   - Can be used on struct definitions or on enum variants.
///   - Allows you to define parse hooks. These always run in the order in which they are defined,
///     and they are of the form `fn(&Parser, &TokenStream) -> PDResult<Option<Self>>`.
///   - When used within an enum, they run directly before the enum variant that they are defined
///     on.
///   - When used on a struct, they run before any other parsing logic for that struct.
///   - If an error is returned that error is returned from the derived parsing function.
///   - If `Ok(None)` is returned, the parsing logic continues as normal.
///   - If `Ok(Some(val))` is returned, said value is returned from the derived parsing function.
/// - `parse_direction(direction)`
///   - Can be used on struct definitions. Should be specified as `#[parse_direction(Left)]` or
///     `#[parse_direction(Right)]`.
///   - By default, the parse direction is `Left`. This means that the fields are parsed from the
///     first field to the last and from left to right when the `Parse::parse` method is called.
///     `#[parse_direction(Right)]` instead will parse from the last field to the first field and
///     from right to left. In both cases, the first field is the leftmost and the last field is
///     the rightmost.
/// - `parse_skip`
///   - Can be used on enum variants.
///   - This causes the macro to not emit logic for parsing a specific variant. This is particularly
///     useful when combined with the `parse_hook` attribute.
#[proc_macro_derive(
    Parse,
    attributes(
        mulch_parse_error,
        error_if_not_found,
        parse_until_next,
        parse_hook,
        parse_direction,
        parse_skip,
    )
)]
pub fn derive_parse(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parser::parse::derive_parse(parse_macro_input!(item as DeriveInput), ParseTrait::Parse)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

/// Derives the `ParseLeft` trait. This has not been implemented yet for enum definitions.
///
/// # Attributes
/// - `mulch_parse_error(error_fn)` (required)
///   - Must be used exactly once on all struct definitions.
///   - Takes in a function which generates an error message for when the given type is expected in
///     a parsed string but some other type or nothing was found instead. Said function should have
///     the following signature:
///   - `fn(Span) -> ParseDiagnostic`
/// - `error_if_not_found`
///   - Can be defined on a struct field.
///   - Normally, if a struct field cannot be matched from the input token stream, `Ok(None)` is
///     returned. This attribute will instead emit an error when this happens. This can result in
///     better error messages but, if used incorrectly, can prevent other alternative parse rules
///     from running (which may match to the input that was rejected by this field).
/// - `parse_until_next`
///   - Can be defined on a struct field.
///   - Normally, when parsing struct fields, the parser implementation will use `parse_from_left`
///     on all of the fields. However, many types (such as `ast::Expression`) do not and should not
///     implement this function. This attribute will make the parser implementation instead call
///     `find_left` on the next field and then use that information to parse this field using the
///     `Parse::parse` function.
/// - `parse_hook(hook)`
///   - Can be used on struct definitions.
///   - Allows you to define parse hooks. These always run in the order in which they are defined,
///     and they are of the form `fn(&Parser, &mut &TokenStream) -> PDResult<Option<T>>`.
///   - When used on a struct, they run before any other parsing logic for that struct.
///   - If an error is returned that error is returned from the derived parsing function.
///   - If `Ok(None)` is returned, the parsing logic continues as normal.
///   - If `Ok(Some(val))` is returned, said value is returned from the derived parsing function.
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

/// Derives the `ParseRight` trait. This has not been implemented yet for enum definitions.
///
/// # Attributes
/// - `mulch_parse_error(error_fn)` (required)
///   - Must be used exactly once on all struct definitions.
///   - Takes in a function which generates an error message for when the given type is expected in
///     a parsed string but some other type or nothing was found instead. Said function should have
///     the following signature:
///   - `fn(Span) -> ParseDiagnostic`
/// - `error_if_not_found`
///   - Can be defined on a struct field.
///   - Normally, if a struct field cannot be matched from the input token stream, `Ok(None)` is
///     returned. This attribute will instead emit an error when this happens. This can result in
///     better error messages but, if used incorrectly, can prevent other alternative parse rules
///     from running (which may match to the input that was rejected by this field).
/// - `parse_until_next`
///   - Can be defined on a struct field.
///   - Normally, when parsing struct fields, the parser implementation will use `parse_from_right`
///     on all of the fields. However, many types (such as `ast::Expression`) do not and should not
///     implement this function. This attribute will make the parser implementation instead call
///     `find_right` on the next field and then use that information to parse this field using the
///     `Parse::parse` function.
/// - `parse_hook(hook)`
///   - Can be used on struct definitions.
///   - Allows you to define parse hooks. These always run in the order in which they are defined,
///     and they are of the form `fn(&Parser, &mut &TokenStream) -> PDResult<Option<T>>`.
///   - When used on a struct, they run before any other parsing logic for that struct.
///   - If an error is returned that error is returned from the derived parsing function.
///   - If `Ok(None)` is returned, the parsing logic continues as normal.
///   - If `Ok(Some(val))` is returned, said value is returned from the derived parsing function.
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

/// Generates methods for converting a fieldless enum to and from a u8.
///
/// These are of the form:
/// ```
/// pub const fn to_u8(&self) -> u8;
/// pub const fn from_u8(val: u8) -> Option<Self>;
/// ```
///
#[proc_macro_derive(FromToU8)]
pub fn derive_from_to_u8(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_to_u8::derive_from_to_u8(parse_macro_input!(item as DeriveInput)).into()
}
