#[allow(unused)] // false positive
use copyspan::Span;

#[allow(unused)] // false positive
use crate::dresult_unwrap;

macro_rules! parse_test {
    ($test_name:ident, $src:expr, $output:expr) => {
        #[test]
        fn $test_name() {
            let db = $crate::error::SourceDB::new();
            db.add(
                format!("{}.mulch", ::core::stringify!($test_name)).into(),
                $src.into(),
            );

            let token_stream = dresult_unwrap!($crate::lexer::Lexer::new($src, 0).lex(), &db);
            let expr = dresult_unwrap!($crate::parser::parse_expression(&token_stream, 0), &db);

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
