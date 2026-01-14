use std::fmt::Debug;

use copyspan::Span;
use itertools::Itertools as _;

use crate::{
    T,
    error::{DResult, FullSpan, PartialSpanned, span_of},
    lexer::Token,
    parser_old::{self, parse_expression},
    util::element_offset,
};

use super::{Expression, NonBracketedIter, TokenStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda {
    pub args: Box<PartialSpanned<Args>>,
    pub expression: Box<PartialSpanned<Expression>>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Args {
    Single(String),
    List(Vec<PartialSpanned<Args>>),
    AttrSet(Vec<ArgAttribute>),
}

#[derive(Clone, PartialEq, Eq)]
pub struct ArgAttribute {
    pub name: PartialSpanned<String>,
    pub default: Option<Box<PartialSpanned<Expression>>>,
}

pub fn parse_lambda(tokens: &TokenStream, file_id: usize) -> DResult<Option<Expression>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let (arrow, arrow_span) = loop {
        let Some(tok) = iter.next().transpose()? else {
            return Ok(None);
        };

        if let PartialSpanned(T!(->), arrow_span) = tok {
            break (element_offset(tokens, tok).unwrap(), arrow_span);
        }
    };

    let args = &tokens[0..arrow];
    let Some(args) = parse_args(args, file_id)? else {
        return Err(parser_old::error::expected_lambda_arguments(FullSpan::new(
            arrow_span.span_at(),
            file_id,
        )));
    };

    let expr = &tokens[arrow + 1..];
    let Some(expr) = parse_expression(expr, file_id)? else {
        return Err(parser_old::error::expected_expression(FullSpan::new(
            arrow_span.span_after(),
            file_id,
        )));
    };

    Ok(Some(Expression::Lambda(Lambda {
        args: Box::new(args),
        expression: Box::new(expr),
    })))
}

fn parse_args(tokens: &TokenStream, file_id: usize) -> DResult<Option<PartialSpanned<Args>>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let parsers = [parse_single_arg, parse_list_args, parse_set_args];

    for parser_old in parsers {
        let res = parser_old(tokens, file_id)?;

        if let Some(parsed) = res {
            return Ok(Some(PartialSpanned(parsed, span_of(tokens).unwrap())));
        }
    }

    Err(super::error::invalid_lambda_arguments(FullSpan::new(
        span_of(tokens).unwrap(),
        file_id,
    )))
}

fn parse_list_args(tokens: &TokenStream, file_id: usize) -> DResult<Option<Args>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!('[') | T!('('), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(PartialSpanned(T!(']') | T!(')'), _)) = iter.next().transpose()? else {
        unreachable!()
    };

    if let Some(tok) = iter.next().transpose()? {
        let start = tok.1.start;
        let end = tokens.last().unwrap().1.end;

        return Err(super::error::unexpected_tokens(FullSpan::new(
            start..end,
            file_id,
        )));
    }

    let [_opening, inside @ .., _closing] = tokens else {
        unreachable!()
    };

    let mut iter = NonBracketedIter::new(inside, file_id)
        .filter_ok(|tok| ***tok == T!(,))
        .map_ok(|tok| crate::util::element_offset(tokens, tok).unwrap());

    let mut start = 1;
    let mut args: Vec<PartialSpanned<Args>> = Vec::new();

    while start < tokens.len() - 1 {
        let end = iter.next().transpose()?.unwrap_or(tokens.len() - 1);

        let Some(argument) = parse_args(&tokens[start..end], file_id)? else {
            return Err(super::error::expected_lambda_arguments(FullSpan::new(
                tokens[start].1.with_len(0),
                file_id,
            )));
        };

        args.push(argument);

        start = end + 1;
    }

    Ok(Some(Args::List(args)))
}

fn parse_single_arg(tokens: &TokenStream, _file_id: usize) -> DResult<Option<Args>> {
    let [PartialSpanned(Token::Identifier(name), _)] = tokens else {
        return Ok(None);
    };

    Ok(Some(Args::Single(name.to_string())))
}

fn parse_set_args(tokens: &TokenStream, file_id: usize) -> DResult<Option<Args>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!('{'), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(PartialSpanned(T!('}'), _)) = iter.next().transpose()? else {
        unreachable!()
    };

    if let Some(tok) = iter.next().transpose()? {
        let start = tok.1.start;
        let end = tokens.last().unwrap().1.end;

        return Err(super::error::unexpected_tokens(FullSpan::new(
            start..end,
            file_id,
        )));
    }

    let [_opening, inside @ .., _closing] = tokens else {
        unreachable!()
    };

    let mut iter = NonBracketedIter::new(inside, file_id)
        .filter_ok(|tok| ***tok == T!(,))
        .map_ok(|tok| crate::util::element_offset(tokens, tok).unwrap());

    let mut start = 1;
    let mut attrs: Vec<ArgAttribute> = Vec::new();

    while start < tokens.len() - 1 {
        let end = iter.next().transpose()?.unwrap_or(tokens.len() - 1);

        let PartialSpanned(Token::Identifier(attr), attr_span) = &tokens[start] else {
            return Err(super::error::expected_attribute_name(
                tokens[start].as_ref().with_file_id(file_id),
            ));
        };

        if let Some(trailing_tokens) = span_of(&tokens[start + 1..end]) {
            return Err(super::error::unexpected_tokens(FullSpan::new(
                trailing_tokens,
                file_id,
            )));
        }

        let bs = attrs.binary_search_by_key(&&**attr, |a| &a.name);
        let idx = bs.unwrap_or_else(|i| i);

        if bs.is_ok() {
            return Err(super::error::multiple_declarations_of_attribute(
                FullSpan::new(attrs[idx].span(), file_id),
                FullSpan::new(*attr_span, file_id),
                attr,
            ));
        };

        attrs.insert(
            idx,
            ArgAttribute {
                name: PartialSpanned::new(attr.to_string(), *attr_span),
                default: None,
            },
        );

        start = end + 1;
    }

    Ok(Some(Args::AttrSet(attrs)))
}

impl ArgAttribute {
    pub fn span(&self) -> Span {
        if let Some(def) = self.default.as_ref() {
            Span::from(self.name.1.start..def.1.end)
        } else {
            self.name.1
        }
    }
}

impl Debug for ArgAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("name", &self.name)
            .field("default", &self.default)
            .finish()
    }
}

impl Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Args::Single(ident) => write!(f, "Single(\"{}\")", ident.escape_debug()),
            Args::List(args) => {
                write!(f, "List")?;
                f.debug_list().entries(args).finish()
            }
            Args::AttrSet(attrs) => {
                write!(f, "AttrSet")?;
                f.debug_list().entries(attrs).finish()
            }
        }
    }
}
