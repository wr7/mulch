mod error;
mod types;

#[cfg(test)]
mod test;

use std::{borrow::Cow, str::CharIndices};

use copyspan::Span;
pub use types::*;

use crate::{
    error::{Diagnostic, FullSpan, PartialSpanned},
    util::MultiPeekable,
};

#[derive(Clone, Debug)]
pub struct Lexer<'a> {
    src: &'a str,
    remaining: MultiPeekable<CharIndices<'a>, 2>,
    file_id: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file_id: usize) -> Self {
        Self {
            src,
            remaining: MultiPeekable::new(src.char_indices()),
            file_id,
        }
    }

    fn full_span(&self, span: Span) -> FullSpan {
        FullSpan {
            span,
            file_id: self.file_id,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<PartialSpanned<Token<'a>>, Diagnostic>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, c) = *self.remaining.peek(0)?;

            if c.is_ascii_whitespace() {
                self.remaining.next();
                continue;
            }

            let start = i;

            let mut rules = [Self::try_lex_identifier, Self::try_lex_symbol].into_iter();

            let token = loop {
                let Some(rule) = rules.next() else {
                    return Some(Err(error::unexpected_character(
                        c,
                        self.full_span(Span::at(start).with_len(c.len_utf8())),
                    )));
                };

                if let Some(token) = rule(self) {
                    break token;
                }
            };

            let end = self.remaining.peek(0).map_or(self.src.len(), |&(i, _)| i);

            return Some(Ok(PartialSpanned::new(token, Span::from(start..end))));
        }
    }
}

impl<'a> Lexer<'a> {
    fn try_lex_identifier(&mut self) -> Option<Token<'a>> {
        let (start, first_char) = *self.remaining.peek(0)?;

        if first_char.is_ascii_alphabetic() || first_char == '_' {
            self.remaining.next();
        } else {
            return None;
        }

        let end = loop {
            let v = self.remaining.peek(0).copied();

            if v.is_some_and(|(_, c)| c.is_ascii_alphanumeric() || c == '_') {
                self.remaining.next();
            } else {
                break v.map_or(self.src.len(), |(i, _)| i);
            }
        };

        Some(Token::Identifier(Cow::Borrowed(&self.src[start..end])))
    }

    fn try_lex_symbol(&mut self) -> Option<Token<'a>> {
        dbg!(self.remaining.peek_all());

        let token = match self.remaining.peek_all() {
            [(_, '-'), (_, '>'), ..] => {
                self.remaining.next();
                T!(->)
            }
            [(_, c), ..] => match c {
                '.' => T!(.),
                ',' => T!(,),
                ';' => T!(;),
                '=' => T!(=),
                '|' => T!(|),
                '+' => T!(+),
                '/' => T!(/),
                '*' => T!(*),
                '^' => T!(^),
                '<' => T!(<),
                '>' => T!(>),
                '-' => T!(-),
                '(' => T!('('),
                ')' => T!(')'),
                '[' => T!('['),
                ']' => T!(']'),
                '{' => T!('{'),
                '}' => T!('}'),
                _ => return None,
            },
            [] => return None,
        };

        self.remaining.next();

        Some(token)
    }
}
