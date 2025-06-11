use copyspan::Span;

use crate::error::{FullSpan, error};

macro_rules! lexer_test {
    {$name:ident, $string:literal, $expected:expr $(,)?} => {
        #[test]
        fn $name() {
            for res in $crate::lexer::Lexer::new($string, 0) {
                if let Err(err) = res {
                    assert_eq!(err, $expected);
                    return;
                }
            }

            panic!("Test failed: no error occured");
        }
    };
}

lexer_test! {
    unexpected_character,
    "hello \"\u{1F602}\" world\u{1F633}\n",
    error!(
        "EL0001",
        "Unexpected character '\u{1F633}'",
        [{"character here", FullSpan {span: Span::from(18..22), file_id: 0}, primary}]
    )
}

lexer_test! {
    no_end_quote1,
    "za warodo = \"\\\nhi",
    error!(
        "EL0003",
        "No end quote found for string literal",
        [{"here", FullSpan {span: Span::from(12..17), file_id: 0}, primary}]
    )
}

lexer_test! {
    no_end_quote2,
    "za warodo2 = \"\nhi",
    error!(
        "EL0003",
        "No end quote found for string literal",
        [{"here", FullSpan {span: Span::from(13..14), file_id: 0}, primary}]
    )
}

lexer_test! {
    no_end_quote3,
    "invalid_escape = \"\\o\"",
    error!(
        "EL0002",
        "Invalid escape sequence \"\\o\"",
        [{"here", FullSpan {span: Span::from(19..20), file_id: 0}, primary}]
    )
}
