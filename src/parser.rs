use std::borrow::Cow;

use copyspan::Span;
use itertools::Itertools as _;
use let_in::{parse_let_in, parse_with_in};
use util::NonBracketedIter;

use crate::{
    T,
    error::{DResult, FullSpan, PartialSpanned, span_of},
    lexer::Token,
};

mod attr_set;
mod error;
mod let_in;
mod test;
pub mod util;

pub use attr_set::parse_attribute_set;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

pub type NameExpressionMap<'src> = Vec<(
    PartialSpanned<Cow<'src, str>>,
    PartialSpanned<Expression<'src>>,
)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithIn<'src> {
    set: Box<PartialSpanned<Expression<'src>>>,
    expression: Box<PartialSpanned<Expression<'src>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetIn<'src> {
    bindings: NameExpressionMap<'src>,
    expression: Box<PartialSpanned<Expression<'src>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression<'src> {
    Variable(Cow<'src, str>),
    StringLiteral(Cow<'src, str>),
    NumericLiteral(Cow<'src, str>),
    /// Attribute set (note: ordered by index)
    Set(NameExpressionMap<'src>),
    List(Vec<PartialSpanned<Expression<'src>>>),
    WithIn(WithIn<'src>),
    LetIn(LetIn<'src>),
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
    let rules = [
        parse_ident_or_literal,
        parse_parenthesized,
        parse_attribute_set,
        parse_list,
        parse_with_in,
        parse_let_in,
    ];

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

pub fn parse_parenthesized<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!('('), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(PartialSpanned(T!(')'), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    if iter.next().transpose()?.is_some() {
        return Ok(None);
    };

    let [_opening, expr @ .., _closing] = tokens else {
        unreachable!()
    };

    Ok(parse_expression(expr, file_id)?.map(|PartialSpanned(expr, _)| expr))
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
        Token::Number(cow) => Expression::NumericLiteral(cow.clone()),
        _ => return Ok(None),
    }))
}

pub fn parse_list<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!('['), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(PartialSpanned(T!(']'), _)) = iter.next().transpose()? else {
        unreachable!()
    };

    if iter.next().is_some() {
        return Ok(None);
    }

    let [_, inside @ .., _] = tokens else {
        unreachable!()
    };

    let mut iter = NonBracketedIter::new(inside, file_id)
        .filter_ok(|tok| &***tok == &T!(,))
        .map_ok(|tok| crate::util::element_offset(tokens, tok).unwrap());

    let mut start = 1;
    let mut elements: Vec<PartialSpanned<Expression<'src>>> = Vec::new();

    while start < tokens.len() - 1 {
        let end = iter.next().transpose()?.unwrap_or(tokens.len() - 1);

        let Some(expression) = parse_expression(&tokens[start..end], file_id)? else {
            return Err(error::expected_expression(FullSpan::new(
                Span::at(start),
                file_id,
            )));
        };

        elements.push(expression);

        start = end + 1;
    }

    Ok(Some(Expression::List(elements)))
}
