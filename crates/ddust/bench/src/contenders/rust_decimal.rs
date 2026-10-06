//! `rust_decimal` 1.43.0: one 128-bit type, a 96-bit integer and a scale of 0 to 28 carried in each
//! value, at 8 decimals and at 18.
//!
//! It has no rounded product or quotient in one call: `checked_mul` and `checked_div` keep every
//! digit that fits 28, and `round_dp_with_strategy` rounds that to the width's decimals, which
//! `MidpointNearestEven` does half to even. At 18 decimals a product has 36, so the first step
//! rounds it to 28 and the second to 18, and a value near a half can land a step off the product
//! rounded once; a quotient the same. Its `to_f64` is not correctly rounded, and its `from_f64`
//! starts from the double's shortest text rather than its exact value, so both differ from the
//! oracle's at either width.

use rust_decimal::prelude::{FromPrimitive as _, ToPrimitive as _};
use rust_decimal::{Decimal, RoundingStrategy};

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound, Parse,
    Rescale, ToF64, display, low_byte,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// The steps of `value` at `decimals`, if it has no digit past them.
fn steps_at(value: &Decimal, decimals: u32) -> Option<i128> {
    let scale = value.scale();
    let lift = 10_i128.checked_pow(decimals.checked_sub(scale)?)?;
    value.mantissa().checked_mul(lift)
}

/// Implements every operation for `rust_decimal` at `$decimals`.
macro_rules! rust_decimal {
    ($(#[$doc:meta])* $name:ident, $decimals:literal, $width:expr, $rounds_once:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = "rust_decimal 1.43.0";
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = Decimal;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Decimal::try_from_i128_with_scale(steps, $decimals).ok()
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                steps_at(value, $decimals)
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(value.mantissa().cast_unsigned())
            }
        }

        /// `checked_add`.
        impl Add for $name {
            fn add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_add(*b)
            }
        }

        /// `<`.
        impl Compare for $name {
            fn less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `checked_mul`, then `round_dp_with_strategy`.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfEven;
            const EXACT: bool = $rounds_once;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a.checked_mul(*b)?.round_dp_with_strategy($decimals, RoundingStrategy::MidpointNearestEven))
            }
        }

        /// `checked_div`, then `round_dp_with_strategy`.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfEven;
            const EXACT: bool = $rounds_once;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a.checked_div(*b)?.round_dp_with_strategy($decimals, RoundingStrategy::MidpointNearestEven))
            }
        }

        /// `round_dp_with_strategy` to 2 decimals.
        impl Rescale for $name {
            const MODE: Mode = Mode::HalfEven;
            type Rounded = Decimal;

            fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.round_dp_with_strategy(2, RoundingStrategy::MidpointNearestEven))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                steps_at(rounded, 2)
            }
        }

        /// `from_str_exact`, which refuses to round, and a check of the scale it read.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                Decimal::from_str_exact(text).ok().filter(|value| value.scale() <= $decimals)
            }
        }

        /// `Display`, which writes every decimal of the value's scale.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                display(value, buffer);
            }
        }

        /// `ToPrimitive::to_f64`, not correctly rounded.
        impl ToF64 for $name {
            const EXACT: bool = false;

            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64().unwrap_or(f64::NAN)
            }
        }

        /// `FromPrimitive::from_f64`, from the double's shortest text, then `round_dp_with_strategy`.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfEven;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Some(Decimal::from_f64(x)?.round_dp_with_strategy($decimals, RoundingStrategy::MidpointNearestEven))
            }
        }
    };
}

rust_decimal!(
    /// `rust_decimal` at 8 decimals, where a product's 16 fit its 28.
    Narrow,
    8,
    Width::Narrow,
    true
);
rust_decimal!(
    /// `rust_decimal` at 18 decimals, where a product's 36 round twice.
    Wide,
    18,
    Width::Wide,
    false
);

/// A price at scale 2 times a quantity at scale 5: `checked_mul`, whose scale is their sum.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "rust_decimal 1.43.0";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Price = Decimal;
    type Quantity = Decimal;
    type Product = Decimal;

    fn price(steps: i128) -> Option<Self::Price> {
        Decimal::try_from_i128_with_scale(steps, 2).ok()
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        Decimal::try_from_i128_with_scale(steps, 5).ok()
    }

    fn mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        price.checked_mul(*quantity)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        steps_at(product, 7)
    }
}
