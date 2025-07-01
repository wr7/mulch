use crate::{
    error::{Diagnostic, FullSpan, Spanned, error},
    lexer::Token,
};

pub fn invalid_expression(span: FullSpan) -> Diagnostic {
    error!("EP0001", "Invalid expression", [{"here", span, primary}])
}

pub fn mismatched_brackets(opening: FullSpan, closing: FullSpan) -> Diagnostic {
    error!("EP0002", "Mismatched brackets", [{"opening bracket here", opening, primary}, {"closing bracket here", closing, primary}])
}

pub fn unmatched_bracket(span: FullSpan) -> Diagnostic {
    error!("EP0003", "No matching bracket found", [{"for bracket here", span, primary}])
}

pub fn expected_expression(span: FullSpan) -> Diagnostic {
    error!("EP0004", "Expected expression", [{"here", span, primary}])
}

pub fn multiple_definitions_of_attribute(def1: FullSpan, def2: FullSpan, attr: &str) -> Diagnostic {
    error!("EP0005", format!("Multiple definitions of attribute {attr}"), [{"First defined here", def1, secondary}, {"Then defined here", def2, primary}])
}

pub fn expected_attribute_name(got: Spanned<&Token>) -> Diagnostic {
    error!("EP0006", format!("Expected attribute name; got `{}`", &got.0), [{"here", got.1, primary}])
}

pub fn expected_token(expected: &Token, got: Spanned<&Token>) -> Diagnostic {
    error!("EP0007", format!("Expected token `{}`; got `{}`", expected, &got.0), [{"here", got.1, primary}])
}
