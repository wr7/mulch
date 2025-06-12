use crate::{
    error::{DResult, FullSpan, PartialSpanned},
    lexer::{BracketType, Token},
    parser::error,
};

/// Iterates over tokens that are not surrounded by brackets.
#[derive(Clone)]
pub(super) struct NonBracketedIter<'a, 'src> {
    remaining: &'a [PartialSpanned<Token<'src>>],
    file_no: usize,
    opening_bracket: Option<PartialSpanned<BracketType>>,
    closing_bracket: Option<PartialSpanned<BracketType>>,
}

impl<'a, 'src> NonBracketedIter<'a, 'src> {
    pub fn new(slc: &'a [PartialSpanned<Token<'src>>], file_no: usize) -> Self {
        Self {
            remaining: slc,
            file_no,
            opening_bracket: None,
            closing_bracket: None,
        }
    }

    pub fn remainder<'b>(&'b self) -> &'a [PartialSpanned<Token<'src>>] {
        self.remaining
    }
}

impl<'a, 'src> Iterator for NonBracketedIter<'a, 'src> {
    type Item = DResult<&'a PartialSpanned<Token<'src>>>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(opening_bracket) = self.opening_bracket.take() else {
            let (tok, remaining) = self.remaining.split_first()?;

            match **tok {
                Token::OpeningBracket(ty) => self.opening_bracket = Some(PartialSpanned(ty, tok.1)),
                Token::ClosingBracket(_) => {
                    return Some(Err(error::unmatched_bracket(FullSpan::new(
                        tok.1,
                        self.file_no,
                    ))));
                }
                _ => {}
            }

            self.remaining = remaining;
            return Some(Ok(tok));
        };

        // Now we must find the matching closing bracket and return that //

        let mut opening_brackets: Vec<PartialSpanned<BracketType>> = vec![opening_bracket];

        while let Some((tok, remaining)) = self.remaining.split_first() {
            self.remaining = remaining;

            match **tok {
                Token::OpeningBracket(bracket_type) => {
                    opening_brackets.push(PartialSpanned(bracket_type, tok.1))
                }
                Token::ClosingBracket(bracket_type) => {
                    let opening_bracket = opening_brackets.pop().unwrap();

                    if *opening_bracket != bracket_type {
                        return Some(Err(error::mismatched_brackets(
                            FullSpan::new(opening_bracket.1, self.file_no),
                            FullSpan::new(tok.1, self.file_no),
                        )));
                    }

                    if opening_brackets.is_empty() {
                        return Some(Ok(tok));
                    }
                }
                _ => {}
            }
        }

        if let Some(closing_bracket) = self.closing_bracket {
            if *closing_bracket == *opening_bracket {
                None
            } else {
                Some(Err(error::mismatched_brackets(
                    FullSpan::new(opening_bracket.1, self.file_no),
                    FullSpan::new(closing_bracket.1, self.file_no),
                )))
            }
        } else {
            Some(Err(error::unmatched_bracket(FullSpan::new(
                opening_bracket.1,
                self.file_no,
            ))))
        }
    }
}

impl<'a, 'src> DoubleEndedIterator for NonBracketedIter<'a, 'src> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let Some(closing_bracket) = self.closing_bracket.take() else {
            let (tok, remaining) = self.remaining.split_last()?;

            match **tok {
                Token::ClosingBracket(ty) => self.closing_bracket = Some(PartialSpanned(ty, tok.1)),
                Token::OpeningBracket(_) => {
                    return Some(Err(error::unmatched_bracket(FullSpan::new(
                        tok.1,
                        self.file_no,
                    ))));
                }
                _ => {}
            }

            self.remaining = remaining;
            return Some(Ok(tok));
        };

        // Now we must find the matching opening bracket and return that //

        let mut closing_brackets: Vec<PartialSpanned<BracketType>> = vec![closing_bracket];

        while let Some((tok, remaining)) = self.remaining.split_last() {
            self.remaining = remaining;

            match **tok {
                Token::ClosingBracket(bracket_type) => {
                    closing_brackets.push(PartialSpanned(bracket_type, tok.1))
                }
                Token::OpeningBracket(bracket_type) => {
                    let closing_bracket = closing_brackets.pop().unwrap();

                    if *closing_bracket != bracket_type {
                        return Some(Err(error::mismatched_brackets(
                            FullSpan::new(tok.1, self.file_no),
                            FullSpan::new(closing_bracket.1, self.file_no),
                        )));
                    }

                    if closing_brackets.is_empty() {
                        return Some(Ok(tok));
                    }
                }
                _ => {}
            }
        }

        if let Some(opening_bracket) = self.opening_bracket {
            if *opening_bracket == *closing_bracket {
                None
            } else {
                Some(Err(error::mismatched_brackets(
                    FullSpan::new(opening_bracket.1, self.file_no),
                    FullSpan::new(closing_bracket.1, self.file_no),
                )))
            }
        } else {
            Some(Err(error::unmatched_bracket(FullSpan::new(
                closing_bracket.1,
                self.file_no,
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{T, dresult_unwrap, error::SourceDB, lexer::Lexer};

    use super::*;

    #[test]
    fn non_bracketed_test() {
        let src = "a + ( {b0 - c}; - e ) = e1";
        let db = SourceDB::new();
        db.add("non_bracketed.mulch".into(), src.into());

        let tokens = Lexer::new(src, 0).lex().unwrap_or_else(|err| {
            panic!("{}", err.display(&db));
        });

        let iter = NonBracketedIter::new(&tokens, 0).map(|r| r.map(|v| &**v));
        let rev_iter = iter.clone().rev();

        let result = dresult_unwrap!(iter.collect::<DResult<Vec<_>>>(), &db);
        let mut rev_result = dresult_unwrap!(rev_iter.collect::<DResult<Vec<_>>>(), &db);
        rev_result.reverse();

        let expected = [&T!(a), &T!(+), &T!('('), &T!(')'), &T!(=), &T!(e1)];
        assert_eq!(&*result, expected);
        assert_eq!(&*rev_result, expected);
    }
}
