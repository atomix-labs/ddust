//! The kernels every decimal operation is written on: each exact, on magnitudes, in a word wide
//! enough for its result, and rounded once by a mode's table.
//!
//! A kernel returns `None` when its result outgrows the word it was given; the caller then runs it
//! again in a [`U256`](crate::word::U256), the widest word, where no kernel but [`mul_up`]
//! outgrows it.

use core::cmp::Ordering;

use crate::word::{Double, Narrow, Word};

/// An exact result: its sign, and its magnitude in a word wide enough to hold it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Exact<D> {
    /// Whether the result is below zero.
    pub(crate) negative: bool,
    /// Its magnitude.
    pub(crate) magnitude: D,
}

/// How a division's remainder compares with half its divisor: 0 when it is zero, 1 below half, 2
/// at half, 3 above.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "three bits sum to at most 3")]
const fn class<D: [const] Word>(remainder: D, divisor: D) -> u32 {
    let rest = divisor.wrapping_sub(remainder);
    u32::from(remainder != D::ZERO) + u32::from(remainder >= rest) + u32::from(remainder > rest)
}

/// `quotient`, one step further from zero when `table` says so for a result of sign `negative`
/// whose division left `class`: every rounding in the crate is this.
#[inline]
const fn settle<D: [const] Word>(quotient: D, class: u32, negative: bool, table: u16) -> D {
    let index = (u32::from(negative) << 3) | (u32::from(quotient.is_odd()) << 2) | class;
    // A quotient moves only when the divisor is at least 2, so it is at most half the word.
    if (table >> index) & 1 == 1 { quotient.wrapping_add(D::ONE) } else { quotient }
}

/// `numerator / divisor` and the class of what it leaves.
#[inline]
const fn quotient<D: [const] Word>(numerator: D, divisor: D) -> (D, u32) {
    let (quotient, remainder) = numerator.div_rem(divisor);
    (quotient, class(remainder, divisor))
}

/// `numerator / 10^k` and the class of what it leaves, or `None` when the power is past the word.
/// In the widest word a power past it is past every numerator too, and past twice any: the
/// quotient is zero and the remainder below half.
#[inline]
const fn quotient_pow10<D: [const] Word>(numerator: D, k: u8) -> Option<(D, u32)> {
    match D::pow10(k) {
        Some(power) => Some(quotient(numerator, power)),
        None if D::WIDEST => Some((D::ZERO, u32::from(numerator != D::ZERO))),
        None => None,
    }
}

/// `a × 10^k`.
#[inline]
pub(crate) const fn scale_up<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, k: u8,
) -> Option<Exact<D>> {
    let magnitude = match D::pow10(k) {
        Some(power) => D::from_narrow(a).checked_mul(power),
        None if a == U::ZERO => Some(D::ZERO),
        None => None,
    };
    match magnitude {
        Some(magnitude) => Some(Exact { negative, magnitude }),
        None => None,
    }
}

/// `a / 10^k`, rounded by `table`, and whether nothing was rounded away.
#[inline]
pub(crate) const fn scale_down<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, k: u8, table: u16,
) -> Option<(Exact<D>, bool)> {
    match quotient_pow10(D::from_narrow(a), k) {
        Some((q, class)) => {
            Some((Exact { negative, magnitude: settle(q, class, negative, table) }, class == 0))
        },
        None => None,
    }
}

/// `a × b / 10^k`, rounded by `table`.
#[inline]
pub(crate) const fn mul_down<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, b: U, k: u8, table: u16,
) -> Option<Exact<D>> {
    match quotient_pow10(D::widening_mul(a, b), k) {
        Some((q, class)) => Some(Exact { negative, magnitude: settle(q, class, negative, table) }),
        None => None,
    }
}

/// `a × b × 10^k`, exactly; `None` past the word, even the widest, where the caller keeps the low
/// bits by wrapping arithmetic.
#[inline]
pub(crate) const fn mul_up<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, b: U, k: u8,
) -> Option<Exact<D>> {
    let product = D::widening_mul(a, b);
    let magnitude = match D::pow10(k) {
        Some(power) => product.checked_mul(power),
        None if product == D::ZERO => Some(D::ZERO),
        None => None,
    };
    match magnitude {
        Some(magnitude) => Some(Exact { negative, magnitude }),
        None => None,
    }
}

/// `a × 10^k / b`, rounded by `table`, for a `b` that is not zero. Past 38 digits the power is
/// applied in two steps of long division, so the numerator never outgrows the widest word.
pub(crate) const fn div_up<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, k: u8, b: U, table: u16,
) -> Option<Exact<D>> {
    let divisor = D::from_narrow(b);
    let (first, rest) = (if k > 38 { 38 } else { k }, k.saturating_sub(38));
    let numerator = match D::pow10(first) {
        Some(power) => D::from_narrow(a).checked_mul(power),
        None => None,
    };
    let Some(numerator) = numerator else { return None };
    let (mut q, mut remainder) = numerator.div_rem(divisor);
    if rest > 0 {
        // q · 10^rest + (remainder · 10^rest) / b: the remainder is below b, so its product fits.
        let Some(power) = D::pow10(rest) else { return None };
        let Some(lifted) = q.checked_mul(power) else { return None };
        let Some(carried) = remainder.checked_mul(power) else { return None };
        let (low, last) = carried.div_rem(divisor);
        let Some(sum) = lifted.checked_add(low) else { return None };
        q = sum;
        remainder = last;
    }
    Some(Exact { negative, magnitude: settle(q, class(remainder, divisor), negative, table) })
}

/// `a / (b × 10^k)`, rounded by `table`, for a `b` that is not zero. In the widest word a divisor
/// past it is past twice any `a`, whose quotient is then zero and its remainder below half.
#[inline]
pub(crate) const fn div_down<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, b: U, k: u8, table: u16,
) -> Option<Exact<D>> {
    let divisor = match D::pow10(k) {
        Some(power) => D::from_narrow(b).checked_mul(power),
        None => None,
    };
    let (q, class) = match divisor {
        Some(divisor) => quotient(D::from_narrow(a), divisor),
        None if D::WIDEST => (D::ZERO, u32::from(a != U::ZERO)),
        None => return None,
    };
    Some(Exact { negative, magnitude: settle(q, class, negative, table) })
}

/// `a × b / c`, rounded by `table`, for a `c` that is not zero: never past the word.
#[inline]
pub(crate) const fn mul_div<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, b: U, c: U, table: u16,
) -> Exact<D> {
    let (q, class) = quotient(D::widening_mul(a, b), D::from_narrow(c));
    Exact { negative, magnitude: settle(q, class, negative, table) }
}

/// The exact sum of two results.
#[inline]
pub(crate) const fn add<D: [const] Word>(x: Exact<D>, y: Exact<D>) -> Option<Exact<D>> {
    if x.negative == y.negative {
        return match x.magnitude.checked_add(y.magnitude) {
            Some(magnitude) => Some(Exact { negative: x.negative, magnitude }),
            None => None,
        };
    }
    // Opposite signs: the larger magnitude keeps its sign, and zero is never negative.
    Some(if x.magnitude >= y.magnitude {
        let magnitude = x.magnitude.wrapping_sub(y.magnitude);
        Exact { negative: x.negative && magnitude != D::ZERO, magnitude }
    } else {
        Exact { negative: y.negative, magnitude: y.magnitude.wrapping_sub(x.magnitude) }
    })
}

/// The order of two results.
#[inline]
pub(crate) const fn compare<D: [const] Word>(x: Exact<D>, y: Exact<D>) -> Ordering {
    let zero = x.magnitude == D::ZERO && y.magnitude == D::ZERO;
    match (x.negative && !zero, y.negative && !zero) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => x.magnitude.cmp(&y.magnitude),
        (true, true) => y.magnitude.cmp(&x.magnitude),
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the reference model's own arithmetic, in a type wide enough"
)]
mod tests {
    use core::cmp::Ordering;

    use proptest::prelude::*;

    use super::{Exact, add, class, compare, div_down, div_up, mul_div, mul_down, scale_down};
    use crate::word::{Double, U256};

    /// Truncation's table.
    const TRUNC: u16 = 0x0000;
    /// Half to even's table.
    const HALF_EVEN: u16 = 0xC8C8;

    /// A 256-bit result as a `u128`, when it fits one.
    fn narrow(wide: U256) -> Option<u128> {
        <U256 as Double<u128>>::narrow(wide)
    }

    /// `n / d`, rounded half to even: the reference.
    fn half_even(n: u128, d: u128) -> u128 {
        let (q, r) = (n / d, n % d);
        q + u128::from(r * 2 > d || (r * 2 == d && q % 2 == 1))
    }

    #[test]
    fn a_remainder_is_classed_against_half_its_divisor() {
        assert_eq!([0, 4, 5, 6].map(|r| class(r, 10_u32)), [0, 1, 2, 3], "zero, below, at, above");
        assert_eq!([0, 1, 2].map(|r| class(r, 3_u32)), [0, 1, 3], "an odd divisor has no half");
    }

    proptest! {
        #[test]
        fn a_product_down_a_power_agrees_with_the_definition(a: u32, b: u32, k in 0_u8..20) {
            let (exact, power) = (u128::from(a) * u128::from(b), 10_u128.pow(u32::from(k)));
            let truncated = mul_down::<u32, u64>(false, a, b, k, TRUNC).map(|e| u128::from(e.magnitude));
            prop_assert_eq!(truncated, Some(exact / power), "truncated, in a u64");
            let rounded = mul_down::<u32, U256>(false, a, b, k, HALF_EVEN).and_then(|e| narrow(e.magnitude));
            prop_assert_eq!(rounded, Some(half_even(exact, power)), "half to even, in a U256");
        }

        #[test]
        fn a_quotient_lifted_past_38_digits_is_the_one_lifted_in_one_step(a: u128, c in 1_u128..=3, k in 39_u8..=76) {
            // a · 10^k / (10^(k - 38) · c) is a · 10^38 / c, remainder and all: the two-step kernel
            // against the one-step one.
            let b = 10_u128.pow(u32::from(k - 38)) * c;
            let two = div_up::<u128, U256>(false, a, k, b, HALF_EVEN).map(|e| e.magnitude);
            let one = div_up::<u128, U256>(false, a, 38, c, HALF_EVEN).map(|e| e.magnitude);
            prop_assert_eq!(two, one);
        }

        #[test]
        fn a_quotient_up_a_power_agrees_with_the_definition(a: u64, b in 1_u64.., k in 0_u8..=19) {
            let numerator = u128::from(a) * 10_u128.pow(u32::from(k));
            let got = div_up::<u64, U256>(false, a, k, b, HALF_EVEN).and_then(|e| narrow(e.magnitude));
            prop_assert_eq!(got, Some(half_even(numerator, u128::from(b))));
        }

        #[test]
        fn a_division_down_a_power_agrees_with_the_definition(a: u64, b in 1_u64.., k in 0_u8..=38) {
            let divisor = u128::from(b).checked_mul(10_u128.pow(u32::from(k)));
            let expected = divisor.map_or(0, |divisor| half_even(u128::from(a), divisor));
            let got = div_down::<u64, U256>(false, a, b, k, HALF_EVEN).and_then(|e| narrow(e.magnitude));
            prop_assert_eq!(got, Some(expected));
        }

        #[test]
        fn a_product_over_a_divisor_agrees_with_the_definition(a: u32, b: u32, c in 1_u32..) {
            let exact = half_even(u128::from(a) * u128::from(b), u128::from(c));
            prop_assert_eq!(u128::from(mul_div::<u32, u64>(false, a, b, c, HALF_EVEN).magnitude), exact);
        }

        #[test]
        fn a_sum_and_an_order_agree_with_signed_arithmetic(x: i32, y: i32) {
            let exact = |v: i32| Exact { negative: v < 0, magnitude: u64::from(v.unsigned_abs()) };
            let sum = add(exact(x), exact(y)).expect("two i32 magnitudes fit a u64");
            let expected = i64::from(x) + i64::from(y);
            prop_assert_eq!((sum.negative, sum.magnitude), (expected < 0, expected.unsigned_abs()));
            prop_assert_eq!(compare(exact(x), exact(y)), x.cmp(&y));
        }
    }

    #[test]
    fn zero_is_never_negative() {
        let zero = add(
            Exact { negative: true, magnitude: 5_u32 },
            Exact { negative: false, magnitude: 5 },
        );
        assert_eq!(zero.map(|e| e.negative), Some(false), "-5 + 5 is zero, unsigned");
        let order = compare(
            Exact { negative: true, magnitude: 0_u32 },
            Exact { negative: false, magnitude: 0 },
        );
        assert_eq!(order, Ordering::Equal, "-0 and 0 are equal");
    }

    #[test]
    fn a_power_past_a_narrow_word_asks_for_a_wider_one() {
        assert!(scale_down::<u8, u16>(false, 200, 5, TRUNC).is_none(), "10^5 is past a u16");
        let (wide, exact) = scale_down::<u8, U256>(false, 200, 5, TRUNC).expect("but not a U256");
        assert_eq!(
            (narrow(wide.magnitude), exact),
            (Some(0), false),
            "200 / 10^5 truncates to zero"
        );
        let (far, _) =
            scale_down::<u8, U256>(false, 200, 90, TRUNC).expect("a power past even a U256");
        assert_eq!(narrow(far.magnitude), Some(0), "is past every numerator");
    }
}
