//! `f64`, correctly rounded both ways: a decimal to its nearest double, and a double's exact value
//! at a scale, rounded once by a mode.

use core::str;

use crate::decimal::Decimal;
use crate::int::Int;
use crate::kernel;
use crate::round::RoundingMode;
use crate::scale::Scale;

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

/// The nearest `f64` to `magnitude × 10^-decimals`, by core's correctly rounded reading of its
/// digits: the general path, where the steps or the power are past what a double holds exactly.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "counts below the buffers' lengths, and division by ten"
)]
fn nearest_f64(magnitude: u128, decimals: u8) -> f64 {
    // At most 39 digits, then `e-` and the two digits of the decimals.
    let mut text = [0_u8; 48];
    let mut digits = [0_u8; 39];
    let (mut left, mut count) = (magnitude, 0);
    loop {
        let (rest, digit) = (left / 10, left % 10);
        if let Some(slot) = digits.get_mut(count) {
            *slot = b'0'.wrapping_add(u8::try_from(digit).unwrap_or_default());
        }
        count += 1;
        left = rest;
        if left == 0 {
            break;
        }
    }
    let mut len = 0;
    for &digit in digits.get(..count).unwrap_or_default().iter().rev() {
        if let Some(slot) = text.get_mut(len) {
            *slot = digit;
        }
        len += 1;
    }
    for byte in [b'e', b'-', b'0'.wrapping_add(decimals / 10), b'0'.wrapping_add(decimals % 10)] {
        if let Some(slot) = text.get_mut(len) {
            *slot = byte;
        }
        len += 1;
    }
    text.get(..len)
        .and_then(|text| str::from_utf8(text).ok())
        .and_then(|text| text.parse().ok())
        .unwrap_or(f64::NAN)
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
        let Some(magnitude) = magnitude.to_u128() else { return None };
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
