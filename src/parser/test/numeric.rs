use crate::parser::test::util::parse_test;

parse_test! {decimal_red1, "3.125", NumericLiteral(25/8)}
parse_test! {decimal_red2, "3.4", NumericLiteral(17/5)}
parse_test! {decimal_red3, "0.24222222222228222222222222222222242222222222222222222822222222222222222222222222222242222222222222222222282",
    NumericLiteral(
        12111111111114111111111111111111121111111111111111111411111111111111111111111111111121111111111111111111141/
        50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
    )
}

parse_test! {fraction1, "3/4", NumericLiteral(3/4)}
parse_test! {fraction2, "18/24", NumericLiteral(3/4)}
parse_test! {fraction3, "3/9", NumericLiteral(1/3)}
parse_test! {fraction4, "4/8", NumericLiteral(1/2)}
parse_test! {fraction5, "45115001655937399968975643230328110579/135345004967812199906926929690984331737", NumericLiteral(1/3)}
parse_test! {fraction7, "45115001655937399968975643230328110579/202483016067906680007894282123497175999",
    NumericLiteral(15038333885312466656325214410109370193/67494338689302226669298094041165725333)
}

parse_test! {zero1, "0", NumericLiteral(0)}
parse_test! {zero2, "0000__00000000000000_0000", NumericLiteral(0)}
parse_test! {zero3, "0__00000.0000000000___0000_0", NumericLiteral(0)}
parse_test! {zero4, "0/4_0", NumericLiteral(0)}
parse_test! {zero5, "0/402949485858940302059483895039987574849599040398475839349585743494930029", NumericLiteral(0)}

parse_test! {integer1, "18446744073709551615", NumericLiteral(18446744073709551615)}
parse_test! {integer2, "18446744073709551616", NumericLiteral(18446744073709551616)}
parse_test! {integer3, "15", NumericLiteral(15)}
parse_test! {integer4, "15.0", NumericLiteral(15)}
parse_test! {integer5,
    "133322436451015201950389973494277248374918476389588719826168658957540155155878225327949950678073977",
    NumericLiteral(133322436451015201950389973494277248374918476389588719826168658957540155155878225327949950678073977
)}

parse_test! {let_in1, "let pi = 3.14159 in e^(i * pi)",
    LetIn {
        variables: [
            NamedValue {
                name: PartialSpanned("pi", 4..6),
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
                    Variable("e"),
                    20..21,
                ),
                operator: Exponentiate,
                rhs: PartialSpanned(
                    BinaryOperation {
                        lhs: PartialSpanned(
                            Variable("i"),
                            23..24,
                        ),
                        operator: Multiply,
                        rhs: PartialSpanned(
                            Variable("pi"),
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
