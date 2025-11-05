use std::borrow::Cow;

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

pub fn multiple_declarations_of_attribute(
    def1: FullSpan,
    def2: FullSpan,
    attr: &str,
) -> Diagnostic {
    error!("EP0005", format!("Multiple declarations of attribute {attr}"), [{"First defined here", def1, secondary}, {"Then defined here", def2, primary}])
}

pub fn expected_attribute_name(got: Spanned<&Token>) -> Diagnostic {
    error!("EP0006", format!("Expected attribute name; got `{}`", &got.0), [{"here", got.1, primary}])
}

pub fn expected_token(expected: &Token, got: Spanned<&Token>) -> Diagnostic {
    error!("EP0007", format!("Expected token `{}`; got `{}`", expected, &got.0), [{"here", got.1, primary}])
}

pub fn let_in_eof(eof_span: FullSpan, let_span: FullSpan) -> Diagnostic {
    error!("EP0008", "Expected `; in <expression>`; got EOF", [
        {"because of `let` expression here", let_span, secondary},
        {"end-of-file reached here", eof_span, primary},
    ])
}

pub fn let_in_unexpected(
    got: Option<&Token>,
    got_span: FullSpan,
    let_span: FullSpan,
) -> Diagnostic {
    let got_text = got.map_or(Cow::Borrowed("EOF"), |t| t.to_string().into());

    error!("EP0009", format!("Expected `<identifier> = <expression;` or `in <expression>`; got {got_text}"), [
        {"let expression starts here", let_span, secondary},
        {"expected here", got_span, primary},
    ])
}

pub fn unexpected_tokens(span: FullSpan) -> Diagnostic {
    error!("EP0010", "Unexpected tokens", [{"here", span, primary}])
}

pub fn expected_lambda_arguments(span: FullSpan) -> Diagnostic {
    error!("EP0011", "Expected lambda arguments", [{"here", span, primary}])
}

pub fn invalid_lambda_arguments(span: FullSpan) -> Diagnostic {
    error!("EP0012", "Invalid lambda arguments", [{"here", span, primary}])
}
