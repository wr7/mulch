use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Symbol {
    Dot,
    Comma,
    Semicolon,
    RightArrow,
    Equals,
    Pipe,
    Plus,
    Hyphen,
    Slash,
    Asterisk,
    Caret,
    LessThan,
    GreaterThan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BracketType {
    Round,
    Square,
    Curly,
}

#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token<'a> {
    Identifier(&'a str),
    StringLiteral(Cow<'a, str>),
    Number(&'a str),
    Symbol(Symbol),
    OpeningBracket(BracketType),
    ClosingBracket(BracketType),
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match *self {
            Token::Identifier(ident) => ident,
            Token::Number(num) => num,
            Token::StringLiteral(ref cow) => {
                return write!(f, "\"{}\"", cow.escape_debug());
            }
            T!(.) => ".",
            T!(,) => ",",
            T!(;) => ";",
            T!(->) => "->",
            T!(=) => "=",
            T!(|) => "|",
            T!(+) => "+",
            T!(-) => "-",
            T!(/) => "/",
            T!(*) => "*",
            T!(^) => "^",
            T!(>) => ">",
            T!(<) => "<",
            T!('(') => "(",
            T!(')') => ")",
            T!('[') => "[",
            T!(']') => "]",
            T!('{') => "{",
            T!('}') => "}",
        };

        write!(f, "{string}")
    }
}

impl<'a> Debug for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if matches!(self, Token::OpeningBracket(_) | Token::ClosingBracket(_)) {
            return write!(f, "Token(`{self}`)");
        }

        write!(f, "Token({self})")
    }
}

macro_rules! T {
    ('(') => {
        $crate::lexer::Token::OpeningBracket($crate::lexer::BracketType::Round)
    };
    (')') => {
        $crate::lexer::Token::ClosingBracket($crate::lexer::BracketType::Round)
    };
    ('[') => {
        $crate::lexer::Token::OpeningBracket($crate::lexer::BracketType::Square)
    };
    (']') => {
        $crate::lexer::Token::ClosingBracket($crate::lexer::BracketType::Square)
    };
    ('{') => {
        $crate::lexer::Token::OpeningBracket($crate::lexer::BracketType::Curly)
    };
    ('}') => {
        $crate::lexer::Token::ClosingBracket($crate::lexer::BracketType::Curly)
    };
    (.) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Dot)
    };
    (,) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Comma)
    };
    (;) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Semicolon)
    };
    (->) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::RightArrow)
    };
    (=) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Equals)
    };
    (|) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Pipe)
    };
    (+) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Plus)
    };
    (-) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Hyphen)
    };
    (/) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Slash)
    };
    (*) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Asterisk)
    };
    (^) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::Caret)
    };
    (<) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::LessThan)
    };
    (>) => {
        $crate::lexer::Token::Symbol($crate::lexer::Symbol::GreaterThan)
    };
    (_) => {
        $crate::lexer::Token::Identifier("_")
    };
    ($lit:literal) => {
        $crate::lexer::TokenLiteralHelper($lit).create(::core::stringify!($lit))
    };
    ($ident:ident) => {
        $crate::lexer::Token::Identifier(::core::stringify!($ident))
    };
}

pub(crate) use T;

// Declarative macros cannot differentiate between string literals and numeric literals. This helper
// struct is used to accomplish this.
#[doc(hidden)]
pub struct TokenLiteralHelper<T>(pub T);

impl TokenLiteralHelper<&'static str> {
    pub const fn create(self, _: &'static str) -> Token<'static> {
        Token::StringLiteral(Cow::Borrowed(&self.0))
    }
}

impl TokenLiteralHelper<f64> {
    pub const fn create(self, stringified: &'static str) -> Token<'static> {
        Token::Number(stringified)
    }
}

impl TokenLiteralHelper<u64> {
    pub const fn create(self, stringified: &'static str) -> Token<'static> {
        Token::Number(stringified)
    }
}
