//! A quotient `a × 10^k / b` from a double's estimate, corrected exactly: one pipelined `fdiv`
//! where the integer divider takes a cycle for every few bits of quotient, and no reciprocal at
//! run time. An estimate is proved never to exceed the quotient and to fall short of it by little
//! enough that one comparison, or one step and a comparison, finishes it; whatever the proof does
//! not cover returns `None`, to the double word's division.
//!
//! The narrow estimate's five roundings of a double, each a factor `1 + δ` with `|δ| ≤ u = 2^-53`,
//! and a divisor raised by `1 + 2^-49`, keep it below `n / b` by a factor between `1 − 21u` and
//! `1 − 11u`, `n = a × 10^k`; the wide estimates' seven, and a reciprocal lowered by `1 − 2^-49`,
//! keep each between `1 − 23u` and `1 − 9u`.

use crate::float::POW10_F64;
use crate::word::{Narrow, pow10_u128, rounds_up};

/// `1 + 2^-49`, which a divisor is raised by, so that no estimate passes the quotient.
const MARGIN: f64 = 1.0 + 1.0 / 562_949_953_421_312.0;
/// `2^32`.
const HALF_WORD: f64 = 4_294_967_296.0;
/// `2^64`.
const WORD: f64 = HALF_WORD * HALF_WORD;
/// `2^-64`.
const PER_WORD: f64 = 1.0 / WORD;
/// `(1 − 2^-49) × 2^64`, over a wide divisor's top word: its reciprocal, lowered so that no estimate
/// passes the quotient.
const LOWERED: f64 = (1.0 - 1.0 / 562_949_953_421_312.0) * WORD;
/// `10^k × 2^-96` for `k` up to 19, each exact: a wide numerator's top word scaled to the estimate.
#[expect(clippy::indexing_slicing, reason = "k below both tables' lengths")]
const POW10_PER_96: [f64; 20] = {
    let mut table = [0.0; 20];
    let mut k = 0;
    while k < table.len() {
        table[k] = POW10_F64[k] * (PER_WORD / HALF_WORD);
        k += 1;
    }
    table
};

/// `x` as the nearest double: the one rounding of an operand the estimate's bound counts.
#[inline]
#[expect(clippy::as_conversions, clippy::cast_precision_loss, reason = "a rounding the bound counts")]
const fn double(x: u64) -> f64 {
    x as f64
}

/// The whole part of `x`, a double from zero up, `u64::MAX` past it, as `fcvtzu` converts.
#[inline]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a non-negative double, saturating past the word"
)]
const fn whole(x: f64) -> u64 {
    x as u64
}

/// A `u128`'s high word.
#[inline]
const fn high(x: u128) -> u64 {
    u64::low_bits(x >> 64)
}

/// `a × 10^k / b`, rounded by `table` for a quotient of sign `negative`, for `k` at most 14 and `b`
/// at most `2^63`: `None` for any other, or a quotient past 64 bits.
///
/// `n < 2^64 × 10^14 < 2^111`, so the estimate's shortfall leaves a remainder
/// `r1 = n − q1 × b < 21u × n + b < 2^64`: `n`'s low word less `q1 × b`, wrapping. The quotient is
/// then `q1` or `q1 + 1` while it is below `2^48.6`; past that, [`div_up_far`] divides `r1`.
#[inline(always)]
#[expect(clippy::inline_always, reason = "inline in the quotient, where the mode's table folds")]
#[expect(clippy::indexing_slicing, reason = "k is at most 14, within the table's 23")]
pub(crate) const fn div_up_narrow(a: u64, k: u8, b: u64, negative: bool, table: u16) -> Option<u64> {
    if k > 14 || b > 1 << 63 {
        return None;
    }
    let Some(power) = pow10_u128(k) else { return None };
    let n = u128::from(a).wrapping_mul(power);
    if high(n) >= b {
        return None;
    }
    let estimate = whole(double(a) * POW10_F64[usize::from(k)] / (double(b) * MARGIN));
    let remainder = u64::low_bits(n).wrapping_sub(estimate.wrapping_mul(b));
    let over = remainder >= b;
    let rest = if over { remainder.wrapping_sub(b) } else { remainder };
    if rest >= b {
        return div_up_far(estimate, remainder, b, negative, table);
    }
    let quotient = estimate.wrapping_add(u64::from(over));
    quotient.checked_add(u64::from(rounds_up(rest, b, quotient & 1 == 1, negative, table)))
}

/// [`div_up_narrow`] for a quotient past `2^48.6`, which its estimate can fall short of by more
/// than one: the estimate and the remainder `r1` it leaves, divided once more.
#[cold]
#[inline(never)]
const fn div_up_far(estimate: u64, remainder: u64, b: u64, negative: bool, table: u16) -> Option<u64> {
    let (Some(more), Some(rest)) = (remainder.checked_div(b), remainder.checked_rem(b)) else {
        return None;
    };
    let quotient = estimate.wrapping_add(more);
    quotient.checked_add(u64::from(rounds_up(rest, b, quotient & 1 == 1, negative, table)))
}

/// `a × 10^k / b`, rounded by `table` for a quotient of sign `negative`, for `k` at most 19 and `b`
/// below `2^127`, by two estimates and one correction: `None` for any other, never for a quotient
/// below `2^96`, and always past `2^96 × (1 + 24u)`, where the first estimate saturates.
///
/// The first estimate is the quotient's top bits, `q1 = 2^32 × ⌊x × r⌋` from the numerator's and
/// the divisor's top words, `r` the one `fdiv`; it leaves an exact remainder `r1 < 2^47.53 × b`
/// while `q < 2^96`, in three words. The second, `⌊r1 × r / 2^64⌋`, leaves less than `1.52 × b`:
/// one conditional subtraction. `q1` is a multiple of `2^32`, so the quotient's parity is the
/// second's and the correction's, read before the sum.
#[inline(always)]
#[expect(clippy::inline_always, reason = "inline in the quotient, where the mode's table folds")]
#[expect(clippy::indexing_slicing, reason = "k is at most 19, within both tables")]
pub(crate) const fn div_up_wide(a: u128, k: u8, b: u128, negative: bool, table: u16) -> Option<u128> {
    if k > 19 || b >= 1 << 127 {
        return None;
    }
    let Some(power) = pow10_u128(k) else { return None };
    // n = a × 10^k, in three words: below 2^128 × 10^19 < 2^192. By hand, from the halves: as
    // `carrying_mul`, a wide quotient measured 5% slower (15.67 ns against 14.90, half to even).
    let power = u128::from(u64::low_bits(power));
    let low = u128::from(u64::low_bits(a)).wrapping_mul(power);
    let upper = u128::from(high(a)).wrapping_mul(power).wrapping_add(low >> 64);
    let (top, rest) = (high(upper), (upper << 64) | u128::from(u64::low_bits(low)));
    if u128::from(top) >= b {
        return None;
    }
    let reciprocal = LOWERED / (double(high(b)) + double(u64::low_bits(b)) * PER_WORD);
    let numerator = double(high(a)) + double(u64::low_bits(a)) * PER_WORD;
    let first = whole(numerator * POW10_PER_96[usize::from(k)] * reciprocal);
    if first == u64::MAX {
        return None;
    }
    // r1 = n − first × b × 2^32, in three words, by hand as above.
    let low = u128::from(first).wrapping_mul(u128::from(u64::low_bits(b)));
    let upper = u128::from(first).wrapping_mul(u128::from(high(b))).wrapping_add(low >> 64);
    let (t2, t1, t0) = (high(upper), u64::low_bits(upper), u64::low_bits(low));
    let shifted = (u128::from((t1 << 32) | (t0 >> 32)) << 64) | u128::from(t0 << 32);
    let (remainder, borrow) = rest.overflowing_sub(shifted);
    let remainder_top = top.wrapping_sub((t2 << 32) | (t1 >> 32)).wrapping_sub(u64::from(borrow));
    if remainder_top >> 48 != 0 {
        return None;
    }
    let scaled = double(remainder_top) * WORD
        + (double(high(remainder)) + double(u64::low_bits(remainder)) * PER_WORD);
    let second = whole(scaled * (reciprocal * PER_WORD));
    let remainder = remainder.wrapping_sub(u128::from(second).wrapping_mul(b));
    let over = remainder >= b;
    let remainder = if over { remainder.wrapping_sub(b) } else { remainder };
    if remainder >= b {
        return None;
    }
    let odd = (second & 1 == 1) != over;
    let quotient =
        (u128::from(first) << 32).wrapping_add(u128::from(second)).wrapping_add(u128::from(over));
    quotient.checked_add(u128::from(rounds_up(remainder, b, odd, negative, table)))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use proptest::sample::select;
    use rstest::rstest;

    use super::{div_up_narrow, div_up_wide};
    use crate::round::{Rounding, RoundingMode};
    use crate::word::{Double, U256, Word};

    /// A value of any size: any bits, shifted down by up to all of them.
    fn sized_u64() -> impl Strategy<Value = u64> {
        (any::<u64>(), 0_u32..64).prop_map(|(bits, shift)| bits >> shift)
    }

    /// A wide value of any size.
    fn sized_u128() -> impl Strategy<Value = u128> {
        (any::<u128>(), 0_u32..128).prop_map(|(bits, shift)| bits >> shift)
    }

    /// The quotient by the double word's own division, the path the estimate stands in front of.
    fn narrow_reference(a: u64, k: u8, b: u64, negative: bool, table: u16) -> Option<u64> {
        let n = u128::from(a).checked_mul(u128::pow10(k)?)?;
        u64::try_from(n.divide_round(u128::from(b), negative, table)).ok()
    }

    /// The quotient by the 256-bit word's own division.
    fn wide_reference(a: u128, k: u8, b: u128, negative: bool, table: u16) -> Option<u128> {
        let n = <U256 as Double<u128>>::from_narrow(a).checked_mul(U256::pow10(k)?)?;
        n.divide_round(<U256 as Double<u128>>::from_narrow(b), negative, table).narrow()
    }

    #[rstest]
    #[case::a_quotient_far_past_the_select(u64::MAX >> 1, 0, 3)]
    #[case::the_largest_divisor(u64::MAX, 14, 1 << 63)]
    #[case::a_quotient_past_the_word((1 << 63) - 1, 4, 7)]
    #[case::eighteen_nines_over_eleven(999_999_999_999_999_999, 1, 11)]
    #[case::the_largest_power(1, 14, 1)]
    #[case::a_quotient_at_the_top_of_the_word(u64::MAX / 10, 1, 1)]
    fn each_estimate_divides_an_edge_as_the_double_word_does(
        #[case] a: u64, #[case] k: u8, #[case] b: u64,
    ) {
        // Past 2^48.6, where a narrow estimate can fall short by more than one, and at the edges of
        // the word and of the divisors either estimate takes: each answered, or refused, as the
        // double word's division answers.
        for &mode in Rounding::MODES {
            for negative in [false, true] {
                let table = mode.table();
                let narrow = narrow_reference(a, k, b, negative, table);
                assert_eq!(div_up_narrow(a, k, b, negative, table), narrow, "narrow, {mode:?}");
                let (a, b) = (u128::from(a) << 40, u128::from(b) << 20);
                let wide = wide_reference(a, k, b, negative, table);
                assert_eq!(div_up_wide(a, k, b, negative, table), wide, "and wide, {mode:?}");
            }
        }
    }

    #[test]
    fn a_wide_divisor_past_a_word_divides_as_the_double_word_does() {
        let (a, b) = (u128::MAX >> 2, (1_u128 << 100) + 12_345);
        for &mode in Rounding::MODES {
            let table = mode.table();
            for k in [0, 7, 19] {
                let expected = wide_reference(a, k, b, true, table);
                assert_eq!(div_up_wide(a, k, b, true, table), expected, "10^{k}, {mode:?}");
            }
        }
    }

    proptest! {
        #[test]
        fn a_narrow_estimate_divides_as_the_double_word_does(
            a in sized_u64(), b in sized_u64(), k in 0_u8..=16, negative: bool, mode in select(Rounding::MODES),
        ) {
            // Wherever the estimate answers, it answers as the double word's division; and it
            // answers every quotient its proof covers, the word's.
            let b = b.max(1);
            let table = mode.table();
            let expected = narrow_reference(a, k, b, negative, table);
            let estimated = div_up_narrow(a, k, b, negative, table);
            if estimated.is_some() {
                prop_assert_eq!(estimated, expected, "{} × 10^{} / {}", a, k, b);
            }
            let covered = k <= 14 && b <= 1 << 63 && expected.is_some();
            prop_assert!(!covered || estimated.is_some(), "{} × 10^{} / {} left", a, k, b);
        }

        #[test]
        fn a_wide_estimate_divides_as_the_double_word_does(
            a in sized_u128(), b in sized_u128(), k in 0_u8..=20, negative: bool, mode in select(Rounding::MODES),
        ) {
            // As the narrow estimate, for quotients below 2^96, which the two estimates reach.
            let b = b.max(1);
            let table = mode.table();
            let expected = wide_reference(a, k, b, negative, table);
            let estimated = div_up_wide(a, k, b, negative, table);
            if estimated.is_some() {
                prop_assert_eq!(estimated, expected, "{} × 10^{} / {}", a, k, b);
            }
            let covered = k <= 19 && b < 1 << 127 && expected.is_some_and(|quotient| quotient < 1 << 96);
            prop_assert!(!covered || estimated.is_some(), "{} × 10^{} / {} left", a, k, b);
        }
    }
}
