use crate::error::{Diagnostic, FullSpan, error};

pub fn invalid_expression(span: FullSpan) -> Diagnostic {
    error!("EP0001", "Invalid expression", [{"here", span, primary}])
}

pub fn mismatched_brackets(opening: FullSpan, closing: FullSpan) -> Diagnostic {
    error!("EP0002", "Mismatched brackets", [{"opening bracket here", opening, primary}, {"closing bracket here", closing, primary}])
}

pub fn unmatched_bracket(span: FullSpan) -> Diagnostic {
    error!("EP0003", "No matching bracket found", [{"for bracket here", span, primary}])
}
