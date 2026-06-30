use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use copyspan::Span;
use mulch_macros::{GCDebug, GCPtr};

use crate::gc::{GCProject, GCPtr, safety::GC};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, GCPtr, GCDebug)]
pub struct PartialSpanned<T>(pub T, pub Span);

impl<T: Debug> Debug for PartialSpanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Spanned(")?;
        Debug::fmt(&self.0, f)?;
        write!(f, ", {:?})", self.1)
    }
}

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

impl<'c, T: GCPtr> GC<'c, PartialSpanned<T>> {
    pub fn map<F: FnOnce(GC<'c, T>) -> GC<'c, U>, U: GCPtr>(
        self,
        f: F,
    ) -> GC<'c, PartialSpanned<U>> {
        let self_proj = self.project();
        PartialSpanned(f(self_proj.0), self_proj.1).into()
    }

    pub fn with_file_id(self, file_id: usize) -> GC<'c, Spanned<T>> {
        let self_proj = self.project();

        Spanned(self_proj.0, FullSpan::new(self_proj.1, file_id)).into()
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

impl<'a, T: GCPtr> From<PartialSpanned<GC<'a, T>>> for GC<'a, PartialSpanned<T>> {
    fn from(value: PartialSpanned<GC<'a, T>>) -> Self {
        unsafe { GC::from_raw_parts(value.0.gc(), PartialSpanned(value.0.raw(), value.1)) }
    }
}

impl<'a, T: GCPtr> GCProject<'a> for PartialSpanned<T> {
    type Projected = PartialSpanned<GC<'a, T>>;

    fn project(value: GC<'a, Self>) -> Self::Projected {
        let gc = value.gc();
        let raw = value.raw();

        unsafe { PartialSpanned(GC::from_raw_parts(gc, raw.0), raw.1) }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, GCPtr, GCDebug)]
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

impl<'c, T: GCPtr> GC<'c, Spanned<T>> {
    pub fn map<F: FnOnce(GC<'c, T>) -> GC<'c, U>, U: GCPtr>(self, f: F) -> GC<'c, Spanned<U>> {
        let self_proj = self.project();
        Spanned(f(self_proj.0), self_proj.1).into()
    }
}

impl<'a, T: GCPtr> From<Spanned<GC<'a, T>>> for GC<'a, Spanned<T>> {
    fn from(value: Spanned<GC<'a, T>>) -> Self {
        unsafe { GC::from_raw_parts(value.0.gc(), Spanned(value.0.raw(), value.1)) }
    }
}

impl<'a, T: GCPtr> GCProject<'a> for Spanned<T> {
    type Projected = Spanned<GC<'a, T>>;

    fn project(value: GC<'a, Self>) -> Self::Projected {
        let gc = value.gc();
        let raw = value.raw();

        unsafe { Spanned(GC::from_raw_parts(gc, raw.0), raw.1) }
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
