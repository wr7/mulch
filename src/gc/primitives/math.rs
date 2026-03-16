use mulch_macros::FromToU8;

mod uint;

/// Equal to `ceil(log2(5) * 256)`
///
/// This is used for certain calculations when parsing decimals
const POW_5_MULTIPLIER: usize = 595;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PowerOfTenFactorization {
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
