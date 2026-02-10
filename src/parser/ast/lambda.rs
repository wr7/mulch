use mulch_macros::{GCDebug, GCPtr, Parse, ParseLeft, punct};

use crate::{
    error::parse::PDResult,
    gc::GCBox,
    parser::{
        self, CurlyBracketed, Ident, Parse, Parser, SeparatedList, SquareBracketed, TokenStream,
        ast::{Expression, ident_or_string::IdentOrString},
    },
};

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct Lambda {
    #[parse_until_next]
    #[error_if_not_found]
    args: Args,
    #[debug_hidden]
    arrow: punct!("->"),
    #[error_if_not_found]
    expr: GCBox<Expression>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(parser::error::expected_lambda_arguments)]
pub enum Args {
    #[debug_direct]
    Single(SingleArg),
    #[debug_direct]
    List(ListArgs),
    #[debug_direct]
    Set(SetArgs),
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct SingleArg {
    name: Ident,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct ListArgs {
    list: SquareBracketed<SeparatedList<Args, punct!(",")>>,
    binding: Option<ArgBinding>,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct SetArgs {
    set: CurlyBracketed<SeparatedList<ArgAttribute, punct!(";")>>,
    binding: Option<ArgBinding>,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| todo!())]
#[parse_hook(parse_simple_arg_attribute)]
pub struct ArgAttribute {
    attr: IdentOrString,
    colon_: punct!(":"),
    arg: Args,
}

#[derive(Clone, Copy, GCPtr, GCDebug, ParseLeft)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct ArgBinding {
    #[debug_hidden]
    at_: punct!("@"),
    name: Ident,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
#[debug_direct]
pub struct ArgDefaultValue {
    #[debug_hidden]
    eq_: punct!("="),
    val: GCBox<Expression>,
}

fn parse_simple_arg_attribute(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<ArgAttribute>> {
    Ok(SingleArg::parse(parser, tokens)?.map(|arg| ArgAttribute {
        attr: IdentOrString(arg.name.0),
        colon_: punct!(":")(),
        arg: Args::Single(arg),
    }))
}
