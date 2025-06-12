use crate::error::{Diagnostic, FullSpan, error};

pub fn invalid_expression(span: FullSpan) -> Diagnostic {
    error!("EP0001", "Invalid expression", [{"here", span, primary}])
}
