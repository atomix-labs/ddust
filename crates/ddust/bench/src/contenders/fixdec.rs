//! `fixdec` 0.1.0: `D64`, an `i64` at 8 decimals.
//!
//! Its product divides by `10^8` through a reciprocal that is not exact, so it lands steps off the
//! true product as the product grows: its row is timed, and marked as giving other results, since
//! its speed comes from that shortcut. Its quotient truncates toward zero, exactly. Its `to_f64`
//! shares the inexact division, and its `from_f64` multiplies in floating point before it rounds.

use fixdec::D64;

use super::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, Kind, MulRound, Parse, Rescale,
    ToF64, cents, display, low_byte,
};
use crate::inputs::Width;
use crate::oracle::Mode;

/// `D64`.
#[derive(Debug, Clone, Copy)]
pub struct Narrow;

impl Contender for Narrow {
    const NAME: &'static str = "fixdec 0.1.0 D64";
    const KIND: Kind = Kind::Decimal;
    const WIDTH: Width = Width::Narrow;
    type Value = D64;

    fn from_steps(steps: i128) -> Option<Self::Value> {
        Some(D64::from_raw(i64::try_from(steps).ok()?))
    }

    fn to_steps(value: &Self::Value) -> Option<i128> {
        Some(i128::from(value.to_raw()))
    }

    fn fingerprint(value: &Self::Value) -> usize {
        low_byte(u128::from(value.to_raw().cast_unsigned()))
    }
}

/// `checked_add`.
impl Add for Narrow {
    fn add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
        a.checked_add(*b)
    }
}

/// `<`.
impl Compare for Narrow {
    fn less(a: &Self::Value, b: &Self::Value) -> bool {
        a < b
    }
}

/// `checked_mul`, meant to truncate, through an inexact reciprocal of `10^8`.
impl MulRound for Narrow {
    const MODE: Mode = Mode::Trunc;
    const EXACT: bool = false;

    fn mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
        a.checked_mul(*b)
    }
}

/// `checked_div`, toward zero.
impl DivRound for Narrow {
    const MODE: Mode = Mode::Trunc;

    fn div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
        a.checked_div(*b)
    }
}

/// `round_dp(2)`, half to even, at the same scale.
impl Rescale for Narrow {
    const MODE: Mode = Mode::HalfEven;
    type Rounded = D64;

    fn rescale(value: &Self::Value) -> Option<Self::Rounded> {
        Some(value.round_dp(2))
    }

    fn rounded_steps(rounded: &Self::Rounded) -> Option<i128> {
        cents(i128::from(rounded.to_raw()), 8)
    }
}

/// `from_str_exact`, which refuses a digit past the scale.
impl Parse for Narrow {
    fn parse(text: &str) -> Option<Self::Value> {
        D64::from_str_exact(text).ok()
    }
}

/// `Display`, the shortest form.
impl Format for Narrow {
    fn format(value: &Self::Value, buffer: &mut Buffer) {
        display(value, buffer);
    }
}

/// `to_f64`, through the inexact division.
impl ToF64 for Narrow {
    const EXACT: bool = false;

    fn to_f64(value: &Self::Value) -> f64 {
        value.to_f64()
    }
}

/// `from_f64`: the double times `10^8` in floating point, rounded half away from zero.
impl FromF64 for Narrow {
    const MODE: Mode = Mode::HalfExpand;
    const EXACT: bool = false;

    fn from_f64(x: f64) -> Option<Self::Value> {
        D64::from_f64(x)
    }
}
