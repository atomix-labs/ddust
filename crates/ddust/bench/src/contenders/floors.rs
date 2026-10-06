//! The floors: the plain integer or float operation a decimal's is built on, the cost no decimal
//! goes below. Each is the bare instruction's semantics, not a decimal's, so the oracle does not
//! check them.

#![expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a floor is the plain cast the hardware does"
)]

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound, Parse,
    Rescale, ToF64, display, low_byte,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// Implements the integer floor over `$integer`.
macro_rules! integer {
    ($(#[$doc:meta])* $name:ident, $integer:ty, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = $label;
            const KIND: Kind = Kind::Floor;
            const WIDTH: Width = $width;
            type Value = $integer;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                <$integer>::try_from(steps).ok()
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(*value))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.cast_unsigned()))
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

        /// `checked_mul`, with no rescaling: the multiply beneath a decimal's.
        impl MulRound for $name {
            const MODE: Mode = Mode::Trunc;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul(*b)
            }
        }

        /// `checked_div`, by a divisor known at run time.
        impl DivRound for $name {
            const MODE: Mode = Mode::Trunc;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div(*b)
            }
        }

        /// A division by the constant power of ten between the decimals and 2.
        impl Rescale for $name {
            const MODE: Mode = Mode::Trunc;
            type Rounded = $integer;

            fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
                const POWER: $integer = (10 as $integer).pow(($width).decimals() as u32 - 2);
                Some(*value / POWER)
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                Some(i128::from(*rounded))
            }
        }

        /// `Display` of the steps, as long as the decimal's text.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                display(value, buffer);
            }
        }

        /// `as f64`.
        impl ToF64 for $name {
            fn to_f64(value: &Self::Value) -> f64 {
                *value as f64
            }
        }

        /// `as`, which saturates.
        impl FromF64 for $name {
            const MODE: Mode = Mode::Trunc;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Some(x as $integer)
            }
        }
    };
}

/// Implements the float floor at `$width`.
macro_rules! float {
    ($(#[$doc:meta])* $name:ident, $width:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = "f64";
            const KIND: Kind = Kind::Floor;
            const WIDTH: Width = $width;
            type Value = f64;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(steps as f64 / 10_f64.powi(i32::from(($width).decimals())))
            }

            fn to_steps(_value: &Self::Value) -> Option<i128> {
                None
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.to_bits()))
            }
        }

        /// `+`.
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

        /// `*`, rounded to the nearest double.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a * b)
            }
        }

        /// `/`, rounded to the nearest double.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a / b)
            }
        }

        /// `(x * 100).round() / 100`, which rounds twice.
        impl Rescale for $name {
            const MODE: Mode = Mode::HalfExpand;
            type Rounded = f64;

            fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
                Some((value * 100.0).round() / 100.0)
            }

            fn rounded_steps(_rounded: &Self::Rounded) -> Option<i128> {
                None
            }
        }

        /// `str::parse`, correctly rounded.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                text.parse().ok()
            }
        }

        /// `Display`, the shortest text that reads back.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                display(value, buffer);
            }
        }

        /// The value itself.
        impl ToF64 for $name {
            fn to_f64(value: &Self::Value) -> f64 {
                *value
            }
        }

        /// The double itself.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfEven;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Some(x)
            }
        }
    };
}

integer!(
    /// `i64`, its steps at 8 decimals.
    I64,
    i64,
    Width::Narrow,
    "i64"
);
integer!(
    /// `i128`, its steps at 18 decimals.
    I128,
    i128,
    Width::Wide,
    "i128"
);
float!(
    /// `f64`, beside the 64-bit decimals.
    F64Narrow,
    Width::Narrow
);
float!(
    /// `f64`, beside the 128-bit decimals.
    F64Wide,
    Width::Wide
);

/// The steps of a price and a quantity multiplied: `i64::checked_mul`.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "i64";
    const KIND: Kind = Kind::Floor;
    const WIDTH: Width = Width::Narrow;
    type Price = i64;
    type Quantity = i64;
    type Product = i64;

    fn price(steps: i128) -> Option<Self::Price> {
        i64::try_from(steps).ok()
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        i64::try_from(steps).ok()
    }

    fn mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        price.checked_mul(*quantity)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        Some(i128::from(*product))
    }
}
