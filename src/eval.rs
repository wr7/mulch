use mulch_macros::{GCDebug, GCPtr};

mod error;
mod lazyvalue;
mod set;

pub use set::Set;

use crate::{
    error::{DResult, Spanned},
    gc::{GCNumber, GCString, GCVec, GarbageCollector},
    parser::ast,
};

pub struct Evaluator<'gc> {
    gc: &'gc GarbageCollector,
    #[allow(unused)]
    scope: (), // placeholder scope field
}

impl<'gc> Evaluator<'gc> {
    pub fn new(gc: &'gc GarbageCollector) -> Self {
        Self { gc, scope: () }
    }

    pub fn evaluate(&self, ast: Spanned<ast::Expression>) -> DResult<MValue> {
        match ast.0 {
            ast::Expression::Variable(ident) => todo!(),
            ast::Expression::StringLiteral(string_literal) => Ok(string_literal.0.into()),
            ast::Expression::NumericLiteral(number_literal) => Ok(number_literal.0.into()),
            ast::Expression::WithIn(_) => todo!(),
            ast::Expression::LetIn(let_in) => todo!(),
            ast::Expression::Lambda(lambda) => todo!(),
            ast::Expression::BinaryOperation(binary_operation) => todo!(),
            ast::Expression::UnaryOperation(unary_operation) => todo!(),
            ast::Expression::MethodCall(method_call) => todo!(),
            ast::Expression::FunctionCall(function_call) => todo!(),
            ast::Expression::MemberAccess(member_access) => {
                self.evaluate_member_access(Spanned(member_access, ast.1))
            }
            ast::Expression::Set(set) => self.evaluate_set(Spanned(set, ast.1)),
            ast::Expression::List(list) => todo!(),
        }
    }
}

/// A `mulch` value
#[derive(GCPtr, GCDebug, Clone)]
#[repr(usize)]
pub enum MValue {
    #[debug_direct]
    String(GCString) = 1,
    #[debug_direct]
    Number(GCNumber),
    #[debug_direct]
    List(GCVec<MValue>),
    #[debug_direct]
    Set(Set),
}

impl From<GCString> for MValue {
    fn from(value: GCString) -> Self {
        MValue::String(value)
    }
}

impl From<GCNumber> for MValue {
    fn from(value: GCNumber) -> Self {
        MValue::Number(value)
    }
}

impl From<GCVec<MValue>> for MValue {
    fn from(value: GCVec<MValue>) -> Self {
        MValue::List(value)
    }
}
