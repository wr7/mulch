use crate::error::{error, Diagnostic, FullSpan};

pub fn unexpected_character(c: char, span: FullSpan) -> Diagnostic {
    error!("EL0001", format!("Unexpected character {c:?}"), [{"character here", span, primary}])
}
