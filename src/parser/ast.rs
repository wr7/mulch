use mulch_macros::{GCDebug, GCPtr, Parse};

use crate::{
    error::PartialSpanned,
    parser::{
        self, CurlyBracketed, Ident, SeparatedList, SquareBracketed,
        ast::ident_or_string::IdentOrString,
    },
    punct,
};

mod ident_or_string;

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[repr(usize)]
#[msb_reserved]
#[mulch_parse_error(parser::error::expected_expression)]
pub enum Expression {
    Variable(Ident),
    // StringLiteral(GCString),
    // NumericLiteral(GCString),
    // Unit(),
    // Attribute set (note: ordered by index)
    Set(CurlyBracketed<SeparatedList<NamedValue, punct![;]>>),
    List(SquareBracketed<SeparatedList<Expression, punct![,]>>),
    // WithIn(WithIn),
    // LetIn(LetIn),
    // FunctionCall(FunctionCall),
    // Lambda(Lambda),
    // BinaryOperation(BinaryOperation),
    // MemberAccess(MemberAccess),
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[mulch_parse_error(IdentOrString::EXPECTED_ERROR_FUNCTION)]
pub struct NamedValue {
    name: PartialSpanned<IdentOrString>,

    #[error_if_not_found]
    eq_: punct![=],

    #[error_if_not_found]
    value: Expression,
}
