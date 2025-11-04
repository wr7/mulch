use crate::{
    error::{DResult, FullSpan, PartialSpanned},
    lexer::Token,
    parser::{self, NonBracketedIter},
    util,
};
use std::fmt::Debug;

use super::{Expression, TokenStream, parse_expression};

#[derive(Clone, PartialEq, Eq)]
pub struct BinaryOperation<'src> {
    pub lhs: Box<PartialSpanned<Expression<'src>>>,
    pub operator: BinaryOperator,
    pub rhs: Box<PartialSpanned<Expression<'src>>>,
}

impl<'src> Debug for BinaryOperation<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(self.operator.name())
            .field(&self.lhs)
            .field(&self.rhs)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Divide,
    Multiply,
    Exponent,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

pub fn parse_binary_operators<'src>(
    operators: &[(BinaryOperator, crate::lexer::Symbol)],
    right_to_left: bool,
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let iter = NonBracketedIter::new(tokens, file_id);
    let mut iter = if right_to_left {
        // Since this is a top-down parser, the direction that we parse is actually opposite to the associativity of the operator
        itertools::Either::Left(iter)
    } else {
        itertools::Either::Right(iter.rev())
    };

    loop {
        let Some(op_tok) = iter.next().transpose()? else {
            return Ok(None);
        };

        let PartialSpanned(Token::Symbol(op_sym), _) = op_tok else {
            continue;
        };

        for (op, exp_sym) in operators {
            if op_sym == exp_sym {
                let op_idx = util::element_offset(tokens, op_tok).unwrap();

                let lhs = &tokens[..op_idx];
                let rhs = &tokens[op_idx + 1..];

                let Some(lhs) = parse_expression(lhs, file_id)? else {
                    return Err(parser::error::expected_expression(FullSpan::new(
                        op_tok.1.span_at(),
                        file_id,
                    )));
                };

                let Some(rhs) = parse_expression(rhs, file_id)? else {
                    return Err(parser::error::expected_expression(FullSpan::new(
                        op_tok.1.span_after(),
                        file_id,
                    )));
                };

                return Ok(Some(Expression::BinaryOperation(BinaryOperation {
                    lhs: Box::new(lhs),
                    operator: *op,
                    rhs: Box::new(rhs),
                })));
            }
        }
    }
}

impl BinaryOperator {
    pub fn name(self) -> &'static str {
        match self {
            BinaryOperator::Add => "Add",
            BinaryOperator::Subtract => "Subtract",
            BinaryOperator::Divide => "Divide",
            BinaryOperator::Multiply => "Multiply",
            BinaryOperator::Exponent => "Exponent",
            BinaryOperator::LessThan => "LessThan",
            BinaryOperator::GreaterThan => "GreaterThan",
            BinaryOperator::LessThanOrEqual => "LessThanOrEqual",
            BinaryOperator::GreaterThanOrEqual => "GreaterThanOrEqual",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Op!(+) => "+",
            Op!(-) => "-",
            Op!(/) => "/",
            Op!(*) => "*",
            Op!(^) => "^",
            Op!(<) => "<",
            Op!(>) => ">",
            Op!(<=) => "<=",
            Op!(>=) => ">=",
        }
    }
}

macro_rules! Op {
    (+) => {
        $crate::parser::binary::BinaryOperator::Add
    };
    (-) => {
        $crate::parser::binary::BinaryOperator::Subtract
    };
    (/) => {
        $crate::parser::binary::BinaryOperator::Divide
    };
    (*) => {
        $crate::parser::binary::BinaryOperator::Multiply
    };
    (^) => {
        $crate::parser::binary::BinaryOperator::Exponent
    };
    (<) => {
        $crate::parser::binary::BinaryOperator::LessThan
    };
    (>) => {
        $crate::parser::binary::BinaryOperator::GreaterThan
    };
    (<=) => {
        $crate::parser::binary::BinaryOperator::LessThanOrEqual
    };
    (>=) => {
        $crate::parser::binary::BinaryOperator::GreaterThanOrEqual
    };
}

pub(crate) use Op;
