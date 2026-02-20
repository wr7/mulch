use mulch_macros::{GCDebug, GCPtr, Parse, ParseRight, keyword};

use crate::{
    error::{PartialSpanned, parse::PDResult},
    gc::{GCBox, GCVec, util::GCDebug},
    parser::{
        self, Bracketed, CurlyBracketed, Ident, Parenthesized, Parse, ParseRight, Parser,
        SeparatedList, SquareBracketed, TokenStream, ast::ident_or_string::IdentOrString, punct,
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

    #[debug_direct]
    Lambda(Lambda),

    MethodCall(MethodCall),
    FunctionCall(FunctionCall),
    // BinaryOperation(BinaryOperation),
    MemberAccess(MemberAccess),
    #[parse_hook(parse_parenthized_expression)]
    #[debug_direct]
    Set(Set),
    List(SquareBracketed<SeparatedList<Expression, punct![","]>>),
}

fn parse_parenthized_expression(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<Expression>> {
    Ok(Parenthesized::<Expression>::parse(parser, tokens)?.map(|val| val.0))
}

#[derive(GCPtr, Parse, Clone, Copy)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct Set(pub CurlyBracketed<SeparatedList<NamedValue, punct![";"]>>);

impl GCDebug for Set {
    unsafe fn gc_debug(
        self,
        gc: &crate::gc::GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(f, "Set ")?;
        unsafe { self.0.gc_debug(gc, f) }
    }
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[parse_direction(Right)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct MemberAccess {
    #[error_if_not_found]
    pub lhs: GCBox<Expression>,

    #[debug_hidden]
    pub dot_: punct!["."],

    pub rhs: IdentOrString,
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[parse_direction(Right)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct FunctionCall {
    function: GCBox<Expression>,

    args: FunctionCallArgs,
}

#[derive(GCPtr, GCDebug, Parse, Clone, Copy)]
#[parse_direction(Right)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct MethodCall {
    lhs: GCBox<Expression>,

    dot_: punct!["."],

    method: PartialSpanned<IdentOrString>,

    args: FunctionCallArgs,
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

#[derive(GCPtr, GCDebug, ParseRight, Clone, Copy)]
#[mulch_parse_error(|_| unimplemented!())]
#[parse_hook(function_call_args_set_hook)]
pub struct FunctionCallArgs(Parenthesized<SeparatedList<PartialSpanned<Expression>, punct![","]>>);

fn function_call_args_set_hook(
    parser: &Parser,
    tokens: &mut &TokenStream,
) -> PDResult<Option<FunctionCallArgs>> {
    let Some(PartialSpanned(set, span)) = PartialSpanned::<
        CurlyBracketed<SeparatedList<NamedValue, punct![";"]>>,
    >::parse_from_right(parser, tokens)?
    else {
        return Ok(None);
    };

    let list = unsafe {
        GCVec::new(
            parser.gc,
            &[PartialSpanned(Expression::Set(Set(set)), span)],
        )
    };

    Ok(Some(FunctionCallArgs(Bracketed(SeparatedList::from(list)))))
}
