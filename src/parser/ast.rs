use mulch_macros::GCDebug;

use crate::gc::{GCPtr, GCString};

#[derive(GCDebug, Clone, Copy)]
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

unsafe impl GCPtr for Expression {
    const MSB_RESERVED: bool = false;

    unsafe fn gc_copy(self, gc: &mut crate::gc::GarbageCollector) -> Self {
        todo!()
    }
}
