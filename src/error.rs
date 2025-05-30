mod spanned;

use std::fmt::Debug;

use codespan_reporting::diagnostic::{
    Diagnostic as CodespanDiagnostic, Label, LabelStyle, Severity,
};
pub use spanned::*;

// We are using `Box` in order to reduce the size of `Result<T, Diagnostic>`.
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Diagnostic(Box<RawDiagnostic>);

impl Debug for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diagnostic")
            .field("severity", &self.0.severity)
            .field("code", &self.0.code)
            .field("message", &self.0.message)
            .field("hints", &self.0.hints)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    message: String,
    span: FullSpan,
    style: LabelStyle,
}

impl Hint {
    pub fn primary(message: String, span: FullSpan) -> Self {
        Self {
            message,
            span,
            style: LabelStyle::Primary,
        }
    }
    pub fn secondary(message: String, span: FullSpan) -> Self {
        Self {
            message,
            span,
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct RawDiagnostic {
    severity: Severity,
    code: &'static str,
    message: String,
    hints: Vec<Hint>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: String, hints: Vec<Hint>) -> Self {
        Self(Box::new(RawDiagnostic {
            severity: Severity::Error,
            code,
            message,
            hints,
        }))
    }
}

impl From<Hint> for Label<usize> {
    fn from(value: Hint) -> Self {
        Label {
            style: value.style,
            file_id: value.span.file_id,
            range: value.span.span.range(),
            message: value.message,
        }
    }
}

impl From<Diagnostic> for CodespanDiagnostic<usize> {
    fn from(value: Diagnostic) -> Self {
        let value: RawDiagnostic = *value.0;
        CodespanDiagnostic {
            severity: value.severity,
            code: Some(value.code.to_owned()),
            message: value.message,
            labels: value.hints.into_iter().map(Label::from).collect(),
            notes: Vec::new(),
        }
    }
}

macro_rules! error {
    (
        $code:literal,
        $msg:expr,
        [
            $(
                {$hintmsg:expr, $span:expr, $type:ident}
            ),*$(,)?
        ]$(,)?
    ) => {
        $crate::error::Diagnostic::error(
            $code,
            $msg,
            vec![$(
                $crate::error::Hint::$type(String::from($hintmsg), $span)
            ),*]
        )
    };
}

pub(crate) use error;
