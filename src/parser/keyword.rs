use copyspan::Span;
use mulch_macros::{GCDebug, GCPtr};

/// Parses a literal that matches a specific keyword. This type should only be referred to using the
/// [`keyword`](mulch_macros::keyword) macro, and the value of `K` should only be accessed through
/// the `KEYWORD` associated constant.
///
/// NOTE: Rust currently doesn't have a way to use string literals as const generics. To get around
/// this, we're storing the `0xff`-terminated bytes in a big-endian u128. The actual string can be
/// accessed through the associated constant `KEYWORD`.
#[derive(GCDebug, GCPtr, Clone, Copy, Debug)]
pub struct Keyword<const K: u128> {
    span: Span,
}

impl<const K: u128> Keyword<K> {
    const RAW_BYTES: [u8; 16] = K.to_be_bytes();
    pub const KEYWORD: &'static str = Self::get();

    const fn get() -> &'static str {
        let mut ret: Option<&'static [u8]> = None;
        let mut remaining = Self::RAW_BYTES.as_slice();

        while let Some((byte, r)) = remaining.split_last() {
            remaining = r;

            if *byte == 0xff {
                ret = Some(remaining);
            }
        }

        if let Some(ret) = ret {
            if let Ok(ret) = std::str::from_utf8(ret) {
                return ret;
            }
        }

        panic!("Invalid string generic for `Keyword`")
    }
}
