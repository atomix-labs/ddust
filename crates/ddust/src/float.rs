//! `f64`, correctly rounded both ways: a decimal to its nearest double, and a double's exact value
//! at a scale, rounded once by a mode.

use crate::decimal::Decimal;
use crate::int::Int;
use crate::round::RoundingMode;
use crate::scale::Scale;
use crate::{kernel, reciprocal};

/// `10^k` for `k` in `0..=22`: every power of ten an `f64` holds exactly.
#[expect(clippy::indexing_slicing, reason = "a const loop within the table's own length")]
const POW10_F64: [f64; 23] = {
    let mut table = [1.0; 23];
    let mut k = 1;
    while k < table.len() {
        // Exact: 10^22 is the last power below 2^53 · 2^22 that a double holds.
        table[k] = table[k - 1] * 10.0;
        k += 1;
    }
    table
};

/// The largest magnitude an `f64` holds every integer up to: `2^53`.
const EXACT_IN_F64: u128 = 1 << 53;

/// The nearest `f64` to `magnitude × 10^-decimals`, ties to even, for a magnitude that is not
/// zero: the general path, where the steps or the power are past what a double holds exactly.
///
/// `10^d` is `5^d · 2^d`, so the value is `⌊magnitude · 2^s / 5^d⌋ · 2^-(s + d)` and a fraction,
/// for the `s` that gives the quotient 63 or 64 bits: rounded to 53 by the bits below them and the
/// remainder, it is the double. A decimal of at most 38 decimals is past neither end of a double,
/// so it is never subnormal nor infinite.
const fn nearest_f64(magnitude: u128, decimals: u8) -> f64 {
    let Some((quotient, shift, inexact)) = reciprocal::quotient_by_pow5(magnitude, decimals) else {
        return f64::NAN;
    };
    // 10 or 11 bits below the 53 kept.
    let dropped = 11_u32.wrapping_sub(quotient.leading_zeros());
    let kept = quotient >> dropped;
    let half = (quotient >> dropped.wrapping_sub(1)) & 1;
    // What is left once the kept bits and the half below them are shifted out.
    let below = quotient << 65_u32.wrapping_sub(dropped) != 0 || inexact;
    let up = half & u64::from(below || kept & 1 == 1);
    // The double kept · 2^e has the biased exponent e + 1075; the implicit bit of `kept` adds one
    // to it, and a carry out of rounding up one more, exactly as the double's next exponent does.
    let exponent = 1074_i32
        .wrapping_add_unsigned(dropped)
        .wrapping_sub(shift)
        .wrapping_sub(i32::from(decimals));
    let biased = match u64::try_from(exponent) {
        Ok(biased) => biased,
        Err(_below_zero) => 0,
    };
    f64::from_bits((biased << 52).wrapping_add(kept).wrapping_add(up))
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The nearest `f64`, ties to even: exact whenever the value is, and correctly rounded
    /// otherwise, as a decimal read from text is.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, D128, dec};
    ///
    /// let reading: D64<2> = dec!(21.37);
    /// assert_eq!(reading.to_f64(), 21.37, "the nearest double");
    /// let fine: D128<18> = dec!(3249036.838193733988751396);
    /// assert_eq!(fine.to_f64(), 3249036.838193734, "rounded once, not twice");
    /// ```
    #[must_use]
    pub fn to_f64(self) -> f64 {
        let (negative, magnitude) = self.steps().sign_and_magnitude();
        let decimals = self.decimals();
        let value = match POW10_F64.get(usize::from(decimals)) {
            // Both exact in a double, so one division rounds once: Clinger's fast path.
            Some(&power) if magnitude < EXACT_IN_F64 => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "below 2^53: exact"
                )]
                let magnitude = magnitude as f64;
                magnitude / power
            },
            _ if magnitude == 0 => 0.0,
            _ => nearest_f64(magnitude, decimals),
        };
        if negative { -value } else { value }
    }

    /// The decimal at `scale` nearest the double's exact binary value by `mode`, or `None` for NaN,
    /// an infinity, or past the range.
    ///
    /// The double `0.1` is 0.1000000000000000055511151231257827…, so it is `0.1` at any scale up to
    /// 17 decimals by [`HalfEven`](crate::round::HalfEven), and `0.10000000000000001` at 17 by
    /// [`Ceil`](crate::round::Ceil).
    ///
    /// # Examples
    /// ```
    /// use ddust::round::{Floor, HalfEven};
    /// use ddust::{D64, Fixed, dec};
    ///
    /// assert_eq!(D64::<2>::from_f64(1.255, Fixed, HalfEven), Some(dec!(1.25)), "1.255 is just below");
    /// assert_eq!(D64::<7>::from_f64(57618.88379205, Fixed, HalfEven), Some(dec!(57618.8837921)));
    /// assert_eq!(D64::<2>::from_f64(-0.001, Fixed, Floor), Some(dec!(-0.01)), "floored");
    /// assert_eq!(D64::<2>::from_f64(f64::NAN, Fixed, HalfEven), None, "not a number");
    /// ```
    #[must_use]
    pub const fn from_f64<R>(x: f64, scale: S, mode: R) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        if !x.is_finite() {
            return None;
        }
        let bits = x.to_bits();
        let negative = bits >> 63 == 1;
        let biased = match i32::try_from((bits >> 52) & 0x7FF) {
            Ok(biased) => biased,
            Err(_out_of_range) => 0,
        };
        let fraction = bits & ((1 << 52) - 1);
        // A subnormal has no implicit leading one; every other double has.
        let (mantissa, exponent) = if biased == 0 {
            (fraction, -1074_i32)
        } else {
            (fraction | (1 << 52), biased.wrapping_sub(1075))
        };
        let Some(magnitude) =
            kernel::binary_at_scale(negative, mantissa, exponent, scale.decimals(), mode.table())
        else {
            return None;
        };
        match I::from_magnitude(negative, magnitude) {
            Some(steps) => Some(Self::from_steps(steps, scale)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString as _;

    use proptest::prelude::*;

    use crate::round::{Ceil, Floor, HalfEven, Trunc};
    use crate::{D64, D128, Decimal, Dynamic, Fixed};

    proptest! {
        #[test]
        fn a_decimal_converts_as_its_text_does(steps: i64, decimals in 0_u8..=18) {
            let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
            let text: f64 = format!("{steps}e-{decimals}").parse().expect("a number");
            prop_assert_eq!(value.to_f64().to_bits(), text.to_bits(), "correctly rounded");
        }

        #[test]
        fn a_wide_decimal_converts_as_its_text_does(steps: i128) {
            let value = D128::<18>::from_steps(steps, Fixed);
            let text: f64 = format!("{steps}e-18").parse().expect("a number");
            prop_assert_eq!(value.to_f64().to_bits(), text.to_bits(), "correctly rounded");
        }

        #[test]
        fn a_decimal_at_any_scale_converts_as_its_text_does(steps: i128, decimals in 0_u8..=38) {
            let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
            let text: f64 = format!("{steps}e-{decimals}").parse().expect("a number");
            prop_assert_eq!(value.to_f64().to_bits(), text.to_bits(), "correctly rounded");
        }

        #[test]
        fn a_double_converts_to_its_nearest_decimal(x in -9e11_f64..9e11) {
            // The double's exact value, through its full decimal expansion, truncated at 7 and
            // compared: Trunc must agree with the expansion's first seven decimals.
            let exact = format!("{x:.60}");
            let (whole, fraction) = exact.split_once('.').expect("a point");
            let truncated: D64<7> = format!("{whole}.{}", fraction.get(..7).expect("sixty digits")).parse().expect("a decimal");
            prop_assert_eq!(D64::<7>::from_f64(x, Fixed, Trunc), Some(truncated));
        }
    }

    #[test]
    fn a_tie_between_doubles_goes_to_the_even() {
        // (2^53 + 1) · 5^k at k decimals is (2^53 + 1) / 2^k, halfway between two doubles, and goes
        // down to the even; (2^53 + 3) / 2^k goes up to it. Past 27 the power takes two words.
        for k in 0_u8..=31 {
            let (power, scale) = (5_i128.pow(u32::from(k)), Dynamic::new(k).expect("at most 38"));
            let below = 9_007_199_254_740_992.0 / f64::from(1_u32 << k);
            let above = 9_007_199_254_740_996.0 / f64::from(1_u32 << k);
            assert_eq!(Decimal::from_steps(((1 << 53) + 1) * power, scale).to_f64(), below, "{k}");
            assert_eq!(Decimal::from_steps(((1 << 53) + 3) * power, scale).to_f64(), above, "{k}");
        }
        let least = D128::<0>::from_steps(i128::MIN, Fixed).to_f64();
        assert_eq!(least, -(2.0_f64.powi(127)), "all 128 bits");
        let text: f64 = "170141183460469231731687303715884105727e-38".parse().expect("a number");
        assert_eq!(D128::<38>::from_steps(i128::MAX, Fixed).to_f64(), text, "the widest power");
    }

    #[test]
    fn a_tie_in_binary_is_settled_by_the_exact_value() {
        let near = D64::<7>::from_f64(57_618.883_792_05, Fixed, HalfEven).map(D64::steps);
        assert_eq!(near, Some(576_188_837_921), "…05000092042: above the tie");
        let cents = D64::<2>::from_f64(96_988_942.685, Fixed, HalfEven).map(D64::steps);
        assert_eq!(cents, Some(9_698_894_269), "…68500002384: above the tie");
        assert_eq!(D64::<2>::from_f64(0.005, Fixed, Ceil).map(D64::steps), Some(1), "up");
        assert_eq!(D64::<2>::from_f64(-0.005, Fixed, Floor).map(D64::steps), Some(-1), "down");
        assert_eq!(D64::<2>::from_f64(1e30, Fixed, HalfEven), None, "past the range");
        assert_eq!(
            D64::<2>::from_f64(5e-324, Fixed, Ceil).map(D64::steps),
            Some(1),
            "the least subnormal, up"
        );
        assert_eq!(D64::<2>::from_steps(150, Fixed).to_f64().to_string(), "1.5", "and back");
    }
}
