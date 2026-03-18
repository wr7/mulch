use mulch_macros::{GCDebug, GCPtr, Parse};

use crate::{
    Sym,
    error::{PartialSpanned, parse::PDResult},
    gc::GCBox,
    lexer::Token,
    parser::{
        Parse, Parser, TokenStream, ast::Expression, punct, traits::single_token_parse_type,
        util::NotPrecededBy,
    },
};

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    pub enum BinaryOperator {
        PartialSpanned(Token::Symbol(Sym!(+)), _) => Add,
        PartialSpanned(Token::Symbol(Sym!(-)), _) => Subtract,
        PartialSpanned(Token::Symbol(Sym!(*)), _) => Multiply,
        PartialSpanned(Token::Symbol(Sym!(/)), _) => Divide,
        PartialSpanned(Token::Symbol(Sym!(^)), _) => Exponentiate,
    }
}

#[derive(Clone, Copy, GCPtr, GCDebug)]
pub struct BinaryOperation {
    lhs: GCBox<PartialSpanned<Expression>>,
    operator: BinaryOperator,
    rhs: GCBox<PartialSpanned<Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct UnaryOperation {
    operator: UnaryOperator,
    arg: GCBox<PartialSpanned<Expression>>,
}

pub(super) fn operation_parse_hook(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<Expression>> {
    let Some(val) = OperationImpl::parse(parser, tokens)? else {
        return Ok(None);
    };

    Ok(Some(match val {
        OperationImpl::AddOrSubtract(val) => Expression::BinaryOperation(val.into()),
        OperationImpl::Unary(val) => Expression::UnaryOperation(val),
        OperationImpl::MultiplyOrDivide(val) => Expression::BinaryOperation(val.into()),
        OperationImpl::Exponentiate(val) => Expression::BinaryOperation(val.into()),
    }))
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
enum OperationImpl {
    AddOrSubtract(AddOrSubtract),
    Unary(UnaryOperation),
    MultiplyOrDivide(MultiplyOrDivide),
    Exponentiate(Exponentiate),
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
#[parse_direction(right)]
struct AddOrSubtract {
    lhs: GCBox<PartialSpanned<Expression>>,

    operator: NotPrecededBy<PlusOrMinus, BinaryOperator>,

    #[parse_until_next]
    rhs: GCBox<PartialSpanned<Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
#[parse_direction(right)]
struct MultiplyOrDivide {
    lhs: GCBox<PartialSpanned<Expression>>,

    operator: NotPrecededBy<SlashOrAsterisk, BinaryOperator>,

    #[parse_until_next]
    rhs: GCBox<PartialSpanned<Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
struct Exponentiate {
    #[parse_until_next]
    lhs: GCBox<PartialSpanned<Expression>>,

    operator: punct!["^"],

    rhs: GCBox<PartialSpanned<Expression>>,
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    pub enum UnaryOperator {
        PartialSpanned(Token::Symbol(Sym!(-)), _) => Negative,
    }
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    enum SlashOrAsterisk {
        PartialSpanned(Token::Symbol(Sym!(/)), _) => Slash,
        PartialSpanned(Token::Symbol(Sym!(*)), _) => Asterisk,
    }
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    enum PlusOrMinus {
        PartialSpanned(Token::Symbol(Sym!(+)), _) => Plus,
        PartialSpanned(Token::Symbol(Sym!(-)), _) => Minus,
    }
}

impl From<AddOrSubtract> for BinaryOperation {
    fn from(value: AddOrSubtract) -> Self {
        let operator = match value.operator.value {
            PlusOrMinus::Plus => BinaryOperator::Add,
            PlusOrMinus::Minus => BinaryOperator::Subtract,
        };

        BinaryOperation {
            lhs: value.lhs,
            operator,
            rhs: value.rhs,
        }
    }
}

impl From<MultiplyOrDivide> for BinaryOperation {
    fn from(value: MultiplyOrDivide) -> Self {
        let operator = match value.operator.value {
            SlashOrAsterisk::Slash => BinaryOperator::Divide,
            SlashOrAsterisk::Asterisk => BinaryOperator::Multiply,
        };

        BinaryOperation {
            lhs: value.lhs,
            operator,
            rhs: value.rhs,
        }
    }
}

impl From<Exponentiate> for BinaryOperation {
    fn from(value: Exponentiate) -> Self {
        BinaryOperation {
            lhs: value.lhs,
            operator: BinaryOperator::Exponentiate,
            rhs: value.rhs,
        }
    }
}
