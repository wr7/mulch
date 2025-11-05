use std::iter::Fuse;

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
    pub fn new(slc: &'a [PartialSpanned<Token<'src>>], file_no: usize) -> Fuse<Self> {
        Self {
            remaining: slc,
            file_no,
            opening_bracket: None,
            closing_bracket: None,
        }
        .fuse()
    }

    #[allow(unused)]
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
    use crate::{
        T,
        error::{SourceDB, dresult_unwrap},
        lexer::Lexer,
    };

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

        let result = dresult_unwrap(iter.collect::<DResult<Vec<_>>>(), &db);
        let mut rev_result = dresult_unwrap(rev_iter.collect::<DResult<Vec<_>>>(), &db);
        rev_result.reverse();

        let expected = [&T!(a), &T!(+), &T!('('), &T!(')'), &T!(=), &T!(e1)];
        assert_eq!(&*result, expected);
        assert_eq!(&*rev_result, expected);
    }
}

/// A macro for more compactly defining abstract syntax trees
///
/// This syntax is in line with what is returned by the `Debug` trait
/// impl for `Expression` and its related types
#[allow(unused_macros)]
macro_rules! ast {
    {
        Variable (
            $name: expr $(,)?
        )
    } => {
        $crate::parser::Expression::Variable(
            ::std::borrow::Cow::from($name)
        )
    };
    {
        StringLiteral (
            $name: expr $(,)?
        )
    } => {
        $crate::parser::Expression::StringLiteral(
            ::std::borrow::Cow::from($name)
        )
    };
    {
        NumericLiteral (
            $name: expr
        )
    } => {
        $crate::parser::Expression::NumericLiteral(
            ::std::borrow::Cow::from($name)
        )
    };
    {
        Unit()
    } => {
        $crate::parser::Expression::Unit()
    };
    {
        Set[$(
            (
                Spanned($attr:literal, $span:expr $(,)?),
                $value_ident:ident $value_args:tt $(,)?
            )
        ),* $(,)?]
    } => {
        $crate::parser::Expression::Set(
            ::std::vec![
                $(
                    (
                        $crate::parser::PartialSpanned(
                            ::std::borrow::Cow::from($attr),
                            ::copyspan::Span::from($span)
                        ),
                        $crate::parser::util::ast!($value_ident $value_args)
                    )
                ),*
            ]
        )
    };
    {
        List[
            $($name:ident $args:tt),*
            $(,)?
        ]
    } => {
        $crate::parser::Expression::List(
            ::std::vec![
                $($crate::parser::util::ast!($name $args)),*
            ]
        )
    };
    {
        WithIn{
            set: $set_name:ident $set_args:tt,
            expression: $exp_name:ident $exp_args:tt $(,)?
        }
    } => {
        $crate::parser::Expression::WithIn(
            $crate::parser::WithIn {
                set:        ::std::boxed::Box::new($crate::parser::util::ast!($set_name $set_args)),
                expression: ::std::boxed::Box::new($crate::parser::util::ast!($exp_name $exp_args))
            }
        )
    };
    {
        LetIn {
            bindings: [
                $((
                    Spanned($var_name:literal,  $var_name_span:expr $(,)?),
                    $value_ident:ident $value_args:tt $(,)?
                )),* $(,)?
            ],
            expression: $expr_ident:ident $expr_args:tt $(,)?
        }
    } => {
        $crate::parser::Expression::LetIn(
            $crate::parser::LetIn {
                bindings: ::std::vec![
                    $(
                        (
                            $crate::parser::PartialSpanned(
                                ::std::borrow::Cow::from($var_name),
                                ::copyspan::Span::from($var_name_span)
                            ),
                            $crate::parser::util::ast!($value_ident $value_args)
                        )
                    ),*
                ],
                expression: ::std::boxed::Box::new(
                    $crate::parser::util::ast!($expr_ident $expr_args)
                )
            }
        )
    };
    {
        FunctionCall{
            function: $function_name:ident $function_args:tt,
            args: $args_name:ident $args_args:tt $(,)?
        }
    } => {
        $crate::parser::Expression::FunctionCall(
            $crate::parser::FunctionCall {
                function: ::std::boxed::Box::new($crate::parser::util::ast!($function_name $function_args)),
                args: ::std::boxed::Box::new($crate::parser::util::ast!($args_name $args_args))
            }
        )
    };
    {
        Lambda{
            args: $args_ident:ident $args_args:tt,
            expression: $expr_name:ident $expr_args:tt $(,)?
        }
    } => {
        $crate::parser::Expression::Lambda(
            $crate::parser::Lambda{
                args: ::std::boxed::Box::new($crate::parser::util::lambda_args_ast!($args_ident $args_args)),
                expression: ::std::boxed::Box::new($crate::parser::util::ast!($expr_name $expr_args))
            }
        )
    };
    {
        $binary_op:ident(
            $lhs_name:ident $lhs_args:tt,
            $rhs_name:ident $rhs_args:tt $(,)?
        )
    } => {
        $crate::parser::Expression::BinaryOperation(
            $crate::parser::BinaryOperation {
                lhs: ::std::boxed::Box::new($crate::parser::util::ast!($lhs_name $lhs_args)),
                operator: $crate::parser::binary::BinaryOperator::$binary_op,
                rhs: ::std::boxed::Box::new($crate::parser::util::ast!($rhs_name $rhs_args))
            }
        )
    };
    {
        Spanned (
            $name:ident $args:tt,
            $span:expr $(,)?
        )
    } => {
        $crate::parser::PartialSpanned(
            $crate::parser::util::ast!($name $args),
            ::copyspan::Span::from($span)
        )
    };
}

#[allow(unused_macros)]
macro_rules! lambda_args_ast {
    {
        Single($name:literal $(,)?)
    } => {
        $crate::parser::lambda::Args::Single(::std::borrow::Cow::from($name))
    };
    {
        List[$(
            $name:ident $args:tt
        ),* $(,)?]
    } => {
        $crate::parser::lambda::Args::List(::std::vec![
            $(
                $crate::parser::util::lambda_args_ast!($name $args)
            ),*
        ])
    };
    {
        AttrSet[$(
            {
                name: Spanned($name:literal, $span:expr $(,)?),
                default: $($d:tt)+
            }
        ),* $(,)?]
    } => {
        $crate::parser::lambda::Args::AttrSet(::std::vec![
            $(
                $crate::parser::lambda::ArgAttribute {
                    name: $crate::parser::PartialSpanned(::std::borrow::Cow::from($name), ::copyspan::Span::from($span)),
                    default: $crate::parser::util::option_ast!($($d)+)
                }
            ),*
        ])
    };
    {
        Spanned (
            $name:ident $args:tt,
            $span:expr $(,)?
        )
    } => {
        $crate::parser::PartialSpanned(
            $crate::parser::util::lambda_args_ast!($name $args),
            ::copyspan::Span::from($span)
        )
    };
}

#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! option_ast {
    (None $(,)?) => {
        None
    };
    (Some($name:ident $args:tt)) => {
        Some($crate::parser::util::ast!($name $args))
    };
}

#[allow(unused_imports)]
pub(crate) use {ast, lambda_args_ast};

#[allow(unused_imports)]
#[doc(hidden)]
pub(crate) use option_ast;
