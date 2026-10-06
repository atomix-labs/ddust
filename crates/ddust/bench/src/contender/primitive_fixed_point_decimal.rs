//! `primitive_fixed_point_decimal` 1.5.0: `ConstScaleFpdec<I, S>`, any integer at a scale known at
//! compile time, as ddust's `Fixed` is.
//!
//! Its rounding is a mode passed to each call, with no half to even: `Rounding::Round` is half away
//! from zero, which every rounded row here uses. Its `f64` conversions multiply or divide by a
//! power of ten in floating point, so neither is correctly rounded.

use primitive_fixed_point_decimal::{ConstScaleFpdec, Rounding};

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound,
    Parse, RescaleRound, ToF64, cents, format_by_display, low_byte,
};
use crate::input::Width;
use crate::oracle::Mode;

/// Implements every operation for `ConstScaleFpdec<$integer, $decimals>`.
macro_rules! fpdec {
    ($(#[$doc:meta])* $name:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("primitive_fixed_point_decimal 1.5.0 ", $label);
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = ConstScaleFpdec<$integer, $decimals>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(ConstScaleFpdec::from_mantissa(<$integer>::try_from(steps).ok()?))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(value.mantissa()))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.mantissa().cast_unsigned()))
            }
        }

        /// `checked_add`.
        impl CheckedAdd for $name {
            fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_add(*b)
            }
        }

        /// `<`.
        impl Compare for $name {
            fn is_less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `checked_mul_ext` at the same scale, one rounding.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul_ext::<$integer, $decimals, $decimals>(*b, Rounding::Round)
            }
        }

        /// `checked_div_ext` at the same scale, one rounding.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div_ext::<$integer, $decimals, $decimals>(*b, Rounding::Round)
            }
        }

        /// `round_ext` to 2 decimals, at the same scale.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::HalfExpand;
            type Rounded = ConstScaleFpdec<$integer, $decimals>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.round_ext(2, Rounding::Round))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                cents(i128::from(rounded.mantissa()), $decimals)
            }
        }

        /// `FromStr`, which refuses a digit past the scale.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                text.parse().ok()
            }
        }

        /// `Display`, the shortest form.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                format_by_display(value, buffer);
            }
        }

        /// `From<ConstScaleFpdec> for f64`: the steps divided by a power of ten in floating point.
        impl ToF64 for $name {
            const EXACT: bool = false;

            fn to_f64(value: &Self::Value) -> f64 {
                f64::from(*value)
            }
        }

        /// `TryFrom<f64>`: the double times a power of ten in floating point, rounded.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                ConstScaleFpdec::try_from(x).ok()
            }
        }
    };
}

fpdec!(
    /// `ConstScaleFpdec<i64, 8>`.
    Narrow,
    i64,
    8,
    Width::Narrow,
    "<i64, 8>"
);
fpdec!(
    /// `ConstScaleFpdec<i128, 18>`.
    Wide,
    i128,
    18,
    Width::Wide,
    "<i128, 18>"
);

/// `ConstScaleFpdec<i64, 2> × ConstScaleFpdec<i64, 5>`: `checked_mul_ext` to scale 7, where
/// nothing is rounded.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "primitive_fixed_point_decimal 1.5.0 <i64, 2> × <i64, 5>";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Price = ConstScaleFpdec<i64, 2>;
    type Quantity = ConstScaleFpdec<i64, 5>;
    type Product = ConstScaleFpdec<i64, 7>;

    fn price(steps: i128) -> Option<Self::Price> {
        Some(ConstScaleFpdec::from_mantissa(i64::try_from(steps).ok()?))
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        Some(ConstScaleFpdec::from_mantissa(i64::try_from(steps).ok()?))
    }

    fn checked_mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        price.checked_mul_ext::<i64, 5, 7>(*quantity, Rounding::Round)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        Some(i128::from(product.mantissa()))
    }
}
