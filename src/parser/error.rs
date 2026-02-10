use std::borrow::Cow;

use copyspan::Span;

use crate::{
    error::{
        PartialSpanned,
        parse::{ParseDiagnostic, parse_error},
    },
    lexer::Token,
    parser::{Keyword, Punct, bracketed::Bracketed},
};

pub fn invalid_expression(span: Span) -> ParseDiagnostic {
    if span.is_empty() {
        parse_error!("EP0001", "Expected expression", [{"here", span, primary}])
    } else {
        parse_error!("EP0001", "Invalid expression", [{"here", span, primary}])
    }
}

pub fn mismatched_brackets(opening: Span, closing: Span) -> ParseDiagnostic {
    parse_error!("EP0002", "Mismatched brackets", [{"opening bracket here", opening, primary}, {"closing bracket here", closing, primary}])
}

pub fn unmatched_bracket(span: Span) -> ParseDiagnostic {
    parse_error!("EP0003", "No matching bracket found", [{"for bracket here", span, primary}])
}

// EP0004 unused

pub fn multiple_declarations_of_attribute(def1: Span, def2: Span, attr: &str) -> ParseDiagnostic {
    parse_error!("EP0005", format!("Multiple declarations of attribute {attr}"), [{"First defined here", def1, secondary}, {"Then defined here", def2, primary}])
}

pub fn expected_ident_or_string(span: Span) -> ParseDiagnostic {
    parse_error!("EP0006", "Expected identifier or string", [{"here", span, primary}])
}

pub fn expected_punctuation<const S: u128>(span: Span) -> ParseDiagnostic {
    parse_error!("EP0007", format!("Expected token `{}`", Punct::<S>::STRING), [{"here", span, primary}])
}

pub fn let_in_eof(eof_span: Span, let_span: Span) -> ParseDiagnostic {
    parse_error!("EP0008", "Expected `; in <expression>`; got EOF", [
        {"because of `let` expression here", let_span, secondary},
        {"end-of-file reached here", eof_span, primary},
    ])
}

pub fn let_in_unexpected(got: Option<&Token>, got_span: Span, let_span: Span) -> ParseDiagnostic {
    let got_text = got.map_or(Cow::Borrowed("EOF"), |t| t.to_string().into());

    parse_error!("EP0009", format!("Expected `<identifier> = <expression>;` or `in <expression>`; got {got_text}"), [
        {"let expression starts here", let_span, secondary},
        {"expected here", got_span, primary},
    ])
}

pub fn unexpected_tokens(span: Span) -> ParseDiagnostic {
    parse_error!("EP0010", "Unexpected tokens", [{"here", span, primary}])
}

pub fn expected_lambda_arguments(span: Span) -> ParseDiagnostic {
    parse_error!("EP0011", "Expected lambda arguments", [{"here", span, primary}])
}

pub fn invalid_lambda_arguments(span: Span) -> ParseDiagnostic {
    parse_error!("EP0012", "Invalid lambda arguments", [{"here", span, primary}])
}

pub fn invalid_function_call_args(
    expr_span: Span,
    open_br: PartialSpanned<&Token>,
) -> ParseDiagnostic {
    parse_error!("EP0013",
        concat!(
            "Invalid function call arguments.\n\n",
            "Function calls must be of the form `function[arg1, arg2, ...]` or `function{arg1 = ...; arg2=...; ...}`"
        ),
        [
            {"function call here", expr_span, primary},
            {format!("expected `{{` or `[`; got `{}`", open_br.0.to_string()), open_br.1, secondary}
        ]
    )
}

pub fn expected_keyword<const K: u128>(span: Span) -> ParseDiagnostic {
    parse_error!("EP0014", format!("Expected keyword `{}`", Keyword::<K>::KEYWORD), [{"here", span, primary}])
}

pub fn expected_opening_bracket<const B: u8>(span: Span) -> ParseDiagnostic {
    let msg = match Bracketed::<B, ()>::BRACKET_TYPE {
        crate::lexer::BracketType::Round => "Expected token `(`",
        crate::lexer::BracketType::Square => "Expected token `[`",
        crate::lexer::BracketType::Curly => "Expected token `{`",
    };

    parse_error!("EP0015", msg, [{"here", span, primary}])
}

pub fn expected_identifier(span: Span) -> ParseDiagnostic {
    parse_error!("EP0016", "Expected identifier", [{"here", span, primary}])
}
