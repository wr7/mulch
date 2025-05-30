mod types;

use std::{iter::Peekable, str::CharIndices};

pub use types::*;

use crate::error::PartialSpanned;

pub struct Lexer<'a> {
    src: &'a str,
    remaining: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            remaining: src.char_indices().peekable(),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = PartialSpanned<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, c) = *self.remaining.peek()?;

            if c.is_ascii_whitespace() {
                self.remaining.next();
                continue;
            }

            let start = i;

            let mut rules = [Self::try_lex_identifier].into_iter();

            let token = loop {
                let Some(rule) = rules.next() else {
                    todo!() // Unexpected character
                };

                if let Some(token) = rule(self) {
                    break token;
                }
            };

            let end = self.remaining.peek().map_or(self.src.len(), |&(i, _)| i);

            Some(PartialSpanned::new(token, (start..end).into()));
        }
    }
}

impl<'a> Lexer<'a> {
    fn try_lex_identifier(&mut self) -> Option<Token<'a>> {
        let start = self.remaining.peek()?.0;

        let end = loop {
            let v = self.remaining.peek();

            if v.is_some_and(|(_, c)| c.is_ascii_alphanumeric()) {
                self.remaining.next();
            } else {
                break v.map_or(self.src.len(), |(i, _)| *i);
            }
        };

        if start == end {
            return None;
        }

        Some(Token::Identifier(&self.src[start..end]))
    }
}
