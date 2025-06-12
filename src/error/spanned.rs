use std::ops::{Deref, DerefMut};

use copyspan::Span;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FullSpan {
    pub span: Span,
    pub file_id: usize,
}

impl FullSpan {
    pub fn new(span: impl Into<Span>, file_id: usize) -> Self {
        Self {
            span: span.into(),
            file_id,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    pub data: T,
    pub span: FullSpan,
}

impl<T> Spanned<T> {
    pub fn new(data: T, span: FullSpan) -> Self {
        Self { data, span }
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> Spanned<U> {
        Spanned::<U> {
            data: f(self.data),
            span: self.span,
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
