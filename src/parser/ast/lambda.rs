use mulch_macros::{GCDebug, GCPtr, Parse, ParseLeft, punct};

use crate::{
    error::parse::PDResult,
    gc::{GCBox, GCVec},
    parser::{
        self, Bracketed, CurlyBracketed, Ident, Parenthesized, Parse, Parser, SeparatedList,
        SquareBracketed, TokenStream,
        ast::{Expression, ident_or_string::IdentOrString},
    },
};

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct Lambda {
    #[parse_until_next]
    #[error_if_not_found]
    args: Arguments,
    #[debug_hidden]
    arrow: punct!("->"),
    #[error_if_not_found]
    expr: GCBox<Expression>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(parser::error::expected_lambda_arguments)]
#[parse_hook(arguments_parse_hook)]
#[debug_direct]
pub struct Arguments(pub Parenthesized<SeparatedList<Argument, punct!(",")>>);

fn arguments_parse_hook(parser: &Parser, tokens: &TokenStream) -> PDResult<Option<Arguments>> {
    let args = if let Some(args) = SingleArgument::parse(parser, tokens)? {
        Argument::Single(args)
    } else if let Some(args) = SetArgument::parse(parser, tokens)? {
        Argument::Set(args)
    } else {
        return Ok(None);
    };

    Ok(Some(Arguments(Bracketed(SeparatedList::from(unsafe {
        GCVec::new(parser.gc, &[args])
    })))))
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(parser::error::expected_lambda_argument)]
pub enum Argument {
    #[debug_direct]
    Single(SingleArgument),
    #[debug_direct]
    List(ListArgument),
    #[debug_direct]
    Set(SetArgument),
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct SingleArgument {
    name: Ident,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct ListArgument {
    list: SquareBracketed<SeparatedList<Argument, punct!(",")>>,
    binding: Option<ArgBinding>,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(|_| unimplemented!())]
pub struct SetArgument {
    set: CurlyBracketed<SeparatedList<ArgAttribute, punct!(";")>>,
    binding: Option<ArgBinding>,
    default_value: Option<ArgDefaultValue>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, Parse)]
#[mulch_parse_error(parser::error::expected_lambda_attribute_argument)]
#[parse_hook(parse_simple_arg_attribute)]
pub struct ArgAttribute {
    #[error_if_not_found]
    attr: IdentOrString,

    #[debug_hidden]
    colon_: punct!(":"),

    arg: Argument,
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

    #[error_if_not_found]
    val: GCBox<Expression>,
}

fn parse_simple_arg_attribute(
    parser: &Parser,
    tokens: &TokenStream,
) -> PDResult<Option<ArgAttribute>> {
    Ok(
        SingleArgument::parse(parser, tokens)?.map(|arg| ArgAttribute {
            attr: IdentOrString(arg.name.0),
            colon_: punct!(":")(),
            arg: Argument::Single(arg),
        }),
    )
}
