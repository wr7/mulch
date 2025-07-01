use std::{
    ffi::OsStr,
    fmt::{Debug, Display},
    mem::{ManuallyDrop, MaybeUninit},
    ptr,
};

pub(crate) struct MultiPeekable<I: Iterator, const N: usize> {
    iter: I,
    buf: [MaybeUninit<I::Item>; N],
    len: usize,
}

impl<I: Iterator + Debug, const N: usize> Debug for MultiPeekable<I, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MultiPeekable({:?})", self.iter)
    }
}

impl<I: Iterator, const N: usize> MultiPeekable<I, N> {
    pub fn new(mut iter: I) -> Self {
        if N == 0 {
            panic!()
        }

        let mut buf = [const { MaybeUninit::uninit() }; N];
        let mut len = N;

        for (i, buf) in buf.iter_mut().enumerate() {
            let Some(item) = iter.next() else {
                len = i;
                break;
            };

            buf.write(item);
        }

        Self { iter, buf, len }
    }

    pub fn peek(&self, i: usize) -> Option<&I::Item> {
        if i >= self.len {
            return None;
        }

        Some(unsafe { self.buf[i].assume_init_ref() })
    }

    pub fn peek_all(&self) -> &[I::Item] {
        let ptr = self.buf[0].as_ptr();
        let len = self.len;

        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl<I: Iterator, const N: usize> Iterator for MultiPeekable<I, N> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let item = unsafe { self.buf[0].assume_init_read() };
        self.len -= 1;

        let buf_ptr = self.buf[0].as_mut_ptr();

        unsafe { std::ptr::copy(buf_ptr.offset(1), buf_ptr, self.len) };

        if self.len + 1 == N {
            if let Some(next) = self.iter.next() {
                self.buf[self.len].write(next);
                self.len += 1;
            }
        }

        Some(item)
    }
}

impl<I: Iterator + Clone, const N: usize> Clone for MultiPeekable<I, N>
where
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        let mut buf = [const { MaybeUninit::uninit() }; N];

        for i in 0..self.len {
            let clone = unsafe { self.buf[i].assume_init_read() }.clone();
            buf[i].write(clone);
        }

        Self {
            iter: self.iter.clone(),
            buf,
            len: self.len,
        }
    }
}

impl<I: Iterator, const N: usize> Drop for MultiPeekable<I, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { self.buf[i].assume_init_drop() };
        }
    }
}

pub fn vec_into_raw_parts<T>(val: Vec<T>) -> (*mut T, usize, usize) {
    let mut val = ManuallyDrop::new(val);

    let len = val.len();
    let capacity = val.capacity();
    let ptr = val.as_mut_ptr();

    (ptr, len, capacity)
}

#[repr(transparent)]
pub struct DisplayableOsStr<'a>(pub &'a OsStr);

impl<'a> Display for DisplayableOsStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.display(), f)
    }
}

// https://doc.rust-lang.org/nightly/std/primitive.slice.html#method.element_offset
pub fn element_offset<T>(slice: &[T], element: &T) -> Option<usize> {
    if size_of::<T>() == 0 {
        panic!("elements are zero-sized");
    }

    let self_start = slice.as_ptr().addr();
    let elem_start = ptr::from_ref(element).addr();

    let byte_offset = elem_start.wrapping_sub(self_start);

    if byte_offset % size_of::<T>() != 0 {
        return None;
    }

    let offset = byte_offset / size_of::<T>();

    if offset < slice.len() {
        Some(offset)
    } else {
        None
    }
}
