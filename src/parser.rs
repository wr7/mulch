use std::borrow::Cow;

use crate::{
    error::{DResult, FullSpan, PartialSpanned, span_of},
    lexer::Token,
};

mod error;
mod test;
pub mod util;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression<'src> {
    Variable(Cow<'src, str>),
    StringLiteral(Cow<'src, str>),
}

/// Parses an expression; returns Ok(None) iff `tokens` is empty.
pub fn parse_expression<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<PartialSpanned<Expression<'src>>>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let span = span_of(tokens).unwrap();
    let rules = [parse_ident_or_literal];

    for rule in rules {
        match rule(tokens, file_id)? {
            Some(expression) => {
                return Ok(Some(PartialSpanned::new(expression, span)));
            }
            None => continue,
        }
    }

    Err(error::invalid_expression(FullSpan::new(span, file_id)))
}

pub fn parse_ident_or_literal<'src>(
    tokens: &TokenStream<'src>,
    _file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let [PartialSpanned(token, _)] = tokens else {
        return Ok(None);
    };

    Ok(Some(match token {
        Token::Identifier(cow) => Expression::Variable(cow.clone()),
        Token::StringLiteral(cow) => Expression::StringLiteral(cow.clone()),
        _ => return Ok(None),
    }))
}
