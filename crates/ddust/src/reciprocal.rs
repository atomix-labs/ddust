//! Division through a reciprocal: by a power of ten, by a power of five, and by a divisor known
//! only at run time.
//!
//! A two-word numerator over a one-word power of ten is one Möller–Granlund step: two multiplies
//! and a correction, where `u128 / u128` is a library call on aarch64 and a long division in
//! software. Each power from `10^0` to `10^19` has its reciprocal in [`RECIPROCALS`], and each
//! power of five a double's conversion divides by in [`FIVES`], which the compiler computes from
//! their `const fn`s; nothing about a constant is computed at run time.
//!
//! The step's precondition, that the numerator's high word is below the divisor, is the test that
//! the quotient fits one word: a quotient past it is past the integer the result goes into, and
//! takes the plain division, off the hot path.

use crate::word::{Narrow, POW5, Word as _, pow10_u128};

/// A one-word divisor, shifted until its top bit is set, with its reciprocal.
#[derive(Clone, Copy, Debug)]
struct Reciprocal {
    /// The divisor, shifted left until its top bit is set.
    divisor: u64,
    /// `⌊(2^128 − 1) / divisor⌋ − 2^64`, where `divisor` is the shifted one.
    inverse: u64,
    /// How far the divisor was shifted.
    shift: u32,
}

impl Reciprocal {
    /// The reciprocal of `divisor`, which is not zero.
    const fn new(divisor: u64) -> Self {
        let shift = divisor.leading_zeros();
        let divisor = divisor << shift;
        // A divisor whose top bit is set puts (2^128 − 1) / divisor in [2^64, 2^65).
        let quotient = match u128::MAX.checked_div(u128::from(divisor)) {
            Some(quotient) => quotient,
            None => 0,
        };
        Self { divisor, inverse: u64::low_bits(quotient), shift }
    }
}

/// The reciprocal of `10^k` for `k` in `0..=19`, every power one word holds.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const RECIPROCALS: [Reciprocal; 20] = {
    let mut table = [Reciprocal { divisor: 0, inverse: 0, shift: 0 }; 20];
    let mut k = 0;
    let mut power = 1_u64;
    while k < table.len() {
        table[k] = Reciprocal::new(power);
        // At most 10^20 is formed after the last entry, past which nothing reads it.
        power = power.wrapping_mul(10);
        k += 1;
    }
    table
};

/// The high word of `value`.
const fn high(value: u128) -> u64 {
    u64::low_bits(value >> 64)
}

/// `(u1:u0) / divisor` and the remainder, for a `divisor` whose top bit is set and `u1 < divisor`,
/// by its `inverse` (Möller and Granlund, "Improved division by invariant integers", 2011,
/// algorithm 4).
#[inline]
const fn divide_two_by_one(u1: u64, u0: u64, divisor: u64, inverse: u64) -> (u64, u64) {
    let estimate = u128::from(inverse)
        .wrapping_mul(u128::from(u1))
        .wrapping_add((u128::from(u1) << 64) | u128::from(u0));
    let (q1, q0) = (high(estimate).wrapping_add(1), u64::low_bits(estimate));
    let remainder = u0.wrapping_sub(q1.wrapping_mul(divisor));
    // Taken about half the time, so a select rather than a branch.
    let over = remainder > q0;
    let q1 = q1.wrapping_sub(u64::from(over));
    let remainder = remainder.wrapping_add(if over { divisor } else { 0 });
    // Rare: at most once in a few billion.
    if remainder >= divisor {
        (q1.wrapping_add(1), remainder.wrapping_sub(divisor))
    } else {
        (q1, remainder)
    }
}

/// `numerator / 10^k`, the remainder and `10^k`, or `None` unless `10^k` is below 2^32, `k` at
/// most 9, and the quotient fits one word: two 64-by-32-bit divisions, each of 32 bits of the low
/// word after the remainder before it, which LLVM performs for a constant power by one
/// multiply-high each and no correction (nexus-decimal's split).
#[inline]
pub(crate) const fn divide_u128_by_short_power(numerator: u128, k: u8) -> Option<(u64, u64, u64)> {
    let Some(power) = pow10_u128(k) else { return None };
    // Below 2^32, so every step's remainder, shifted 32 bits up, still fits a word.
    if power >= 1 << 32 {
        return None;
    }
    let divisor = u64::low_bits(power);
    let (n1, n0) = (high(numerator), u64::low_bits(numerator));
    // The quotient fits a word exactly when the high word is below the divisor; then each step's
    // numerator, a remainder below 2^32 and 32 bits, is below divisor·2^32, and its quotient below
    // 2^32.
    if n1 >= divisor {
        return None;
    }
    let upper = (n1 << 32) | (n0 >> 32);
    let (q1, r1) = upper.div_rem(divisor);
    let lower = (r1 << 32) | (n0 & 0xFFFF_FFFF);
    let (q0, r0) = lower.div_rem(divisor);
    Some(((q1 << 32) | q0, r0, divisor))
}

/// `numerator / 10^k`, the remainder and `10^k`, or `None` unless `k` is at most 19 and the
/// quotient fits one word.
#[inline]
pub(crate) const fn divide_u128(numerator: u128, k: u8) -> Option<(u64, u64, u64)> {
    let Some(Reciprocal { divisor, inverse, shift }) = reciprocal(k) else { return None };
    let (n1, n0) = (high(numerator), u64::low_bits(numerator));
    // The shifted divisor is below 2^64, and n1 below it before the shift exactly when after it.
    if n1 >= divisor >> shift {
        return None;
    }
    let (u1, u0) = shifted(n1, n0, shift);
    let (quotient, remainder) = divide_two_by_one(u1, u0, divisor, inverse);
    Some((quotient, remainder >> shift, divisor >> shift))
}

/// `(high:low) / 10^k` for a 256-bit numerator, the remainder and `10^k`, or `None` unless `k` is
/// at most 19 and the quotient fits 128 bits: two steps, one word of the quotient each.
#[inline]
pub(crate) const fn divide_u256(
    high_half: u128, low_half: u128, k: u8,
) -> Option<(u128, u64, u64)> {
    let Some(Reciprocal { divisor, inverse, shift }) = reciprocal(k) else { return None };
    // The quotient fits 128 bits exactly when the high half is below the power, which then fits
    // one word.
    if high_half >= u128::from(divisor >> shift) {
        return None;
    }
    let (n2, n1, n0) = (u64::low_bits(high_half), high(low_half), u64::low_bits(low_half));
    // The three live words, shifted as one number: each takes the bits the next one shifts out.
    let (u2, _) = shifted(n2, n1, shift);
    let (u1, u0) = shifted(n1, n0, shift);
    let (q1, partial) = divide_two_by_one(u2, u1, divisor, inverse);
    let (q0, remainder) = divide_two_by_one(partial, u0, divisor, inverse);
    Some(((u128::from(q1) << 64) | u128::from(q0), remainder >> shift, divisor >> shift))
}

/// `(n1:n0) << shift`, its two words, for a `shift` below 64.
#[inline]
const fn shifted(n1: u64, n0: u64, shift: u32) -> (u64, u64) {
    ((n1 << shift) | n0.unbounded_shr(64_u32.wrapping_sub(shift)), n0 << shift)
}

/// The reciprocal of `10^k`, or `None` past one word.
#[inline]
#[expect(clippy::indexing_slicing, reason = "checked against the table's length first")]
const fn reciprocal(k: u8) -> Option<Reciprocal> {
    let k = usize::from(k);
    if k < RECIPROCALS.len() { Some(RECIPROCALS[k]) } else { None }
}

// Division by a value known only at run time: its reciprocal computed with no division, for a
// divisor used twice, and `u128`'s division for a word used once.

/// `⌊(2^19 − 3 · 2^8) / d9⌋` for each value `d9` of a normalized divisor's top nine bits, `256` to
/// `511`: the first estimate of its reciprocal (Möller and Granlund, algorithm 2).
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const FIRST_ESTIMATE: [u16; 256] = {
    let mut table = [0_u16; 256];
    let mut index = 0;
    while index < table.len() {
        // At most 523,520 / 256 = 2,045, which a `u16` holds.
        let estimate = 523_520 / (256 + index);
        table[index] = match u16::try_from(estimate) {
            Ok(estimate) => estimate,
            Err(_out_of_range) => 0,
        };
        index += 1;
    }
    table
};

/// `⌊(2^128 − 1) / divisor⌋ − 2^64` for a `divisor` whose top bit is set, with no division: nine
/// bits from [`FIRST_ESTIMATE`], then Newton's steps to 64 (Möller and Granlund, algorithm 2).
#[inline]
#[expect(
    clippy::indexing_slicing,
    reason = "the top nine bits of a normalized word are 256 to 511"
)]
const fn reciprocal_word(divisor: u64) -> u64 {
    let d0 = divisor & 1;
    let d9 = divisor >> 55;
    let d40 = (divisor >> 24).wrapping_add(1);
    let d63 = (divisor >> 1).wrapping_add(d0);
    let v0 = u64::from(FIRST_ESTIMATE[usize_from(d9.wrapping_sub(256) & 0xFF)]);
    let v1 = (v0 << 11).wrapping_sub(v0.wrapping_mul(v0).wrapping_mul(d40) >> 40).wrapping_sub(1);
    let correction =
        u128::from(v1).wrapping_mul(u128::from((1_u64 << 60).wrapping_sub(v1.wrapping_mul(d40))));
    let v2 = (v1 << 13).wrapping_add(u64::low_bits(correction >> 47));
    let e = u64::low_bits(
        u128::from((v2 >> 1) & 0_u64.wrapping_sub(d0))
            .wrapping_sub(u128::from(v2).wrapping_mul(u128::from(d63))),
    );
    let v3 =
        (v2 << 31).wrapping_add(u64::low_bits(u128::from(v2).wrapping_mul(u128::from(e)) >> 65));
    let t = u128::from(v3).wrapping_mul(u128::from(divisor)).wrapping_add(u128::from(divisor));
    v3.wrapping_sub(high(t).wrapping_add(divisor))
}

/// `value` as an index.
const fn usize_from(value: u64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_out_of_range) => 0,
    }
}

impl Reciprocal {
    /// The reciprocal of `divisor`, which is not zero, computed at run time with no division.
    #[inline]
    const fn of(divisor: u64) -> Self {
        let shift = divisor.leading_zeros();
        let divisor = divisor << shift;
        Self { divisor, inverse: reciprocal_word(divisor), shift }
    }

    /// `(n1:n0) / divisor` and the remainder, for `n1` below the divisor.
    #[inline]
    const fn divide(self, n1: u64, n0: u64) -> (u64, u64) {
        let (u1, u0) = shifted(n1, n0, self.shift);
        let (quotient, remainder) = divide_two_by_one(u1, u0, self.divisor, self.inverse);
        (quotient, remainder >> self.shift)
    }
}

/// The reciprocal of a two-word divisor `(d1:d0)` whose top bit is set, for
/// [`divide_three_by_two`] (Möller and Granlund, algorithm 6).
#[inline]
const fn reciprocal_two_words(d1: u64, d0: u64) -> u64 {
    let mut inverse = reciprocal_word(d1);
    let mut p = d1.wrapping_mul(inverse).wrapping_add(d0);
    if p < d0 {
        inverse = inverse.wrapping_sub(1);
        if p >= d1 {
            inverse = inverse.wrapping_sub(1);
            p = p.wrapping_sub(d1);
        }
        p = p.wrapping_sub(d1);
    }
    let t = u128::from(inverse).wrapping_mul(u128::from(d0));
    p = p.wrapping_add(high(t));
    if p < high(t) {
        inverse = inverse.wrapping_sub(1);
        if ((u128::from(p) << 64) | u128::from(u64::low_bits(t)))
            >= ((u128::from(d1) << 64) | u128::from(d0))
        {
            inverse = inverse.wrapping_sub(1);
        }
    }
    inverse
}

/// `(u2:u1:u0) / (d1:d0)`, a one-word quotient and a two-word remainder, for a divisor whose top
/// bit is set and `(u2:u1)` below it (Möller and Granlund, algorithm 5).
#[inline]
const fn divide_three_by_two(
    u2: u64, u1: u64, u0: u64, d1: u64, d0: u64, inverse: u64,
) -> (u64, u128) {
    let estimate = u128::from(inverse)
        .wrapping_mul(u128::from(u2))
        .wrapping_add((u128::from(u2) << 64) | u128::from(u1));
    let (q1, q0) = (high(estimate), u64::low_bits(estimate));
    let r1 = u1.wrapping_sub(q1.wrapping_mul(d1));
    let divisor = (u128::from(d1) << 64) | u128::from(d0);
    let remainder = ((u128::from(r1) << 64) | u128::from(u0))
        .wrapping_sub(u128::from(d0).wrapping_mul(u128::from(q1)))
        .wrapping_sub(divisor);
    let q1 = q1.wrapping_add(1);
    // Taken about half the time, so a select rather than a branch.
    let over = high(remainder) >= q0;
    let q1 = q1.wrapping_sub(u64::from(over));
    let remainder = remainder.wrapping_add(if over { divisor } else { 0 });
    // Rare.
    if remainder >= divisor {
        (q1.wrapping_add(1), remainder.wrapping_sub(divisor))
    } else {
        (q1, remainder)
    }
}

/// `(high:low) / divisor` and the remainder, or `None` unless `high` is below the divisor, so the
/// quotient fits 128 bits: for a one-word divisor, two 2-by-1 steps on its reciprocal; for a
/// wider one, Knuth's algorithm D in two 3-by-2 steps.
#[inline]
pub(crate) const fn divide_u256_by_u128(
    high_half: u128, low_half: u128, divisor: u128,
) -> Option<(u128, u128)> {
    if high_half >= divisor {
        return None;
    }
    if let Ok(word) = u64::try_from(divisor) {
        // high_half < divisor < 2^64: three live words.
        let reciprocal = Reciprocal::of(word);
        let (q1, partial) = reciprocal.divide(u64::low_bits(high_half), high(low_half));
        let (q0, remainder) = reciprocal.divide(partial, u64::low_bits(low_half));
        return Some(((u128::from(q1) << 64) | u128::from(q0), u128::from(remainder)));
    }
    let shift = divisor.leading_zeros();
    let divisor = divisor << shift;
    let n_high = (high_half << shift) | low_half.unbounded_shr(128_u32.wrapping_sub(shift));
    let n_low = low_half << shift;
    let (d1, d0) = (high(divisor), u64::low_bits(divisor));
    let inverse = reciprocal_two_words(d1, d0);
    let (q1, partial) =
        divide_three_by_two(high(n_high), u64::low_bits(n_high), high(n_low), d1, d0, inverse);
    let (q0, remainder) = divide_three_by_two(
        high(partial),
        u64::low_bits(partial),
        u64::low_bits(n_low),
        d1,
        d0,
        inverse,
    );
    Some(((u128::from(q1) << 64) | u128::from(q0), remainder >> shift))
}

/// `numerator / divisor` and the remainder, or `None` unless the numerator's high word is below
/// the divisor, so the quotient fits one word. The remainder is then what the quotient's product
/// leaves of the numerator's low word. The division is `u128`'s, a call to compiler-builtins: on
/// `x86_64` it ends in one `div`, and on aarch64 it takes one hardware division for a quotient of
/// 32 bits or fewer.
#[inline]
pub(crate) const fn divide_u128_by_u64(numerator: u128, divisor: u64) -> Option<(u64, u64)> {
    if high(numerator) >= divisor {
        return None;
    }
    match numerator.checked_div(u128::from(divisor)) {
        Some(quotient) => {
            let quotient = u64::low_bits(quotient);
            Some((quotient, u64::low_bits(numerator).wrapping_sub(quotient.wrapping_mul(divisor))))
        },
        None => None,
    }
}

/// `5^d`, shifted until its top bit is set, as a divisor of two words, with its reciprocal and its
/// width in bits. A power that fits a word is its high word alone, whose reciprocal by algorithm 6
/// is the word's own.
#[derive(Clone, Copy, Debug)]
struct Five {
    /// The shifted power's high word.
    high: u64,
    /// Its low word, zero for a power that fits a word.
    low: u64,
    /// Its reciprocal, as [`reciprocal_two_words`] defines it.
    inverse: u64,
    /// The power's width in bits, before it was shifted.
    bits: u32,
}

/// `5^d` for `d` in `0..=38`, every power a decimal's scale divides a double by.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const FIVES: [Five; 39] = {
    let mut table = [Five { high: 0, low: 0, inverse: 0, bits: 0 }; 39];
    let mut d = 0;
    while d < table.len() {
        let shift = POW5[d].leading_zeros();
        let shifted = POW5[d] << shift;
        let (high, low) = (high(shifted), u64::low_bits(shifted));
        let inverse = reciprocal_two_words(high, low);
        table[d] = Five { high, low, inverse, bits: 128_u32.wrapping_sub(shift) };
        d += 1;
    }
    table
};

/// `⌊magnitude · 2^s / 5^d⌋` for the `s` that puts it in `[2^62, 2^64)`, with `s`, and whether the
/// division left a remainder; or `None` past 38. The magnitude is not zero.
///
/// Shifted until its top bit is bit 126, the magnitude over the power shifted until its top bit is
/// set is that quotient, for `s = 63 + bits(5^d) − bits(magnitude)`: one 2-by-1 step for a power
/// that fits a word, and one 3-by-2 step, on the magnitude shifted a word further, for one that
/// does not.
#[inline]
#[expect(clippy::indexing_slicing, reason = "checked against the table's length first")]
pub(crate) const fn quotient_by_pow5(magnitude: u128, d: u8) -> Option<(u64, i32, bool)> {
    let d = usize::from(d);
    if d >= FIVES.len() {
        return None;
    }
    let five = FIVES[d];
    let bits = 128_u32.wrapping_sub(magnitude.leading_zeros());
    // A magnitude of all 128 bits is shifted right instead, and loses its lowest.
    let (shifted, lost) = if bits == 128 {
        (magnitude >> 1, magnitude & 1 == 1)
    } else {
        (magnitude << 127_u32.wrapping_sub(bits), false)
    };
    let (quotient, remainder) = if five.low == 0 {
        let (quotient, remainder) =
            divide_two_by_one(high(shifted), u64::low_bits(shifted), five.high, five.inverse);
        (quotient, u128::from(remainder))
    } else {
        divide_three_by_two(
            high(shifted),
            u64::low_bits(shifted),
            0,
            five.high,
            five.low,
            five.inverse,
        )
    };
    // Both widths are at most 128, so the shift is within ±128.
    let shift = 63_i32.wrapping_add_unsigned(five.bits).wrapping_sub_unsigned(bits);
    Some((quotient, shift, remainder != 0 || lost))
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference's own arithmetic, in u128")]
mod tests {
    use proptest::prelude::*;

    use super::{RECIPROCALS, divide_u128, divide_u128_by_short_power, divide_u256};
    use crate::word::U256;

    /// `10^k`.
    fn power(k: u8) -> u128 {
        10_u128.pow(u32::from(k))
    }

    #[test]
    fn a_short_power_refuses_what_its_steps_cannot_divide() {
        assert_eq!(divide_u128_by_short_power(power(9) << 64, 9), None, "a quotient past a word");
        assert_eq!(divide_u128_by_short_power(1, 10), None, "10^10 is past 2^32");
        let largest = ((power(9) - 1) << 64) | u128::from(u64::MAX);
        let expected =
            (u64::try_from(largest / power(9)).ok(), u64::try_from(largest % power(9)).ok());
        assert_eq!(
            divide_u128_by_short_power(largest, 9).map(|(q, r, _)| (Some(q), Some(r))),
            Some(expected),
            "the largest numerator whose quotient fits a word"
        );
    }

    #[test]
    fn every_reciprocal_is_its_definition() {
        for (k, reciprocal) in (0_u8..).zip(RECIPROCALS) {
            let divisor = u64::try_from(power(k)).expect("10^19 fits a word");
            assert_eq!(reciprocal.divisor, divisor << divisor.leading_zeros(), "10^{k}, shifted");
            let inverse = u128::MAX / u128::from(reciprocal.divisor) - (1 << 64);
            assert_eq!(u128::from(reciprocal.inverse), inverse, "10^{k}'s inverse");
        }
    }

    #[test]
    fn every_power_divides_the_edges_of_its_range() {
        for k in 0_u8..=19 {
            let d = power(k);
            for n in [0, 1, d - 1, d, d + 1, (d << 64) - 1, u128::from(u64::MAX), (d - 1) << 64] {
                let expected =
                    u64::try_from(n / d).ok().map(|q| (q, u64::try_from(n % d).unwrap_or(0)));
                assert_eq!(divide_u128(n, k).map(|(q, r, _)| (q, r)), expected, "{n} / 10^{k}");
            }
            assert_eq!(divide_u128(d << 64, k), None, "a quotient of 2^64, past a word");
        }
        assert_eq!(divide_u128(1, 20), None, "10^20 is past a word");
    }

    #[test]
    fn a_wide_numerator_divides_in_two_steps() {
        let n = U256::widening(u128::MAX, 10_u128.pow(18));
        let (high, low) = n.halves();
        let power = 10_u64.pow(18);
        assert_eq!(
            divide_u256(high, low, 18),
            Some((u128::MAX, 0, power)),
            "(2^128 − 1) × 10^18 / 10^18"
        );
        assert_eq!(divide_u256(10_u128.pow(18), 0, 18), None, "a quotient of 2^128, past 128 bits");
        assert_eq!(divide_u256(0, 7, 0), Some((7, 0, 1)), "10^0");
    }

    #[test]
    fn a_reciprocal_at_run_time_is_its_definition() {
        for divisor in [1_u64 << 63, u64::MAX, (1 << 63) | 1, 0xD000_0000_0000_0001, 10_u64.pow(19)]
        {
            let shifted = divisor << divisor.leading_zeros();
            let inverse = u128::MAX / u128::from(shifted) - (1 << 64);
            assert_eq!(u128::from(super::reciprocal_word(shifted)), inverse, "{divisor:#x}");
        }
    }

    proptest! {
        #[test]
        fn every_reciprocal_at_run_time_is_its_definition(divisor in 1_u64..) {
            let shifted = divisor << divisor.leading_zeros();
            let inverse = u128::MAX / u128::from(shifted) - (1 << 64);
            prop_assert_eq!(u128::from(super::reciprocal_word(shifted)), inverse);
        }

        #[test]
        fn a_short_power_divides_in_two_steps_as_division_does(high: u64, low: u64, k in 0_u8..=9) {
            let d = power(k);
            // A high word below the divisor, so the quotient fits a word: the steps' domain.
            let n = (u128::from(high % u64::try_from(d).unwrap_or(1)) << 64) | u128::from(low);
            let expected = (u64::try_from(n / d).unwrap_or(0), u64::try_from(n % d).unwrap_or(0));
            prop_assert_eq!(divide_u128_by_short_power(n, k), Some((expected.0, expected.1, u64::try_from(d).unwrap_or(0))));
        }

        #[test]
        fn a_one_word_quotient_divides_as_division_does(n: u128, divisor in 1_u64..) {
            let d = u128::from(divisor);
            let expected = u64::try_from(n / d).ok().map(|q| (q, u64::try_from(n % d).unwrap_or(0)));
            prop_assert_eq!(super::divide_u128_by_u64(n, divisor), expected);
        }

        #[test]
        fn a_quotient_of_128_bits_divides_as_division_does(
            q: u128, divisor in prop_oneof![1_u128..=u128::from(u64::MAX), 1_u128..], r: u128,
        ) {
            // q × divisor + r, for r below the divisor: the numerator a quotient of q leaves.
            let r = r % divisor;
            let product = U256::widening(q, divisor);
            let (high, low) = product.halves();
            let (low, carry) = low.overflowing_add(r);
            let Some(high) = high.checked_add(u128::from(carry)) else { return Ok(()) };
            prop_assert_eq!(super::divide_u256_by_u128(high, low, divisor), Some((q, r)));
        }

        #[test]
        fn a_two_word_numerator_divides_as_division_does(n: u128, k in 0_u8..=19) {
            let d = power(k);
            let expected = u64::try_from(n / d).ok().map(|q| (q, u64::try_from(n % d).unwrap_or(0)));
            prop_assert_eq!(divide_u128(n, k).map(|(q, r, _)| (q, r)), expected);
        }

        #[test]
        fn a_three_word_numerator_divides_as_division_does(seed: u64, low_half: u128, k in 0_u8..=19) {
            // A high half below 10^k, so the quotient fits 128 bits, against long division in base
            // 2^64 in two u128 steps.
            let d = power(k);
            let high = u128::from(seed) % d;
            let top = (high << 64) | (low_half >> 64);
            let (qa, ra) = (top / d, top % d);
            let bottom = (ra << 64) | (low_half & u128::from(u64::MAX));
            let (qb, rb) = (bottom / d, bottom % d);
            let expected = Some(((qa << 64) | qb, u64::try_from(rb).unwrap_or(0)));
            prop_assert_eq!(divide_u256(high, low_half, k).map(|(q, r, _)| (q, r)), expected);
            prop_assert_eq!(divide_u256(d, low_half, k), None, "a high half at the power");
        }
    }
}
