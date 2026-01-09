mod gc_debug;

#[proc_macro_derive(GCDebug, attributes(debug_direct))]
pub fn derive_gc_debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gc_debug::derive_gc_debug(item.into()).into()
}
