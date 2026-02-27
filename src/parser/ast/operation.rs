use mulch_macros::{GCDebug, GCPtr, Parse};

use crate::{
    Sym,
    error::{PartialSpanned, parse::PDResult},
    gc::GCBox,
    lexer::Token,
    parser::{
        Parse, Parser, TokenStream, ast, punct, traits::single_token_parse_type,
        util::NotPrecededBy,
    },
};

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    pub enum BinaryOperator {
        Token::Symbol(Sym!(+)) => Add,
        Token::Symbol(Sym!(-)) => Subtract,
        Token::Symbol(Sym!(*)) => Multiply,
        Token::Symbol(Sym!(/)) => Divide,
        Token::Symbol(Sym!(^)) => Exponentiate,
    }
}

#[derive(Clone, Copy, GCPtr, GCDebug)]
pub struct BinaryOperation {
    lhs: GCBox<PartialSpanned<ast::Expression>>,
    operator: BinaryOperator,
    rhs: GCBox<PartialSpanned<ast::Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct UnaryOperation {
    operator: UnaryOperator,
    arg: GCBox<PartialSpanned<ast::Expression>>,
}

pub(super) fn operation_parse_hook(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<ast::Expression>> {
    let Some(val) = OperationImpl::parse(parser, tokens)? else {
        return Ok(None);
    };

    Ok(Some(match val {
        OperationImpl::AddOrSubtract(val) => ast::Expression::BinaryOperation(val.into()),
        OperationImpl::Unary(val) => ast::Expression::UnaryOperation(val),
        OperationImpl::MultiplyOrDivide(val) => ast::Expression::BinaryOperation(val.into()),
        OperationImpl::Exponentiate(val) => ast::Expression::BinaryOperation(val.into()),
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
    lhs: GCBox<PartialSpanned<ast::Expression>>,

    operator: NotPrecededBy<PlusOrMinus, BinaryOperator>,

    #[parse_until_next]
    rhs: GCBox<PartialSpanned<ast::Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
#[parse_direction(right)]
struct MultiplyOrDivide {
    lhs: GCBox<PartialSpanned<ast::Expression>>,

    operator: NotPrecededBy<SlashOrAsterisk, BinaryOperator>,

    #[parse_until_next]
    rhs: GCBox<PartialSpanned<ast::Expression>>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
struct Exponentiate {
    #[parse_until_next]
    lhs: GCBox<PartialSpanned<ast::Expression>>,

    operator: punct!["^"],

    rhs: GCBox<PartialSpanned<ast::Expression>>,
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    pub enum UnaryOperator {
        Token::Symbol(Sym!(-)) => Negative,
    }
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    enum SlashOrAsterisk {
        Token::Symbol(Sym!(/)) => Slash,
        Token::Symbol(Sym!(*)) => Asterisk,
    }
}

single_token_parse_type! {
    error_function = |_| unimplemented!();

    #[derive(Clone, Copy, GCPtr, GCDebug)]
    enum PlusOrMinus {
        Token::Symbol(Sym!(+)) => Plus,
        Token::Symbol(Sym!(-)) => Minus,
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
