#![allow(unexpected_cfgs)] // because `cfg(rust_analyzer)` is not part of the standard

use crate::parser::test::util::parse_test;

mod util;

parse_test! {nested_set, "{ x = a; b={x=cat; y=dog}; hi=foo;}",
    Set [
        NamedValue {
            name: PartialSpanned(
                "x",
                2..3,
            ),
            value: PartialSpanned(
                Variable(
                    "a",
                ),
                6..7,
            ),
        },
        NamedValue {
            name: PartialSpanned(
                "b",
                9..10,
            ),
            value: PartialSpanned(
                Set [
                    NamedValue {
                        name: PartialSpanned(
                            "x",
                            12..13,
                        ),
                        value: PartialSpanned(
                            Variable(
                                "cat",
                            ),
                            14..17,
                        ),
                    },
                    NamedValue {
                        name: PartialSpanned(
                            "y",
                            19..20,
                        ),
                        value: PartialSpanned(
                            Variable(
                                "dog",
                            ),
                            21..24,
                        ),
                    },
                ],
                11..25,
            ),
        },
        NamedValue {
            name: PartialSpanned(
                "hi",
                27..29,
            ),
            value: PartialSpanned(
                Variable(
                    "foo",
                ),
                30..33,
            ),
        },
    ]
}

#[cfg(any(not(miri), rust_analyzer))]
parse_test! {let_in1, "let pi = 3.14159 in e^(i * pi)",
    LetIn {
        variables: [
            NamedValue {
                name: PartialSpanned(
                    "pi",
                    4..6,
                ),
                value: PartialSpanned(
                    NumericLiteral(
                        314159/100000,
                    ),
                    9..16,
                ),
            },
        ],
        val: PartialSpanned(
            BinaryOperation {
                lhs: PartialSpanned(
                    Variable(
                        "e",
                    ),
                    20..21,
                ),
                operator: Exponentiate,
                rhs: PartialSpanned(
                    BinaryOperation {
                        lhs: PartialSpanned(
                            Variable(
                                "i",
                            ),
                            23..24,
                        ),
                        operator: Multiply,
                        rhs: PartialSpanned(
                            Variable(
                                "pi",
                            ),
                            27..29,
                        ),
                    },
                    22..30,
                ),
            },
            20..30,
        ),
    }
}
