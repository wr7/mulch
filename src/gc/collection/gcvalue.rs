use std::ptr::addr_of;

use crate::{
    eval::MValue,
    gc::{GCBox, GCPtr, GCString, GCVec, util::GCDebug},
};

#[derive(Clone, Copy)]
#[repr(C)]
pub union GCValue {
    mvalue: MValue,
    other: OtherGCValue,
}

impl GCValue {
    #[inline]
    pub fn get(&self) -> GCValueEnum {
        let discriminant = unsafe { addr_of!(self).cast::<usize>().read() };

        if discriminant & 1usize.rotate_right(1) == 0 {
            GCValueEnum::MValue(unsafe { self.mvalue })
        } else {
            GCValueEnum::Other(unsafe { self.other })
        }
    }
}

pub enum GCValueEnum {
    MValue(MValue),
    Other(OtherGCValue),
}

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum OtherGCValue {
    // This variant is mostly a placeholder and will probably be removed in the future.
    BoxMValue(GCBox<MValue>) = 1usize.rotate_right(1),
}

unsafe impl GCPtr for GCValue {
    const MSB_RESERVED: bool = false;

    unsafe fn gc_copy(self, gc: &mut crate::gc::GarbageCollector) -> Self {
        unsafe {
            match self.get() {
                GCValueEnum::MValue(mvalue) => mvalue.gc_copy(gc).into(),
                GCValueEnum::Other(other_gcvalue) => match other_gcvalue {
                    OtherGCValue::BoxMValue(mbox) => mbox.gc_copy(gc).into(),
                },
            }
        }
    }
}

impl GCDebug for GCValue {
    unsafe fn gc_debug(
        self,
        gc: &crate::gc::GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        unsafe {
            match self.get() {
                GCValueEnum::MValue(mvalue) => mvalue.gc_debug(gc, f),
                GCValueEnum::Other(other_gcvalue) => match other_gcvalue {
                    OtherGCValue::BoxMValue(gcbox) => gcbox.gc_debug(gc, f),
                },
            }
        }
    }
}

impl TryFrom<GCValue> for MValue {
    type Error = ();

    fn try_from(value: GCValue) -> Result<Self, Self::Error> {
        if let GCValueEnum::MValue(value) = value.get() {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl From<OtherGCValue> for GCValue {
    fn from(value: OtherGCValue) -> Self {
        Self { other: value }
    }
}

impl From<MValue> for GCValue {
    fn from(value: MValue) -> Self {
        Self { mvalue: value }
    }
}

impl From<GCBox<MValue>> for GCValue {
    fn from(value: GCBox<MValue>) -> Self {
        OtherGCValue::BoxMValue(value).into()
    }
}

impl From<GCString> for GCValue {
    fn from(value: GCString) -> Self {
        MValue::String(value).into()
    }
}

impl From<GCVec<MValue>> for GCValue {
    fn from(value: GCVec<MValue>) -> Self {
        MValue::List(value).into()
    }
}
