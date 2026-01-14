use mulch_macros::{GCDebug, GCPtr};

use crate::gc::GCString;

#[derive(GCPtr, GCDebug, Clone, Copy)]
#[repr(usize)]
#[msb_reserved]
pub enum Expression {
    Variable(GCString),
    StringLiteral(GCString),
    NumericLiteral(GCString),
    Unit(),
    // /// Attribute set (note: ordered by index)
    // Set(NameExpressionMap),
    // List(Vec<PartialSpanned<Expression>>),
    // WithIn(WithIn),
    // LetIn(LetIn),
    // FunctionCall(FunctionCall),
    // Lambda(Lambda),
    // BinaryOperation(BinaryOperation),
    // MemberAccess(MemberAccess),
}
