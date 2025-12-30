use crate::gc::{GCPtr, GCString, GCVec, util::GCDebug};

/// A `mulch` value
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum MValue {
    String(GCString) = 1,
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

impl GCDebug for MValue {
    unsafe fn gc_debug(
        self,
        gc: &crate::gc::GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        unsafe {
            match self {
                MValue::String(gcstring) => gcstring.gc_debug(gc, f),
                MValue::List(gcvec) => gcvec.gc_debug(gc, f),
            }
        }
    }
}
