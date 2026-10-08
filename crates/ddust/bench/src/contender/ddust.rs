//! ddust: `D64<8>` and `D128<18>`, through the calls a program makes, each rounding half to even,
//! and again in `Trunc` and `HalfExpand` for the rounded rows, to meet each contender in its mode.

use ddust::round::{HalfEven, HalfExpand, Trunc};
use ddust::scale::Sum;
use ddust::{Decimal, Divisor, Fixed};

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact, MulRound,
    Parse, PreparedDivRound, RescaleRound, ToF64, low_byte,
};
use crate::input::Width;
use crate::oracle::Mode;

/// ddust's type over `$integer` at `$decimals`, named `$name` in a table: its steps, and what every
/// row has, the checked sum and the order.
macro_rules! ddust_type {
    ($(#[$doc:meta])* $type:ident, $integer:ty, $decimals:literal, $width:expr, $name:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $type;

        impl Contender for $type {
            const NAME: &'static str = $name;
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
        impl CheckedAdd for $type {
            fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_add(*b)
            }
        }

        /// `<`.
        impl Compare for $type {
            fn is_less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }
    };
}

/// The rounded operations of ddust's type `$type` over `$integer`, each rounding by `$mode`.
macro_rules! ddust_rounded {
    ($type:ident, $integer:ty, $decimals:literal, $mode:ident) => {
        /// `checked_mul_round`, one rounding of the exact product.
        impl MulRound for $type {
            const MODE: Mode = Mode::$mode;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul_round(*b, $mode)
            }
        }

        /// `checked_div_round`, one rounding of the exact quotient.
        impl DivRound for $type {
            const MODE: Mode = Mode::$mode;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div_round(*b, $mode)
            }
        }

        /// A `Divisor`, prepared once, and `checked_div_round` by it.
        impl PreparedDivRound for $type {
            type Prepared = Divisor<$integer, Fixed<$decimals>>;

            fn prepare(b: &Self::Value) -> Option<Self::Prepared> {
                Divisor::new(*b)
            }

            fn checked_div_round_prepared(
                a: &Self::Value, b: &Self::Prepared,
            ) -> Option<Self::Value> {
                a.checked_div_round(*b, $mode)
            }
        }

        /// `rescale_round` to `Fixed<2>`, another type.
        impl RescaleRound for $type {
            const MODE: Mode = Mode::$mode;
            type Rounded = Decimal<$integer, Fixed<2>>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                value.rescale_round(Fixed, $mode).ok()
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                Some(i128::from(rounded.steps()))
            }
        }
    };
}

/// Implements every operation for ddust's type over `$integer` at `$decimals`, rounding half to
/// even.
macro_rules! ddust {
    ($(#[$doc:meta])* $type:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal) => {
        ddust_type!($(#[$doc])* $type, $integer, $decimals, $width, concat!("ddust ", $label));
        ddust_rounded!($type, $integer, $decimals, HalfEven);

        /// `from_ascii`, on the text's bytes.
        impl Parse for $type {
            fn parse(text: &str) -> Option<Self::Value> {
                Decimal::from_ascii(text.as_bytes(), Fixed).ok()
            }
        }

        /// `write_ascii`, into the buffer's bytes.
        impl Format for $type {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                let (bytes, len) = buffer.fill_from_front();
                *len = value.write_ascii(bytes).unwrap_or(0);
            }
        }

        /// `to_f64`, correctly rounded.
        impl ToF64 for $type {
            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64()
            }
        }

        /// `from_f64`, from the double's exact value.
        impl FromF64 for $type {
            const MODE: Mode = Mode::HalfEven;

            fn from_f64(x: f64) -> Option<Self::Value> {
                Decimal::from_f64(x, Fixed, HalfEven)
            }
        }
    };
}

/// A ddust row in another rounding mode, to meet each contender's rounded rows in their own mode:
/// the type's sum and order, and its rounded operations in `$mode`.
macro_rules! ddust_mode {
    ($type:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal, $mode:ident) => {
        ddust_type!(
            /// ddust in one rounding mode.
            $type,
            $integer,
            $decimals,
            $width,
            concat!("ddust ", $label, " ", stringify!($mode))
        );
        ddust_rounded!($type, $integer, $decimals, $mode);
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

ddust_mode!(NarrowTrunc, i64, 8, Width::Narrow, "D64<8>", Trunc);
ddust_mode!(NarrowHalfExpand, i64, 8, Width::Narrow, "D64<8>", HalfExpand);
ddust_mode!(WideTrunc, i128, 18, Width::Wide, "D128<18>", Trunc);
ddust_mode!(WideHalfExpand, i128, 18, Width::Wide, "D128<18>", HalfExpand);
