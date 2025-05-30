use crate::lexer::Token;

use super::Lexer;

use proptest::prelude::*;

proptest! {
    #[test]
    fn ident(
        s1 in "[ \\n\\t]*",
        i1 in "[A-Za-z_][a-zA-Z0-9_]*",
        s2 in "[ \\n\\t]+",
        i2 in "[A-Za-z_][a-zA-Z0-9_]*",
        s3 in "[ \\n\\t]*",
    ) {
        let tokens = [Some(&*i1), Some(&*i2), None];
        let src = format!("{s1}{i1}{s2}{i2}{s3}");

        let lexer = Lexer::new(&src, 0);
        prop_assert_eq!(lexer.clone().count(), 2);

        for (expected, got) in tokens.iter().zip(lexer) {
            prop_assert_eq!(expected.map(|e| Token::Identifier(e)), got.ok().map(|p| p.data));
        }
    }
}
