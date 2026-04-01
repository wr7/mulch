#![allow(unexpected_cfgs)] // because `cfg(rust_analyzer)` is not part of the standard

use crate::parser::test::util::parse_test;
use indoc::indoc;

mod util;

#[cfg(any(not(miri), rust_analyzer))]
mod numeric;

parse_test! {nested_set, "{ x = a; b={x=cat; y=dog}; hi=foo;}",
    Set [
        NamedValue {
            name: PartialSpanned("x", 2..3),
            value: PartialSpanned(
                Variable("a"),
                6..7,
            ),
        },
        NamedValue {
            name: PartialSpanned("b", 9..10),
            value: PartialSpanned(
                Set [
                    NamedValue {
                        name: PartialSpanned("x", 12..13),
                        value: PartialSpanned(
                            Variable("cat"),
                            14..17,
                        ),
                    },
                    NamedValue {
                        name: PartialSpanned("y", 19..20),
                        value: PartialSpanned(
                            Variable("dog"),
                            21..24,
                        ),
                    },
                ],
                11..25,
            ),
        },
        NamedValue {
            name: PartialSpanned("hi", 27..29),
            value: PartialSpanned(
                Variable("foo"),
                30..33,
            ),
        },
    ]
}

parse_test! {nested_list, "[a, b, c, [d, [e, f], [g,],]]",
    List [
        PartialSpanned(
            Variable("a"),
            1..2,
        ),
        PartialSpanned(
            Variable("b"),
            4..5,
        ),
        PartialSpanned(
            Variable("c"),
            7..8,
        ),
        PartialSpanned(
            List [
                PartialSpanned(
                    Variable("d"),
                    11..12,
                ),
                PartialSpanned(
                    List [
                        PartialSpanned(
                            Variable("e"),
                            15..16,
                        ),
                        PartialSpanned(
                            Variable("f"),
                            18..19,
                        ),
                    ],
                    14..20,
                ),
                PartialSpanned(
                    List [
                        PartialSpanned(
                            Variable("g"),
                            23..24,
                        ),
                    ],
                    22..26,
                ),
            ],
            10..28,
        ),
    ]
}

parse_test! {with_in, r#"with {a = "hello";} in a"#,
    WithIn {
        variables: PartialSpanned(
            Set [
                NamedValue {
                    name: PartialSpanned("a", 6..7),
                    value: PartialSpanned(
                        StringLiteral("hello"),
                        10..17,
                    ),
                },
            ],
            5..19,
        ),
        val: PartialSpanned(
            Variable("a"),
            23..24,
        ),
    }
}

parse_test! {let_in,
    indoc!{r#"
        let
            a = "0";
            b = "1";
        in
        [a, b]
    "#},
    LetIn {
        variables: [
            NamedValue {
                name: PartialSpanned("a", 8..9),
                value: PartialSpanned(
                    StringLiteral("0"),
                    12..15,
                ),
            },
            NamedValue {
                name: PartialSpanned("b", 21..22),
                value: PartialSpanned(
                    StringLiteral("1"),
                    25..28,
                ),
            },
        ],
        val: PartialSpanned(
            List [
                PartialSpanned(
                    Variable("a"),
                    34..35,
                ),
                PartialSpanned(
                    Variable("b"),
                    37..38,
                ),
            ],
            33..39,
        ),
    }
}

parse_test! {binop_1, r#""0" + "1" *"2"^"3" ^ "4" * "5" - "6""#,
    BinaryOperation {
        lhs: PartialSpanned(
            BinaryOperation {
                lhs: PartialSpanned(
                    StringLiteral("0"),
                    0..3,
                ),
                operator: Add,
                rhs: PartialSpanned(
                    BinaryOperation {
                        lhs: PartialSpanned(
                            BinaryOperation {
                                lhs: PartialSpanned(
                                    StringLiteral("1"),
                                    6..9,
                                ),
                                operator: Multiply,
                                rhs: PartialSpanned(
                                    BinaryOperation {
                                        lhs: PartialSpanned(
                                            StringLiteral("2"),
                                            11..14,
                                        ),
                                        operator: Exponentiate,
                                        rhs: PartialSpanned(
                                            BinaryOperation {
                                                lhs: PartialSpanned(
                                                    StringLiteral("3"),
                                                    15..18,
                                                ),
                                                operator: Exponentiate,
                                                rhs: PartialSpanned(
                                                    StringLiteral("4"),
                                                    21..24,
                                                ),
                                            },
                                            15..24,
                                        ),
                                    },
                                    11..24,
                                ),
                            },
                            6..24,
                        ),
                        operator: Multiply,
                        rhs: PartialSpanned(
                            StringLiteral("5"),
                            27..30,
                        ),
                    },
                    6..30,
                ),
            },
            0..30,
        ),
        operator: Subtract,
        rhs: PartialSpanned(
            StringLiteral("6"),
            33..36,
        ),
    }
}

parse_test! {binop_2, r#""0" + -"2" - "3" * --"4" + "7""#,
    BinaryOperation {
        lhs: PartialSpanned(
            BinaryOperation {
                lhs: PartialSpanned(
                    BinaryOperation {
                        lhs: PartialSpanned(
                            StringLiteral("0"),
                            0..3,
                        ),
                        operator: Add,
                        rhs: PartialSpanned(
                            UnaryOperation {
                                operator: Negative,
                                arg: PartialSpanned(
                                    StringLiteral("2"),
                                    7..10,
                                ),
                            },
                            6..10,
                        ),
                    },
                    0..10,
                ),
                operator: Subtract,
                rhs: PartialSpanned(
                    BinaryOperation {
                        lhs: PartialSpanned(
                            StringLiteral("3"),
                            13..16,
                        ),
                        operator: Multiply,
                        rhs: PartialSpanned(
                            UnaryOperation {
                                operator: Negative,
                                arg: PartialSpanned(
                                    UnaryOperation {
                                        operator: Negative,
                                        arg: PartialSpanned(
                                            StringLiteral("4"),
                                            21..24,
                                        ),
                                    },
                                    20..24,
                                ),
                            },
                            19..24,
                        ),
                    },
                    13..24,
                ),
            },
            0..24,
        ),
        operator: Add,
        rhs: PartialSpanned(
            StringLiteral("7"),
            27..30,
        ),
    }
}

parse_test! {lambda_1, r#"let inc = a -> a + "1"; in map(["1", "2"], inc)"#,
    LetIn {
        variables: [
            NamedValue {
                name: PartialSpanned("inc", 4..7,),
                value: PartialSpanned(
                    Lambda {
                        args: [
                            SingleArgument {
                                name: "a",
                                default_value: None,
                            },
                        ],
                        expr: PartialSpanned(
                            BinaryOperation {
                                lhs: PartialSpanned(
                                    Variable("a"),
                                    15..16,
                                ),
                                operator: Add,
                                rhs: PartialSpanned(
                                    StringLiteral("1"),
                                    19..22,
                                ),
                            },
                            15..22,
                        ),
                    },
                    10..22,
                ),
            },
        ],
        val: PartialSpanned(
            FunctionCall {
                function: PartialSpanned(
                    Variable("map"),
                    27..30,
                ),
                args: FunctionCallArgs [
                    PartialSpanned(
                        List [
                            PartialSpanned(
                                StringLiteral("1"),
                                32..35,
                            ),
                            PartialSpanned(
                                StringLiteral("2"),
                                37..40,
                            ),
                        ],
                        31..41,
                    ),
                    PartialSpanned(
                        Variable("inc"),
                        43..46,
                    ),
                ],
            },
            27..47,
        ),
    }
}

parse_test! {lambda_2, r#"with {add = (a, b) -> a + b} in add("9", "10")"#,
    WithIn {
        variables: PartialSpanned(
            Set [
                NamedValue {
                    name: PartialSpanned("add", 6..9),
                    value: PartialSpanned(
                        Lambda {
                            args: [
                                SingleArgument {
                                    name: "a",
                                    default_value: None,
                                },
                                SingleArgument {
                                    name: "b",
                                    default_value: None,
                                },
                            ],
                            expr: PartialSpanned(
                                BinaryOperation {
                                    lhs: PartialSpanned(
                                        Variable("a"),
                                        22..23,
                                    ),
                                    operator: Add,
                                    rhs: PartialSpanned(
                                        Variable("b"),
                                        26..27,
                                    ),
                                },
                                22..27,
                            ),
                        },
                        12..27,
                    ),
                },
            ],
            5..28,
        ),
        val: PartialSpanned(
            FunctionCall {
                function: PartialSpanned(
                    Variable("add"),
                    32..35,
                ),
                args: FunctionCallArgs [
                    PartialSpanned(
                        StringLiteral("9"),
                        36..39,
                    ),
                    PartialSpanned(
                        StringLiteral("10"),
                        41..45,
                    ),
                ],
            },
            32..46,
        ),
    }
}
