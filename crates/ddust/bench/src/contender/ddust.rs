//! ddust: `D64<8>` and `D128<18>`, through the calls a program makes, each rounding half to even.

use ddust::round::HalfEven;
use ddust::scale::Sum;
use ddust::{Decimal, Fixed};

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound,
    Parse, RescaleRound, ToF64, low_byte,
};
use crate::input::Width;
use crate::oracle::Mode;

/// Implements every operation for ddust's type over `$integer` at `$decimals`.
macro_rules! ddust {
    ($(#[$doc:meta])* $name:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("ddust ", $label);
            const KIND: Kind = Kind::Ddust;
            const WIDTH: Width = $width;
            type Value = Decimal<$integer, Fixed<$decimals>>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(Decimal::from_steps(<$integer>::try_from(steps).ok()?, Fixed))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(value.steps()))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.steps().cast_unsigned()))
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

        /// `checked_mul_round`, one rounding of the exact product.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul_round(*b, HalfEven)
            }
        }

        /// `checked_div_round`, one rounding of the exact quotient.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div_round(*b, HalfEven)
            }
        }

        /// `rescale_round` to `Fixed<2>`, another type.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::HalfEven;
            type Rounded = Decimal<$integer, Fixed<2>>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                value.rescale_round(Fixed, HalfEven).ok()
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                Some(i128::from(rounded.steps()))
            }
        }

        /// `from_ascii`, on the text's bytes.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                Decimal::from_ascii(text.as_bytes(), Fixed).ok()
            }
        }

        /// `write_ascii`, into the buffer's bytes.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                let (bytes, len) = buffer.fill_from_front();
                *len = value.write_ascii(bytes).unwrap_or(0);
            }
        }

        /// `to_f64`, correctly rounded.
        impl ToF64 for $name {
            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64()
            }
        }

        /// `from_f64`, from the double's exact value.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfEven;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Decimal::from_f64(x, Fixed, HalfEven)
            }
        }
    };
}

ddust!(
    /// `D64<8>`: an `i64` of steps of `10^-8`.
    Narrow,
    i64,
    8,
    Width::Narrow,
    "D64<8>"
);
ddust!(
    /// `D128<18>`: an `i128` of steps of `10^-18`.
    Wide,
    i128,
    18,
    Width::Wide,
    "D128<18>"
);

/// `D64<2> × D64<5>`: `checked_mul`, whose product is at `Sum<Fixed<2>, Fixed<5>>`, 7 decimals.
#[derive(Debug, Clone, Copy)]
pub struct Notional;

impl MulExact for Notional {
    const NAME: &'static str = "ddust D64<2> × D64<5>";
    const KIND: Kind = Kind::Ddust;
    const WIDTH: Width = Width::Narrow;
    type Price = Decimal<i64, Fixed<2>>;
    type Quantity = Decimal<i64, Fixed<5>>;
    type Product = Decimal<i64, Sum<Fixed<2>, Fixed<5>>>;

    fn price(steps: i128) -> Option<Self::Price> {
        Some(Decimal::from_steps(i64::try_from(steps).ok()?, Fixed))
    }

    fn quantity(steps: i128) -> Option<Self::Quantity> {
        Some(Decimal::from_steps(i64::try_from(steps).ok()?, Fixed))
    }

    fn checked_mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product> {
        price.checked_mul(*quantity)
    }

    fn product_steps(product: &Self::Product) -> Option<i128> {
        Some(i128::from(product.steps()))
    }
}

/// A ddust row in another rounding mode, so each contender's rounded rows meet ddust in the mode
/// they round by.
macro_rules! ddust_mode {
    ($name:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal, $mode:ident) => {
        /// ddust in one rounding mode.
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("ddust ", $label, " ", stringify!($mode));
            const KIND: Kind = Kind::Ddust;
            const WIDTH: Width = $width;
            type Value = Decimal<$integer, Fixed<$decimals>>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(Decimal::from_steps(<$integer>::try_from(steps).ok()?, Fixed))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(value.steps()))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.steps().cast_unsigned()))
            }
        }

        /// `checked_add`, as the row in half to even.
        impl CheckedAdd for $name {
            fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_add(*b)
            }
        }

        /// `<`, as the row in half to even.
        impl Compare for $name {
            fn is_less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// In the mode.
        impl MulRound for $name {
            const MODE: Mode = Mode::$mode;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul_round(*b, ddust::round::$mode)
            }
        }

        /// In the mode.
        impl DivRound for $name {
            const MODE: Mode = Mode::$mode;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div_round(*b, ddust::round::$mode)
            }
        }

        /// In the mode.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::$mode;
            type Rounded = Decimal<$integer, Fixed<2>>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                value.rescale_round(Fixed, ddust::round::$mode).ok()
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                Some(i128::from(rounded.steps()))
            }
        }
    };
}

ddust_mode!(NarrowTrunc, i64, 8, Width::Narrow, "D64<8>", Trunc);
ddust_mode!(NarrowHalfExpand, i64, 8, Width::Narrow, "D64<8>", HalfExpand);
ddust_mode!(WideTrunc, i128, 18, Width::Wide, "D128<18>", Trunc);
ddust_mode!(WideHalfExpand, i128, 18, Width::Wide, "D128<18>", HalfExpand);
