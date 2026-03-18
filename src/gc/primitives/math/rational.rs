use std::num::NonZeroUsize;

use gmp_mpfr_sys::gmp::limb_t;

use crate::{
    error::{PartialSpanned, parse::PDResult},
    gc::{
        GCBuffer, GCPtr, GarbageCollector,
        primitives::math::{
            Digit, PowerOfTenFactorization, num_decimal_digits, strip_unneeded_zeroes, uint::GCUInt,
        },
        util::GCDebug,
    },
};

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

    pub fn parse_from_decimal(
        gc: &GarbageCollector,
        decimal: PartialSpanned<&str>,
    ) -> PDResult<Self> {
        let decimal = decimal.map(|decimal| strip_unneeded_zeroes(&decimal));

        let (num_digits_after_decimal_point, num_digits) = num_decimal_digits(decimal)?;

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

        let [numerator, denominator] = self.numerator_and_denominator_from_metadata(metadata);

        if !matches!(unsafe { denominator.as_slice(gc) }, [1]) {
            return None;
        }

        // `limb_t` is probably always the same as `usize`, but that could theoretically change in
        // the future.
        const NUM_LIMBS_IN_USIZE: usize =
            std::mem::size_of::<usize>() / std::mem::size_of::<limb_t>();

        let numerator = unsafe { numerator.as_slice(gc) };

        debug_assert_ne!(numerator.len(), 0);

        if numerator.len() > NUM_LIMBS_IN_USIZE || numerator.len() == 0 {
            return None;
        }

        let mut retval: usize = 0;
        for limb in numerator.iter().rev().copied() {
            retval = retval.unbounded_shl(limb_t::BITS);
            retval |= limb as usize;
        }

        Some(retval | 1usize.rotate_right(1))
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
        let denominator = unsafe { denominator.to_naive_string(gc) };

        write!(f, "{}", numerator)?;

        if denominator != "1" {
            write!(f, "/")?;

            write!(f, "{}", denominator)?
        }

        Ok(())
    }
}
