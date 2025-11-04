use crate::error::PartialSpanned;
use std::fmt::Debug;

use super::Expression;

#[derive(Clone, PartialEq, Eq)]
pub struct BinaryOperation<'src> {
    lhs: Box<PartialSpanned<Expression<'src>>>,
    operator: BinaryOperator,
    rhs: Box<PartialSpanned<Expression<'src>>>,
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
