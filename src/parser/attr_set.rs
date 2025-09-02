use std::borrow::Cow;

use itertools::Itertools as _;

use crate::{
    T,
    error::{DResult, FullSpan, PartialSpanned},
    lexer::Token,
    parser::{AttributeSet, Expression, NonBracketedIter, TokenStream, error, parse_expression},
};

pub fn parse_attribute_set<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!('{'), _)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(PartialSpanned(T!('}'), _)) = iter.next().transpose()? else {
        unreachable!()
    };

    if iter.next().is_some() {
        return Ok(None);
    }

    let [_, inside @ .., _] = tokens else {
        unreachable!()
    };

    let mut iter = NonBracketedIter::new(inside, file_id)
        .filter_ok(|tok| &***tok == &T!(;))
        .map_ok(|tok| crate::util::element_offset(tokens, tok).unwrap());

    let mut start = 1;
    let mut entries: AttributeSet<'src> = Vec::new();

    while start < tokens.len() - 1 {
        let end = iter.next().transpose()?.unwrap_or(tokens.len() - 1);

        if start == end {
            return Err(error::expected_attribute_name(
                tokens[end].as_ref().with_file_id(file_id),
            ));
        }

        let (name, expression) = parse_attribute_set_entry(&tokens[start..=end], file_id)?;

        let res = entries.binary_search_by_key(&&*name, |(k, _)| &**k);
        match res {
            Ok(idx) => {
                return Err(error::multiple_definitions_of_attribute(
                    FullSpan::new(entries[idx].0.1, file_id),
                    FullSpan::new(name.1, file_id),
                    &**name,
                ));
            }
            Err(idx) => entries.insert(idx, (name, expression)),
        }

        start = end + 1;
    }

    Ok(Some(Expression::Set(entries)))
}

/// Parses an attribute set entry.
///
/// NOTE: `tokens` should also include a trailing semicolon or closing bracket. This is used for
/// generating error messages.
fn parse_attribute_set_entry<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<(
    PartialSpanned<Cow<'src, str>>,
    PartialSpanned<Expression<'src>>,
)> {
    let PartialSpanned(Token::Identifier(attr) | Token::StringLiteral(attr), attr_span) =
        &tokens[0]
    else {
        return Err(error::expected_attribute_name(
            tokens[0].as_ref().with_file_id(file_id),
        ));
    };

    let Some(tok) = tokens.get(1) else {
        unreachable!()
    };

    let PartialSpanned(T!(=), _) = tok else {
        return Err(error::expected_token(
            &T!(=),
            tok.as_ref().with_file_id(file_id),
        ));
    };

    let [
        _name,
        _equals,
        expr @ ..,
        PartialSpanned(T!(;) | T!('}'), terminator),
    ] = tokens
    else {
        unreachable!()
    };

    let expr = parse_expression(expr, file_id)?;
    let Some(expr) = expr else {
        return Err(error::expected_expression(FullSpan::new(
            *terminator,
            file_id,
        )));
    };

    Ok((PartialSpanned(attr.clone(), *attr_span), expr))
}
