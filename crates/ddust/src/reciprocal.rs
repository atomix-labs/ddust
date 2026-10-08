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

use crate::word::{Narrow, POW5};

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

/// The low word of `⌊(2^192 − 1) / divisor⌋ − 2^128`, the two-word inverse of a `divisor` whose
/// top bit is set: its high word is the one-word inverse, and the low one the next word of the
/// long division, which carries on from `(2^128 − 1) % divisor`.
const fn inverse_low(divisor: u64) -> u64 {
    let divisor = u128::from(divisor);
    let remainder = match u128::MAX.checked_rem(divisor) {
        Some(remainder) => remainder,
        None => 0,
    };
    // The remainder is below the divisor, so the next word of the quotient fits one word.
    match ((remainder << 64) | u128::from(u64::MAX)).checked_div(divisor) {
        Some(quotient) => u64::low_bits(quotient),
        None => 0,
    }
}

/// `a × b`, both words.
#[inline]
const fn product(a: u64, b: u64) -> u128 {
    u128::from(a).wrapping_mul(u128::from(b))
}

/// `(u2:u1:u0) / divisor` and the remainder, for a `divisor` whose top bit is set and `u2` below
/// it, so the quotient fits two words.
///
/// Both words at once by the divisor's two-word inverse, four products that do not wait on one
/// another and one correction (GMP's `mpn_div_qr_1n_pi2`, the step of "Improved division by
/// invariant integers" for three words).
#[inline]
const fn divide_three_by_one(
    u2: u64, u1: u64, u0: u64, divisor: u64, inverse_high: u64, inverse_low: u64,
) -> (u128, u64) {
    let crossed = high(product(u1, inverse_low));
    let low = product(u1, inverse_high).wrapping_add(1 << 64);
    let (sum, carry) = low.overflowing_add((u128::from(u1) << 64) | u128::from(u0));
    let mut top = u2.wrapping_add(u64::from(carry));
    let (sum, carry) = sum.overflowing_add(product(u2, inverse_low));
    top = top.wrapping_add(u64::from(carry));
    let upper = product(u2, inverse_high);
    let (sum, carry) = sum.overflowing_add((upper << 64) | u128::from(crossed));
    top = top.wrapping_add(u64::from(carry)).wrapping_add(high(upper));
    let (q2, q1) = (high(sum), u64::low_bits(sum));
    let remainder = u0.wrapping_sub(q2.wrapping_mul(divisor));
    // Taken about half the time, so a select rather than a branch.
    let over = remainder >= q1;
    let remainder = remainder.wrapping_add(if over { divisor } else { 0 });
    let quotient = ((u128::from(top) << 64) | u128::from(q2)).wrapping_sub(u128::from(over));
    // Rare.
    if remainder >= divisor {
        (quotient.wrapping_add(1), remainder.wrapping_sub(divisor))
    } else {
        (quotient, remainder)
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

/// The low word of `10^k`'s two-word inverse, shifted as in [`RECIPROCALS`], for `k` in `0..=19`.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const INVERSES_LOW: [u64; 20] = {
    let mut table = [0_u64; 20];
    let mut k = 0;
    while k < table.len() {
        table[k] = inverse_low(RECIPROCALS[k].divisor);
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

/// `numerator / 10^k`, the remainder and `10^k`, or `None` unless `k` is from 1 to 9, so `10^k` is
/// below 2^32, and the quotient fits one word.
///
/// Two 64-by-32-bit divisions, nexus-decimal's split, each of 32 bits of the low word after the
/// remainder before it, and each by one multiply-high with [`SHORT_POWERS`]' multiplier and no
/// correction, so that a power known only at run time costs a table load where a hardware
/// division would wait twice.
#[inline]
pub(crate) const fn divide_u128_by_short_power(numerator: u128, k: u8) -> Option<(u64, u64, u64)> {
    let Some(ShortPower { divisor, multiplier, shift }) = short_power(k) else { return None };
    let (n1, n0) = (high(numerator), u64::low_bits(numerator));
    // The quotient fits a word exactly when the high word is below the divisor; then each step's
    // numerator, a remainder below 2^30 and 32 bits, is below 2^62, where the multiplier is exact.
    if n1 >= divisor {
        return None;
    }
    let (q1, r1) = short_step((n1 << 32) | (n0 >> 32), divisor, multiplier, shift);
    let (q0, r0) = short_step((r1 << 32) | (n0 & 0xFFFF_FFFF), divisor, multiplier, shift);
    Some(((q1 << 32) | q0, r0, divisor))
}

/// `n / divisor` and the remainder, for an `n` below 2^62, by [`SHORT_POWERS`]' `multiplier` and
/// `shift` for the divisor.
#[inline]
const fn short_step(n: u64, divisor: u64, multiplier: u64, shift: u32) -> (u64, u64) {
    let quotient = high(u128::from(n).wrapping_mul(u128::from(multiplier))) >> shift;
    (quotient, n.wrapping_sub(quotient.wrapping_mul(divisor)))
}

/// A power of ten below 2^32, with the multiplier `⌈2^(64+s) / d⌉` and the shift `s = ⌊log2 d⌋`
/// that divide by it: the high word of a numerator below 2^63 times the multiplier, shifted, is
/// its quotient, since the multiplier's excess, below `d`, times the numerator is below
/// `2^(64+s)`.
#[derive(Clone, Copy)]
struct ShortPower {
    /// The power.
    divisor: u64,
    /// `⌈2^(64+s) / divisor⌉`, below 2^64 since `2^s ≤ divisor`.
    multiplier: u64,
    /// `⌊log2 divisor⌋`.
    shift: u32,
}

impl ShortPower {
    /// The power `divisor`'s multiplier and shift.
    const fn new(divisor: u64) -> Self {
        let shift = 63_u32.wrapping_sub(divisor.leading_zeros());
        let wide = u128::from(divisor);
        let multiplier = (1_u128 << 64_u32.wrapping_add(shift)).div_ceil(wide);
        Self { divisor, multiplier: u64::low_bits(multiplier), shift }
    }
}

/// `10^k` for `k` in `0..=9`, every power below 2^32, each with what divides by it; the first, 1,
/// has no multiplier a word holds, and is never read.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const SHORT_POWERS: [ShortPower; 10] = {
    let mut table = [ShortPower { divisor: 0, multiplier: 0, shift: 0 }; 10];
    let mut k = 0;
    let mut power = 1_u64;
    while k < table.len() {
        table[k] = ShortPower::new(power);
        power = power.wrapping_mul(10);
        k += 1;
    }
    table
};

/// [`SHORT_POWERS`]' entry for `k`, from 1 to 9; `None` for 0, whose multiplier, 2^64, no word
/// holds, and past 9.
#[inline]
#[expect(clippy::indexing_slicing, reason = "checked against the table's length first")]
const fn short_power(k: u8) -> Option<ShortPower> {
    let k = usize::from(k);
    if k > 0 && k < SHORT_POWERS.len() { Some(SHORT_POWERS[k]) } else { None }
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

/// `numerator / 10^k`, the remainder and `10^k`, for a quotient past one word, or `None` unless `k`
/// is at most 19: two steps, a word of the quotient each, which for a numerator of two words do
/// less than [`divide_u256`]'s one step for three.
#[inline]
pub(crate) const fn divide_u128_past_a_word(numerator: u128, k: u8) -> Option<(u128, u64, u64)> {
    let Some(Reciprocal { divisor, inverse, shift }) = reciprocal(k) else { return None };
    let (n1, n0) = (high(numerator), u64::low_bits(numerator));
    // The two words and the bits the shift carries past them, as one number of three.
    let u2 = n1.unbounded_shr(64_u32.wrapping_sub(shift));
    let (u1, u0) = shifted(n1, n0, shift);
    let (q1, partial) = divide_two_by_one(u2, u1, divisor, inverse);
    let (q0, remainder) = divide_two_by_one(partial, u0, divisor, inverse);
    Some(((u128::from(q1) << 64) | u128::from(q0), remainder >> shift, divisor >> shift))
}

/// `numerator / 10^k` and the remainder, for a numerator below 2^127, or `None` unless `k`
/// is from 1 to 19.
///
/// A multiply-high by `m = ⌊2^(128 + l) / 10^k⌋ + 1 − 2^128`, `l` being `⌈log2 10^k⌉`, and a
/// shift, `⌊(n + ⌊n × m / 2^128⌋) / 2^l⌋`, exact for every numerator below 2^128 (Granlund and
/// Montgomery, "Division by invariant integers using multiplication", 1994, theorem 4.2); below
/// 2^127 the sum fits a `u128`. `m` is one more than the shifted power's two-word inverse.
#[inline]
#[expect(
    clippy::indexing_slicing,
    reason = "`reciprocal` has checked `k` against the tables' length"
)]
pub(crate) const fn divide_u127(numerator: u128, k: u8) -> Option<(u128, u64)> {
    // 10^0's multiplier is 2^128, past a `u128`.
    if k == 0 {
        return None;
    }
    let Some(Reciprocal { divisor, inverse, shift }) = reciprocal(k) else { return None };
    let multiplier =
        ((u128::from(inverse) << 64) | u128::from(INVERSES_LOW[usize::from(k)])).wrapping_add(1);
    let (_, product_high) = numerator.carrying_mul(multiplier, 0);
    let quotient = numerator.wrapping_add(product_high) >> 64_u32.wrapping_sub(shift);
    // The remainder is below the power, so one word of each side holds it.
    let power = divisor >> shift;
    let remainder =
        u64::low_bits(numerator).wrapping_sub(u64::low_bits(quotient).wrapping_mul(power));
    Some((quotient, remainder))
}

/// `(high:low) / 10^k` for any 256-bit numerator, the quotient's two halves, high first, the
/// remainder and `10^k`, or `None` unless `k` is at most 19: one step for the quotient's low half,
/// after two for its high half when there is one, a long division in base 2^128.
#[inline(always)]
#[expect(
    clippy::indexing_slicing,
    reason = "`reciprocal` has checked `k` against the tables' length"
)]
#[expect(
    clippy::inline_always,
    reason = "out of line, LLVM loses the constant `k`, and a wide rounded product keeps the long \
              division for a power past `10^19`: a call, and a stack frame on every product"
)]
pub(crate) const fn divide_u256(
    high_half: u128, low_half: u128, k: u8,
) -> Option<(u128, u128, u64, u64)> {
    let Some(reciprocal) = reciprocal(k) else { return None };
    let power = reciprocal.divisor >> reciprocal.shift;
    // A high half below the power is the remainder of a quotient whose high half is zero.
    let (quotient_high, carried) = if high_half < u128::from(power) {
        (0, u64::low_bits(high_half))
    } else {
        match divide_u128_past_a_word(high_half, k) {
            Some((quotient, remainder, _)) => (quotient, remainder),
            None => return None,
        }
    };
    let inverse_low = INVERSES_LOW[usize::from(k)];
    let (quotient_low, remainder) = divide_below_power(carried, low_half, reciprocal, inverse_low);
    Some((quotient_high, quotient_low, remainder, power))
}

/// `(high_word:low_half) / 10^k` and the remainder, for a `high_word` below the power, so the
/// quotient fits 128 bits, by the power's reciprocal and the low word of its two-word inverse.
#[inline]
const fn divide_below_power(
    high_word: u64, low_half: u128, reciprocal: Reciprocal, inverse_low: u64,
) -> (u128, u64) {
    let Reciprocal { divisor, inverse, shift } = reciprocal;
    let (n1, n0) = (high(low_half), u64::low_bits(low_half));
    // The three live words, shifted as one number: each takes the bits the next one shifts out.
    let (u2, _) = shifted(high_word, n1, shift);
    let (u1, u0) = shifted(n1, n0, shift);
    let (quotient, remainder) = divide_three_by_one(u2, u1, u0, divisor, inverse, inverse_low);
    (quotient, remainder >> shift)
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
    use rstest::rstest;

    use super::{
        INVERSES_LOW, RECIPROCALS, divide_u127, divide_u128, divide_u128_by_short_power,
        divide_u128_past_a_word, divide_u256,
    };
    use crate::word::U256;

    /// A remainder at an edge of its range: zero, one, or one below the power.
    #[derive(Debug, Clone, Copy)]
    enum Edge {
        Zero,
        One,
        Top,
    }

    /// `10^k`.
    fn power(k: u8) -> u128 {
        10_u128.pow(u32::from(k))
    }

    #[test]
    fn a_short_power_refuses_what_its_steps_cannot_divide() {
        assert_eq!(divide_u128_by_short_power(power(9) << 64, 9), None, "a quotient past a word");
        assert_eq!(divide_u128_by_short_power(1, 10), None, "10^10 is past 2^32");
        assert_eq!(divide_u128_by_short_power(1, 0), None, "1's multiplier is past a word");
    }

    #[test]
    fn the_largest_numerator_whose_quotient_fits_a_word_divides_in_two_steps() {
        let largest = ((power(9) - 1) << 64) | u128::from(u64::MAX);
        let expected =
            (u64::try_from(largest / power(9)).ok(), u64::try_from(largest % power(9)).ok());
        assert_eq!(
            divide_u128_by_short_power(largest, 9).map(|(q, r, _)| (Some(q), Some(r))),
            Some(expected),
            "its quotient and remainder are `u128` division's"
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
    fn every_two_word_inverse_is_its_definition() {
        // ⌊(2^192 − 1) / d⌋ by long division a word at a time: its top word is 1, which the
        // inverse drops, and the next two are the one-word inverse and the low word.
        for (k, (reciprocal, low)) in (0_u8..).zip(RECIPROCALS.into_iter().zip(INVERSES_LOW)) {
            let d = u128::from(reciprocal.divisor);
            let mut remainder = 0;
            let mut words = [0; 3];
            for word in &mut words {
                let numerator = (remainder << 64) | u128::from(u64::MAX);
                *word = numerator / d;
                remainder = numerator % d;
            }
            let expected = [1, u128::from(reciprocal.inverse), u128::from(low)];
            assert_eq!(words, expected, "10^{k}'s two-word inverse");
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
    fn a_wide_numerator_divides_by_a_power_whatever_its_quotient() {
        let n = U256::widening(u128::MAX, 10_u128.pow(18));
        let (high, low) = n.halves();
        let power = 10_u64.pow(18);
        assert_eq!(
            divide_u256(high, low, 18),
            Some((0, u128::MAX, 0, power)),
            "(2^128 − 1) × 10^18 / 10^18"
        );
        assert_eq!(
            divide_u256(10_u128.pow(18), 0, 18),
            Some((1, 0, 0, power)),
            "a quotient of 2^128"
        );
        assert_eq!(
            divide_u256(u128::MAX, u128::MAX, 19).map(|(high, _, _, _)| high),
            Some(u128::MAX / 10_u128.pow(19)),
            "(2^256 − 1) / 10^19, its high half"
        );
        assert_eq!(divide_u256(0, 7, 0), Some((0, 7, 0, 1)), "10^0");
        assert_eq!(divide_u256(0, 7, 20), None, "10^20 is past a word");
    }

    #[rstest]
    fn a_wide_numerator_divides_back_at_every_edge_of_its_quotient_and_remainder(
        #[values(0, 1, 9, 18, 19)] k: u8,
        #[values(0, 1, u128::from(u64::MAX), 1 << 64, u128::MAX >> 1, u128::MAX - 1, u128::MAX)]
        quotient: u128,
        #[values(Edge::Zero, Edge::One, Edge::Top)] edge: Edge,
    ) {
        // q × 10^k + r with r at 0, 1 and 10^k − 1: where the step's two corrections run, the one
        // taken about half the time and the rare one.
        let d = power(k);
        let remainder = match edge {
            Edge::Zero => 0,
            Edge::One => 1 % d,
            Edge::Top => d - 1,
        };
        let (high, low) = U256::widening(quotient, d).halves();
        let (low, carry) = low.overflowing_add(remainder);
        let high = high + u128::from(carry);
        let expected = (quotient, u64::try_from(remainder).unwrap_or(0));
        assert_eq!(
            divide_u256(high, low, k).map(|(top, q, r, _)| (top, q, r)),
            Some((0, expected.0, expected.1)),
            "{quotient} × 10^{k} + {remainder}"
        );
    }

    #[test]
    fn a_numerator_below_the_top_bit_divides_by_one_product_at_every_edge() {
        let top = (1_u128 << 127) - 1;
        for k in 1_u8..=19 {
            let d = power(k);
            let edges = [
                0,
                1,
                d - 1,
                d,
                d + 1,
                top,
                top - top % d,
                top - top % d - 1,
                d << 64,
                (d << 64) - 1,
            ];
            // 10^19 × 2^64 is past the top bit, outside the domain.
            for n in edges.into_iter().filter(|&n| n <= top) {
                let expected = Some((n / d, u64::try_from(n % d).unwrap_or(0)));
                assert_eq!(divide_u127(n, k), expected, "{n} / 10^{k}");
            }
        }
        assert_eq!(divide_u127(7, 0), None, "10^0's multiplier is past two words");
        assert_eq!(divide_u127(7, 20), None, "10^20 is past a word");
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
        fn a_short_power_divides_in_two_steps_as_division_does(high: u64, low: u64, k in 1_u8..=9) {
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
        fn a_numerator_below_the_top_bit_divides_as_division_does(n in 0_u128..1 << 127, k in 1_u8..=19) {
            let d = power(k);
            let expected = Some((n / d, u64::try_from(n % d).unwrap_or(0)));
            prop_assert_eq!(divide_u127(n, k), expected);
        }

        #[test]
        fn a_quotient_past_a_word_divides_as_division_does(n: u128, k in 0_u8..=19) {
            let d = power(k);
            let expected = Some((n / d, u64::try_from(n % d).unwrap_or(0)));
            prop_assert_eq!(divide_u128_past_a_word(n, k).map(|(q, r, _)| (q, r)), expected);
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
            let expected = Some((0, (qa << 64) | qb, u64::try_from(rb).unwrap_or(0)));
            prop_assert_eq!(divide_u256(high, low_half, k).map(|(top, q, r, _)| (top, q, r)), expected);
            let past = Some((1, low_half / d, u64::try_from(low_half % d).unwrap_or(0)));
            prop_assert_eq!(divide_u256(d, low_half, k).map(|(top, q, r, _)| (top, q, r)), past, "2^128 and the rest");
        }

        #[test]
        fn a_three_word_numerator_divides_back_to_its_quotient_and_remainder(
            quotient: u128, remainder: u64, k in 0_u8..=19,
        ) {
            // q × 10^k + r for every r below the power, near it as often as near 0.
            let d = power(k);
            let remainder = u128::from(remainder) % d;
            let (high, low) = U256::widening(quotient, d).halves();
            let (low, carry) = low.overflowing_add(remainder);
            let high = high + u128::from(carry);
            let expected = Some((0, quotient, u64::try_from(remainder).unwrap_or(0)));
            prop_assert_eq!(divide_u256(high, low, k).map(|(top, q, r, _)| (top, q, r)), expected);
        }
    }
}
