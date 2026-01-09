use attr_set::parse_attribute_set_raw;
use binary::parse_binary_operators;
use itertools::Itertools as _;
use lambda::parse_lambda;
use let_in::{parse_let_in, parse_with_in};
use std::fmt::Debug;
use util::NonBracketedIter;

use crate::{
    Op, T,
    error::{DResult, FullSpan, PartialSpanned, span_of},
    lexer::{BracketType, Sym, Token},
};

/// Module for new, garbage-collected, AST nodes
mod ast;
mod attr_set;
mod error;
mod let_in;
mod test;

pub mod binary;
pub mod lambda;
pub mod util;

pub use attr_set::parse_attribute_set;
pub use binary::BinaryOperation;
pub use lambda::Lambda;

pub type TokenStream<'src> = [PartialSpanned<Token<'src>>];

pub type NameExpressionMap = Vec<(PartialSpanned<String>, PartialSpanned<Expression>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithIn {
    set: Box<PartialSpanned<Expression>>,
    expression: Box<PartialSpanned<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetIn {
    bindings: NameExpressionMap,
    expression: Box<PartialSpanned<Expression>>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum FunctionArgs {
    Set(NameExpressionMap),
    List(Vec<PartialSpanned<Expression>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    function: Box<PartialSpanned<Expression>>,
    args: Box<PartialSpanned<FunctionArgs>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAccess {
    lhs: Box<PartialSpanned<Expression>>,
    rhs: PartialSpanned<String>,
}

#[derive(Clone, PartialEq, Eq)]
// `repr(usize)` so that it can be stored in the garbage collector. With this representation, the
// `MSB will always be `0`, so we can use a 1 in the MSB to indicate a forwarding pointer.
#[repr(usize)]
pub enum Expression {
    Variable(String),
    StringLiteral(String),
    NumericLiteral(String),
    Unit(),
    /// Attribute set (note: ordered by index)
    Set(NameExpressionMap),
    List(Vec<PartialSpanned<Expression>>),
    WithIn(WithIn),
    LetIn(LetIn),
    FunctionCall(FunctionCall),
    Lambda(Lambda),
    BinaryOperation(BinaryOperation),
    MemberAccess(MemberAccess),
}

impl Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Expression::Variable(val) => write!(f, "Variable(\"{}\")", val.escape_debug()),
            Expression::StringLiteral(val) => {
                write!(f, "StringLiteral(\"{}\")", val.escape_debug())
            }
            Expression::NumericLiteral(val) => {
                write!(f, "NumericLiteral(\"{}\")", val.escape_debug())
            }
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
            Expression::BinaryOperation(binop) => Debug::fmt(binop, f),
            Expression::MemberAccess(acc) => Debug::fmt(acc, f),
        }
    }
}

impl Debug for FunctionArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionArgs::Set(entries) => {
                write!(f, "Set ")?;
                f.debug_list().entries(entries).finish()
            }
            FunctionArgs::List(items) => {
                write!(f, "List ")?;
                f.debug_list().entries(items).finish()
            }
        }
    }
}

/// Parses an expression; returns Ok(None) iff `tokens` is empty.
pub fn parse_expression<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<PartialSpanned<Expression>>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let span = span_of(tokens).unwrap();
    const RULES: &[fn(&TokenStream, usize) -> DResult<Option<Expression>>] = &[
        parse_ident_or_literal,
        parse_parenthesized,
        parse_attribute_set,
        parse_list,
        parse_with_in,
        parse_let_in,
        parse_lambda,
        |tokens, file_id| {
            parse_binary_operators(
                &[(Op!(+), Sym!(+)), (Op!(-), Sym!(-))],
                false,
                tokens,
                file_id,
            )
        },
        |tokens, file_id| {
            parse_binary_operators(
                &[(Op!(*), Sym!(*)), (Op!(/), Sym!(/))],
                false,
                tokens,
                file_id,
            )
        },
        |tokens, file_id| parse_binary_operators(&[(Op!(^), Sym!(^))], true, tokens, file_id),
        parse_member_access,
        parse_function_call,
    ];

    for rule in RULES {
        match rule(tokens, file_id)? {
            Some(expression) => {
                return Ok(Some(PartialSpanned::new(expression, span)));
            }
            None => continue,
        }
    }

    Err(error::invalid_expression(FullSpan::new(span, file_id)))
}

pub fn parse_parenthesized(tokens: &TokenStream, file_id: usize) -> DResult<Option<Expression>> {
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

pub fn parse_ident_or_literal(
    tokens: &TokenStream,
    _file_id: usize,
) -> DResult<Option<Expression>> {
    let [PartialSpanned(token, _)] = tokens else {
        return Ok(None);
    };

    Ok(Some(match token {
        Token::Identifier(cow) => Expression::Variable(cow.to_string()),
        Token::StringLiteral(cow) => Expression::StringLiteral(cow.to_string()),
        Token::Number(cow) => Expression::NumericLiteral(cow.to_string()),
        _ => return Ok(None),
    }))
}

pub fn parse_list(tokens: &TokenStream, file_id: usize) -> DResult<Option<Expression>> {
    let Some(list) = parse_list_raw(tokens, file_id)? else {
        return Ok(None);
    };
    return Ok(Some(Expression::List(list)));
}

fn parse_list_raw(
    tokens: &TokenStream,
    file_id: usize,
) -> DResult<Option<Vec<PartialSpanned<Expression>>>> {
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
    let mut elements: Vec<PartialSpanned<Expression>> = Vec::new();

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

    Ok(Some(elements))
}

pub fn parse_function_args(
    tokens: &TokenStream,
    file_id: usize,
) -> DResult<Option<PartialSpanned<FunctionArgs>>> {
    let Some(span) = crate::error::span_of(tokens) else {
        return Ok(None);
    };

    let args = if let Some(list) = parse_list_raw(tokens, file_id)? {
        FunctionArgs::List(list)
    } else if let Some(set) = parse_attribute_set_raw(tokens, file_id)? {
        FunctionArgs::Set(set)
    } else {
        return Ok(None);
    };

    Ok(Some(PartialSpanned(args, span)))
}

/// Parses a function call or index operation such as `my_array[0]`, `my_function[a, b]`, or `my_function{foo = "bar"}`
pub fn parse_function_call(tokens: &TokenStream, file_id: usize) -> DResult<Option<Expression>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(Token::ClosingBracket(_), _)) = iter.next_back().transpose()? else {
        return Ok(None);
    };

    let Some(opening_bracket_tok @ PartialSpanned(Token::OpeningBracket(bracket_type), _)) =
        iter.next_back().transpose()?
    else {
        unreachable!()
    };

    let opening_bracket = crate::util::element_offset(tokens, opening_bracket_tok).unwrap();

    let function = &tokens[..opening_bracket];
    let Some(function) = parse_expression(function, file_id)? else {
        return Ok(None);
    };

    if !matches!(bracket_type, BracketType::Square | BracketType::Curly) {
        let opening_bracket = opening_bracket_tok.as_ref().with_file_id(file_id);
        let expr_span = crate::error::span_of(tokens).unwrap();

        return Err(crate::parser::error::invalid_function_call_args(
            FullSpan::new(expr_span, file_id),
            opening_bracket,
        ));
    }

    let args = &tokens[opening_bracket..];
    let args = parse_function_args(args, file_id)?.unwrap();

    Ok(Some(Expression::FunctionCall(FunctionCall {
        function: Box::new(function),
        args: Box::new(args),
    })))
}

pub fn parse_member_access(tokens: &TokenStream, file_id: usize) -> DResult<Option<Expression>> {
    let [
        lhs @ ..,
        PartialSpanned(T!(.), dot_span),
        PartialSpanned(Token::Identifier(rhs), rhs_span),
    ] = tokens
    else {
        return Ok(None);
    };

    let Some(lhs) = parse_expression(lhs, file_id)? else {
        return Err(error::expected_expression(FullSpan::new(
            *dot_span, file_id,
        )));
    };

    Ok(Some(Expression::MemberAccess(MemberAccess {
        lhs: Box::new(lhs),
        rhs: PartialSpanned(rhs.to_string(), *rhs_span),
    })))
}
