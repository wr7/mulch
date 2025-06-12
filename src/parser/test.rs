#[allow(unused)] // false positive
macro_rules! dresult_unwrap {
    ($result:expr, $db:expr) => {
        match $result {
            Ok(inner) => inner,
            Err(diag) => {
                panic!("\ndresult_unwrap failed:\n\n{}", diag.display($db))
            }
        }
    };
}

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

parse_test! {ident, " test_123 ", crate::error::PartialSpanned {
    data: crate::parser::Expression::Variable("test_123".into()),
    span: ::copyspan::Span::from(1..9)
}}

parse_test! {string_literal, r#" "my \"string\"!" "#, crate::error::PartialSpanned {
    data: crate::parser::Expression::StringLiteral(r#"my "string"!"#.into()),
    span: ::copyspan::Span::from(1..17)
}}
