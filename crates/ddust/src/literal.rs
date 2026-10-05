//! Literals checked at compile time: [`dec!`](crate::dec!) reads its number's text exactly in a
//! `const fn`, when the constant is evaluated.

use crate::decimal::Decimal;
use crate::int::Int;
use crate::scale::{Scale, StaticScale};

/// A literal's text at `decimals`: its sign and magnitude. A string literal's quotes, a sign,
/// Rust's digit separators and an exponent are allowed; anything the decimal cannot hold exactly
/// fails the build.
#[expect(
    clippy::indexing_slicing,
    reason = "evaluated at compile time: an index past the end fails the build"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "evaluated at compile time: overflow fails the build"
)]
const fn magnitude(text: &str, decimals: u8) -> (bool, u128) {
    let bytes = text.as_bytes();
    let (mut at, mut end) = (0, bytes.len());
    if end >= 2 && bytes[0] == b'"' && bytes[end - 1] == b'"' {
        at = 1;
        end -= 1;
    }
    let negative = at < end && bytes[at] == b'-';
    if at < end && (bytes[at] == b'-' || bytes[at] == b'+') {
        at += 1;
    }
    let (mut value, mut digits, mut fraction, mut exponent) = (0_u128, 0_u32, 0_i32, 0_i32);
    let mut point = false;
    while at < end {
        let byte = bytes[at];
        at += 1;
        if byte == b'_' {
            continue;
        }
        if byte == b'.' {
            assert!(!point, "a literal has one point");
            point = true;
            continue;
        }
        if byte == b'e' || byte == b'E' {
            let negative = at < end && bytes[at] == b'-';
            if at < end && (bytes[at] == b'-' || bytes[at] == b'+') {
                at += 1;
            }
            assert!(at < end, "an exponent has digits");
            while at < end {
                assert!(bytes[at].is_ascii_digit(), "an exponent is digits");
                exponent = exponent * 10 + i32::from(bytes[at] - b'0');
                at += 1;
            }
            if negative {
                exponent = -exponent;
            }
            break;
        }
        assert!(byte.is_ascii_digit(), "a literal is a decimal number, as `-12.34` or `1.5e3`");
        value = value * 10 + u128::from(byte - b'0');
        digits += 1;
        if point {
            fraction += 1;
        }
    }
    assert!(digits > 0, "a literal has digits");
    let power = i32::from(decimals) + exponent - fraction;
    if power >= 0 {
        (negative, value * 10_u128.pow(power.cast_unsigned()))
    } else {
        let drop = 10_u128.pow(power.unsigned_abs());
        assert!(value % drop == 0, "the literal has more decimals than its scale");
        (negative, value / drop)
    }
}

// Always-const bounds, not `[const]` ones: a crate that enables no nightly feature then calls it in
// the constant `dec!` expands to.
impl<I: const Int, S: StaticScale + const Scale> Decimal<I, S> {
    /// The decimal a literal spells, checked when the constant is evaluated: what
    /// [`dec!`](crate::dec!) expands to.
    #[doc(hidden)]
    #[must_use]
    pub const fn __literal(text: &str) -> Self {
        let (negative, magnitude) = magnitude(text, S::DECIMALS);
        let steps = I::from_magnitude(negative, magnitude);
        assert!(
            !negative || magnitude == 0 || I::MIN < I::ZERO,
            "an unsigned decimal is never negative"
        );
        assert!(steps.is_some(), "the literal is past the decimal's range");
        match steps {
            Some(steps) => Self::from_bits(steps, S::INSTANCE),
            None => Self::ZERO,
        }
    }
}

/// A decimal from a literal, checked when the constant is evaluated: its type comes from where it
/// goes, or is named after a colon, as `fixed`'s literals are.
///
/// A literal with more decimals than its scale, past its range, or negative for an unsigned
/// decimal fails the build; the value is the literal exactly, never rounded.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// const FEE: D64<4> = dec!(0.0025);
/// let price: D64<2> = dec!(60_000.37);
/// assert_eq!(price.mul_round(FEE, ddust::round::Ceil), dec!(150.01), "the type from its place");
/// assert_eq!(dec!(-1.5: D64<1>).to_string(), "-1.5", "or named");
/// ```
///
/// A literal the type cannot hold fails the build:
///
/// ```compile_fail,E0080
/// let cents: ddust::D64<2> = ddust::dec!(0.125);
/// ```
#[macro_export]
macro_rules! dec {
    (- $value:literal : $t:ty) => {
        const { <$t>::__literal(concat!("-", stringify!($value))) }
    };
    ($value:literal : $t:ty) => {
        const { <$t>::__literal(stringify!($value)) }
    };
    (- $value:literal) => {
        const { $crate::Decimal::<_, _>::__literal(concat!("-", stringify!($value))) }
    };
    ($value:literal) => {
        const { $crate::Decimal::<_, _>::__literal(stringify!($value)) }
    };
}

#[cfg(test)]
mod tests {
    use crate::{D64, D128, Decimal, Fixed, UD64};

    #[test]
    fn a_literal_is_its_exact_decimal() {
        const PRICE: D64<7> = dec!(60_000.5);
        const SHORT: D64<7> = dec!(-37.63);
        const TINY: D64<17> = dec!(1.2e-5);
        const QUOTED: UD64<11> = dec!("0.5");
        const WIDE: D128<18> = dec!(-1.5);
        const NAMED: Decimal<i64, Fixed<2>> = dec!(12.34: D64<2>);
        assert_eq!(PRICE.to_bits(), 600_005_000_000, "seven decimals");
        assert_eq!(SHORT.to_bits(), -376_300_000, "negative");
        assert_eq!(TINY.to_bits(), 1_200_000_000_000, "an exponent");
        assert_eq!(QUOTED.to_bits(), 50_000_000_000, "a string literal, unsigned");
        assert_eq!(WIDE.to_bits(), -1_500_000_000_000_000_000, "eighteen decimals");
        assert_eq!(NAMED.to_bits(), 1_234, "named after a colon");
    }
}
