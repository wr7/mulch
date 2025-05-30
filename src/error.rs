use std::ops::{Deref, DerefMut};

use copyspan::Span;

#[repr(C)]
pub struct PartialSpanned<T> {
    pub data: T,
    pub span: Span,
}

impl<T> PartialSpanned<T> {
    pub fn new(data: T, span: Span) -> Self {
        Self { data, span }
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> PartialSpanned<U> {
        PartialSpanned::<U> {
            data: f(self.data),
            span: self.span,
        }
    }
}

impl<T> Deref for PartialSpanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for PartialSpanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[repr(C)]
pub struct Spanned<T> {
    pub data: T,
    pub span: Span,
    pub file_no: usize,
}

impl<T> Spanned<T> {
    pub fn new(data: T, span: Span, file_no: usize) -> Self {
        Self {
            data,
            span,
            file_no,
        }
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> Spanned<U> {
        Spanned::<U> {
            data: f(self.data),
            span: self.span,
            file_no: self.file_no,
        }
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
