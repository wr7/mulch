mod sourcedb;
mod spanned;

use std::fmt::Debug;

use codespan_reporting::{
    diagnostic::{Diagnostic as CodespanDiagnostic, Label, LabelStyle, Severity},
    term::termcolor::Buffer,
};
pub use sourcedb::SourceDB;
pub use spanned::*;

pub type DResult<T> = Result<T, Diagnostic>;

#[macro_export]
macro_rules! dresult_unwrap {
    ($result:expr, $db:expr) => {
        match $result {
            Ok(inner) => inner,
            Err(diag) => {
                ::core::panic!("\ndresult_unwrap failed:\n\n{}", diag.display($db))
            }
        }
    };
}

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

/// Used for displaying diagnostics: returned by [`Diagnostic::display`]
pub struct Display<'a> {
    diag: CodespanDiagnostic<usize>,
    db: &'a SourceDB,
}

impl<'a> std::fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Buffer::ansi();

        if let Err(err) =
            codespan_reporting::term::emit(&mut buf, &Default::default(), self.db, &self.diag)
        {
            return writeln!(
                f,
                "mulch internal error: unable to format error diagnostic: {err}\n\n{:?}",
                &self.diag
            );
        }

        let buf = buf.as_slice();
        let buf = String::from_utf8_lossy(buf);

        write!(f, "{}", buf)
    }
}

impl Diagnostic {
    pub fn display(self, db: &SourceDB) -> Display {
        Display {
            diag: self.into(),
            db,
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
            String::from($msg),
            vec![$(
                $crate::error::Hint::$type(String::from($hintmsg), $span)
            ),*]
        )
    };
}

pub(crate) use error;
