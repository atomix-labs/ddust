//! `bigdecimal` 0.4.11: `BigDecimal`, a big integer and a scale, on the heap: every operation
//! allocates, and none overflows.
//!
//! Its product is exact, and `with_scale_round` rounds it once, half to even. Its quotient runs
//! long division to 100 digits, rounding the last half up, before `with_scale_round` rounds again.
//! Its text is written by `write_plain_string`, which never uses an exponent, as `Display` does
//! below `10^-6`.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "a big decimal's operators, which neither overflow nor wrap"
)]

use bigdecimal::{BigDecimal, RoundingMode, ToPrimitive as _};
use num_bigint::BigInt;

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound, Parse,
    Rescale, ToF64, low_byte, steps_of,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// The steps at `decimals` of `value`.
fn steps(value: &BigDecimal, decimals: u8) -> Option<i128> {
    let (digits, scale) = value.as_bigint_and_scale();
    steps_of(digits.to_i128()?, scale, decimals)
}

/// Implements every operation for `BigDecimal` at `$decimals`.
macro_rules! bigdecimal {
    ($(#[$doc:meta])* $name:ident, $decimals:literal, $width:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = "bigdecimal 0.4.11";
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = BigDecimal;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(BigDecimal::new(BigInt::from(steps), $decimals))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                steps(value, $decimals)
            }

            fn fingerprint(value: &Self::Value) -> usize {
                let (digits, _) = value.as_bigint_and_scale();
                low_byte(u128::from(digits.iter_u64_digits().next().unwrap_or(0)))
            }
        }

        /// `+` on references, exact.
        impl Add for $name {
            fn add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a + b)
            }
        }

        /// `<`.
        impl Compare for $name {
            fn less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `*`, exact, then `with_scale_round`.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some((a * b).with_scale_round($decimals, RoundingMode::HalfEven))
            }
        }

        /// `/`, to 100 digits, then `with_scale_round`.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some((a / b).with_scale_round($decimals, RoundingMode::HalfEven))
            }
        }

        /// `with_scale_round(2)`.
        impl Rescale for $name {
            const MODE: Mode = Mode::HalfEven;
            type Rounded = BigDecimal;

            fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.with_scale_round(2, RoundingMode::HalfEven))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                steps(rounded, 2)
            }
        }

        /// `FromStr`, which never rounds, and a check of the decimals it read.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                text.parse::<BigDecimal>().ok().filter(|value| value.fractional_digit_count() <= $decimals)
            }
        }

        /// `write_plain_string`, with no exponent.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                buffer.clear();
                let _fits = value.write_plain_string(buffer);
            }
        }

        /// `ToPrimitive::to_f64`, which reads its own text, correctly rounded.
        impl ToF64 for $name {
            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64().unwrap_or(f64::NAN)
            }
        }

        /// `TryFrom<f64>`, the double's exact value, then `with_scale_round`.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfEven;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Some(BigDecimal::try_from(x).ok()?.with_scale_round($decimals, RoundingMode::HalfEven))
            }
        }
    };
}

bigdecimal!(
    /// `BigDecimal` at 8 decimals.
    Narrow,
    8,
    Width::Narrow
);
bigdecimal!(
    /// `BigDecimal` at 18 decimals.
    Wide,
    18,
    Width::Wide
);

/// `BigDecimal` at scale 2 times one at scale 5: `*` on references, exact, at scale 7.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "bigdecimal 0.4.11";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Price = BigDecimal;
    type Quantity = BigDecimal;
    type Product = BigDecimal;

    fn price(steps: i128) -> Option<Self::Price> {
        Some(BigDecimal::new(BigInt::from(steps), 2))
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        Some(BigDecimal::new(BigInt::from(steps), 5))
    }

    fn mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        Some(price * quantity)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        steps(product, 7)
    }
}
