use std::ops::Deref;

use std::ffi::OsString;

use std::ffi::OsStr;

use std::borrow::Borrow;

use std::hash::Hash;
use std::slice;

use crate::util;

// Note: we are using these Raw* structs to avoid UB with Rust aliasing
pub(crate) struct RawString(pub(super) RawVec<u8>);

pub(crate) struct RawOsString(pub(super) RawVec<u8>);

#[derive(Clone, Copy)]
pub(crate) struct RawOsStr {
    pub(crate) data: *mut u8,
    pub(crate) len: usize,
}

impl Hash for RawOsString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Borrow::<OsStr>::borrow(self).hash(state);
    }
}

impl Borrow<OsStr> for RawOsString {
    fn borrow(&self) -> &OsStr {
        unsafe {
            OsStr::from_encoded_bytes_unchecked(slice::from_raw_parts(self.0.ptr, self.0.len))
        }
    }
}

impl PartialEq for RawOsString {
    fn eq(&self, other: &Self) -> bool {
        let self_ = Borrow::<OsStr>::borrow(self);
        let other = Borrow::<OsStr>::borrow(other);

        self_.eq(other)
    }
}

impl Eq for RawOsString {}

pub(crate) struct RawVec<T> {
    pub(crate) ptr: *mut T,
    pub(crate) len: usize,
    pub(crate) capacity: usize,
}

impl<T> From<Vec<T>> for RawVec<T> {
    fn from(value: Vec<T>) -> Self {
        let (ptr, len, capacity) = util::vec_into_raw_parts(value);
        Self { ptr, len, capacity }
    }
}

impl From<String> for RawString {
    fn from(value: String) -> Self {
        Self(value.into_bytes().into())
    }
}

impl From<OsString> for RawOsString {
    fn from(value: OsString) -> Self {
        Self(value.into_encoded_bytes().into())
    }
}

impl Deref for RawString {
    type Target = RawVec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        unsafe {
            drop(Vec::from_raw_parts(self.ptr, self.len, self.capacity));
        }
    }
}
