#[allow(unused)] // false positive
use {crate::parser_old::util::ast, indoc::indoc};

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
                $crate::dresult_unwrap($crate::parser_old::parse_expression(&token_stream, 0), &db);

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

parse_test! {binop_1, "0 + 1 *2^3 ^ 4 * 5 - 6", ast! {
    Spanned(Subtract(
        Spanned(Add(
            Spanned(NumericLiteral("0"), 0..1),
            Spanned(Multiply(
                Spanned(Multiply(
                    Spanned(NumericLiteral("1"), 4..5),
                    Spanned(Exponent(
                        Spanned(NumericLiteral("2"), 7..8),
                        Spanned(Exponent(
                            Spanned(NumericLiteral("3"), 9..10),
                            Spanned(NumericLiteral("4"), 13..14),
                        ), 9..14),
                    ), 7..14),
                ), 4..14),
                Spanned(NumericLiteral("5"), 17..18),
            ), 4..18),
        ), 0..18),
        Spanned(NumericLiteral("6"), 21..22),
    ), 0..22)
}}

parse_test! {binop_2, "0 + 2 - 3 * 4 + 7", ast! {
    Spanned(Add(
        Spanned(Subtract(
            Spanned(Add(
                Spanned(NumericLiteral("0"), 0..1),
                Spanned(NumericLiteral("2"), 4..5),
            ), 0..5),
            Spanned(Multiply(
                Spanned(NumericLiteral("3"), 8..9),
                Spanned(NumericLiteral("4"), 12..13),
            ), 8..13),
        ), 0..13),
        Spanned(NumericLiteral("7"), 16..17),
    ), 0..17)
}}

parse_test! {lambda_1, "let x = a -> add[a, 1]; in map[[1, 2], x]", ast! {
    Spanned(LetIn {
        bindings: [
            (
                Spanned("x", 4..5),
                Spanned(Lambda {
                    args: Spanned(Single("a"), 8..9),
                    expression: Spanned(FunctionCall {
                        function: Spanned(Variable("add"), 13..16),
                        args: Spanned(List [
                            Spanned(Variable("a"), 17..18),
                            Spanned(NumericLiteral("1"), 20..21),
                        ], 16..22),
                    }, 13..22),
                }, 8..22),
            ),
        ],
        expression: Spanned(FunctionCall {
            function: Spanned(Variable("map"), 27..30),
            args: Spanned(List [
                Spanned(List [
                    Spanned(NumericLiteral("1"), 32..33),
                    Spanned(NumericLiteral("2"), 35..36),
                ], 31..37),
                Spanned(Variable("x"), 39..40),
            ], 30..41),
        }, 27..41),
    }, 0..41)
}}

parse_test! {lambda_2, "with {add = [a, b] -> a + b}; in add[9, 10]", ast! {
    Spanned(WithIn {
        set: Spanned(Set [
            (
                Spanned("add", 6..9),
                Spanned(Lambda {
                    args: Spanned(List[
                        Spanned(Single("a"), 13..14),
                        Spanned(Single("b"), 16..17),
                    ], 12..18),
                    expression: Spanned(Add(
                        Spanned(Variable("a"), 22..23),
                        Spanned(Variable("b"), 26..27),
                    ), 22..27),
                }, 12..27),
            ),
        ], 5..28),
        expression: Spanned(FunctionCall {
            function: Spanned(Variable("add"), 33..36),
            args: Spanned(List [
                Spanned(NumericLiteral("9"), 37..38),
                Spanned(NumericLiteral("10"), 40..42),
            ], 36..43),
        }, 33..43),
    }, 0..43)
}}

parse_test! {lamda_3, "{foo, bar} -> biz", ast! {
    Spanned(Lambda {
        args: Spanned(AttrSet[
            {
                name: Spanned("bar", 6..9),
                default: None,
            },
            {
                name: Spanned("foo", 1..4),
                default: None,
            },
        ], 0..10),
        expression: Spanned(Variable("biz"), 14..17),
    }, 0..17)
}}

parse_test! {member_access, "string.push_str[\"howdy!\"]", ast! {
    Spanned(FunctionCall {
        function: Spanned(MemberAccess {
            lhs: Spanned(Variable("string"), 0..6),
            rhs: Spanned("push_str", 7..15),
        }, 0..15),
        args: Spanned(List [
            Spanned(StringLiteral("howdy!"), 16..24),
        ], 15..25),
    }, 0..25)
}}
