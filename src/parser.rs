use std::borrow::Cow;

use copyspan::Span;
use itertools::Itertools as _;
use util::NonBracketedIter;

use crate::{
    T,
    error::{DResult, FullSpan, PartialSpanned, span_of},
    lexer::Token,
};

mod attr_set;
mod error;
mod test;
pub mod util;

pub use attr_set::parse_attribute_set;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

pub type AttributeSet<'src> = Vec<(
    PartialSpanned<Cow<'src, str>>,
    PartialSpanned<Expression<'src>>,
)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithIn<'src> {
    set: Box<PartialSpanned<Expression<'src>>>,
    expression: Box<PartialSpanned<Expression<'src>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression<'src> {
    Variable(Cow<'src, str>),
    StringLiteral(Cow<'src, str>),
    /// Attribute set (note: ordered by index)
    Set(AttributeSet<'src>),
    List(Vec<PartialSpanned<Expression<'src>>>),
    WithIn(WithIn<'src>),
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
        parse_attribute_set,
        parse_list,
        parse_with_in,
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

pub fn parse_with_in<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!(with), with_span)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(semicolon) = iter
        .find(|t| matches!(t, Err(_) | Ok(PartialSpanned(T!(;), _))))
        .transpose()?
    else {
        return Ok(None);
    };

    let semicolon = crate::util::element_offset(tokens, semicolon).unwrap();

    let Some(in_ @ PartialSpanned(T!(in), in_span)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let in_ = crate::util::element_offset(tokens, in_).unwrap();

    let set = &tokens[1..semicolon];
    let Some(set) = parse_expression(set, file_id)? else {
        return Err(error::expected_expression(FullSpan::new(
            with_span.span_after(),
            file_id,
        )));
    };

    let expression = &tokens[in_ + 1..];
    let Some(expression) = parse_expression(expression, file_id)? else {
        return Err(error::expected_expression(FullSpan::new(
            in_span.span_after(),
            file_id,
        )));
    };

    Ok(Some(Expression::WithIn(WithIn {
        set: Box::new(set),
        expression: Box::new(expression),
    })))
}
