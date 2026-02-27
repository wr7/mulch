use crate::{
    error::parse::PDResult,
    parser::{Parser, TokenStream},
};

mod non_bracketed_iter;
pub use non_bracketed_iter::NonBracketedIter;


pub fn run_parse_hook<T>(
    parser: &Parser,
    tokens: &TokenStream,
    hook: fn(&Parser, &TokenStream) -> PDResult<Option<T>>,
) -> PDResult<Option<T>> {
    hook(parser, tokens)
}

pub fn run_directional_parse_hook<T>(
    parser: &Parser,
    tokens_input: &mut &TokenStream,
    hook: fn(&Parser, &mut &TokenStream) -> PDResult<Option<T>>,
) -> PDResult<Option<T>> {
    let mut tokens = *tokens_input;
    let res = hook(parser, &mut tokens)?;
    if res.is_some() {
        *tokens_input = tokens;
    }

    Ok(res)
}
