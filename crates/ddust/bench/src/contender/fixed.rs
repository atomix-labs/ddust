//! `fixed` 1.31.0: `I64F64`, an `i128` with 64 fraction bits: binary, not decimal.
//!
//! It stands beside the decimals for contrast, in the sum, the order and the product, the
//! operations a binary type has in common with them; its values are the nearest to the inputs'
//! decimals, so the oracle does not check them.

use fixed::types::I64F64;

use super::{CheckedAdd, Compare, Contender, Kind, MulRound, low_byte};
use crate::input::Width;
use crate::oracle::{self, Mode};

/// `I64F64`, beside the 64-bit decimals.
#[derive(Debug, Clone, Copy)]
pub struct Narrow;

impl Contender for Narrow {
    const NAME: &'static str = "fixed 1.31.0 I64F64";
    const KIND: Kind = Kind::Binary;
    const WIDTH: Width = Width::Narrow;
    type Value = I64F64;

    fn from_steps(steps: i128) -> Option<Self::Value> {
        I64F64::from_str(&oracle::text(steps, 8)).ok()
    }

    fn to_steps(_value: &Self::Value) -> Option<i128> {
        None
    }

    fn fingerprint(value: &Self::Value) -> usize {
        low_byte(value.to_bits().cast_unsigned())
    }
}

/// `checked_add`.
impl CheckedAdd for Narrow {
    fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
        a.checked_add(*b)
    }
}

/// `<`.
impl Compare for Narrow {
    fn is_less(a: &Self::Value, b: &Self::Value) -> bool {
        a < b
    }
}

/// `checked_mul`, which rounds toward negative infinity in binary.
impl MulRound for Narrow {
    const MODE: Mode = Mode::Floor;

    fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value> {
        a.checked_mul(*b)
    }
}
