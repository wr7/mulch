use mulch_macros::GCDebug;

use crate::gc::{GCPtr, GCString, GCVec, util::GCDebug};

/// A `mulch` value
#[derive(Clone, Copy, GCDebug)]
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

unsafe impl GCPtr for MValue {
    const MSB_RESERVED: bool = true;

    unsafe fn gc_copy(self, gc: &mut crate::gc::GarbageCollector) -> Self {
        match self {
            MValue::String(gcstring) => unsafe { gcstring.gc_copy(gc) }.into(),
            MValue::List(gcvec) => unsafe { gcvec.gc_copy(gc) }.into(),
        }
    }
}
