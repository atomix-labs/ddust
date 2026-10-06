//! Division by a power of ten through its reciprocal.
//!
//! A two-word numerator over a one-word power of ten is one Möller–Granlund step: two multiplies
//! and a correction, where `u128 / u128` is a library call on aarch64 and a long division in
//! software. Each power from `10^0` to `10^19` has its reciprocal in [`RECIPROCALS`], which the
//! compiler computes from its `const fn`; nothing is computed at run time.
//!
//! The step's precondition, that the numerator's high word is below the divisor, is the test that
//! the quotient fits one word: a quotient past it is past the integer the result goes into, and
//! takes the plain division, off the hot path.

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
        let quotient = match u128::MAX.checked_div(u128_of(divisor)) {
            Some(quotient) => quotient,
            None => 0,
        };
        Self { divisor, inverse: low(quotient), shift }
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

/// `value` as a `u128`.
const fn u128_of(value: u64) -> u128 {
    u128::from(value)
}

/// The low word of `value`.
const fn low(value: u128) -> u64 {
    // The masked value always fits: the conversion is the truncation, spelled exactly.
    match u64::try_from(value & u128::from(u64::MAX)) {
        Ok(low) => low,
        Err(_out_of_range) => 0,
    }
}

/// The high word of `value`.
const fn high(value: u128) -> u64 {
    low(value >> 64)
}

/// `(u1:u0) / divisor` and the remainder, for a `divisor` whose top bit is set and `u1 < divisor`,
/// by its `inverse` (Möller and Granlund, "Improved division by invariant integers", 2011,
/// algorithm 4).
#[inline]
const fn divide_two_by_one(u1: u64, u0: u64, divisor: u64, inverse: u64) -> (u64, u64) {
    let estimate =
        u128_of(inverse).wrapping_mul(u128_of(u1)).wrapping_add((u128_of(u1) << 64) | u128_of(u0));
    let (q1, q0) = (high(estimate).wrapping_add(1), low(estimate));
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

/// `numerator / 10^k`, the remainder and `10^k`, or `None` unless `k` is at most 19 and the
/// quotient fits one word.
#[inline]
pub(crate) const fn divide_u128(numerator: u128, k: u8) -> Option<(u64, u64, u64)> {
    let Some(Reciprocal { divisor, inverse, shift }) = reciprocal(k) else { return None };
    let (n1, n0) = (high(numerator), low(numerator));
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
    if high_half >= u128_of(divisor >> shift) {
        return None;
    }
    let (n2, n1, n0) = (low(high_half), high(low_half), low(low_half));
    // The three live words, shifted as one number: each takes the bits the next one shifts out.
    let (u2, _) = shifted(n2, n1, shift);
    let (u1, u0) = shifted(n1, n0, shift);
    let (q1, partial) = divide_two_by_one(u2, u1, divisor, inverse);
    let (q0, remainder) = divide_two_by_one(partial, u0, divisor, inverse);
    Some(((u128_of(q1) << 64) | u128_of(q0), remainder >> shift, divisor >> shift))
}

/// `(n1:n0) << shift`, its two words, for a `shift` below 64.
#[inline]
const fn shifted(n1: u64, n0: u64, shift: u32) -> (u64, u64) {
    if shift == 0 {
        (n1, n0)
    } else {
        ((n1 << shift) | (n0 >> 64_u32.wrapping_sub(shift)), n0 << shift)
    }
}

/// The reciprocal of `10^k`, or `None` past one word.
#[inline]
#[expect(clippy::indexing_slicing, reason = "checked against the table's length first")]
const fn reciprocal(k: u8) -> Option<Reciprocal> {
    let k = usize::from(k);
    if k < RECIPROCALS.len() { Some(RECIPROCALS[k]) } else { None }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference's own arithmetic, in u128")]
mod tests {
    use proptest::prelude::*;

    use super::{RECIPROCALS, divide_u128, divide_u256};
    use crate::word::U256;

    /// `10^k`.
    fn power(k: u8) -> u128 {
        10_u128.pow(u32::from(k))
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

    proptest! {
        #[test]
        fn a_two_word_numerator_divides_as_division_does(n: u128, k in 0_u8..=19) {
            let d = power(k);
            let expected = u64::try_from(n / d).ok().map(|q| (q, u64::try_from(n % d).unwrap_or(0)));
            prop_assert_eq!(divide_u128(n, k).map(|(q, r, _)| (q, r)), expected);
        }

        #[test]
        fn a_four_word_numerator_divides_as_division_does(a: u128, b: u64, low: u64, k in 0_u8..=19) {
            // a × b + low, a 192-bit numerator, against the same division in two u128 steps.
            let d = power(k);
            let n = U256::widening(a, u128::from(b));
            let (high, low_half) = n.halves();
            let low_half = low_half.wrapping_add(u128::from(low));
            let high = high + u128::from(low_half < u128::from(low));
            let expected = if high < d {
                let (q1, r1) = (high / d, high % d);
                prop_assume!(q1 == 0);
                // (r1 · 2^128 + low_half) / d by long division in base 2^64.
                let top = (r1 << 64) | (low_half >> 64);
                let (qa, ra) = (top / d, top % d);
                let bottom = (ra << 64) | (low_half & u128::from(u64::MAX));
                let (qb, rb) = (bottom / d, bottom % d);
                Some(((qa << 64) | qb, u64::try_from(rb).unwrap_or(0)))
            } else {
                None
            };
            prop_assert_eq!(divide_u256(high, low_half, k).map(|(q, r, _)| (q, r)), expected);
        }
    }
}
