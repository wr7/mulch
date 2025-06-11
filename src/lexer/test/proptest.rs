use crate::lexer::{T, Token};

use crate::lexer::Lexer;

use itertools::Itertools;
use proptest::prelude::*;
use std::iter;

const WHITESPACE: &'static str = "[ \t\r\n]+";
const OPT_WHITESPACE: &'static str = "[ \t\r\n]*";

const IDENT: &'static str = "[A-Za-z_][a-zA-Z0-9_]*";

fn arb_token() -> impl Strategy<Value = (Token<'static>, String)> + Clone {
    prop_oneof! {
        IDENT.prop_map(|id| (Token::Identifier(id.clone().into()), id)),
        "\\p{Any}*".prop_map(|content| {
            let quoted = quote(&content);
            (Token::StringLiteral(content.into()), quoted)
        }),
        Just((T!(.),  ".".to_string())),
        Just((T!(,),  ",".to_string())),
        Just((T!(;),  ";".to_string())),
        Just((T!(=),  "=".to_string())),
        Just((T!(|),  "|".to_string())),
        Just((T!(+),  "+".to_string())),
        Just((T!(/),  "/".to_string())),
        Just((T!(*),  "*".to_string())),
        Just((T!(^),  "^".to_string())),
        Just((T!(<),  "<".to_string())),
        Just((T!(>),  ">".to_string())),
        Just((T!(-),  "-".to_string())),
        Just((T!(->), "->".to_string())),
        Just((T!('('),  "(".to_string())),
        Just((T!(')'),  ")".to_string())),
        Just((T!('['),  "[".to_string())),
        Just((T!(']'),  "]".to_string())),
        Just((T!('{'),  "{".to_string())),
        Just((T!('}'),  "}".to_string())),
    }
}

fn arb_tokenstream() -> impl Strategy<Value = (Vec<Token<'static>>, String)> {
    (0usize..10).prop_flat_map(|n| {
        (
            OPT_WHITESPACE,
            vec![arb_token(); n],
            vec![WHITESPACE; n.saturating_sub(1)],
            OPT_WHITESPACE,
        )
            .prop_map(|(fws, tokens, ws, ews)| {
                let tokens_strings = tokens.iter().map(|(_, s)| &**s);
                let ws = ws.iter().map(|t| &**t);

                let iter = iter::once(&*fws)
                    .chain(tokens_strings.interleave(ws))
                    .chain(iter::once(&*ews));

                let string = iter.collect::<String>();

                (tokens.into_iter().map(|(t, _)| t).collect(), string)
            })
    })
}

fn quote(str: &str) -> String {
    let mut ret = String::new();

    ret.push('"');
    for char in str.chars() {
        match char {
            '\\' => {
                ret.push_str("\\\\");
            }
            '\n' => {
                ret.push_str("\\n");
            }
            '\r' => {
                ret.push_str("\\r");
            }
            '\t' => {
                ret.push_str("\\t");
            }
            '"' => {
                ret.push_str("\\\"");
            }
            _ => {
                ret.push(char);
            }
        }
    }
    ret.push('"');

    ret
}

proptest! {
    #[test]
    fn proptest(
        (tokens, src) in arb_tokenstream()
    ) {
        let mut lexer = Lexer::new(&src, 0);
        let mut tokens = tokens.iter();

        for (expected, got) in tokens.by_ref().zip(lexer.by_ref()) {
            let got = got.ok().map(|p| p.data);
            prop_assert_eq!(Some(expected), got.as_ref());
        }

        prop_assert!(tokens.next().is_none() && lexer.next().is_none());
    }
}
