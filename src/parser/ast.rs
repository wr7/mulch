use mulch_macros::{GCDebug, GCPtr, Parse, keyword};

use crate::{
    error::{PartialSpanned, parse::PDResult},
    gc::GCBox,
    parser::{
        self, CurlyBracketed, Ident, Parenthesized, Parse, Parser, SeparatedList, SquareBracketed,
        TokenStream, ast::ident_or_string::IdentOrString, punct,
    },
};

mod ident_or_string;
pub mod lambda;

#[doc(inline)]
pub use lambda::Lambda;

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[repr(usize)]
#[msb_reserved]
#[mulch_parse_error(parser::error::invalid_expression)]
pub enum Expression {
    Variable(Ident),
    // StringLiteral(GCString),
    // NumericLiteral(GCString),
    // Unit(),
    // Attribute set (note: ordered by index)
    #[debug_direct]
    WithIn(WithIn),

    #[debug_direct]
    LetIn(LetIn),
    // FunctionCall(FunctionCall),
    #[debug_direct]
    Lambda(Lambda),
    // BinaryOperation(BinaryOperation),
    // MemberAccess(MemberAccess),
    #[parse_hook(parse_parenthized_expression)]
    Set(CurlyBracketed<SeparatedList<NamedValue, punct![";"]>>),
    List(SquareBracketed<SeparatedList<Expression, punct![","]>>),
}

fn parse_parenthized_expression(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<Expression>> {
    Ok(Parenthesized::<Expression>::parse(parser, tokens)?.map(|val| val.0))
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[mulch_parse_error(<keyword!["let"]>::EXPECTED_ERROR_FUNCTION)]
pub struct LetIn {
    #[debug_hidden]
    pub let_: keyword!["let"],

    #[parse_until_next]
    #[error_if_not_found]
    pub variables: SeparatedList<NamedValue, punct![";"]>,

    #[debug_hidden]
    pub in_: keyword!["in"],

    #[error_if_not_found]
    pub val: GCBox<Expression>,
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[mulch_parse_error(<keyword!["with"]>::EXPECTED_ERROR_FUNCTION)]
pub struct WithIn {
    #[debug_hidden]
    pub with_: keyword!["with"],

    #[parse_until_next]
    #[error_if_not_found]
    pub variables: GCBox<Expression>,

    #[debug_hidden]
    pub in_: keyword!["in"],

    #[error_if_not_found]
    pub val: GCBox<Expression>,
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[mulch_parse_error(IdentOrString::EXPECTED_ERROR_FUNCTION)]
pub struct NamedValue {
    #[error_if_not_found]
    pub name: PartialSpanned<IdentOrString>,

    #[error_if_not_found]
    #[debug_hidden]
    pub eq_: punct!["="],

    #[error_if_not_found]
    pub value: Expression,
}
