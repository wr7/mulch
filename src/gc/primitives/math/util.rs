use copyspan::Span;
use mulch_macros::FromToU8;

use crate::{
    error::{PartialSpanned, parse::PDResult},
    parser,
};

/// Equal to `ceil(log2(5) * 256)`
///
/// This is used for certain calculations when parsing decimals
pub(super) const POW_5_MULTIPLIER: usize = 595;

#[derive(Clone, Copy, Debug)]
pub(super) struct PowerOfTenFactorization {
    pub pow_5: usize,
    pub pow_2: usize,
}

#[derive(Clone, Copy, FromToU8, Debug)]
#[repr(u8)]
pub enum Digit {
    Zero = 0,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

#[derive(Clone, Copy, FromToU8, Debug)]
pub(super) enum NumLiteralType {
    Decimal,
    Fraction,
}

pub(super) fn strip_integer_zeroes(str: &str) -> &str {
    let start = str
        .char_indices()
        .find(|(_, c)| !matches!(c, '_' | '0'))
        .map_or(str.len(), |(i, _)| i);

    &str[start..]
}

/// Strips unneeded trailing and leading zeroes/underscores.
///
/// `0_014_.2_00_` -> `14_.2`
///
/// `_3.0` -> `3`
///
/// Requires that the input is not empty
pub(super) fn strip_decimal_zeroes(str: &str) -> &str {
    assert!(!str.is_empty());

    let decimal_point = str
        .char_indices()
        .find(|(_, c)| *c == '.')
        .map_or(str.len(), |(i, _)| i);

    let last_nonzero = str
        .get(decimal_point + 1..)
        .and_then(|str| str.char_indices().rfind(|(_, c)| !matches!(c, '0' | '_')))
        .map(|(i, _)| i + decimal_point + 1);

    let end = last_nonzero.map_or(decimal_point, |i| i + 1);

    let start = str[..decimal_point]
        .char_indices()
        .find(|(_, c)| !matches!(c, '0' | '_'))
        .map_or(decimal_point, |(i, _)| i);

    &str[start..end]
}

pub(super) fn literal_type(literal: PartialSpanned<&str>) -> PDResult<NumLiteralType> {
    let mut iter = literal.chars().filter(|c| matches!(c, '.' | '/'));

    let num_type = iter
        .next()
        .and_then(|c| (c == '/').then_some(NumLiteralType::Fraction))
        .unwrap_or(NumLiteralType::Decimal);

    if iter.next().is_some() {
        return Err(parser::error::multiple_decimals_or_slashes_in_number(
            literal.1,
        ));
    }

    Ok(num_type)
}

pub(super) fn decimal_literal_info(decimal: PartialSpanned<&str>) -> PDResult<(usize, usize)> {
    let mut digits_after_decimal_point = 0;
    let mut num_digits = 0;
    let mut hit_decimal = false;

    for char in decimal.chars() {
        match char {
            '0'..='9' => {
                if hit_decimal {
                    digits_after_decimal_point += 1;
                }

                num_digits += 1;
            }
            '_' => continue,
            '.' => {
                if hit_decimal {
                    debug_assert!(false);
                } else {
                    hit_decimal = true;
                }
            }
            _ => {
                return Err(parser::error::unexpected_character_in_number(
                    char, decimal.1,
                ));
            }
        }
    }

    Ok((digits_after_decimal_point, num_digits))
}

pub(super) fn num_integer_digits(integer: &str, span: Span) -> PDResult<usize> {
    let mut ret = 0;

    for c in integer.chars() {
        match c {
            '0'..='9' => ret += 1,
            '_' => {}
            _ => return Err(parser::error::unexpected_character_in_number(c, span)),
        }
    }

    Ok(ret)
}
