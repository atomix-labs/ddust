//! The exact answer to every operation the suite times, in big integers that neither overflow nor
//! round, against which each contender's result is checked before any is timed.
//!
//! It is written apart from ddust, on num-bigint, so it checks ddust as it checks the rest: a value
//! is its steps, an `i128`, at the decimals its width carries.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "a big integer's arithmetic, which neither overflows nor wraps"
)]

use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};

/// How a result that does not fit its decimals is rounded: the nine modes of ECMA-402, by the same
/// names ddust uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceil,
    /// Toward zero.
    Trunc,
    /// Away from zero.
    Expand,
    /// To the nearer, a tie toward negative infinity.
    HalfFloor,
    /// To the nearer, a tie toward positive infinity.
    HalfCeil,
    /// To the nearer, a tie toward zero.
    HalfTrunc,
    /// To the nearer, a tie away from zero.
    HalfExpand,
    /// To the nearer, a tie to the even neighbour.
    HalfEven,
}

impl Mode {
    /// The mode's name, as a table names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Trunc => "trunc",
            Self::Expand => "expand",
            Self::HalfFloor => "half-floor",
            Self::HalfCeil => "half-ceil",
            Self::HalfTrunc => "half-trunc",
            Self::HalfExpand => "half-expand",
            Self::HalfEven => "half-even",
        }
    }
}

/// `a + b`, or `None` past an `i128`.
#[must_use]
pub const fn add(a: i128, b: i128) -> Option<i128> {
    a.checked_add(b)
}

/// The order of `a` and `b`.
#[must_use]
pub fn compare(a: i128, b: i128) -> Ordering {
    a.cmp(&b)
}

/// The exact product of a price's steps at `2` decimals and a quantity's at `5`: steps at `7`.
#[must_use]
pub const fn mul_exact(price: i128, quantity: i128) -> Option<i128> {
    price.checked_mul(quantity)
}

/// `a * b`, both at `decimals`, at `decimals`, rounded by `mode`; `None` past an `i128`.
#[must_use]
pub fn mul_round(a: i128, b: i128, decimals: u8, mode: Mode) -> Option<i128> {
    narrow(divide(&(BigInt::from(a) * b), &power(decimals), mode))
}

/// `a / b`, both at `decimals`, at `decimals`, rounded by `mode`; `None` for a zero `b`, or past an
/// `i128`.
#[must_use]
pub fn div_round(a: i128, b: i128, decimals: u8, mode: Mode) -> Option<i128> {
    if b == 0 {
        return None;
    }
    narrow(divide(&(BigInt::from(a) * power(decimals)), &BigInt::from(b), mode))
}

/// `a`, at `from` decimals, at `to`, rounded by `mode` where `to` is the fewer; `None` past an
/// `i128`.
#[must_use]
pub fn rescale(a: i128, from: u8, to: u8, mode: Mode) -> Option<i128> {
    match from.cmp(&to) {
        Ordering::Greater => narrow(divide(&BigInt::from(a), &power(from - to), mode)),
        Ordering::Equal => Some(a),
        Ordering::Less => narrow(BigInt::from(a) * power(to - from)),
    }
}

/// The shortest exact decimal of `steps` at `decimals`: no trailing zero after the point, and no
/// point for a whole number.
#[must_use]
pub fn text(steps: i128, decimals: u8) -> String {
    let digits = steps.unsigned_abs().to_string();
    let decimals = usize::from(decimals);
    let padded = format!("{digits:0>width$}", width = decimals + 1);
    let (whole, fraction) = padded.split_at(padded.len() - decimals);
    let fraction = fraction.trim_end_matches('0');
    let sign = if steps < 0 { "-" } else { "" };
    if fraction.is_empty() { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") }
}

/// The steps at `decimals` of `text`, exactly: a sign, digits with an optional point, and an
/// optional exponent, as `1.5`, `-0.25` or `3.4E-7`; `None` for anything else, or for a digit past
/// `decimals`.
#[must_use]
pub fn parse(text: &str, decimals: u8) -> Option<i128> {
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(at) => (text.get(..at)?, text.get(at + 1..)?.parse::<i64>().ok()?),
        None => (text, 0),
    };
    let (negative, unsigned) = mantissa.strip_prefix('-').map_or_else(
        || (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        |rest| (true, rest),
    );
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digit = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() && fraction.is_empty() || !digit(whole) || !digit(fraction) {
        return None;
    }
    // The digits are a whole number at `fraction.len() - exponent` decimals.
    let digits: BigInt = format!("{whole}{fraction}").parse().ok()?;
    let digits = if negative { -digits } else { digits };
    let scale = i64::try_from(fraction.len()).ok()? - exponent;
    let target = i64::from(decimals);
    let steps = if scale <= target {
        digits * BigInt::from(10).pow(u32::try_from(target - scale).ok()?)
    } else {
        let divisor = BigInt::from(10).pow(u32::try_from(scale - target).ok()?);
        if (&digits % &divisor).sign() != Sign::NoSign {
            return None;
        }
        digits / divisor
    };
    narrow(steps)
}

/// `steps` at `decimals` as the nearest `f64`, a tie to the even one: core's reading of the exact
/// text, which is correctly rounded.
#[must_use]
pub fn to_f64(steps: i128, decimals: u8) -> f64 {
    text(steps, decimals).parse().unwrap_or(f64::NAN)
}

/// The steps at `decimals` of `x`'s exact binary value, rounded by `mode`; `None` for a NaN, an
/// infinity, or past an `i128`.
#[must_use]
pub fn from_f64(x: f64, decimals: u8, mode: Mode) -> Option<i128> {
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let biased = i32::try_from((bits >> 52) & 0x7FF).ok()?;
    let fraction = bits & ((1 << 52) - 1);
    // x = significand · 2^exponent, the subnormals without the hidden bit.
    let (significand, exponent) =
        if biased == 0 { (fraction, -1074) } else { (fraction | (1 << 52), biased - 1075) };
    let signed =
        if x.is_sign_negative() { -BigInt::from(significand) } else { BigInt::from(significand) };
    let scaled = signed * power(decimals);
    let steps = if exponent >= 0 {
        scaled << usize::try_from(exponent).ok()?
    } else {
        divide(&scaled, &(BigInt::from(1) << usize::try_from(-exponent).ok()?), mode)
    };
    narrow(steps)
}

/// `10^decimals`.
fn power(decimals: u8) -> BigInt {
    BigInt::from(10).pow(u32::from(decimals))
}

/// `value`, if an `i128` holds it.
fn narrow(value: BigInt) -> Option<i128> {
    i128::try_from(value).ok()
}

/// `numerator / denominator`, rounded by `mode`.
fn divide(numerator: &BigInt, denominator: &BigInt, mode: Mode) -> BigInt {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder.sign() == Sign::NoSign {
        return quotient;
    }
    let negative = (numerator.sign() == Sign::Minus) != (denominator.sign() == Sign::Minus);
    let half = (remainder.magnitude() * 2_u8).cmp(denominator.magnitude());
    let odd = quotient.bit(0);
    let away = match mode {
        Mode::Trunc => false,
        Mode::Expand => true,
        Mode::Floor => negative,
        Mode::Ceil => !negative,
        Mode::HalfFloor => half == Ordering::Greater || half == Ordering::Equal && negative,
        Mode::HalfCeil => half == Ordering::Greater || half == Ordering::Equal && !negative,
        Mode::HalfTrunc => half == Ordering::Greater,
        Mode::HalfExpand => half != Ordering::Less,
        Mode::HalfEven => half == Ordering::Greater || half == Ordering::Equal && odd,
    };
    match (away, negative) {
        (false, _) => quotient,
        (true, false) => quotient + 1,
        (true, true) => quotient - 1,
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::Ordering;

    use rstest::rstest;

    use super::{Mode, compare, div_round, from_f64, mul_round, parse, rescale, text, to_f64};

    /// Every mode on the ties and near-ties of ECMA-402's table, at one decimal to none.
    #[rstest]
    #[case::floor(Mode::Floor, [1, 1, 2, -2, -2, -3])]
    #[case::ceil(Mode::Ceil, [2, 2, 3, -1, -1, -2])]
    #[case::trunc(Mode::Trunc, [1, 1, 2, -1, -1, -2])]
    #[case::expand(Mode::Expand, [2, 2, 3, -2, -2, -3])]
    #[case::half_floor(Mode::HalfFloor, [1, 1, 3, -1, -2, -3])]
    #[case::half_ceil(Mode::HalfCeil, [1, 2, 3, -1, -1, -3])]
    #[case::half_trunc(Mode::HalfTrunc, [1, 1, 3, -1, -1, -3])]
    #[case::half_expand(Mode::HalfExpand, [1, 2, 3, -1, -2, -3])]
    #[case::half_even(Mode::HalfEven, [1, 2, 3, -1, -2, -3])]
    fn every_mode_rounds_as_ecma_402_says(#[case] mode: Mode, #[case] expected: [i128; 6]) {
        let rounded = [14, 15, 26, -14, -15, -26]
            .map(|tenths| rescale(tenths, 1, 0, mode).expect("in range"));
        assert_eq!(rounded, expected, "{mode:?}: 1.4, 1.5, 2.6, -1.4, -1.5, -2.6");
    }

    #[test]
    fn a_tie_to_even_looks_at_the_quotient() {
        assert_eq!(rescale(25, 1, 0, Mode::HalfEven), Some(2), "2.5 to 2");
        assert_eq!(rescale(35, 1, 0, Mode::HalfEven), Some(4), "3.5 to 4");
    }

    #[test]
    fn a_product_and_a_quotient_round_once() {
        // 1.5 × 1.5 = 2.25 at one decimal: 2.2 half-even, 2.3 half-expand.
        assert_eq!(mul_round(15, 15, 1, Mode::HalfEven), Some(22), "a tie to even");
        assert_eq!(mul_round(15, 15, 1, Mode::HalfExpand), Some(23), "a tie away");
        // 1 / 3 at two decimals, and -2 / 3.
        assert_eq!(div_round(100, 300, 2, Mode::HalfEven), Some(33), "0.33");
        assert_eq!(div_round(-200, 300, 2, Mode::HalfEven), Some(-67), "-0.67");
        assert_eq!(div_round(1, 0, 2, Mode::HalfEven), None, "no quotient by zero");
        assert_eq!(mul_round(i128::MAX, 10, 0, Mode::Trunc), None, "past an i128");
    }

    #[rstest]
    #[case::whole(1_200_000_000, 8, "12")]
    #[case::fraction(1_234_500_000, 8, "12.345")]
    #[case::small(-5, 8, "-0.00000005")]
    #[case::zero(0, 8, "0")]
    fn a_value_is_written_shortest_and_read_back(
        #[case] steps: i128, #[case] decimals: u8, #[case] written: &str,
    ) {
        assert_eq!(text(steps, decimals), written, "{steps}");
        assert_eq!(parse(written, decimals), Some(steps), "{written}");
    }

    #[rstest]
    #[case::trailing_zeros("12.50000000000", Some(1_250_000_000))]
    #[case::plus("+1", Some(100_000_000))]
    #[case::bare_point(".5", Some(50_000_000))]
    #[case::too_fine("0.000000001", None)]
    #[case::exponent("1e3", Some(100_000_000_000))]
    #[case::negative_exponent("-3.4E-7", Some(-34))]
    #[case::exponent_too_fine("1e-9", None)]
    #[case::empty("", None)]
    #[case::lone_point(".", None)]
    fn text_reads_exactly_or_not_at_all(#[case] written: &str, #[case] steps: Option<i128>) {
        assert_eq!(parse(written, 8), steps, "{written:?}");
    }

    #[test]
    fn a_double_converts_exactly_both_ways() {
        assert_eq!(to_f64(10_000_000, 8), 0.1, "0.1 is the nearest double to it");
        // 0.1 is 0.1000000000000000055511151231257827…, below a tie at 17 decimals and above it at
        // none.
        assert_eq!(
            from_f64(0.1, 18, Mode::Trunc),
            Some(100_000_000_000_000_005),
            "its exact digits"
        );
        assert_eq!(from_f64(0.1, 8, Mode::HalfEven), Some(10_000_000), "at 8 decimals, 0.1");
        assert_eq!(from_f64(-2.5, 0, Mode::HalfEven), Some(-2), "a tie, to even");
        assert_eq!(from_f64(f64::NAN, 8, Mode::HalfEven), None, "no NaN");
        assert_eq!(from_f64(5e-324, 8, Mode::Ceil), Some(1), "the least subnormal, up a step");
    }

    #[test]
    fn order_is_the_steps() {
        assert_eq!(compare(-1, 1), Ordering::Less, "below");
    }
}
