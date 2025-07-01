use std::ops::{Deref, DerefMut};

use copyspan::Span;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartialSpanned<T>(pub T, pub Span);

pub fn span_of<T>(arr: &[PartialSpanned<T>]) -> Option<Span> {
    Some(Span::from(arr.first()?.1.start..arr.last()?.1.end))
}

impl<T> PartialSpanned<T> {
    pub fn new(data: T, span: Span) -> Self {
        Self(data, span)
    }

    pub fn with_file_id(self, file_id: usize) -> Spanned<T> {
        Spanned(self.0, FullSpan::new(self.1, file_id))
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> PartialSpanned<U> {
        PartialSpanned::<U>(f(self.0), self.1)
    }

    pub fn as_ref(&self) -> PartialSpanned<&T> {
        PartialSpanned(&self.0, self.1)
    }
}

impl<T> Deref for PartialSpanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for PartialSpanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
pub struct Spanned<T>(pub T, pub FullSpan);

impl<T> Spanned<T> {
    pub fn new(data: T, span: FullSpan) -> Self {
        Self(data, span)
    }

    pub fn map<F: FnOnce(T) -> U, U>(self, f: F) -> Spanned<U> {
        Spanned::<U>(f(self.0), self.1)
    }

    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned(&self.0, self.1)
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
