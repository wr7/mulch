use std::{cmp::Ordering, num::NonZeroUsize};

use gmp_mpfr_sys::gmp::{limb_t, mpn_gcd, mpn_tdiv_qr, size_t};

use crate::{
    error::{PartialSpanned, parse::PDResult},
    gc::{
        GCBuffer, GCPtr, GarbageCollector,
        math::{literal_type, num_integer_digits, strip_integer_zeroes},
        primitives::math::{
            Digit, PowerOfTenFactorization, decimal_literal_info, strip_decimal_zeroes,
            uint::GCUInt,
        },
        util::{GCDebug, GCEq, GCWrap},
    },
    parser,
};

use super::NumLiteralType;

/// A garbage-collected, infinite precision rational number.
///
/// # Layout
/// The numerator and denominator SHOULD NOT have leading zero limbs unless the value itself is
/// zero. In which case, it there should be exactly one limb.
///
/// Pointed to data:
/// ```
/// union {
///     struct has_value {
///         u64 {
///             SET_TO_ZERO : u1
///             numerator_len: u31
///             positive_if_set: u1
///             denominator_len: u31
///         }
///         PADDING_TIL_NEXT_BLOCK
///         numerator: [limb_t; numerator_len]
///         denominator: [limb_t; denominator_len]
///     }
///
///     struct forward {
///         u64 {
///             SET_TO_ONE : u1
///             forward: u63
///         }
///     }
/// }
/// ```
#[derive(Clone, Copy)]
pub struct GCRational {
    ptr: NonZeroUsize,
}

#[derive(Clone, Copy, Debug)]
struct RationalMetadata {
    numerator_len: u32,
    is_negative: bool,
    denominator_len: u32,
}

impl From<RationalMetadata> for u64 {
    fn from(value: RationalMetadata) -> Self {
        const _31_BIT_MASK: u64 = !(u64::MAX << 31);

        ((u64::from(value.numerator_len) & _31_BIT_MASK) << 32)
            | (u64::from(value.is_negative) << 31)
            | (u64::from(value.denominator_len) & _31_BIT_MASK)
    }
}

impl From<u64> for RationalMetadata {
    fn from(value: u64) -> Self {
        const _31_BIT_MASK: u64 = !(u64::MAX << 31);

        let numerator_len = (value >> 32 & _31_BIT_MASK) as u32;
        let is_negative = value >> 31 & 1 != 0;
        let denominator_len = (value & _31_BIT_MASK) as u32;

        RationalMetadata {
            numerator_len,
            is_negative,
            denominator_len,
        }
    }
}

impl GCRational {
    const METADATA_SIZE_BLOCKS: usize = 8usize.div_ceil(GarbageCollector::BLOCK_SIZE);

    const FORWARD_BIT: u64 = 1u64 << 63;

    pub unsafe fn from_raw(ptr: NonZeroUsize) -> Self {
        Self { ptr }
    }

    pub fn gc_ptr(self) -> NonZeroUsize {
        self.ptr
    }

    pub fn parse_from_literal(
        gc: &GarbageCollector,
        literal: PartialSpanned<&str>,
    ) -> PDResult<Self> {
        match literal_type(literal)? {
            NumLiteralType::Decimal => Self::parse_from_decimal_literal(gc, literal),
            NumLiteralType::Fraction => Self::parse_from_fraction_literal(gc, literal),
        }
    }

    fn parse_from_fraction_literal(
        gc: &GarbageCollector,
        literal: PartialSpanned<&str>,
    ) -> PDResult<Self> {
        let slash = literal
            .0
            .char_indices()
            .find(|(_, c)| matches!(c, '/'))
            .map(|(i, _)| i)
            .unwrap();

        let numerator = &literal[0..slash];
        let denominator = &literal[slash + 1..];

        let numerator = strip_integer_zeroes(numerator);
        let denominator = strip_integer_zeroes(denominator);

        let numerator_len = num_integer_digits(numerator, literal.1)?;
        let denominator_len = num_integer_digits(denominator, literal.1)?;

        let metadata_ptr = gc.from_space.len();
        gc.from_space
            .set_len(metadata_ptr + Self::METADATA_SIZE_BLOCKS);

        let numerator = unsafe {
            GCUInt::parse_from_digits(
                gc,
                numerator
                    .bytes()
                    .filter_map(|c| Digit::from_u8(c.checked_sub(b'0')?)),
                numerator_len,
            )
        };

        let denominator = unsafe {
            GCUInt::parse_from_digits(
                gc,
                denominator
                    .bytes()
                    .filter_map(|c| Digit::from_u8(c.checked_sub(b'0')?)),
                denominator_len,
            )
        };

        let metadata = RationalMetadata {
            numerator_len: numerator.data.size_blocks() as u32,
            is_negative: false,
            denominator_len: denominator.data.size_blocks() as u32,
        };

        unsafe {
            gc.from_space
                .block_ptr(metadata_ptr)
                .cast::<u64>()
                .write(u64::from(metadata))
        };

        let rational = Self {
            ptr: unsafe { NonZeroUsize::new_unchecked(metadata_ptr) },
        };

        unsafe {
            if denominator.is_zero(gc) {
                rational.deallocate_from_end(gc);

                return Err(parser::error::denominator_of_zero(literal.1));
            }
        }

        unsafe { rational.reduce_from_end(gc) };

        Ok(rational)
    }

    fn parse_from_decimal_literal(
        gc: &GarbageCollector,
        decimal: PartialSpanned<&str>,
    ) -> PDResult<Self> {
        let decimal = decimal.map(|decimal| strip_decimal_zeroes(&decimal));

        let (num_digits_after_decimal_point, num_digits) = decimal_literal_info(decimal)?;

        let metadata_ptr = gc.from_space.len();
        gc.from_space
            .set_len(metadata_ptr + Self::METADATA_SIZE_BLOCKS);

        let mut numerator = unsafe {
            GCUInt::parse_from_digits(
                gc,
                decimal
                    .bytes()
                    .filter_map(|c| Digit::from_u8(c.checked_sub(b'0')?)),
                num_digits,
            )
        };

        let mut denominator = PowerOfTenFactorization {
            pow_5: num_digits_after_decimal_point,
            pow_2: num_digits_after_decimal_point,
        };

        unsafe { numerator.reduce_pow_10_at_end(gc, &mut denominator) };

        let denominator = GCUInt::from_pow10_factorization(gc, denominator);

        let metadata = RationalMetadata {
            numerator_len: numerator.data.len() as u32,
            is_negative: false,
            denominator_len: denominator.data.len() as u32,
        };

        let metadata = u64::from(metadata);

        unsafe {
            gc.from_space
                .block_ptr(metadata_ptr)
                .cast::<u64>()
                .write(metadata);
        }

        Ok(Self {
            ptr: unsafe { NonZeroUsize::new_unchecked(metadata_ptr) },
        })
    }

    pub unsafe fn as_usize(self, gc: &GarbageCollector) -> Option<usize> {
        let metadata = unsafe { self.metadata(gc) };

        if metadata.is_negative {
            return None;
        }

        let [numerator, denominator] = self
            .numerator_and_denominator_from_metadata(metadata)
            .map(|b| GCUInt { data: b });

        if unsafe { GCWrap::new(denominator, gc) != (1 as limb_t) } {
            return None;
        }

        numerator.as_usize(gc)
    }

    pub(in crate::gc::primitives) unsafe fn deallocate_from_end(self, gc: &GarbageCollector) {
        let [numerator, denominator] = unsafe { self.numerator_and_denominator(gc) };

        let total_len =
            Self::METADATA_SIZE_BLOCKS + numerator.size_blocks() + denominator.size_blocks();

        debug_assert_eq!(
            self.ptr.get() + total_len,
            gc.from_space.len(),
            "deallocate_from_end can only be called if the current allocation is the last allocation made"
        );

        gc.from_space.set_len(self.ptr.get());
    }

    unsafe fn numerator_and_denominator(self, gc: &GarbageCollector) -> [GCBuffer<limb_t>; 2] {
        self.numerator_and_denominator_from_metadata(unsafe { self.metadata(gc) })
    }

    unsafe fn metadata(self, gc: &GarbageCollector) -> RationalMetadata {
        unsafe {
            gc.from_space
                .block_ptr(self.ptr)
                .cast::<u64>()
                .read()
                .into()
        }
    }

    fn numerator_and_denominator_from_metadata(
        self,
        metadata: RationalMetadata,
    ) -> [GCBuffer<limb_t>; 2] {
        let numerator_ptr = self.ptr.get() + Self::METADATA_SIZE_BLOCKS;

        let denominator_ptr = numerator_ptr
            + GCBuffer::<limb_t>::allocation_size_blocks(metadata.numerator_len as usize);

        unsafe {
            [
                GCBuffer::from_raw_parts(
                    NonZeroUsize::new_unchecked(numerator_ptr),
                    metadata.numerator_len as usize,
                ),
                GCBuffer::from_raw_parts(
                    NonZeroUsize::new_unchecked(denominator_ptr),
                    metadata.denominator_len as usize,
                ),
            ]
        }
    }

    /// Reduces the fraction. Requires that the `self` is the last object in the `GCSpace`.
    /// Additionally, the denominator must not be zero.
    unsafe fn reduce_from_end(self, gc: &GarbageCollector) {
        let old_metadata = unsafe { self.metadata(gc) };

        let [mut numerator, mut denominator] = self
            .numerator_and_denominator_from_metadata(old_metadata)
            .map(|b| GCUInt::from(b));

        unsafe {
            debug_assert!(!denominator.is_zero(gc));
        }

        // Our `gcd` function requires that at-least one input is odd. This means that we will have
        // to remove as many trailing zeroes as we can before-hand.

        let trailing_zeroes = unsafe {
            numerator
                .trailing_zeroes(gc)
                .min(denominator.trailing_zeroes(gc))
        };

        unsafe {
            let numerator_len = numerator.shr_unchecked(gc, trailing_zeroes);
            let denominator_len = denominator.shr_unchecked(gc, trailing_zeroes);

            numerator.data.set_length(numerator_len);
            denominator.data.set_length_at_end(gc, denominator_len);
        };

        // GCD will also destroy our inputs, so we need to make copies

        let mut tmp1 = GCUInt::from(GCBuffer::<limb_t>::new_uninit(gc, numerator.data.len()));
        unsafe {
            std::ptr::copy_nonoverlapping(
                numerator.data.as_ptr(gc),
                tmp1.data.as_mut_ptr(gc),
                tmp1.data.len(),
            );

            tmp1.trim_leading_zero_limbs_at_end(gc);
        };

        let mut tmp2 = GCUInt::from(GCBuffer::<limb_t>::new_uninit(gc, denominator.data.len()));
        unsafe {
            std::ptr::copy_nonoverlapping(
                denominator.data.as_ptr(gc),
                tmp2.data.as_mut_ptr(gc),
                tmp2.data.len(),
            );

            tmp2.trim_leading_zero_limbs_at_end(gc);
        };

        let mut tmp_greater = tmp1;
        let mut tmp_less = tmp2;

        if unsafe { tmp_greater.cmp(gc, tmp_less) } == Ordering::Less {
            std::mem::swap(&mut tmp_greater, &mut tmp_less);
        }

        let mut gcd_buf = GCBuffer::<limb_t>::new_uninit(gc, tmp_less.data.len());

        unsafe {
            let gcd_len = mpn_gcd(
                gcd_buf.as_mut_ptr(gc),
                tmp_greater.data.as_mut_ptr(gc),
                tmp_greater.data.len() as size_t,
                tmp_less.data.as_mut_ptr(gc),
                tmp_less.data.len() as size_t,
            ) as usize;

            gcd_buf.set_length_at_end(gc, gcd_len);

            GCUInt::from(gcd_buf).trim_leading_zero_limbs_at_end(gc);
        }

        // Now we need to divide, but the buffers cannot overlap (aside from the remainder and the
        // dividend), so we need to copy our numerator and denominator into tmp1 and tmp2 again.

        unsafe {
            std::ptr::copy_nonoverlapping(
                numerator.data.as_ptr(gc),
                tmp1.data.as_mut_ptr(gc),
                tmp1.data.len(),
            );
            std::ptr::copy_nonoverlapping(
                denominator.data.as_ptr(gc),
                tmp2.data.as_mut_ptr(gc),
                tmp2.data.len(),
            );

            mpn_tdiv_qr(
                numerator.data.as_mut_ptr(gc),
                tmp1.data.as_mut_ptr(gc),
                0,
                tmp1.data.as_ptr(gc),
                tmp1.data.len() as size_t,
                gcd_buf.as_ptr(gc),
                gcd_buf.len() as size_t,
            );

            numerator
                .data
                .set_length(tmp1.data.len() - gcd_buf.len() + 1);

            mpn_tdiv_qr(
                denominator.data.as_mut_ptr(gc),
                tmp2.data.as_mut_ptr(gc),
                0,
                tmp2.data.as_ptr(gc),
                tmp2.data.len() as size_t,
                gcd_buf.as_ptr(gc),
                gcd_buf.len() as size_t,
            );

            denominator
                .data
                .set_length(tmp2.data.len() - gcd_buf.len() + 1);
        }

        // Fix the memory layout since our numerator and denominator may have shrunk

        unsafe {
            let numerator = numerator.without_leading_zero_limbs(gc);
            let denominator = denominator.without_leading_zero_limbs(gc);

            let new_denominator_ptr = numerator.data.gc_ptr().get() + numerator.data.size_blocks();

            std::ptr::copy(
                denominator.data.as_ptr(gc),
                gc.from_space
                    .block_ptr(new_denominator_ptr)
                    .cast::<limb_t>(),
                denominator.data.len(),
            );

            let denominator = GCBuffer::<limb_t>::from_raw_parts(
                NonZeroUsize::new_unchecked(new_denominator_ptr),
                denominator.data.len(),
            );

            gc.from_space
                .set_len(new_denominator_ptr + denominator.size_blocks());

            // Adjust metadata to use new sizes

            gc.from_space
                .block_ptr(self.ptr)
                .cast::<u64>()
                .write(u64::from(RationalMetadata {
                    numerator_len: numerator.data.len() as u32,
                    is_negative: old_metadata.is_negative,
                    denominator_len: denominator.len() as u32,
                }));
        }
    }
}

unsafe impl GCPtr for GCRational {
    const MSB_RESERVED: bool = true;

    unsafe fn gc_copy(self, gc: &mut crate::gc::GarbageCollector) -> Self {
        let raw_metadata = unsafe { gc.from_space.block_ptr(self.ptr).cast::<u64>().read() };

        if raw_metadata & Self::FORWARD_BIT != 0 {
            let forward = raw_metadata & !Self::FORWARD_BIT;

            return Self {
                ptr: unsafe { NonZeroUsize::new_unchecked(forward as usize) },
            };
        }

        let metadata = RationalMetadata::from(raw_metadata);

        let [old_numerator_buf, old_denominator_buf] =
            self.numerator_and_denominator_from_metadata(metadata);

        let new_ptr = gc.to_space.len();
        gc.to_space.expand_to(new_ptr + Self::METADATA_SIZE_BLOCKS);

        let new_numerator_buf =
            GCBuffer::<limb_t>::new_uninit_in_space(&gc.to_space, old_numerator_buf.len());
        let new_denominator_buf =
            GCBuffer::<limb_t>::new_uninit_in_space(&gc.to_space, old_denominator_buf.len());

        let new_metadata_ptr = gc.to_space.block_ptr(new_ptr).cast::<u64>();

        unsafe { new_metadata_ptr.write(raw_metadata) };

        unsafe {
            std::ptr::copy_nonoverlapping(
                old_numerator_buf.as_ptr_in_space(&gc.from_space),
                new_numerator_buf.as_mut_ptr_in_space(&gc.to_space),
                new_numerator_buf.len(),
            );

            std::ptr::copy_nonoverlapping(
                old_denominator_buf.as_ptr_in_space(&gc.from_space),
                new_denominator_buf.as_mut_ptr_in_space(&gc.to_space),
                new_denominator_buf.len(),
            );
        };

        Self {
            ptr: unsafe { NonZeroUsize::new_unchecked(new_ptr) },
        }
    }
}

impl GCDebug for GCRational {
    unsafe fn gc_debug(
        self,
        gc: &GarbageCollector,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let metadata = unsafe { self.metadata(gc) };

        let [numerator, denominator] =
            unsafe { self.numerator_and_denominator(gc) }.map(|b| GCUInt::from(b));

        if metadata.is_negative {
            write!(f, "-")?;
        }

        let numerator = unsafe { numerator.to_naive_string(gc) };

        write!(f, "{}", numerator)?;

        if unsafe { GCWrap::new(denominator, gc) != (1 as limb_t) } {
            write!(f, "/")?;

            let denominator = unsafe { denominator.to_naive_string(gc) };

            write!(f, "{}", denominator)?
        }

        Ok(())
    }
}

impl GCEq<GCRational> for GCRational {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &GCRational) -> bool {
        let metadata = unsafe { self.metadata(gc) };
        let [numerator, denominator] = self
            .numerator_and_denominator_from_metadata(metadata)
            .map(|b| unsafe { GCWrap::new(GCUInt { data: b }, gc) });

        let rhs_metadata = unsafe { rhs.metadata(gc) };
        let [rhs_numerator, rhs_denominator] = rhs
            .numerator_and_denominator_from_metadata(rhs_metadata)
            .map(|b| unsafe { GCWrap::new(GCUInt { data: b }, gc) });

        metadata.is_negative == rhs_metadata.is_negative
            && numerator == rhs_numerator
            && denominator == rhs_denominator
    }
}

impl GCEq<usize> for GCRational {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &usize) -> bool {
        let metadata = unsafe { self.metadata(gc) };
        let [numerator, denominator] = self
            .numerator_and_denominator_from_metadata(metadata)
            .map(|b| unsafe { GCWrap::new(GCUInt { data: b }, gc) });

        !metadata.is_negative && numerator == *rhs && denominator == (1 as limb_t)
    }
}
