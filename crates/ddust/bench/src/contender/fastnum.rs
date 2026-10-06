//! `fastnum` 0.7.5: `D64` and `D128`, decimal floats with a 64-bit or 128-bit coefficient, a scale
//! and a rounding mode carried in each value, beside sticky flags.
//!
//! It has no checked arithmetic: an operation returns a value whose flags say what happened, so the
//! sum here is refused when it rounded or left the finite numbers. Its product and quotient keep
//! what the coefficient holds, and `round` rounds that to the width's decimals: two roundings where
//! the first already dropped digits, which a 64-bit coefficient does for a product, a quotient or a
//! double of more than 19 digits. Its half-to-even mode looks at one discarded digit alone, so the
//! rounded rows use `HalfUp`, half away from zero, which it gets right. Its `to_f64` is not
//! correctly rounded past a 64-bit coefficient. Its `Display` writes an exponent below `10^-6`.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "a decimal float's operators, whose flags the adapter reads"
)]

use fastnum::bint::UInt;
use fastnum::decimal::{Context, RoundingMode, Sign};
use fastnum::{D64, D128};

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound,
    Parse, RescaleRound, ToF64, format_by_display, low_byte, steps_of,
};
use crate::input::Width;
use crate::oracle::Mode;

/// The sign of `steps`, as fastnum names it.
const fn sign(steps: i128) -> Sign {
    if steps < 0 { Sign::Minus } else { Sign::Plus }
}

/// Implements every operation for `$type`, its coefficient built by `$digits`.
macro_rules! fastnum {
    (
        $(#[$doc:meta])* $name:ident, $type:ty, $decimals:literal, $width:expr, $label:literal, $digits:expr,
        exact: product $product:literal, quotient $quotient:literal, to $to:literal, from $from:literal
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl $name {
            /// The steps at `decimals` of `value`, from its coefficient and its scale, its flags aside.
            fn steps(value: &$type, decimals: u8) -> Option<i128> {
                if !value.is_finite() {
                    return None;
                }
                let magnitude = i128::try_from(value.digits().to_u128().ok()?).ok()?;
                let digits = if value.is_negative() { magnitude.checked_neg()? } else { magnitude };
                steps_of(digits, i64::from(value.fractional_digits_count()), decimals)
            }
        }

        impl Contender for $name {
            const NAME: &'static str = concat!("fastnum 0.7.5 ", $label);
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = $type;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                let digits = $digits(steps.unsigned_abs())?;
                Some(<$type>::from_parts(digits, -$decimals, sign(steps), Context::default()))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Self::steps(value, $decimals)
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(value.digits().to_u128().unwrap_or(0))
            }
        }

        /// `+`, refused where the flags say it rounded or overflowed.
        impl CheckedAdd for $name {
            fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                let sum = *a + *b;
                (sum.is_finite() && !sum.is_op_rounded()).then_some(sum)
            }
        }

        /// `<`.
        impl Compare for $name {
            fn is_less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `*`, then `round` with `HalfUp`.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = $product;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                let product = (*a * *b).with_rounding_mode(RoundingMode::HalfUp).round($decimals);
                product.is_finite().then_some(product)
            }
        }

        /// `/`, then `round` with `HalfUp`.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = $quotient;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                let quotient = (*a / *b).with_rounding_mode(RoundingMode::HalfUp).round($decimals);
                quotient.is_finite().then_some(quotient)
            }
        }

        /// `round(2)` with `HalfUp`.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::HalfExpand;
            type Rounded = $type;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.with_rounding_mode(RoundingMode::HalfUp).round(2))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                Self::steps(rounded, 2)
            }
        }

        /// `from_str`, which never rounds, and a check of the decimals it read.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                <$type>::from_str(text, Context::default())
                    .ok()
                    .filter(|value| value.is_finite() && value.fractional_digits_count() <= $decimals)
            }
        }

        /// `Display`, which allocates, and writes every decimal of the scale.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                format_by_display(value, buffer);
            }
        }

        /// `to_f64`.
        impl ToF64 for $name {
            const EXACT: bool = $to;

            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64()
            }
        }

        /// `from_f64`, to the coefficient's digits, then `round` with `HalfUp`.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = $from;

            fn from_f64(x: f64) -> Option<Self::Value> {
                let value = <$type>::from_f64(x).with_rounding_mode(RoundingMode::HalfUp).round($decimals);
                value.is_finite().then_some(value)
            }
        }
    };
}

fastnum!(
    /// `D64`, at 8 decimals.
    Narrow,
    D64,
    8,
    Width::Narrow,
    "D64",
    |magnitude: u128| u64::try_from(magnitude).ok().map(UInt::<1>::from_u64),
    exact: product false, quotient false, to true, from false
);
fastnum!(
    /// `D128`, at 18 decimals.
    Wide,
    D128,
    18,
    Width::Wide,
    "D128",
    |magnitude: u128| UInt::<2>::from_u128(magnitude).ok(),
    exact: product true, quotient true, to false, from true
);

/// `D64` at scale 2 times `D64` at scale 5: `*`, whose scale is 7, refused where it rounded.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "fastnum 0.7.5 D64";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Price = D64;
    type Quantity = D64;
    type Product = D64;

    fn price(steps: i128) -> Option<Self::Price> {
        let digits = UInt::<1>::from_u64(u64::try_from(steps.unsigned_abs()).ok()?);
        Some(D64::from_parts(digits, -2, sign(steps), Context::default()))
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        let digits = UInt::<1>::from_u64(u64::try_from(steps.unsigned_abs()).ok()?);
        Some(D64::from_parts(digits, -5, sign(steps), Context::default()))
    }

    fn checked_mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        let product = *price * *quantity;
        (product.is_finite() && !product.is_op_rounded()).then_some(product)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        Narrow::steps(product, 7)
    }
}
