use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromToU8)]
#[repr(u8)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromToU8)]
#[repr(u8)]
pub enum BracketType {
    Round,
    Square,
    Curly,
}

#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Token<'src> {
    Identifier(Cow<'src, str>),
    StringLiteral(Cow<'src, str>),
    Number(Cow<'src, str>),
    Symbol(Symbol),
    OpeningBracket(BracketType),
    ClosingBracket(BracketType),
}

impl<'src> Display for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match *self {
            Token::Identifier(ref ident) => {
                return write!(f, "{ident}");
            }
            Token::Number(ref num) => {
                return write!(f, "{num}");
            }
            Token::StringLiteral(ref cow) => {
                return write!(f, "\"{}\"", cow.escape_debug());
            }
            Token::Symbol(sym) => sym.str(),
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

impl Symbol {
    /// Gets the string representation of the symbol
    pub const fn str(self) -> &'static str {
        match self {
            Sym!(.) => ".",
            Sym!(,) => ",",
            Sym!(;) => ";",
            Sym!(->) => "->",
            Sym!(=) => "=",
            Sym!(|) => "|",
            Sym!(+) => "+",
            Sym!(-) => "-",
            Sym!(/) => "/",
            Sym!(*) => "*",
            Sym!(^) => "^",
            Sym!(>) => ">",
            Sym!(<) => "<",
        }
    }
}

impl<'src> Debug for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if matches!(self, Token::OpeningBracket(_) | Token::ClosingBracket(_)) {
            return write!(f, "Token(`{self}`)");
        }

        write!(f, "Token({self})")
    }
}

/// Macro for defining [`Token`]s
#[macro_export]
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
        $crate::lexer::Token::Symbol($crate::Sym!(.))
    };
    (,) => {
        $crate::lexer::Token::Symbol($crate::Sym!(,))
    };
    (;) => {
        $crate::lexer::Token::Symbol($crate::Sym!(;))
    };
    (->) => {
        $crate::lexer::Token::Symbol($crate::Sym!(->))
    };
    (=) => {
        $crate::lexer::Token::Symbol($crate::Sym!(=))
    };
    (|) => {
        $crate::lexer::Token::Symbol($crate::Sym!(|))
    };
    (+) => {
        $crate::lexer::Token::Symbol($crate::Sym!(+))
    };
    (-) => {
        $crate::lexer::Token::Symbol($crate::Sym!(-))
    };
    (/) => {
        $crate::lexer::Token::Symbol($crate::Sym!(/))
    };
    (*) => {
        $crate::lexer::Token::Symbol($crate::Sym!(*))
    };
    (^) => {
        $crate::lexer::Token::Symbol($crate::Sym!(^))
    };
    (<) => {
        $crate::lexer::Token::Symbol($crate::Sym!(<))
    };
    (>) => {
        $crate::lexer::Token::Symbol($crate::Sym!(>))
    };
    (_) => {
        $crate::lexer::Token::Identifier("_")
    };
    ($lit:literal) => {
        $crate::lexer::TokenLiteralHelper($lit).create(::core::stringify!($lit))
    };
    ($ident:ident) => {
        $crate::lexer::Token::Identifier(::std::borrow::Cow::Borrowed(::core::stringify!($ident)))
    };
}

pub(crate) use T;

// Declarative macros cannot differentiate between string literals and numeric literals. This helper
// struct is used to accomplish this.
#[doc(hidden)]
pub struct TokenLiteralHelper<T>(pub T);

impl TokenLiteralHelper<&'static str> {
    pub const fn create(self, _: &'static str) -> Token<'static> {
        Token::StringLiteral(Cow::Borrowed(self.0))
    }
}

impl TokenLiteralHelper<f64> {
    pub const fn create(self, stringified: &'static str) -> Token<'static> {
        Token::Number(Cow::Borrowed(stringified))
    }
}

impl TokenLiteralHelper<u64> {
    pub const fn create(self, stringified: &'static str) -> Token<'static> {
        Token::Number(Cow::Borrowed(stringified))
    }
}

/// Macro for defining [`Symbol`]s
#[macro_export]
macro_rules! Sym {
    (.) => {
        $crate::lexer::Symbol::Dot
    };
    (,) => {
        $crate::lexer::Symbol::Comma
    };
    (;) => {
        $crate::lexer::Symbol::Semicolon
    };
    (->) => {
        $crate::lexer::Symbol::RightArrow
    };
    (=) => {
        $crate::lexer::Symbol::Equals
    };
    (|) => {
        $crate::lexer::Symbol::Pipe
    };
    (+) => {
        $crate::lexer::Symbol::Plus
    };
    (-) => {
        $crate::lexer::Symbol::Hyphen
    };
    (/) => {
        $crate::lexer::Symbol::Slash
    };
    (*) => {
        $crate::lexer::Symbol::Asterisk
    };
    (^) => {
        $crate::lexer::Symbol::Caret
    };
    (<) => {
        $crate::lexer::Symbol::LessThan
    };
    (>) => {
        $crate::lexer::Symbol::GreaterThan
    };
}

pub(crate) use Sym;
use mulch_macros::FromToU8;
