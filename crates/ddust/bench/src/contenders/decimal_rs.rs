//! `decimal-rs` 0.2.0: `Decimal`, up to 38 significant digits of a `u128` with a scale and a sign
//! carried in each value.
//!
//! Its rounding is half away from zero, with no mode to choose: `round` gives the width's decimals
//! after `checked_mul` or `checked_div`, each of which keeps what fits 38 digits. Its `from_f64`
//! cuts the double to 17 significant digits, half up, before `round` rounds again.

use decimal_rs::Decimal;

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound, Parse,
    Rescale, ToF64, display, low_byte, steps_of,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// The value of `steps` steps at `decimals`.
fn value(steps: i128, decimals: i16) -> Option<Decimal> {
    Decimal::from_parts(steps.unsigned_abs(), decimals, steps < 0).ok()
}

/// The steps at `decimals` of `value`.
fn steps(value: &Decimal, decimals: u8) -> Option<i128> {
    let (magnitude, scale, negative) = value.into_parts();
    let magnitude = i128::try_from(magnitude).ok()?;
    steps_of(
        if negative { magnitude.checked_neg()? } else { magnitude },
        i64::from(scale),
        decimals,
    )
}

/// Implements every operation for `Decimal` at `$decimals`.
macro_rules! decimal_rs {
    ($(#[$doc:meta])* $name:ident, $decimals:literal, $width:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = "decimal-rs 0.2.0";
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = Decimal;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                value(steps, $decimals)
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                steps(value, $decimals)
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(value.into_parts().0)
            }
        }

        /// `checked_add`.
        impl Add for $name {
            fn add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_add(b)
            }
        }

        /// `<`.
        impl Compare for $name {
            fn less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `checked_mul`, then `round`.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a.checked_mul(b)?.round($decimals))
            }
        }

        /// `checked_div`, to 38 digits, then `round`.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                Some(a.checked_div(b)?.round($decimals))
            }
        }

        /// `round(2)`.
        impl Rescale for $name {
            const MODE: Mode = Mode::HalfExpand;
            type Rounded = Decimal;

            fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.round(2))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                steps(rounded, 2)
            }
        }

        /// `FromStr`, and a check of the decimals it read.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                text.parse::<Decimal>().ok().filter(|value| value.scale() <= $decimals)
            }
        }

        /// `Display`.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                display(value, buffer);
            }
        }

        /// `From<&Decimal> for f64`.
        impl ToF64 for $name {
            fn to_f64(value: &Self::Value) -> f64 {
                f64::from(value)
            }
        }

        /// `TryFrom<f64>`, to 17 significant digits, then `round`.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Some(Decimal::try_from(x).ok()?.round($decimals))
            }
        }
    };
}

decimal_rs!(
    /// `Decimal` at 8 decimals.
    Narrow,
    8,
    Width::Narrow
);
decimal_rs!(
    /// `Decimal` at 18 decimals.
    Wide,
    18,
    Width::Wide
);

/// `Decimal` at scale 2 times one at scale 5: `checked_mul`, at scale 7.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "decimal-rs 0.2.0";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Price = Decimal;
    type Quantity = Decimal;
    type Product = Decimal;

    fn price(steps: i128) -> Option<Self::Price> {
        value(steps, 2)
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        value(steps, 5)
    }

    fn mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        price.checked_mul(quantity)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        steps(product, 7)
    }
}
