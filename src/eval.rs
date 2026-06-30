use mulch_macros::{GCDebug, GCProject, GCPtr, gc_fn};

mod error;
mod lazyvalue;
mod set;

pub use set::Set;

use crate::{
    error::{DResult, Spanned},
    eval::set::{evaluate_member_access, evaluate_set},
    gc::{
        GCNumber, GCString, GCVec,
        safety::{GC, Projected, gc_args, rebind},
    },
    parser::ast,
};

#[gc_fn]
pub fn evaluate<'c>(ctx: &'c mut gc!(ast: Spanned<ast::Expression>)) -> DResult<GC<'c, MValue>> {
    let ast = ast.project();
    let ast_span = ast.1;
    let ast = ast.0;

    match ast.project() {
        Projected::<ast::Expression>::Variable(_) => todo!(),
        Projected::<ast::Expression>::StringLiteral(string_literal) => {
            let string_literal = rebind!(ctx, string_literal);
            Ok(string_literal.project().0.into())
        }
        Projected::<ast::Expression>::NumericLiteral(number_literal) => {
            let number_literal = rebind!(ctx, number_literal);
            Ok(number_literal.project().0.into())
        }
        Projected::<ast::Expression>::WithIn(_) => todo!(),
        Projected::<ast::Expression>::LetIn(_let_in) => todo!(),
        Projected::<ast::Expression>::Lambda(_lambda) => todo!(),
        Projected::<ast::Expression>::BinaryOperation(_binary_operation) => todo!(),
        Projected::<ast::Expression>::UnaryOperation(_unary_operation) => todo!(),
        Projected::<ast::Expression>::MethodCall(_method_call) => todo!(),
        Projected::<ast::Expression>::FunctionCall(_function_call) => todo!(),
        Projected::<ast::Expression>::MemberAccess(member_access) => {
            evaluate_member_access(gc_args!(ctx, Spanned(member_access, ast_span).into()))
        }
        Projected::<ast::Expression>::Set(set) => {
            evaluate_set(gc_args!(ctx, Spanned(set, ast_span).into()))
        }
        Projected::<ast::Expression>::List(_list) => todo!(),
    }
}

/// A `mulch` value
#[derive(GCPtr, GCDebug, Clone, GCProject)]
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

impl<'c> From<GC<'c, GCString>> for GC<'c, MValue> {
    fn from(value: GC<'c, GCString>) -> Self {
        Projected::<MValue>::String(value).into()
    }
}

impl<'c> From<GC<'c, GCNumber>> for GC<'c, MValue> {
    fn from(value: GC<'c, GCNumber>) -> Self {
        Projected::<MValue>::Number(value).into()
    }
}

impl<'c> From<GC<'c, GCVec<MValue>>> for GC<'c, MValue> {
    fn from(value: GC<'c, GCVec<MValue>>) -> Self {
        Projected::<MValue>::List(value).into()
    }
}
