#[allow(unused)]
use std::borrow::Cow;

#[allow(unused)] // false positive
use copyspan::Span;

#[allow(unused)]
use indoc::indoc;

use crate::parser::{FunctionCall, Lambda, lambda, util::ast};
#[allow(unused)] // false positives
use crate::{
    error::PartialSpanned,
    parser::{Expression, LetIn, WithIn},
};

macro_rules! parse_test {
    ($test_name:ident, $src:expr, $expected_ast:expr) => {
        #[test]
        fn $test_name() {
            let db = $crate::error::SourceDB::new();
            db.add(
                format!("{}.mulch", ::core::stringify!($test_name)).into(),
                $src.into(),
            );

            let token_stream =
                $crate::dresult_unwrap($crate::lexer::Lexer::new($src, 0).lex(), &db);
            let expr =
                $crate::dresult_unwrap($crate::parser::parse_expression(&token_stream, 0), &db);

            assert_eq!(expr, Some($expected_ast));
        }
    };
}

parse_test! {ident, " test_123 ", ast! {
    Spanned(Variable("test_123"), 1..9)
}}

parse_test! {string_literal, r#" "my \"string\"!" "#, ast!{
    Spanned(StringLiteral("my \"string\"!"), 1..17)
}}

parse_test! {nested_set, "{ x = a; b={x=cat; y=dog}; hi=foo;}", ast!{
    Spanned(Set [
        (
            Spanned("b", 9..10),
            Spanned(Set [
                (
                    Spanned("x", 12..13),
                    Spanned(Variable("cat"), 14..17),
                ),
                (
                    Spanned("y", 19..20),
                    Spanned(Variable("dog"), 21..24),
                ),
            ], 11..25),
        ),
        (
            Spanned("hi", 27..29),
            Spanned(Variable("foo"), 30..33),
        ),
        (
            Spanned("x", 2..3),
            Spanned(Variable("a"), 6..7),
        ),
    ], 0..35)
}}

parse_test! {nested_list, "[a, b, c, [d, [e, f], [g,],]]", ast! {
    Spanned(List [
        Spanned(Variable("a"), 1..2),
        Spanned(Variable("b"), 4..5),
        Spanned(Variable("c"), 7..8),
        Spanned(List [
            Spanned(Variable("d"), 11..12),
            Spanned(List [
                Spanned(Variable("e"), 15..16),
                Spanned(Variable("f"), 18..19),
            ], 14..20),
            Spanned(List [
                Spanned(Variable("g"), 23..24),
            ], 22..26),
        ], 10..28),
    ], 0..29)
}}

parse_test! {with_in, r#"with {a = "hello";}; in a"#, ast! {
    Spanned(WithIn {
        set: Spanned(Set [
            (
                Spanned("a", 6..7),
                Spanned(StringLiteral("hello"), 10..17),
            ),
        ], 5..19),
        expression: Spanned(Variable("a"), 24..25),
    }, 0..25)
}}

parse_test! {let_in,
    indoc!{r#"
        let
            a = "0";
            b = "1";
        in
        [a, b]
    "#},
    ast! {
        Spanned(LetIn {
            bindings: [
                (
                    Spanned("a", 8..9),
                    Spanned(StringLiteral("0"), 12..15),
                ),
                (
                    Spanned("b", 21..22),
                    Spanned(StringLiteral("1"), 25..28),
                ),
            ],
            expression: Spanned(List [
                Spanned(Variable("a"), 34..35),
                Spanned(Variable("b"), 37..38),
            ], 33..39),
        }, 0..39)
    }
}

parse_test! {lambda_1, "let x = a -> add[a, 1]; in map[[1, 2], x]",
    PartialSpanned(
        Expression::LetIn(
            LetIn {
                bindings: vec![
                    (
                        PartialSpanned(
                            Cow::from("x"),
                            Span::from(4..5),
                        ),
                        PartialSpanned(
                            Expression::Lambda(
                                Lambda {
                                    args: Box::new(PartialSpanned(
                                        lambda::Args::Single(
                                            Cow::from("a"),
                                        ),
                                        Span::from(8..9),
                                    )),
                                    expression: Box::new(PartialSpanned(
                                        Expression::FunctionCall(
                                            FunctionCall {
                                                function: Box::new(PartialSpanned(
                                                    Expression::Variable(
                                                        Cow::from("add"),
                                                    ),
                                                    Span::from(13..16),
                                                )),
                                                args: Box::new(PartialSpanned(
                                                    Expression::List(
                                                        vec![
                                                            PartialSpanned(
                                                                Expression::Variable(
                                                                    Cow::from("a"),
                                                                ),
                                                                Span::from(17..18),
                                                            ),
                                                            PartialSpanned(
                                                                Expression::NumericLiteral(
                                                                    Cow::from("1"),
                                                                ),
                                                                Span::from(20..21),
                                                            ),
                                                        ],
                                                    ),
                                                    Span::from(16..22),
                                                )),
                                            },
                                        ),
                                        Span::from(13..22),
                                    )),
                                },
                            ),
                            Span::from(8..22),
                        ),
                    ),
                ],
                expression: Box::new(PartialSpanned(
                    Expression::FunctionCall(
                        FunctionCall {
                            function: Box::new(PartialSpanned(
                                Expression::Variable(
                                    Cow::from("map"),
                                ),
                                Span::from(27..30),
                            )),
                            args: Box::new(PartialSpanned(
                                Expression::List(
                                    vec![
                                        PartialSpanned(
                                            Expression::List(
                                                vec![
                                                    PartialSpanned(
                                                        Expression::NumericLiteral(
                                                            Cow::from("1"),
                                                        ),
                                                        Span::from(32..33),
                                                    ),
                                                    PartialSpanned(
                                                        Expression::NumericLiteral(
                                                            Cow::from("2"),
                                                        ),
                                                        Span::from(35..36),
                                                    ),
                                                ],
                                            ),
                                            Span::from(31..37),
                                        ),
                                        PartialSpanned(
                                            Expression::Variable(
                                                Cow::from("x"),
                                            ),
                                            Span::from(39..40),
                                        ),
                                    ],
                                ),
                                Span::from(30..41),
                            )),
                        },
                    ),
                    Span::from(27..41),
                )),
            },
        ),
        Span::from(0..41),
    )
}
