use crate::{
    T,
    error::{DResult, FullSpan, PartialSpanned, Spanned},
    lexer::Token,
    parser::{
        Expression, LetIn, NameExpressionMap, NonBracketedIter, TokenStream, WithIn, error,
        parse_expression,
    },
};

pub fn parse_with_in<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(PartialSpanned(T!(with), with_span)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let Some(semicolon) = iter
        .find(|t| matches!(t, Err(_) | Ok(PartialSpanned(T!(;), _))))
        .transpose()?
    else {
        return Ok(None);
    };

    let semicolon = crate::util::element_offset(tokens, semicolon).unwrap();

    let Some(in_ @ PartialSpanned(T!(in), in_span)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let in_ = crate::util::element_offset(tokens, in_).unwrap();

    let set = &tokens[1..semicolon];
    let Some(set) = parse_expression(set, file_id)? else {
        return Err(error::expected_expression(FullSpan::new(
            with_span.span_after(),
            file_id,
        )));
    };

    let expression = &tokens[in_ + 1..];
    let Some(expression) = parse_expression(expression, file_id)? else {
        return Err(error::expected_expression(FullSpan::new(
            in_span.span_after(),
            file_id,
        )));
    };

    Ok(Some(Expression::WithIn(WithIn {
        set: Box::new(set),
        expression: Box::new(expression),
    })))
}

pub fn parse_let_in<'src>(
    tokens: &TokenStream<'src>,
    file_id: usize,
) -> DResult<Option<Expression<'src>>> {
    let mut iter = NonBracketedIter::new(tokens, file_id);

    let Some(_let @ PartialSpanned(T!(let), let_span)) = iter.next().transpose()? else {
        return Ok(None);
    };

    let mut name_expression_map: NameExpressionMap<'src> = Vec::new();
    let mut last_span = *let_span;

    let (in_, in_span) = loop {
        match [iter.next().transpose()?, iter.next().transpose()?] {
            [
                Some(PartialSpanned(Token::Identifier(name), name_span)),
                Some(equals @ PartialSpanned(T!(=), equals_span)),
            ] => {
                let equals = crate::util::element_offset(tokens, equals).unwrap();

                let Some(semicolon) = iter
                    .find(|t| matches!(t, Err(_) | Ok(PartialSpanned(T!(;), _))))
                    .transpose()?
                else {
                    return Err(error::let_in_eof(
                        FullSpan::new(tokens.last().unwrap().1, file_id),
                        FullSpan::new(*let_span, file_id),
                    ));
                };

                last_span = semicolon.1;
                let semicolon = crate::util::element_offset(tokens, semicolon).unwrap();

                let expression = &tokens[equals + 1..semicolon];
                let Some(expression) = parse_expression(expression, file_id)? else {
                    return Err(error::expected_expression(FullSpan::new(
                        equals_span.span_after(),
                        file_id,
                    )));
                };

                name_expression_map.push((PartialSpanned(name.clone(), *name_span), expression));
            }
            [Some(in_ @ PartialSpanned(T!(in), in_span)), _] => {
                let in_ = crate::util::element_offset(tokens, in_).unwrap();
                break (in_, in_span);
            }
            [Some(PartialSpanned(Token::Identifier(name), name_span)), _] => {
                return Err(error::expected_token(
                    &T!(=),
                    Spanned(
                        &Token::Identifier(name.clone()),
                        FullSpan::new(*name_span, file_id),
                    ),
                ));
            }
            [tok, _] => {
                if name_expression_map.is_empty() {
                    return Ok(None);
                }

                let got_span = FullSpan::new(
                    tok.map_or_else(|| last_span.span_after(), |tok| tok.1),
                    file_id,
                );

                return Err(error::let_in_unexpected(
                    tok.map(|t| &**t),
                    got_span,
                    FullSpan::new(*let_span, file_id),
                ));
            }
        }
    };

    let expression = &tokens[in_ + 1..];
    let expression = parse_expression(expression, file_id)?
        .ok_or_else(|| error::expected_expression(FullSpan::new(in_span.span_after(), file_id)))?;

    Ok(Some(Expression::LetIn(LetIn {
        bindings: name_expression_map,
        expression: Box::new(expression),
    })))
}
