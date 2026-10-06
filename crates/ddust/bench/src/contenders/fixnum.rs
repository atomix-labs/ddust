//! `fixnum` 0.9.5: `FixedPoint<I, P>`, an `i16` to `i128` at decimals a typenum names.
//!
//! It has no operators: every operation is a method returning a `Result`. Its rounding is a mode
//! passed to each call, `Nearest` half away from zero, and it has no rounding to fewer decimals, so
//! it has no row for rounding to cents. Its `f64` conversions go through floating point, neither
//! correctly rounded.

use fixnum::FixedPoint;
use fixnum::ops::{CheckedAdd as _, RoundMode, RoundingDiv as _, RoundingMul as _};
use fixnum::typenum::{U8, U18};

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulRound, Parse, ToF64,
    display, low_byte,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// Implements every operation for `FixedPoint<$integer, $precision>`.
macro_rules! fixnum {
    ($(#[$doc:meta])* $name:ident, $integer:ty, $precision:ty, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("fixnum 0.9.5 ", $label);
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = FixedPoint<$integer, $precision>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some(<Self::Value>::from_bits(<$integer>::try_from(steps).ok()?))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(*value.as_bits()))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.as_bits().cast_unsigned()))
            }
        }

        /// `cadd`.
        impl Add for $name {
            fn add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.cadd(*b).ok()
            }
        }

        /// `<`.
        impl Compare for $name {
            fn less(a: &Self::Value, b: &Self::Value) -> bool {
                a < b
            }
        }

        /// `rmul` with `RoundMode::Nearest`, one rounding.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.rmul(*b, RoundMode::Nearest).ok()
            }
        }

        /// `rdiv` with `RoundMode::Nearest`, one rounding.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfExpand;

            fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.rdiv(*b, RoundMode::Nearest).ok()
            }
        }

        /// `from_str_exact`, which refuses a digit past the scale.
        impl Parse for $name {
            fn parse(text: &str) -> Option<Self::Value> {
                <Self::Value>::from_str_exact(text).ok()
            }
        }

        /// `Display`, which writes a whole number as `5.0`.
        impl Format for $name {
            fn format(value: &Self::Value, buffer: &mut Buffer) {
                display(value, buffer);
            }
        }

        /// `From<FixedPoint> for f64`: the whole part and the fraction, in floating point.
        impl ToF64 for $name {
            const EXACT: bool = false;

            fn to_f64(value: &Self::Value) -> f64 {
                f64::from(*value)
            }
        }

        /// `TryFrom<f64>`: the double cut to about 17 digits, then rounded half up.
        impl FromF64 for $name {
            const MODE: Mode = Mode::HalfExpand;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                <Self::Value>::try_from(x).ok()
            }
        }
    };
}

fixnum!(
    /// `FixedPoint<i64, U8>`.
    Narrow,
    i64,
    U8,
    Width::Narrow,
    "<i64, U8>"
);
fixnum!(
    /// `FixedPoint<i128, U18>`.
    Wide,
    i128,
    U18,
    Width::Wide,
    "<i128, U18>"
);
