//! The unsigned words the kernels compute in: an integer's magnitude, and a word twice its width
//! that holds the exact result of an operation on two magnitudes before it is narrowed back.

use crate::reciprocal;

/// `10^k` for `k` in `0..=38`: every power a `u128` holds, and so every power a scale needs.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
pub(crate) const POW10: [u128; 39] = {
    let mut table = [1_u128; 39];
    let mut k = 1;
    while k < table.len() {
        // At most 10^38, below u128::MAX.
        table[k] = table[k - 1].wrapping_mul(10);
        k += 1;
    }
    table
};

/// How a division's remainder compares with half its divisor: 0 when it is zero, 1 below half, 2
/// at half, 3 above.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "three bits sum to at most 3")]
pub(crate) const fn class<D: [const] Word>(remainder: D, divisor: D) -> u32 {
    let rest = divisor.wrapping_sub(remainder);
    u32::from(remainder != D::ZERO) + u32::from(remainder >= rest) + u32::from(remainder > rest)
}

/// `10^k` as a `u128`, or `None` past `10^38`.
#[inline]
#[expect(clippy::indexing_slicing, reason = "checked against the table's length first")]
pub(crate) const fn pow10_u128(k: u8) -> Option<u128> {
    let k = usize::from(k);
    if k < POW10.len() { Some(POW10[k]) } else { None }
}

/// An unsigned word: a magnitude, or the wider word an exact result is held in.
pub(crate) const trait Word: Copy + [const] Ord {
    /// Zero.
    const ZERO: Self;
    /// One.
    const ONE: Self;
    /// Whether the word is the widest, past which nothing is computed.
    const WIDEST: bool = false;

    /// `10^k`, or `None` past the word.
    fn pow10(k: u8) -> Option<Self>;
    /// The sum, or `None` past the word.
    fn checked_add(self, other: Self) -> Option<Self>;
    /// The product, or `None` past the word.
    fn checked_mul(self, other: Self) -> Option<Self>;
    /// The sum, modulo the word.
    fn wrapping_add(self, other: Self) -> Self;
    /// The difference, modulo the word.
    fn wrapping_sub(self, other: Self) -> Self;
    /// The product, modulo the word.
    fn wrapping_mul(self, other: Self) -> Self;
    /// The quotient and the remainder, for a divisor that is not zero.
    fn div_rem(self, divisor: Self) -> (Self, Self);

    /// The quotient and the [`class`] of its remainder, for a divisor that is not zero. A word
    /// wider than a primitive's classes the remainder in the narrowest word that holds it.
    #[inline]
    fn divide(self, divisor: Self) -> (Self, u32) {
        let (quotient, remainder) = self.div_rem(divisor);
        (quotient, class(remainder, divisor))
    }

    /// `self / 10^k` and the [`class`] of its remainder, or `None` when `10^k` is past the
    /// word, or a word below the widest leaves it to the double. A word whose compiler has no
    /// cheap division by a constant divides by the power's reciprocal, and classes the remainder in
    /// the narrowest word that holds it.
    #[inline]
    fn divide_pow10(self, k: u8) -> Option<(Self, u32)> {
        match Self::pow10(k) {
            Some(power) => {
                let (quotient, remainder) = self.div_rem(power);
                Some((quotient, class(remainder, power)))
            },
            None => None,
        }
    }
    /// Whether the value is odd.
    fn is_odd(self) -> bool;
}

/// A primitive word, which every magnitude is: it converts to and from a `u128`.
pub(crate) const trait Narrow: [const] Word {
    /// The value as a `u128`.
    fn to_u128(self) -> u128;
    /// The low bits of a `u128`.
    fn truncate(wide: u128) -> Self;
}

/// The word twice as wide as `U`, which holds the product of any two `U`s.
pub(crate) const trait Double<U: [const] Narrow>: [const] Word {
    /// A narrow value, widened.
    fn from_narrow(narrow: U) -> Self;
    /// The value, or `None` past the narrow word.
    fn narrow(self) -> Option<U>;
    /// The low bits: what wrapping keeps.
    fn wrapping_narrow(self) -> U;
    /// The exact product of two narrow values.
    fn widening_mul(a: U, b: U) -> Self;
}

/// The words for each primitive, forwarded to its own methods, with the items of a block a word is
/// given.
macro_rules! word {
    ($($t:ty $({ $($more:item)* })?),*) => {$(
        const impl Word for $t {
            $($($more)*)?

            const ZERO: Self = 0;
            const ONE: Self = 1;

            #[inline]
            fn pow10(k: u8) -> Option<Self> {
                match pow10_u128(k) {
                    Some(power) => match <$t>::try_from(power) {
                        Ok(power) => Some(power),
                        Err(_out_of_range) => None,
                    },
                    None => None,
                }
            }

            #[inline]
            fn checked_add(self, other: Self) -> Option<Self> {
                <$t>::checked_add(self, other)
            }

            #[inline]
            fn checked_mul(self, other: Self) -> Option<Self> {
                <$t>::checked_mul(self, other)
            }

            #[inline]
            fn wrapping_add(self, other: Self) -> Self {
                <$t>::wrapping_add(self, other)
            }

            #[inline]
            fn wrapping_sub(self, other: Self) -> Self {
                <$t>::wrapping_sub(self, other)
            }

            #[inline]
            fn wrapping_mul(self, other: Self) -> Self {
                <$t>::wrapping_mul(self, other)
            }

            #[inline]
            #[expect(clippy::arithmetic_side_effects, reason = "the divisor is not zero")]
            fn div_rem(self, divisor: Self) -> (Self, Self) {
                (self / divisor, self % divisor)
            }

            #[inline]
            fn is_odd(self) -> bool {
                self & 1 == 1
            }
        }

        const impl Narrow for $t {
            #[inline]
            fn to_u128(self) -> u128 {
                u128::from(self)
            }

            #[inline]
            fn truncate(wide: u128) -> Self {
                // The masked value always fits: the conversion is the truncation, spelled exactly.
                match <$t>::try_from(wide & u128::from(<$t>::MAX)) {
                    Ok(low) => low,
                    Err(_out_of_range) => 0,
                }
            }
        }
    )*};
}

word!(u8, u16, u32, u64, u128 {
    // A quotient that fits a word, by a divisor that fits one, is a 128-by-64-bit division, whose
    // remainder fits a word too.
    #[inline]
    fn divide(self, divisor: Self) -> (Self, u32) {
        if let Ok(word) = u64::try_from(divisor)
            && let Some((quotient, remainder)) = reciprocal::divide_u128_by_u64(self, word)
        {
            return (u128::from(quotient), class(remainder, word));
        }
        divide_past_a_word(self, divisor)
    }

    // LLVM divides a `u128` by a constant through a multiply-high of four multiplies, and by a
    // power known only at run time through a library call. A value with no high word divides as a
    // `u64`; any other, by one Möller–Granlund step when its quotient fits a word, and by two when
    // it does not, inline: a call on the path, even one never taken, makes every caller save its
    // registers. A power past `10^19` is left to the double.
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        reason = "measured, as `kernel::quotient_pow10`: out of line it loses the constant `k`"
    )]
    fn divide_pow10(self, k: u8) -> Option<(Self, u32)> {
        let Some(power) = pow10_u128(k) else { return None };
        if let (Ok(value), Ok(divisor)) = (u64::try_from(self), u64::try_from(power)) {
            let (quotient, remainder) = value.div_rem(divisor);
            return Some((u128::from(quotient), class(remainder, divisor)));
        }
        if let Some((quotient, remainder, divisor)) = reciprocal::divide_u128(self, k) {
            return Some((u128::from(quotient), class(remainder, divisor)));
        }
        match reciprocal::divide_u256(0, self, k) {
            Some((quotient, remainder, divisor)) => Some((quotient, class(remainder, divisor))),
            None => None,
        }
    }
});

/// `value / divisor` and the class of its remainder, for a quotient or a divisor past a word: off
/// the hot path and out of line, so that LLVM does not divide once for both paths and class the
/// remainder in both words.
#[cold]
#[inline(never)]
const fn divide_past_a_word(value: u128, divisor: u128) -> (u128, u32) {
    let (quotient, remainder) = value.div_rem(divisor);
    (quotient, class(remainder, divisor))
}

/// Each primitive's double: the next primitive, for every magnitude but a `u128`'s.
macro_rules! double {
    ($($narrow:ty => $wide:ty),*) => {$(
        const impl Double<$narrow> for $wide {
            #[inline]
            fn from_narrow(narrow: $narrow) -> Self {
                <$wide>::from(narrow)
            }

            #[inline]
            fn narrow(self) -> Option<$narrow> {
                match <$narrow>::try_from(self) {
                    Ok(narrow) => Some(narrow),
                    Err(_out_of_range) => None,
                }
            }

            #[inline]
            fn wrapping_narrow(self) -> $narrow {
                // The masked value always fits: the conversion is the truncation, spelled exactly.
                match <$narrow>::try_from(self & <$wide>::from(<$narrow>::MAX)) {
                    Ok(low) => low,
                    Err(_out_of_range) => 0,
                }
            }

            #[inline]
            fn widening_mul(a: $narrow, b: $narrow) -> Self {
                // The product of two halves fits the whole: wrapping never wraps.
                <$wide>::from(a).wrapping_mul(<$wide>::from(b))
            }
        }
    )*};
}

double!(u8 => u16, u16 => u32, u32 => u64, u64 => u128);

/// A 256-bit unsigned word: the double of a `u128`, and the word an exact result falls back to
/// when it outgrows a narrower double.
#[derive(Clone, Copy, Debug)]
#[derive_const(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct U256 {
    /// The high 128 bits; declared first, so the derived order is the numeric one.
    high: u128,
    /// The low 128 bits.
    low: u128,
}

impl U256 {
    /// The value of a `u128`.
    #[inline]
    pub(crate) const fn from_u128(low: u128) -> Self {
        Self { high: 0, low }
    }

    /// `(high:low) / divisor` and the remainder, for `high < divisor`, so the quotient fits 128
    /// bits: two 128-by-64-bit steps of long division (Hacker's Delight, 2nd ed., figure 9-3).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "every step is bounded as Hacker's Delight shows: each partial quotient is checked \
                  against the half base before it is multiplied"
    )]
    const fn div_rem_narrow(high: u128, low: u128, divisor: u128) -> (u128, u128) {
        const HALF: u128 = 1 << 64;
        let shift = divisor.leading_zeros();
        let divisor = divisor << shift;
        let (divisor_high, divisor_low) = (divisor >> 64, divisor & (HALF - 1));
        let top = if shift == 0 { high } else { (high << shift) | (low >> (128 - shift)) };
        let rest = low << shift;
        let (next_high, next_low) = (rest >> 64, rest & (HALF - 1));
        let (q1, partial) = Self::digit(top, next_high, divisor_high, divisor_low, divisor);
        let (q0, remainder) = Self::digit(partial, next_low, divisor_high, divisor_low, divisor);
        ((q1 << 64) | q0, remainder >> shift)
    }

    /// One digit of the long division: `(top:next) / (divisor_high:divisor_low)`, in base `2^64`,
    /// and the remainder.
    #[expect(clippy::arithmetic_side_effects, reason = "bounded as `div_rem_narrow` says")]
    const fn digit(
        top: u128, next: u128, divisor_high: u128, divisor_low: u128, divisor: u128,
    ) -> (u128, u128) {
        const HALF: u128 = 1 << 64;
        let mut quotient = top / divisor_high;
        let mut estimate = top - quotient * divisor_high;
        while quotient >= HALF || quotient * divisor_low > ((estimate << 64) | next) {
            quotient -= 1;
            estimate += divisor_high;
            if estimate >= HALF {
                break;
            }
        }
        let remainder = ((top << 64) | next).wrapping_sub(quotient.wrapping_mul(divisor));
        (quotient, remainder)
    }

    /// Its high and its low 128 bits.
    #[cfg(test)]
    pub(crate) const fn halves(self) -> (u128, u128) {
        (self.high, self.low)
    }

    /// The product of two `u128`s.
    #[inline]
    pub(crate) const fn widening(a: u128, b: u128) -> Self {
        let (low, high) = a.carrying_mul(b, 0);
        Self { high, low }
    }

    /// The value as a `u128`, or `None` past one.
    #[inline]
    pub(crate) const fn to_u128(self) -> Option<u128> {
        if self.high == 0 { Some(self.low) } else { None }
    }

    /// How many leading bits are zero.
    #[inline]
    const fn leading_zeros(self) -> u32 {
        if self.high == 0 {
            128_u32.wrapping_add(self.low.leading_zeros())
        } else {
            self.high.leading_zeros()
        }
    }

    /// The value shifted `shift` bits left, or `None` when a set bit would leave the word.
    pub(crate) const fn checked_shl(self, shift: u32) -> Option<Self> {
        if self.high == 0 && self.low == 0 {
            return Some(self);
        }
        if shift >= self.leading_zeros().wrapping_add(1) {
            return None;
        }
        Some(if shift >= 128 {
            Self { high: self.low << shift.wrapping_sub(128), low: 0 }
        } else if shift == 0 {
            self
        } else {
            Self {
                high: (self.high << shift) | (self.low >> 128_u32.wrapping_sub(shift)),
                low: self.low << shift,
            }
        })
    }

    /// The value shifted `shift` bits right, and how the bits shifted out compare with half of
    /// `2^shift`: 0 when they are zero, 1 below half, 2 at half, 3 above.
    pub(crate) const fn shr_classed(self, shift: u32) -> (Self, u32) {
        if shift == 0 {
            return (self, 0);
        }
        if shift > 256 {
            // 2^(shift - 1) is past every value: all of it shifts out, below half.
            return (Self::ZERO, u32::from(self.high != 0 || self.low != 0));
        }
        let half = Self::ONE.checked_shl(shift.wrapping_sub(1));
        let (quotient, rest) = if shift >= 256 {
            (Self::ZERO, self)
        } else if shift >= 128 {
            let low_bits = shift.wrapping_sub(128);
            let mask = if low_bits == 0 { 0 } else { u128::MAX >> 128_u32.wrapping_sub(low_bits) };
            (
                Self { high: 0, low: self.high >> low_bits },
                Self { high: self.high & mask, low: self.low },
            )
        } else {
            let mask = u128::MAX >> 128_u32.wrapping_sub(shift);
            (
                Self {
                    high: self.high >> shift,
                    low: (self.low >> shift) | (self.high << 128_u32.wrapping_sub(shift)),
                },
                Self { high: 0, low: self.low & mask },
            )
        };
        let class = match half {
            Some(half) => u32::from(rest != Self::ZERO)
                .wrapping_add(u32::from(rest >= half))
                .wrapping_add(u32::from(rest > half)),
            None => u32::from(rest != Self::ZERO),
        };
        (quotient, class)
    }

    /// The value shifted one bit left, and the bit shifted out.
    #[inline]
    const fn shifted_left(self) -> (Self, bool) {
        let carry = self.high >> 127 == 1;
        (Self { high: (self.high << 1) | (self.low >> 127), low: self.low << 1 }, carry)
    }

    /// Bit `i` of the value.
    #[inline]
    const fn bit(self, i: u32) -> bool {
        if i >= 128 {
            (self.high >> i.wrapping_sub(128)) & 1 == 1
        } else {
            (self.low >> i) & 1 == 1
        }
    }

    /// The value with bit `i` set.
    #[inline]
    const fn with_bit(self, i: u32) -> Self {
        if i >= 128 {
            Self { high: self.high | (1 << i.wrapping_sub(128)), low: self.low }
        } else {
            Self { high: self.high, low: self.low | (1 << i) }
        }
    }
}

const impl Word for U256 {
    const ZERO: Self = Self::from_u128(0);
    const ONE: Self = Self::from_u128(1);
    const WIDEST: bool = true;

    /// Every power to `10^77`, the last below `2^256`: a product of two scales' decimals needs 76.
    fn pow10(k: u8) -> Option<Self> {
        // 10^k as table powers of at most 10^38 multiplied together.
        let mut power = Self::ONE;
        let mut left = k;
        while left > 0 {
            let step = if left > 38 { 38 } else { left };
            let Some(factor) = pow10_u128(step) else { return None };
            power = match power.checked_mul(Self::from_u128(factor)) {
                Some(power) => power,
                None => return None,
            };
            left = left.wrapping_sub(step);
        }
        Some(power)
    }

    #[inline]
    fn checked_add(self, other: Self) -> Option<Self> {
        let (low, carry) = self.low.overflowing_add(other.low);
        let Some(high) = self.high.checked_add(other.high) else { return None };
        match high.checked_add(u128::from(carry)) {
            Some(high) => Some(Self { high, low }),
            None => None,
        }
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        if self.high != 0 && other.high != 0 {
            return None;
        }
        // At most one high half is non-zero, so the cross term is one 128-by-128 product.
        let (low, carry) = self.low.carrying_mul(other.low, 0);
        let (cross, cross_high) = if self.high == 0 {
            other.high.carrying_mul(self.low, 0)
        } else {
            self.high.carrying_mul(other.low, 0)
        };
        if cross_high != 0 {
            return None;
        }
        match carry.checked_add(cross) {
            Some(high) => Some(Self { high, low }),
            None => None,
        }
    }

    #[inline]
    fn wrapping_add(self, other: Self) -> Self {
        let (low, carry) = self.low.overflowing_add(other.low);
        Self { high: self.high.wrapping_add(other.high).wrapping_add(u128::from(carry)), low }
    }

    #[inline]
    fn wrapping_mul(self, other: Self) -> Self {
        // The low 256 bits: the low halves' whole product, and the cross terms' low halves above
        // it.
        let (low, carry) = self.low.carrying_mul(other.low, 0);
        let cross =
            self.high.wrapping_mul(other.low).wrapping_add(self.low.wrapping_mul(other.high));
        Self { high: carry.wrapping_add(cross), low }
    }

    #[inline]
    fn wrapping_sub(self, other: Self) -> Self {
        let (low, borrow) = self.low.overflowing_sub(other.low);
        Self { high: self.high.wrapping_sub(other.high).wrapping_sub(u128::from(borrow)), low }
    }

    #[expect(clippy::arithmetic_side_effects, reason = "the divisor is not zero")]
    fn div_rem(self, divisor: Self) -> (Self, Self) {
        if self < divisor {
            return (Self::ZERO, self);
        }
        if divisor.high == 0 {
            let d = divisor.low;
            if let Some((low, remainder)) = reciprocal::divide_u256_by_u128(self.high, self.low, d)
            {
                return (Self::from_u128(low), Self::from_u128(remainder));
            }
            let (high, rest) = (self.high / d, self.high % d);
            let (low, remainder) = Self::div_rem_narrow(rest, self.low, d);
            return (Self { high, low }, Self::from_u128(remainder));
        }
        // A divisor past 128 bits and below the dividend: long division a bit at a time. Only a
        // remainder lined up at a finer scale divides so, and only off the hot path.
        let (mut quotient, mut remainder) = (Self::ZERO, Self::ZERO);
        let mut i = 256_u32;
        while i > 0 {
            i = i.wrapping_sub(1);
            let (shifted, carry) = remainder.shifted_left();
            remainder = if self.bit(i) { shifted.wrapping_add(Self::ONE) } else { shifted };
            if carry || remainder >= divisor {
                remainder = remainder.wrapping_sub(divisor);
                quotient = quotient.with_bit(i);
            }
        }
        (quotient, remainder)
    }

    // The remainder by a divisor that fits 128 bits fits them too.
    #[inline]
    fn divide(self, divisor: Self) -> (Self, u32) {
        if divisor.high == 0
            && let Some((quotient, remainder)) =
                reciprocal::divide_u256_by_u128(self.high, self.low, divisor.low)
        {
            return (Self::from_u128(quotient), class(remainder, divisor.low));
        }
        let (quotient, remainder) = self.div_rem(divisor);
        (quotient, class(remainder, divisor))
    }

    #[inline]
    fn is_odd(self) -> bool {
        self.low & 1 == 1
    }

    // Two Möller–Granlund steps for every quotient that fits 128 bits, where the long division
    // calls the library three times.
    #[inline]
    fn divide_pow10(self, k: u8) -> Option<(Self, u32)> {
        match reciprocal::divide_u256(self.high, self.low, k) {
            Some((quotient, remainder, divisor)) => {
                Some((Self::from_u128(quotient), class(remainder, divisor)))
            },
            None => match Self::pow10(k) {
                Some(power) => Some(divide_pow10_by_long_division(self, power)),
                None => None,
            },
        }
    }
}

/// `value / power` and the class of its remainder, for a power past `10^19` or a quotient past 128
/// bits: by the run-time reciprocal for a power of 128 bits or fewer, and by long division past
/// them, off the hot path and out of line.
#[cold]
#[inline(never)]
const fn divide_pow10_by_long_division(value: U256, power: U256) -> (U256, u32) {
    let (quotient, remainder) = value.div_rem(power);
    (quotient, class(remainder, power))
}

const impl<U: [const] Narrow> Double<U> for U256 {
    #[inline]
    fn from_narrow(narrow: U) -> Self {
        Self::from_u128(narrow.to_u128())
    }

    #[inline]
    fn narrow(self) -> Option<U> {
        if self.high != 0 {
            return None;
        }
        let narrow = U::truncate(self.low);
        if narrow.to_u128() == self.low { Some(narrow) } else { None }
    }

    #[inline]
    fn wrapping_narrow(self) -> U {
        U::truncate(self.low)
    }

    #[inline]
    fn widening_mul(a: U, b: U) -> Self {
        let (low, high) = a.to_u128().carrying_mul(b.to_u128(), 0);
        Self { high, low }
    }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference model's own arithmetic")]
mod tests {
    use proptest::prelude::*;

    use super::{Double, U256, Word};

    /// A 256-bit word from its two halves.
    const fn wide(high: u128, low: u128) -> U256 {
        U256 { high, low }
    }

    proptest! {
        #[test]
        fn a_narrow_division_agrees_with_the_definition(high: u64, low: u128, divisor in 1_u128..) {
            let high = u128::from(high) % divisor;
            let (quotient, remainder) = U256::div_rem_narrow(high, low, divisor);
            let back = U256::widening_mul(quotient, divisor).checked_add(<U256 as Double<u128>>::from_narrow(remainder));
            prop_assert_eq!(back, Some(wide(high, low)), "q·d + r is the dividend");
            prop_assert!(remainder < divisor, "the remainder is below the divisor");
        }

        #[test]
        fn a_full_division_agrees_with_the_definition(high: u128, low: u128, divisor in 1_u128..) {
            let (quotient, remainder) = wide(high, low).div_rem(wide(0, divisor));
            let back = quotient.checked_mul(wide(0, divisor)).and_then(|product| product.checked_add(remainder));
            prop_assert_eq!(back, Some(wide(high, low)));
            prop_assert!(remainder < wide(0, divisor));
        }

        #[test]
        fn a_division_by_a_wide_divisor_agrees_with_the_definition(high: u128, low: u128, divisor_high in 1_u128.., divisor_low: u128) {
            let divisor = wide(divisor_high, divisor_low);
            let (quotient, remainder) = wide(high, low).div_rem(divisor);
            let back = quotient.checked_mul(divisor).and_then(|product| product.checked_add(remainder));
            prop_assert_eq!(back, Some(wide(high, low)));
            prop_assert!(remainder < divisor);
        }

        #[test]
        fn a_product_is_exact_or_refused(a: u128, b: u128, c: u128) {
            let product = wide(0, a).checked_mul(wide(0, b));
            prop_assert_eq!(product, Some(U256::widening_mul(a, b)));
            let past = wide(1, a).checked_mul(wide(1, c));
            prop_assert_eq!(past, None, "two high halves are past the word");
        }

        #[test]
        fn a_small_word_narrows_back(a: u64, b: u64) {
            let product: u128 = Double::<u64>::widening_mul(a, b);
            prop_assert_eq!(Double::<u64>::narrow(product), u64::try_from(product).ok());
            let through: U256 = Double::<u64>::widening_mul(a, b);
            prop_assert_eq!(Double::<u64>::narrow(through), u64::try_from(product).ok());
            prop_assert_eq!(Double::<u64>::wrapping_narrow(through), Double::<u64>::wrapping_narrow(product));
        }
    }

    #[test]
    fn the_powers_of_ten_stop_where_each_word_does() {
        assert_eq!(<u8 as Word>::pow10(2), Some(100), "a u8 holds 10^2");
        assert_eq!(<u8 as Word>::pow10(3), None, "but not 10^3");
        assert_eq!(<u64 as Word>::pow10(19), Some(10_u64.pow(19)), "a u64 holds 10^19");
        assert_eq!(<u128 as Word>::pow10(38), Some(10_u128.pow(38)), "a u128 holds 10^38");
        assert_eq!(
            <U256 as Word>::pow10(77),
            U256::from_u128(10_u128.pow(38))
                .checked_mul(U256::from_u128(10_u128.pow(38)))
                .and_then(|power| power.checked_mul(U256::from_u128(10))),
            "a U256 holds 10^77"
        );
        assert_eq!(<U256 as Word>::pow10(78), None, "but not 10^78");
    }
}
