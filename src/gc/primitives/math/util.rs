use copyspan::Span;
use gmp_mpfr_sys::gmp::limb_t;
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

/// Calculates the number of limbs required to store any base-10 number of a specific length.
#[allow(unexpected_cfgs)]
pub(super) fn maximum_limbs_from_num_digits(digits: usize) -> usize {
    // We're calculating the maximum number of limbs required to store our number using:
    //
    // `floor((d * m) / 2^(k + o)) + 1`
    //
    // where `d` is the number of digits, `k` is `usize::BITS`, `o` is `MAGIC_SHIFT`, and m is
    // `MAGIC_NUM` which is calculated with:
    //
    // m = ceil(log2(10) * 2^(k + o) / k)
    //
    // `o` is either zero or the largest number that allows `m < 2^k` where `m` is odd.
    //
    // This is mathematically guarenteed to be greater-than or equal-to the actual number of limbs
    // required. It works nicely because it only uses simple integer operations at run time.
    //
    //    floor(log_(2^k)(10^d - 1) ) + 1 # this is the real, 100% accurate maximum
    // <= floor(log_(2^k)(10^d    ) ) + 1
    //  = floor(log2(10^d)/log2(2^k)) + 1
    //  = floor(log2(10^d)/k        ) + 1
    //  = floor( d * log2(10)/k     ) + 1
    //  = floor((d * log2(10) * 2^(k + o) / k)/2^(k+o)) + 1
    // <= floor((d * ceil(log2(10) * 2^(k + o) / k)) / 2^(k + o)) + 1 # The estimate we're using

    const VALUES: (usize, u32) = {
        if limb_t::BITS != usize::BITS {
            unimplemented!()
        }

        match usize::BITS {
            16 => (54427, 2),
            #[cfg(target_pointer_width = "32")]
            32 => (891723283, 1),
            #[cfg(target_pointer_width = "64")]
            64 => (15319689349413178111, 4),
            #[cfg(target_pointer_width = "128")]
            128 => (282598388717358879668502530574794087245, 5),
            _ => unimplemented!(),
        }
    };

    const MAGIC_NUM: usize = VALUES.0;
    const MAGIC_SHIFT: u32 = VALUES.1;

    let (_low, high) = digits.carrying_mul(MAGIC_NUM, 0);
    (high >> MAGIC_SHIFT) + 1
}

/// Calculates the number of base 10 digits required to store a number with a certain number of
/// limbs.
#[allow(unexpected_cfgs)]
pub(super) fn maximum_digits_from_num_limbs(digits: usize) -> usize {
    // We're calculating the maximum number of digits required to store our number using:
    //
    // `n * w + floor[(n*m) / 2^(k+o)] + 1`
    //
    // where `n` is the number of limbs, `k` is `usize::BITS`, `o` is `MAGIC_SHIFT`, m is
    // `MAGIC_NUM` which is calculated with:
    //
    // m = ceil[mod(k * log10(2), 1) * 2^(k+o)]
    //
    // and `w` is `WHOLE_PART` which is calculated with:
    //
    // w = floor[k * log10(2)]
    //
    // `o` is either zero or the largest number that allows `m < 2^k` where `m` is odd. In this
    // `function, `o` happens to be zero in every case so it has been omitted.
    //
    // This is mathematically guarenteed to be greater-than or equal-to the actual number of digits
    // required. It works nicely because it only uses simple integer operations at run time.
    //
    //    floor(log10((2^k)^n - 1) ) + 1 # this is the real, 100% accurate maximum
    // <= floor(log10((2^k)^n    ) ) + 1
    //  = floor(log10(2^(k*n))) + 1
    //  = floor(n*k*log10(2)) + 1
    //  = floor(n*floor[k * log10(2)] + n * mod[k * log10(2), 1]) + 1
    //  = n*floor[k * log10(2)] + floor(n * mod[k * log10(2), 1]) + 1
    //  = n*floor[k * log10(2)] + floor(n * mod[k * log10(2), 1] * 2^(k+o) / 2^(k+o)) + 1
    // <= n*floor[k * log10(2)] + floor(n * ceil[mod(k * log10(2), 1) * 2^(k+o)] / 2^(k+o)) + 1 # the approximation we're using

    const VALUES: (usize, usize) = {
        if limb_t::BITS != usize::BITS {
            unimplemented!()
        }

        match usize::BITS {
            16 => (53509, 4),
            #[cfg(target_pointer_width = "32")]
            32 => (2718541904, 9),
            #[cfg(target_pointer_width = "64")]
            64 => (4905353065013375762, 19),
            #[cfg(target_pointer_width = "128")]
            128 => (180975585162976948393314901606021750282, 38),
            _ => unimplemented!(),
        }
    };

    const MAGIC_NUM: usize = VALUES.0;
    const WHOLE_PART: usize = VALUES.1;

    let (_low, high) = digits.carrying_mul(MAGIC_NUM, 0);
    digits * WHOLE_PART + high + 1
}

/// Calculates the number of limbs required to store a number of the form `2^a * 5^b`
#[allow(unexpected_cfgs)]
pub(super) fn num_limbs_from_pow10_factorization(factorization: PowerOfTenFactorization) -> usize {
    // We're calculating the maximum number of limbs required to store our number using:
    //
    // floor([a*2^p + b*m] / 2^(k+o)) + 1
    //
    // where `d` is the number of digits, `k` is `usize::BITS`, `o` is `MAGIC_SHIFT`, m is
    // `MAGIC_NUM` which is calculated with:
    //
    // m = ceil[log2(5)*2^(k+o)/k]
    //
    // and `p` is `pow2_shift` which is calculated with:
    //
    // p = k+o-log2(k)
    //
    // `o` is either zero or the largest number that allows `m < 2^(k - 1)` and `o <= log2(k) - 1`
    // `where `m` is even.
    //
    // This estimate is mathematically guarenteed to be greater-than or equal-to the actual number
    // of limbs required. It works nicely because it only uses simple integer operations at run
    // time.
    //
    //    floor(log_(2^k)(2^a * 5^b)) + 1 # this is the real, 100% accurate maximum
    //  = floor(log2(2^a * 5^b)/log2(2^k)) + 1
    //  = floor([log2(2^a)+log2(5^b)]/log2(2^k)) + 1
    //  = floor([a + b * log2(5)]/k) + 1
    //  = floor([a*2^(k+o)/k + b*log2(5)*2^(k+o)/k] / 2^(k+o)) + 1
    // <= floor([a*2^(k+o-log2(k)) + b*ceil(log2(5)*2^(k+o)/k)] / 2^(k+o)) + 1 # The estimate we're using

    const VALUES: (usize, u32) = {
        if limb_t::BITS != usize::BITS {
            unimplemented!()
        }

        match usize::BITS {
            16 => (9511, 0),
            #[cfg(target_pointer_width = "32")]
            32 => (2493151308, 3),
            #[cfg(target_pointer_width = "64")]
            64 => (669250208186611888, 0),
            #[cfg(target_pointer_width = "128")]
            128 => (98763898493562131901329439358426017191, 4),
            _ => unimplemented!(),
        }
    };

    const MAGIC_NUM: usize = VALUES.0;
    const MAGIC_SHIFT: u32 = VALUES.1;

    const BITS_LOG2: u32 = usize::BITS.trailing_zeros();
    const POW2_SHIFT: u32 = usize::BITS + MAGIC_SHIFT - BITS_LOG2;

    let lower_add = factorization.pow_2 << POW2_SHIFT;
    let upper_add = factorization.pow_2 >> (usize::BITS - POW2_SHIFT);

    let (_low, high) = factorization.pow_5.carrying_mul(MAGIC_NUM, lower_add);

    ((high + upper_add) >> MAGIC_SHIFT) + 1
}
