use mulch_macros::{GCDebug, GCPtr};

use crate::gc::{GCString, GCVec};

/// A `mulch` value
#[derive(GCPtr, GCDebug, Clone, Copy)]
#[repr(usize)]
pub enum MValue {
    #[debug_direct]
    String(GCString) = 1,
    #[debug_direct]
    List(GCVec<MValue>),
}

impl From<GCString> for MValue {
    fn from(value: GCString) -> Self {
        MValue::String(value)
    }
}

impl From<GCVec<MValue>> for MValue {
    fn from(value: GCVec<MValue>) -> Self {
        MValue::List(value)
    }
}
