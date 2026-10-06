//! `nexus-decimal` 1.2.2: `Decimal<B, D>`, an `i32`, `i64` or `i128` at decimals known at compile
//! time.
//!
//! Its product and quotient truncate toward zero, with no mode to ask for another, so those rows
//! are toward zero; its `round_dp` rounds half to even. Its `to_f64` adds the fraction to the whole
//! part in floating point, which is not correctly rounded, and its `from_f64` multiplies by a power
//! of ten in floating point before it rounds.

use nexus_decimal::Decimal;

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulRound, Parse,
    RescaleRound, ToF64, cents, low_byte,
};
use crate::input::Width;
use crate::oracle::Mode;

/// Implements every operation for `Decimal<$integer, $decimals>`.
macro_rules! nexus {
    ($(#[$doc:meta])* $name:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("nexus-decimal 1.2.2 ", $label);
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = Decimal<$integer, $decimals>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(<Self::Value>::from_raw(<$integer>::try_from(steps).ok()?))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(value.to_raw()))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.to_raw().cast_unsigned()))
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

        /// `checked_mul`, toward zero.
        impl MulRound for $name {
            const MODE: Mode = Mode::Trunc;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul(*b)
            }
        }

        /// `checked_div`, toward zero.
        impl DivRound for $name {
            const MODE: Mode = Mode::Trunc;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div(*b)
            }
        }

        /// `round_dp(2)`, half to even, at the same scale.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::HalfEven;
            type Rounded = Decimal<$integer, $decimals>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                Some(value.round_dp(2))
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                cents(i128::from(rounded.to_raw()), $decimals)
            }
        }

        /// `from_str_exact`, which refuses a digit past the scale.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                <Self::Value>::from_str_exact(text).ok()
            }
        }

        /// `write_to_buf`, into the buffer's 64 bytes.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                let (bytes, len) = buffer.fill_from_front();
                *len = value.write_to_buf(bytes);
            }
        }

        /// `to_f64`: the whole part plus the fraction, in floating point.
        impl ToF64 for $name {
            const EXACT: bool = false;

            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64()
            }
        }

        /// `from_f64`: the double times a power of ten, rounded half away from zero.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                <Self::Value>::from_f64(x).ok()
            }
        }
    };
}

nexus!(
    /// `Decimal<i64, 8>`.
    Narrow,
    i64,
    8,
    Width::Narrow,
    "<i64, 8>"
);
nexus!(
    /// `Decimal<i128, 18>`.
    Wide,
    i128,
    18,
    Width::Wide,
    "<i128, 18>"
);
