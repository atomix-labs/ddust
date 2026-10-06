//! The kernels every decimal operation is written on: each exact, on magnitudes, in a word wide
//! enough for its result, and rounded once by a mode's table.
//!
//! A kernel returns `None` when its result outgrows the word it was given; the caller then runs it
//! again in a [`U256`], the widest word. There, [`mul_up`] alone has no result past it, and the
//! caller keeps its low bits by wrapping arithmetic; [`div_up`] past it keeps them itself.

use core::cmp::Ordering;

use crate::word::{Double, Narrow, U256, Word, class};

/// An exact result: its sign, and its magnitude in a word wide enough to hold it, or, past even
/// the widest word, its low bits, which wrapping keeps.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Exact<D> {
    /// Whether the result is below zero.
    pub(crate) negative: bool,
    /// Its magnitude, modulo the word.
    pub(crate) magnitude: D,
    /// Whether the magnitude overflowed the word, and so is its low bits alone.
    pub(crate) overflowed: bool,
}

impl<D> Exact<D> {
    /// A result the word holds.
    #[inline]
    pub(crate) const fn new(negative: bool, magnitude: D) -> Self {
        Self { negative, magnitude, overflowed: false }
    }
}

/// `quotient`, one step further from zero when `table` says so for a result of sign `negative`
/// whose division left `class`: every rounding in the crate is this.
#[inline]
const fn settle<D: [const] Word>(quotient: D, class: u32, negative: bool, table: u16) -> D {
    let index = (u32::from(negative) << 3) | (u32::from(quotient.is_odd()) << 2) | class;
    // A quotient moves only when the divisor is at least 2, so it is at most half the word.
    if (table >> index) & 1 == 1 { quotient.wrapping_add(D::ONE) } else { quotient }
}

/// `numerator / 10^k` and the class of what it leaves, or `None` when the word leaves it to a
/// wider one, as [`Word::divide_pow10`] says. In the widest word a power past it is past every
/// numerator too, and past twice any: the quotient is zero and the remainder below half.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: with `#[inline]` LLVM calls it out of line, loses the constant `k`, and a \
              64-bit product rounds in 133 instructions where it otherwise takes 98"
)]
const fn quotient_pow10<D: [const] Word>(numerator: D, k: u8) -> Option<(D, u32)> {
    match numerator.divide_pow10(k) {
        Some(divided) => Some(divided),
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
        Some(magnitude) => Some(Exact::new(negative, magnitude)),
        None => None,
    }
}

/// `a / 10^k`, rounded by `table`, and whether nothing was rounded away. The quotient of a value
/// fits its own word, which divides it; only what that word leaves to the double takes it.
#[inline]
pub(crate) const fn scale_down<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, k: u8, table: u16,
) -> Option<(Exact<D>, bool)> {
    if let Some((q, class)) = quotient_pow10(a, k) {
        let q = D::from_narrow(settle(q, class, negative, table));
        return Some((Exact::new(negative, q), class == 0));
    }
    match quotient_pow10(D::from_narrow(a), k) {
        Some((q, class)) => {
            Some((Exact::new(negative, settle(q, class, negative, table)), class == 0))
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
        Some((q, class)) => Some(Exact::new(negative, settle(q, class, negative, table))),
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
        Some(magnitude) => Some(Exact::new(negative, magnitude)),
        None => None,
    }
}

/// `a × 10^k / b`, rounded by `table`, for a `b` that is not zero. Past 38 digits the power is
/// applied in two steps of long division, so the numerator never outgrows the widest word; a
/// quotient past even that keeps its low bits, which wrapping arithmetic computes exactly.
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
    let (q, class, overflowed) = if rest == 0 {
        let (q, class) = numerator.divide(divisor);
        (q, class, false)
    } else {
        let Some(divided) = divide_past_38(numerator, divisor, rest) else { return None };
        divided
    };
    let magnitude = settle(q, class, negative, table);
    Some(Exact { negative, magnitude, overflowed })
}

/// `numerator × 10^rest / divisor`, the class of what it leaves, and whether the quotient
/// overflowed the word: the second step of [`div_up`]'s long division, off its path.
#[cold]
#[inline(never)]
const fn divide_past_38<D: [const] Word>(
    numerator: D, divisor: D, rest: u8,
) -> Option<(D, u32, bool)> {
    // q · 10^rest + (remainder · 10^rest) / divisor: the remainder is below the divisor, so its
    // product fits the widest word; q's may not, and then only its low bits are kept.
    let (q, remainder) = numerator.div_rem(divisor);
    let Some(power) = D::pow10(rest) else { return None };
    let Some(carried) = remainder.checked_mul(power) else { return None };
    let (low, class) = carried.divide(divisor);
    let mut overflowed = false;
    let lifted = match q.checked_mul(power) {
        Some(lifted) => lifted,
        None if D::WIDEST => {
            overflowed = true;
            q.wrapping_mul(power)
        },
        None => return None,
    };
    let q = match lifted.checked_add(low) {
        Some(sum) => sum,
        None if D::WIDEST => {
            overflowed = true;
            lifted.wrapping_add(low)
        },
        None => return None,
    };
    Some((q, class, overflowed))
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
        Some(divisor) => D::from_narrow(a).divide(divisor),
        None if D::WIDEST => (D::ZERO, u32::from(a != U::ZERO)),
        None => return None,
    };
    Some(Exact::new(negative, settle(q, class, negative, table)))
}

/// `a × b / c`, rounded by `table`, for a `c` that is not zero: never past the word.
#[inline]
pub(crate) const fn mul_div<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, b: U, c: U, table: u16,
) -> Exact<D> {
    let (q, class) = D::widening_mul(a, b).divide(D::from_narrow(c));
    Exact::new(negative, settle(q, class, negative, table))
}

/// The multiple of `step` that `a / step`, rounded by `table`, comes to, for a `step` that is not
/// zero: never past the word, since it is at most `a + step`.
#[inline]
pub(crate) const fn multiple<U: [const] Narrow, D: [const] Double<U>>(
    negative: bool, a: U, step: U, table: u16,
) -> Exact<D> {
    let step = D::from_narrow(step);
    let (q, class) = D::from_narrow(a).divide(step);
    let count = settle(q, class, negative, table);
    // count × step ≤ a + step < 2 × 2^bits(U), which the double word holds.
    let magnitude = count.wrapping_mul(step);
    Exact::new(negative && magnitude != D::ZERO, magnitude)
}

/// `5^k` for `k` in `0..=38`: with a power of two, every power of ten a scale needs.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const POW5: [u128; 39] = {
    let mut table = [1_u128; 39];
    let mut k = 1;
    while k < table.len() {
        table[k] = table[k.wrapping_sub(1)].wrapping_mul(5);
        k = k.wrapping_add(1);
    }
    table
};

/// The magnitude of `mantissa × 2^exponent × 10^decimals`, rounded by `table` for a value of sign
/// `negative`: a double's exact value at a scale, rounded once. `None` past 128 bits, which no
/// integer holds.
#[expect(clippy::indexing_slicing, reason = "decimals are at most 38, the table's last index")]
pub(crate) const fn binary_at_scale(
    negative: bool, mantissa: u64, exponent: i32, decimals: u8, table: u16,
) -> Option<u128> {
    // mantissa · 2^e · 10^d is mantissa · 5^d · 2^(e + d): one product, then one shift.
    let power = POW5[usize::from(decimals)];
    let shift = exponent.wrapping_add(i32::from(decimals));
    let Ok(five) = u64::try_from(power) else {
        // Past 27 decimals: a product of up to 142 bits.
        let product = U256::widening(u128::from(mantissa), power);
        let magnitude = if shift >= 0 {
            let Some(magnitude) = product.checked_shl(shift.cast_unsigned()) else { return None };
            magnitude
        } else {
            let (quotient, class) = product.shr_classed(shift.unsigned_abs());
            settle(quotient, class, negative, table)
        };
        return magnitude.to_u128();
    };
    // 53 bits by at most 63: a product below 2^117.
    let product = u128::from(mantissa).wrapping_mul(u128::from(five));
    if shift >= 0 {
        let shift = shift.cast_unsigned();
        return if product == 0 {
            Some(0)
        } else if shift <= product.leading_zeros() {
            product.checked_shl(shift)
        } else {
            None
        };
    }
    let (quotient, class) = shr_classed(product, shift.unsigned_abs());
    Some(settle(quotient, class, negative, table))
}

/// `value >> shift` and the [`class`] of the bits shifted out, for a `shift` that is not zero and a
/// value below `2^127`: past 127 bits all of it shifts out, below half.
#[inline]
const fn shr_classed(value: u128, shift: u32) -> (u128, u32) {
    if shift >= 128 {
        return (0, u32::from(value != 0));
    }
    let rest = value & (u128::MAX >> 128_u32.wrapping_sub(shift));
    (value >> shift, class(rest, 1 << shift))
}

/// The exact sum of two results.
#[inline]
pub(crate) const fn add<D: [const] Word>(x: Exact<D>, y: Exact<D>) -> Option<Exact<D>> {
    if x.negative == y.negative {
        return match x.magnitude.checked_add(y.magnitude) {
            Some(magnitude) => Some(Exact::new(x.negative, magnitude)),
            None => None,
        };
    }
    // Opposite signs: the larger magnitude keeps its sign, and zero is never negative.
    Some(if x.magnitude >= y.magnitude {
        let magnitude = x.magnitude.wrapping_sub(y.magnitude);
        Exact::new(x.negative && magnitude != D::ZERO, magnitude)
    } else {
        Exact::new(y.negative, y.magnitude.wrapping_sub(x.magnitude))
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
            let exact = |v: i32| Exact::new(v < 0, u64::from(v.unsigned_abs()));
            let sum = add(exact(x), exact(y)).expect("two i32 magnitudes fit a u64");
            let expected = i64::from(x) + i64::from(y);
            prop_assert_eq!((sum.negative, sum.magnitude), (expected < 0, expected.unsigned_abs()));
            prop_assert_eq!(compare(exact(x), exact(y)), x.cmp(&y));
        }
    }

    #[test]
    fn zero_is_never_negative() {
        let zero = add(Exact::new(true, 5_u32), Exact::new(false, 5));
        assert_eq!(zero.map(|e| e.negative), Some(false), "-5 + 5 is zero, unsigned");
        let order = compare(Exact::new(true, 0_u32), Exact::new(false, 0));
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
