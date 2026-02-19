use std::ops::{Deref, DerefMut};

use copyspan::Span;
use itertools::Itertools;
use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::PartialSpanned,
    gc::{GCPtr, util::GCDebug},
    lexer::{BracketType, Token},
    parser::{
        self, FindLeft, Parse, ParseLeft,
        traits::{FindRight, ParseRight, impl_using_parse_left},
        util::NonBracketedIter,
    },
    util::element_offset,
};

pub type Parenthesized<T> = Bracketed<{ BracketType::Round.to_u8() }, T>;
pub type SquareBracketed<T> = Bracketed<{ BracketType::Square.to_u8() }, T>;
pub type CurlyBracketed<T> = Bracketed<{ BracketType::Curly.to_u8() }, T>;

/// Parses a syntax construct surrounded by brackets. This type typically shouldn't be referred to
/// directly. Instead, its type aliases should be used.
#[derive(Clone, Copy, GCPtr, GCDebug)]
#[debug_direct]
pub struct Bracketed<const BRACKET_TYPE: u8, T: GCPtr + GCDebug>(pub T);

impl<const B: u8, T: GCPtr + GCDebug> Bracketed<B, T> {
    pub const BRACKET_TYPE: BracketType = if let Some(bt) = BracketType::from_u8(B) {
        bt
    } else {
        panic!("Invalid bracket type supplied as generic argument")
    };
}

impl<const B: u8, T: GCPtr + GCDebug + Parse> Parse for Bracketed<B, T> {
    impl_using_parse_left!();
}

impl<const B: u8, T: GCPtr + GCDebug + Parse> ParseLeft for Bracketed<B, T> {
    const EXPECTED_ERROR_FUNCTION_LEFT: fn(Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_opening_bracket::<B>;

    fn parse_from_left(
        parser: &super::Parser,
        tokens: &mut &super::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        let mut nb_iter = NonBracketedIter::new_unfused(&tokens);

        let Some(&PartialSpanned(Token::OpeningBracket(bt), _)) = nb_iter.next().transpose()?
        else {
            return Ok(None);
        };

        if bt != Self::BRACKET_TYPE {
            return Ok(None);
        }

        let Some(closing @ &PartialSpanned(Token::ClosingBracket(bt), _)) =
            nb_iter.next().transpose()?
        else {
            unreachable!();
        };

        assert!(bt == Self::BRACKET_TYPE);

        let closing_idx = element_offset(&tokens, closing).unwrap();

        let inner = &tokens[1..closing_idx];
        let inner = T::parse(parser, inner)?.ok_or_else(|| {
            let span = crate::error::span_of(inner).unwrap_or(closing.1.span_at());
            T::EXPECTED_ERROR_FUNCTION(span)
        })?;

        *tokens = nb_iter.remainder();

        Ok(Some(Self(inner)))
    }
}

impl<const B: u8, T: GCPtr + GCDebug> FindLeft for Bracketed<B, T> {
    fn find_left(
        _parser: &parser::Parser,
        tokens: &parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<std::ops::RangeFrom<usize>>> {
        Ok(
            NonBracketedIter::new(tokens)
                .process_results(|mut iter|
                    iter.find(|tok|
                        matches!(tok, PartialSpanned(Token::OpeningBracket(bt), _) if *bt == Self::BRACKET_TYPE)
                    )
                )?
                .and_then(|tok| element_offset(tokens, tok))
                .map(|idx| idx..)
        )
    }
}

impl<const B: u8, T: GCPtr + GCDebug + Parse> ParseRight for Bracketed<B, T> {
    const EXPECTED_ERROR_FUNCTION_RIGHT: fn(Span) -> crate::error::parse::ParseDiagnostic =
        parser::error::expected_opening_bracket::<B>;

    fn parse_from_right(
        parser: &parser::Parser,
        tokens: &mut &parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<Self>> {
        {
            let mut nb_iter = NonBracketedIter::new_unfused(&tokens);

            let Some(&PartialSpanned(Token::ClosingBracket(bt), _)) =
                nb_iter.next_back().transpose()?
            else {
                return Ok(None);
            };

            if bt != Self::BRACKET_TYPE {
                return Ok(None);
            }

            let Some(opening @ &PartialSpanned(Token::OpeningBracket(bt), _)) =
                nb_iter.next_back().transpose()?
            else {
                unreachable!();
            };

            assert!(bt == Self::BRACKET_TYPE);

            let opening_idx = element_offset(&tokens, opening).unwrap();

            let inner = &tokens[opening_idx + 1..tokens.len() - 1];
            let inner = T::parse(parser, inner)?.ok_or_else(|| {
                let span = crate::error::span_of(inner).unwrap_or(opening.1.span_at());
                T::EXPECTED_ERROR_FUNCTION(span)
            })?;

            *tokens = nb_iter.remainder();

            Ok(Some(Self(inner)))
        }
    }
}

impl<const B: u8, T: GCPtr + GCDebug> FindRight for Bracketed<B, T> {
    fn find_right(
        _parser: &parser::Parser,
        tokens: &parser::TokenStream,
    ) -> crate::error::parse::PDResult<Option<std::ops::RangeTo<usize>>> {
        Ok(
            NonBracketedIter::new(tokens)
                .rev()
                .process_results(|mut iter|
                    iter.find(|tok|
                        matches!(tok, PartialSpanned(Token::ClosingBracket(bt), _) if *bt == Self::BRACKET_TYPE)
                    )
                )?
                .and_then(|tok| element_offset(tokens, tok))
                .map(|idx| ..(idx + 1))
        )
    }
}

impl<const B: u8, T: GCPtr + GCDebug> AsRef<T> for Bracketed<B, T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<const B: u8, T: GCPtr + GCDebug> AsMut<T> for Bracketed<B, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<const B: u8, T: GCPtr + GCDebug> Deref for Bracketed<B, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const B: u8, T: GCPtr + GCDebug> DerefMut for Bracketed<B, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
