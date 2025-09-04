use crate::error::{Diagnostic, FullSpan, error};

pub fn unexpected_character(c: char, span: FullSpan) -> Diagnostic {
    error!("EL0001", format!("Unexpected character {c:?}"), [{"character here", span, primary}])
}

pub fn invalid_escape(c: char, span: FullSpan) -> Diagnostic {
    error!("EL0002", format!("Invalid escape sequence \"\\{}\"", c), [{"here", span, primary}])
}

pub fn no_end_quote(span: FullSpan) -> Diagnostic {
    error!("EL0003", "No end quote found for string literal", [{"here", span, primary}])
}

pub fn unexpected_character_in_numeric_literal(c: char, span: FullSpan) -> Diagnostic {
    error!("EL0004", format!("Unexpected character {c:?} in numeric literal"), [{"character here", span, primary}])
}
