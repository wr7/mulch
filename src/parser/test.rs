#[allow(unused)]
use std::borrow::Cow;

#[allow(unused)] // false positive
use copyspan::Span;

#[allow(unused)]
use indoc::indoc;

#[allow(unused)] // false positives
use crate::{
    error::PartialSpanned,
    parser::{Expression, LetIn, WithIn},
};

macro_rules! parse_test {
    ($test_name:ident, $src:expr, $output:expr) => {
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

            assert_eq!(expr, Some($output));
        }
    };
}

parse_test! {ident, " test_123 ", crate::error::PartialSpanned (
    crate::parser::Expression::Variable("test_123".into()),
    ::copyspan::Span::from(1..9)
)}

parse_test! {string_literal, r#" "my \"string\"!" "#, crate::error::PartialSpanned (
    crate::parser::Expression::StringLiteral(r#"my "string"!"#.into()),
    ::copyspan::Span::from(1..17)
)}

parse_test! {nested_set, "{ x = a; b={x=cat; y=dog}; hi=foo;}",
    crate::error::PartialSpanned(
        crate::parser::Expression::Set(
            vec![
                (
                    crate::error::PartialSpanned(
                        "b".into(),
                        Span::from(9..10),
                    ),
                    crate::error::PartialSpanned(
                        crate::parser::Expression::Set(
                            vec![
                                (
                                    crate::error::PartialSpanned(
                                        "x".into(),
                                        Span::from(12..13),
                                    ),
                                    crate::error::PartialSpanned(
                                        crate::parser::Expression::Variable(
                                            "cat".into(),
                                        ),
                                        Span::from(14..17),
                                    ),
                                ),
                                (
                                    crate::error::PartialSpanned(
                                        "y".into(),
                                        Span::from(19..20),
                                    ),
                                    crate::error::PartialSpanned(
                                        crate::parser::Expression::Variable(
                                            "dog".into(),
                                        ),
                                        Span::from(21..24),
                                    ),
                                ),
                            ],
                        ),
                        Span::from(11..25),
                    ),
                ),
                (
                    crate::error::PartialSpanned(
                        "hi".into(),
                        Span::from(27..29),
                    ),
                    crate::error::PartialSpanned(
                        crate::parser::Expression::Variable(
                            "foo".into(),
                        ),
                        Span::from(30..33),
                    ),
                ),
                (
                    crate::error::PartialSpanned(
                        "x".into(),
                        Span::from(2..3),
                    ),
                    crate::error::PartialSpanned(
                        crate::parser::Expression::Variable(
                            "a".into(),
                        ),
                        Span::from(6..7),
                    ),
                ),
            ],
        ),
        Span::from(0..35),
    )
}

parse_test! {nested_list, "[a, b, c, [d, [e, f], [g,],]]", PartialSpanned(
    Expression::List(vec![
        PartialSpanned(Expression::Variable("a".into()), Span::from(1..2)),
        PartialSpanned(Expression::Variable("b".into()), Span::from(4..5)),
        PartialSpanned(Expression::Variable("c".into()), Span::from(7..8)),
        PartialSpanned(
            Expression::List(vec![
                PartialSpanned(Expression::Variable("d".into()), Span::from(11..12)),
                PartialSpanned(
                    Expression::List(vec![
                        PartialSpanned(Expression::Variable("e".into()), Span::from(15..16)),
                        PartialSpanned(Expression::Variable("f".into()), Span::from(18..19))
                    ]),
                    Span::from(14..20)
                ),
                PartialSpanned(
                    Expression::List(vec![
                        PartialSpanned(Expression::Variable("g".into()), Span::from(23..24))
                    ]),
                    Span::from(22..26)
                )
            ]),
            Span::from(10..28)
        )
    ]), Span::from(0..29)
)}

parse_test! {with_in, r#"with {a = "hello";}; in a"#,
    PartialSpanned(
        Expression::WithIn(WithIn{
            set: Box::new(PartialSpanned(
             Expression::Set(vec![
                 (PartialSpanned(Cow::Borrowed("a"), Span::from(6..7)), PartialSpanned(Expression::StringLiteral(Cow::Borrowed("hello")), Span::from(10..17)))
             ]),
             Span::from(5..19)
            )),
            expression: Box::new(PartialSpanned(Expression::Variable(Cow::Borrowed("a")), Span::from(24..25)))
        }),
        Span::from(0..25)
    )
}
parse_test! {let_in,
    indoc!{r#"
        let
            a = "0";
            b = "1";
        in
        [a, b]
    "#},
    PartialSpanned(
        Expression::LetIn(
            LetIn {
                bindings: vec![
                    (
                        PartialSpanned(
                            Cow::Borrowed("a"),
                            Span::from(8..9),
                        ),
                        PartialSpanned(
                            Expression::StringLiteral(
                                Cow::Borrowed("0"),
                            ),
                            Span::from(12..15),
                        ),
                    ),
                    (
                        PartialSpanned(
                            Cow::Borrowed("b"),
                            Span::from(21..22),
                        ),
                        PartialSpanned(
                            Expression::StringLiteral(
                                Cow::Borrowed("1"),
                            ),
                            Span::from(25..28),
                        ),
                    ),
                ],
                expression: Box::new(PartialSpanned(
                    Expression::List(
                        vec![
                            PartialSpanned(
                                Expression::Variable(
                                    Cow::Borrowed("a"),
                                ),
                                Span::from(34..35),
                            ),
                            PartialSpanned(
                                Expression::Variable(
                                    Cow::Borrowed("b"),
                                ),
                                Span::from(37..38),
                            ),
                        ],
                    ),
                    Span::from(33..39),
                )),
            },
        ),
        Span::from(0..39),
    )
}
