use std::ffi::c_uint;

use super::PowerOfTenFactorization;

use gmp_mpfr_sys::gmp::{
    limb_t, mpn_addmul_1, mpn_copyd, mpn_copyi, mpn_divmod_1, mpn_get_str, mpn_lshift, mpn_mul_1,
    mpn_rshift, mpn_zero, mpn_zero_p, size_t,
};

use crate::gc::{
    GCBuffer, GarbageCollector,
    primitives::math::{Digit, POW_5_MULTIPLIER},
    util::GCEq,
};

/// An unsigned bigint stored in GC space. Note: this doesn't implement `GCPtr`. It is meant to be
/// used similarly to [`GCBuffer`].
#[derive(Clone, Copy)]
pub struct GCUInt {
    pub(crate) data: GCBuffer<limb_t>,
}

impl GCUInt {
    pub unsafe fn is_zero(self, gc: &GarbageCollector) -> bool {
        unsafe { mpn_zero_p(self.data.as_ptr(gc), self.data.len() as size_t) != 0 }
    }

    /// Shifts left by `shift_amnt` bits.
    ///
    /// # Safety
    /// - `shift_amnt` must be less than `size * limb_t::BITS`
    /// - `self` must be in `gc`, and `size` must be accurate
    pub unsafe fn shift_left_unchecked(self, gc: &GarbageCollector, shift_amnt: usize) {
        debug_assert!(shift_amnt < self.data.len() * limb_t::BITS as usize);

        let shift_limbs = shift_amnt / limb_t::BITS as usize;
        let shift_bits = shift_amnt % limb_t::BITS as usize;

        let data_ptr = self.data.as_mut_ptr(gc);

        unsafe {
            mpn_lshift(
                data_ptr,
                data_ptr,
                (self.data.len() - shift_limbs) as size_t,
                shift_bits as c_uint,
            );

            mpn_copyd(
                data_ptr.add(shift_limbs),
                data_ptr,
                (self.data.len() - shift_limbs) as size_t,
            );

            mpn_zero(data_ptr, shift_limbs as size_t);
        };
    }

    /// Parses a `GCUInt` from base10 digits
    ///
    /// # Safety
    /// - `num_digits` must be greater-than or equal-to `digits.count()`
    pub unsafe fn parse_from_digits(
        gc: &GarbageCollector,
        digits: impl IntoIterator<Item = Digit>,
        num_digits: usize,
    ) -> Self {
        let digits = digits.into_iter();

        // We're calculating the maximum number of limbs required to parse our number using:
        //
        // `floor(d / floor(k * log10(2))) + 1`
        //
        // where `d` is the number of digits, and `k` is `limb_t::BITS`. This is an estimate for the
        // number of limbs required that is mathematically guarenteed to be greater-than or equal-to
        // the actual amount of limbs required.
        //
        //    floor(log_(2^k)(10^d - 1)                  ) + 1 # this is the real, 100% accurate maximum
        //  = floor(log10(10^d - 1) /       log10(2^k)   ) + 1
        // <= floor(log10(10^d    ) /       log10(2^k)   ) + 1
        //  = floor(d               /       log10(2^k)   ) + 1
        //  = floor(d               /      (k * log10(2))) + 1
        //  = floor(d               /      (k / log2(10))) + 1
        // <= floor(d               / floor(k / log2(10))) + 1 # more conservative estimate that we're using
        //
        // With 64-bit limbs, this becomes:
        // `floor(d / 19) + 1`
        //
        // This is suprisingly accurate despite using only integer operations. For 1_000 digits with k=64, here are the numbers:
        // estimate: 53
        // actual required limbs: 52
        //
        // on 32-bit platforms, it's a little bit worse:
        // estimate: 112
        // actual: 104

        // The fact that we're using floating-point numbers is really janky, but it actually
        // shouldn't be a problem here.
        const ESTIMATE_DENOMINATOR: usize =
            (limb_t::BITS as f64 / std::f64::consts::LOG2_10) as usize;

        let maximum_limbs_required = num_digits / ESTIMATE_DENOMINATOR + 1;

        let len = maximum_limbs_required;

        let mut output = Self {
            data: GCBuffer::new_uninit(gc, len),
        };

        let buffer = GCBuffer::<limb_t>::new_uninit(gc, len);

        let mut out_ptr = output.data.as_mut_ptr(gc);
        let mut tmp_ptr = buffer.as_mut_ptr(gc);

        unsafe { mpn_zero(out_ptr, len as size_t) };

        for digit in digits {
            unsafe { mpn_zero(tmp_ptr, len as size_t) };
            unsafe { tmp_ptr.write(digit as limb_t) };

            unsafe { mpn_addmul_1(tmp_ptr, out_ptr, len as i64, 10) };

            std::mem::swap(&mut out_ptr, &mut tmp_ptr);
        }

        if out_ptr.cast_const() != output.data.as_ptr(gc) {
            unsafe { std::ptr::copy_nonoverlapping(out_ptr, output.data.as_mut_ptr(gc), len) };
        }

        unsafe { buffer.deallocate_from_end(gc) };
        unsafe { output.trim_leading_zero_limbs_at_end(gc) };

        output
    }

    pub unsafe fn to_naive_string(self, gc: &GarbageCollector) -> String {
        const MULTIPLIER: usize =
            (limb_t::BITS as f64 * 256f64 / std::f64::consts::LOG2_10).ceil() as usize;

        let digits_required = self.data.len() * MULTIPLIER / 256 + 1;

        let mut string = Vec::<u8>::with_capacity(digits_required + 1);

        let input = unsafe { self.without_leading_zero_limbs(gc) };

        let temp = GCBuffer::<limb_t>::new_uninit(gc, input.data.len() + 1);

        unsafe {
            mpn_copyi(
                temp.as_mut_ptr(gc),
                input.data.as_ptr(gc),
                input.data.len() as size_t,
            );

            let string_size = mpn_get_str(
                string.as_mut_ptr(),
                10,
                temp.as_mut_ptr(gc),
                input.data.len() as size_t,
            );

            string.set_len(string_size);
        }

        unsafe { temp.deallocate_from_end(gc) };

        if string.is_empty() {
            string.push(0);
        }

        // Remove leading zereos
        let real_length = string
            .iter()
            .position(|d| *d != 0)
            .map_or(1, |i| string.len() - i);

        string.drain(..(string.len() - real_length));

        for digit in string.iter_mut() {
            *digit += b'0';
            debug_assert!(matches!(*digit, b'0'..=b'9'))
        }

        unsafe { String::from_utf8_unchecked(string) }
    }

    /// Creates a number from `5^a * 2 ^ b`. This is used for parsing decimals
    pub fn from_pow10_factorization(
        gc: &GarbageCollector,
        factorization: PowerOfTenFactorization,
    ) -> Self {
        // A conservative estimate of the number of limbs required to hold the number. This has been
        // mathematically proven to always be larger than the actual number of limbs required.
        //
        // This was determined using algabraic methods similar to the ones used in [`parse_from_digits`]
        let limbs_required = (factorization.pow_2
            + (factorization.pow_5 * POW_5_MULTIPLIER).div_ceil(256))
            / (limb_t::BITS as usize)
            + 1;

        let mut output = Self {
            data: GCBuffer::<limb_t>::new_uninit(gc, limbs_required),
        };

        unsafe {
            mpn_zero(output.data.as_mut_ptr(gc), limbs_required as size_t);
            output.write_pow_5(gc, factorization.pow_5);
            output.shift_left_unchecked(gc, factorization.pow_2);
            output.trim_leading_zero_limbs_at_end(gc);
        };

        output
    }

    pub fn as_usize(self, gc: &GarbageCollector) -> Option<usize> {
        const PTR_SIZE: usize = std::mem::size_of::<usize>();

        match std::mem::size_of::<limb_t>() {
            0 => unreachable!(),
            1..PTR_SIZE => {
                const LIMBS_IN_USIZE: usize = PTR_SIZE / std::mem::size_of::<limb_t>();

                if self.data.len() > LIMBS_IN_USIZE {
                    return None;
                }

                let mut retval = 0usize;

                for limb in unsafe { self.data.as_slice(gc).iter().rev() } {
                    retval = retval.unbounded_shl(limb_t::BITS);
                    retval |= *limb as usize;
                }

                Some(retval)
            }
            PTR_SIZE => {
                let limbs = unsafe { self.data.as_slice(gc) };

                if limbs.len() > 1 {
                    return None;
                }

                Some(limbs.first().map_or(0usize, |l| *l as usize))
            }
            _ => {
                let limbs = unsafe { self.data.as_slice(gc) };

                if limbs.len() > 1 {
                    return None;
                }

                let limb = limbs.first().copied().unwrap_or(0);

                usize::try_from(limb).ok()
            }
        }
    }

    /// Writes a power of five to `self`. Requires that `self` is big enough and is set to zero.
    unsafe fn write_pow_5(self, gc: &GarbageCollector, pow_5: usize) {
        let limbs_required = (pow_5 * POW_5_MULTIPLIER).div_ceil(256) / (limb_t::BITS as usize) + 1;

        let data_ptr = self.data.as_mut_ptr(gc);

        unsafe { data_ptr.write(1) };

        // This can be simplified significantly if we use other powers of 5 in the `mpn_mul_1`
        // function, but there's not really a point in optimizing that right now.
        for _ in 0..pow_5 {
            unsafe { mpn_mul_1(data_ptr, data_ptr, limbs_required as size_t, 5) };
        }
    }

    /// Reduces the fraction `self/(2^a * 5^b)` where `a` and `b` are given by `denominator`.
    /// Requires that the current allocation is the last allocation in the `GCSpace`.
    ///
    /// This is used for parsing decimals.
    pub unsafe fn reduce_pow_10_at_end(
        &mut self,
        gc: &GarbageCollector,
        denominator: &mut PowerOfTenFactorization,
    ) {
        unsafe {
            if self.is_zero(gc) {
                denominator.pow_2 = 0;
                denominator.pow_5 = 0;
            } else {
                self.reduce_pow_2_at_end(gc, &mut denominator.pow_2);
                self.reduce_pow_5_at_end(gc, &mut denominator.pow_5);
            }
        }

        unsafe { self.trim_leading_zero_limbs_at_end(gc) };
    }

    unsafe fn count_required_digits(self, gc: &GarbageCollector) -> usize {
        unsafe { self.data.as_slice(gc) }
            .iter()
            .rposition(|limb| *limb != 0)
            .map_or(1, |p| p + 1)
    }

    pub unsafe fn trim_leading_zero_limbs_at_end(&mut self, gc: &GarbageCollector) {
        unsafe {
            let new_length = self.count_required_digits(gc);

            self.data.set_length_at_end(gc, new_length)
        };
    }

    pub unsafe fn without_leading_zero_limbs(self, gc: &GarbageCollector) -> Self {
        debug_assert_ne!(self.data.len(), 0);

        let new_length = unsafe { self.count_required_digits(gc) };

        Self {
            data: GCBuffer::from_raw_parts(self.data.gc_ptr(), new_length),
        }
    }

    pub unsafe fn trailing_zeroes(self, gc: &GarbageCollector) -> usize {
        let mut trailing_zeroes = 0;

        for limb in unsafe { self.data.as_slice(gc) } {
            let limb_zeroes = limb.trailing_zeros() as usize;

            trailing_zeroes += limb_zeroes;

            if limb_zeroes != limb_t::BITS as usize {
                break;
            }
        }

        trailing_zeroes
    }

    /// Shifts `self` to the right by `amnt` bits. Returns the new length.
    ///
    /// NOTE: This function will only write the returned number of limbs to `self`, so the caller
    /// should manually zero those limbs or set the length of `self` to the return value of this
    /// function using [`GCBuffer::set_length`] or [`GCBuffer::set_length_at_end`].
    ///
    /// # Safety
    /// - `self` must be valid
    /// - `amnt <= self.data.len() * limb_t::BITS`
    pub unsafe fn shr_unchecked(self, gc: &GarbageCollector, amnt: usize) -> usize {
        let limb_count = self.data.len();
        let data_ptr = self.data.as_mut_ptr(gc);

        let shr_limbs = amnt / limb_t::BITS as usize;
        let shr_bits = amnt % limb_t::BITS as usize;

        let limb_count = limb_count - shr_limbs;

        unsafe {
            mpn_copyi(data_ptr, data_ptr.add(shr_limbs), limb_count as size_t);

            mpn_rshift(data_ptr, data_ptr, limb_count as size_t, shr_bits as c_uint);
        }

        limb_count
    }

    /// Compares `self` with `other`.
    ///
    /// Requires there to be no leading zeroes on `self` or `other`
    pub unsafe fn cmp(self, gc: &GarbageCollector, other: Self) -> std::cmp::Ordering {
        match self.data.len().cmp(&other.data.len()) {
            std::cmp::Ordering::Equal => unsafe {
                self.data
                    .as_slice(gc)
                    .iter()
                    .rev()
                    .cmp(other.data.as_slice(gc).iter().rev())
            },
            order => order,
        }
    }

    unsafe fn reduce_pow_2_at_end(&mut self, gc: &GarbageCollector, den_pow_2: &mut usize) {
        let pow2_reduction = unsafe { self.trailing_zeroes(gc) as usize }.min(*den_pow_2);
        *den_pow_2 -= pow2_reduction;

        unsafe {
            let len = self.shr_unchecked(gc, pow2_reduction);
            self.data.set_length_at_end(gc, len);
        }
    }

    unsafe fn reduce_pow_5_at_end(&mut self, gc: &GarbageCollector, den_pow_5: &mut usize) {
        if *den_pow_5 == 0 {
            return;
        }

        let len = self.data.len();

        let buf = GCBuffer::<limb_t>::new_uninit(gc, len);

        let data_ptr = self.data.as_mut_ptr(gc);
        let buf_ptr = buf.as_mut_ptr(gc);

        let mut lhs_ptr = data_ptr;
        let mut out_ptr = buf_ptr;

        while *den_pow_5 != 0 {
            let remainder = unsafe { mpn_divmod_1(out_ptr, lhs_ptr, len as size_t, 5) };

            if remainder == 0 {
                std::mem::swap(&mut lhs_ptr, &mut out_ptr);
                *den_pow_5 -= 1;
            } else {
                break;
            }
        }

        if lhs_ptr != data_ptr {
            unsafe { std::ptr::copy_nonoverlapping(lhs_ptr, data_ptr, len) };
        }

        unsafe { buf.deallocate_from_end(gc) };
    }
}

impl From<GCBuffer<limb_t>> for GCUInt {
    fn from(value: GCBuffer<limb_t>) -> Self {
        Self { data: value }
    }
}

impl GCEq<GCUInt> for GCUInt {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &GCUInt) -> bool {
        unsafe { self.data.as_slice(gc) == rhs.data.as_slice(gc) }
    }
}

impl GCEq<usize> for GCUInt {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &usize) -> bool {
        self.as_usize(gc).is_some_and(|val| val == *rhs)
    }
}

impl GCEq<limb_t> for GCUInt {
    unsafe fn gc_eq(&self, gc: &GarbageCollector, rhs: &limb_t) -> bool {
        unsafe { self.data.as_slice(gc) == &[*rhs] }
    }
}
