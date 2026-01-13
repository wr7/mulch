use std::fmt::Debug;

use codespan_reporting::diagnostic::{LabelStyle, Severity};

use copyspan::Span;

pub type PDResult<T> = Result<T, ParseDiagnostic>;

// We are using `Box` in order to reduce the size of `Result<T, Diagnostic>`.
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ParseDiagnostic(Box<RawParseDiagnostic>);

impl Debug for ParseDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParseDiagnostic")
            .field("severity", &self.0.severity)
            .field("code", &self.0.code)
            .field("message", &self.0.message)
            .field("hints", &self.0.hints)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseHint {
    message: String,
    span: Span,
    style: LabelStyle,
}

impl ParseHint {
    pub fn primary(message: String, span: Span) -> Self {
        Self {
            message,
            span,
            style: LabelStyle::Primary,
        }
    }
    pub fn secondary(message: String, span: Span) -> Self {
        Self {
            message,
            span,
            style: LabelStyle::Secondary,
        }
    }

    fn get(self, file_id: usize) -> error::Hint {
        error::Hint {
            message: self.message,
            span: FullSpan::new(self.span, file_id),
            style: self.style,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct RawParseDiagnostic {
    severity: Severity,
    code: &'static str,
    message: String,
    hints: Vec<ParseHint>,
}

impl ParseDiagnostic {
    pub fn error(code: &'static str, message: String, hints: Vec<ParseHint>) -> Self {
        Self(Box::new(RawParseDiagnostic {
            severity: Severity::Error,
            code,
            message,
            hints,
        }))
    }

    pub fn with_file_id(self, file_id: usize) -> error::Diagnostic {
        let hints = self
            .0
            .hints
            .into_iter()
            .map(|hint| hint.get(file_id))
            .collect::<Vec<error::Hint>>();

        error::Diagnostic(Box::new(error::RawDiagnostic {
            severity: self.0.severity,
            code: self.0.code,
            message: self.0.message,
            hints,
        }))
    }
}

macro_rules! parse_error {
    (
        $code:literal,
        $msg:expr,
        [
            $(
                {$hintmsg:expr, $span:expr, $type:ident}
            ),*$(,)?
        ]$(,)?
    ) => {
        $crate::error::parse::ParseDiagnostic::error(
            $code,
            String::from($msg),
            vec![$(
                $crate::error::parse::ParseHint::$type(String::from($hintmsg), $span)
            ),*]
        )
    };
}

pub(crate) use parse_error;

use crate::error::{self, FullSpan};
