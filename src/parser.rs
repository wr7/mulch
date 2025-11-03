use std::borrow::Cow;

use copyspan::Span;
use itertools::Itertools as _;
use let_in::{parse_let_in, parse_with_in};
use std::fmt::Debug;
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

pub mod lambda;
pub mod util;

pub use attr_set::parse_attribute_set;
pub use lambda::Lambda;

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
pub struct FunctionCall<'src> {
    function: Box<PartialSpanned<Expression<'src>>>,
    args: Box<PartialSpanned<Expression<'src>>>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Expression<'src> {
    Variable(Cow<'src, str>),
    StringLiteral(Cow<'src, str>),
    NumericLiteral(Cow<'src, str>),
    Unit(),
    /// Attribute set (note: ordered by index)
    Set(NameExpressionMap<'src>),
    List(Vec<PartialSpanned<Expression<'src>>>),
    WithIn(WithIn<'src>),
    LetIn(LetIn<'src>),
    FunctionCall(FunctionCall<'src>),
    Lambda(Lambda<'src>),
}

impl<'src> Debug for Expression<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Expression::Variable(val) => f.debug_tuple("Variable").field(val).finish(),
            Expression::StringLiteral(val) => f.debug_tuple("StringLiteral").field(val).finish(),
            Expression::NumericLiteral(val) => f.debug_tuple("NumericLiteral").field(val).finish(),
            Expression::Unit() => f.debug_tuple("Unit").finish(),
            Expression::Set(entries) => {
                write!(f, "Set ")?;
                f.debug_list().entries(entries).finish()
            }
            Expression::List(items) => {
                write!(f, "List ")?;
                f.debug_list().entries(items).finish()
            }
            Expression::WithIn(with_in) => Debug::fmt(with_in, f),
            Expression::LetIn(let_in) => Debug::fmt(let_in, f),
            Expression::FunctionCall(function_call) => Debug::fmt(function_call, f),
            Expression::Lambda(lambda) => Debug::fmt(lambda, f),
        }
    }
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
        lambda::parse_lambda,
        parse_function_call,
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

    Ok(Some(
        parse_expression(expr, file_id)?.map_or(Expression::Unit(), |PartialSpanned(e, _)| e),
    ))
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
                tokens[start + 1].1.with_len(0),
                file_id,
            )));
        };

        elements.push(expression);

        start = end + 1;
    }

    Ok(Some(Expression::List(elements)))
}

/// Parses a function call or index operation such as `my_array[0]`, `my_function[a, b]`, `my_function{foo = "bar"}`, or `my_function()`
pub fn parse_function_call<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(Token::ClosingBracket(_), _)) = iter.next_back().transpose()? else {
        return Ok(None);
    };

    let Some(opening_bracket @ PartialSpanned(Token::OpeningBracket(_), _)) =
        iter.next_back().transpose()?
    else {
        unreachable!()
    };

    let opening_bracket = crate::util::element_offset(tokens, opening_bracket).unwrap();

    let function = &tokens[..opening_bracket];
    let Some(function) = parse_expression(function, file_id)? else {
        return Ok(None);
    };

    let args = &tokens[opening_bracket..];
    let args = parse_expression(args, file_id)?.unwrap();

    Ok(Some(Expression::FunctionCall(FunctionCall {
        function: Box::new(function),
        args: Box::new(args),
    })))
}
