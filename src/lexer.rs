mod error;
mod types;

#[cfg(test)]
mod test;

use std::{borrow::Cow, str::CharIndices};

use copyspan::Span;
pub use types::*;

use crate::{
    error::{DResult, FullSpan, PartialSpanned},
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

    pub fn lex(self) -> DResult<Vec<PartialSpanned<Token<'a>>>> {
        self.collect()
    }

    fn full_span(&self, span: impl Into<Span>) -> FullSpan {
        FullSpan {
            span: span.into(),
            file_id: self.file_id,
        }
    }

    fn full_span_at(&self, idx: usize) -> FullSpan {
        let rem = &self.src[idx..];
        let char = rem.chars().next().unwrap();
        let len = char.len_utf8();

        self.full_span(Span::at(idx).with_len(len))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = DResult<PartialSpanned<Token<'a>>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, c) = *self.remaining.peek(0)?;

            if c.is_ascii_whitespace() {
                self.remaining.next();
                continue;
            }

            let start = i;

            let mut rules = [
                Self::try_lex_identifier,
                Self::try_lex_symbol,
                Self::try_lex_string_literal,
                Self::try_lex_numeric_literal,
            ]
            .into_iter();

            let token = loop {
                let Some(rule) = rules.next() else {
                    return Some(Err(error::unexpected_character(
                        c,
                        self.full_span_at(start),
                    )));
                };

                if let Some(token) = rule(self) {
                    break token;
                }
            };

            let token = match token {
                Ok(t) => t,
                Err(err) => return Some(Err(err)),
            };

            let end = self.remaining.peek(0).map_or(self.src.len(), |&(i, _)| i);

            return Some(Ok(PartialSpanned::new(token, Span::from(start..end))));
        }
    }
}

impl<'a> Lexer<'a> {
    fn try_lex_identifier(&mut self) -> Option<DResult<Token<'a>>> {
        let (start, first_char) = *self.remaining.peek(0)?;

        if first_char.is_ascii_alphabetic() || first_char == '_' {
            self.remaining.next();
        } else {
            return None;
        }

        let end = loop {
            let v = self.remaining.peek(0).copied();

            if let Some((_, c)) = v
                && (c.is_ascii_alphanumeric() || c == '_')
            {
                self.remaining.next();
            } else {
                break v.map_or(self.src.len(), |(i, _)| i);
            }
        };

        Some(Ok(Token::Identifier(Cow::Borrowed(&self.src[start..end]))))
    }

    fn try_lex_symbol(&mut self) -> Option<DResult<Token<'a>>> {
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
                '@' => T!(@),
                ':' => T!(:),
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

        Some(Ok(token))
    }

    fn try_lex_string_literal(&mut self) -> Option<DResult<Token<'a>>> {
        let (start, c) = *self.remaining.peek(0)?;

        if c != '"' {
            return None;
        }

        self.remaining.next();
        let string_start = start + '"'.len_utf8();

        let mut escape = false;
        let mut has_escapes = false;
        let mut buf = String::new();

        let mut end = None;

        for (i, c) in self.remaining.by_ref() {
            if escape {
                escape = false;

                let char = match c {
                    'r' => '\r',
                    'n' => '\n',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    '\n' => continue,
                    _ => return Some(Err(error::invalid_escape(c, self.full_span_at(i)))),
                };

                buf.push(char);
                continue;
            }

            match c {
                '\\' => {
                    escape = true;
                    has_escapes = true;
                    continue;
                }
                '"' => {
                    end = Some(i);
                    break;
                }
                '\n' => {
                    return Some(Err(error::no_end_quote(self.full_span(start..i))));
                }
                _ => {
                    buf.push(c);
                }
            }
        }

        let Some(end) = end else {
            return Some(Err(error::no_end_quote(
                self.full_span(start..self.src.len()),
            )));
        };

        if has_escapes {
            return Some(Ok(Token::StringLiteral(buf.into())));
        }

        Some(Ok(Token::StringLiteral(Cow::Borrowed(
            &self.src[string_start..end],
        ))))
    }

    fn try_lex_numeric_literal(&mut self) -> Option<DResult<Token<'a>>> {
        let (start, c) = *self.remaining.peek(0)?;

        if !c.is_ascii_digit() {
            return None;
        }

        let mut hit_decimal = false;

        let end = loop {
            match self.remaining.peek_all() {
                [(_, '0'..='9' | '_'), ..] => {}
                [(_, '.'), (_, '0'..='9')] if !hit_decimal => {
                    hit_decimal = true;
                }
                [(idx, c @ ('a'..='z' | 'A'..='Z')), ..] => {
                    return Some(Err(error::unexpected_character_in_numeric_literal(
                        *c,
                        self.full_span_at(*idx),
                    )));
                }
                tokens => {
                    let end = tokens.first().map_or(self.src.len(), |&(idx, _)| idx);

                    break end;
                }
            }

            self.remaining.next();
        };

        Some(Ok(Token::Number(Cow::Borrowed(&self.src[start..end]))))
    }
}
