#![allow(unexpected_cfgs)] // because `cfg(rust_analyzer)` is not part of the standard

mod error_test;

#[cfg(any(not(miri), rust_analyzer))]
mod proptest; // proptests do not work properly under MIRI
