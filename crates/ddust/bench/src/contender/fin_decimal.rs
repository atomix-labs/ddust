//! `fin_decimal` 0.4.0: `Decimal<D>` on an `i64` and `Decimal128<D>` on an `i128`, at decimals
//! known at compile time.
//!
//! Its rounded product, quotient and rounding to cents take a mode, half to even among them. Its
//! parser rounds a digit past the scale rather than refusing it, so its row is marked as giving
//! other results, though on the suite's texts, at the scale or under, it reads every one. Its
//! `to_f64` divides in floating point, and its `from_f64` multiplies in floating point and
//! truncates, so neither is correctly rounded.

use fin_decimal::{AmountSign, Decimal, Decimal128, Rounding, str_i64};

use super::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulRound, Parse,
    RescaleRound, ToF64, cents, format_by_display, low_byte,
};
use crate::input::Width;
use crate::oracle::Mode;

/// Implements every operation but writing for `$type`, on `$integer` at `$decimals`.
macro_rules! fin_decimal {
    ($(#[$doc:meta])* $name:ident, $type:ident, $integer:ty, $decimals:literal, $width:expr, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy)]
        pub struct $name;

        impl Contender for $name {
            const NAME: &'static str = concat!("fin_decimal 0.4.0 ", $label);
            const KIND: Kind = Kind::Decimal;
            const WIDTH: Width = $width;
            type Value = $type<$decimals>;

            fn from_steps(steps: i128) -> Option<Self::Value> {
                Some($type(<$integer>::try_from(steps).ok()?))
            }

            fn to_steps(value: &Self::Value) -> Option<i128> {
                Some(i128::from(value.0))
            }

            fn fingerprint(value: &Self::Value) -> usize {
                low_byte(u128::from(value.0.cast_unsigned()))
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

        /// `checked_mul_rounded` with `Rounding::HalfEven`, one rounding.
        impl MulRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_mul_rounded::<$decimals>(*b, Rounding::HalfEven)
            }
        }

        /// `checked_div_rounded` with `Rounding::HalfEven`, one rounding.
        impl DivRound for $name {
            const MODE: Mode = Mode::HalfEven;

            fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
                a.checked_div_rounded(*b, Rounding::HalfEven)
            }
        }

        /// `checked_round_dp(2)` with `Rounding::HalfEven`, at the same scale.
        impl RescaleRound for $name {
            const MODE: Mode = Mode::HalfEven;
            type Rounded = $type<$decimals>;

            fn rescale_round(value: &Self::Value) -> Option<Self::Rounded> {
                value.checked_round_dp(2, Rounding::HalfEven)
            }

            fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
                cents(i128::from(rounded.0), $decimals)
            }
        }

        /// `FromStr`, which rounds a digit past the scale half up, where the rest refuse it.
        impl Parse for $name {
            const EXACT: bool = false;

            fn parse(text: &str) -> Option<Self::Value> {
                text.parse().ok()
            }
        }

        /// `to_f64`: the steps divided in floating point.
        impl ToF64 for $name {
            const EXACT: bool = false;

            fn to_f64(value: &Self::Value) -> f64 {
                value.to_f64()
            }
        }

        /// `from_f64`: the double multiplied in floating point, then truncated.
        impl FromF64 for $name {
            const MODE: Mode = Mode::Trunc;
            const EXACT: bool = false;

            fn from_f64(x: f64) -> Option<Self::Value> {
                $type::from_f64(x).ok()
            }
        }
    };
}

fin_decimal!(
    /// `Decimal<8>`, on an `i64`.
    Narrow,
    Decimal,
    i64,
    8,
    Width::Narrow,
    "Decimal<8>"
);
fin_decimal!(
    /// `Decimal128<18>`, on an `i128`.
    Wide,
    Decimal128,
    i128,
    18,
    Width::Wide,
    "Decimal128<18>"
);

/// `str_i64`, which fills the buffer from the back.
impl Format for Narrow {
    fn format(value: &Self::Value, buffer: &mut Buffer) {
        let (bytes, start) = buffer.fill_from_back();
        let written = str_i64(value.0, 8, None, AmountSign::Negative, bytes).map_or(0, str::len);
        *start = bytes.len().saturating_sub(written);
    }
}

/// `Display`: the 128-bit type has no writer of its own.
impl Format for Wide {
    fn format(value: &Self::Value, buffer: &mut Buffer) {
        format_by_display(value, buffer);
    }
}
