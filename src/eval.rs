use crate::gc::{GCPtr, GCString, GCVec};

/// A pointer to a `mulch` value
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Value {
    String(GCString),
    List(GCVec<Value>),
}

impl From<GCString> for Value {
    fn from(value: GCString) -> Self {
        Value::String(value)
    }
}

impl From<GCVec<Value>> for Value {
    fn from(value: GCVec<Value>) -> Self {
        Value::List(value)
    }
}

unsafe impl GCPtr for Value {
    const MSB_RESERVED: bool = true;

    unsafe fn gc_copy(self, gc: &mut crate::gc::GarbageCollector) -> Self {
        match self {
            Value::String(gcstring) => unsafe { gcstring.gc_copy(gc) }.into(),
            Value::List(gcvec) => unsafe { gcvec.gc_copy(gc) }.into(),
        }
    }
}
