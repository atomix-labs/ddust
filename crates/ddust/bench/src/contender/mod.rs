//! The crates the suite compares, each behind the same traits.
//!
//! A [`Contender`] is a crate's type at one width, and each operation it has is a trait: its
//! fastest correct call, with the rounding it uses named. An operation a crate lacks is a trait it
//! does not implement, and its row is absent from the table, never filled with another operation.
//! Each value is built from the inputs' steps outside the timed loop, and each result is checked
//! against the [oracle](crate::oracle) before any is timed, by `tests/equivalence.rs`.
//!
//! Beside the decimal crates stand the floors: `i64`, `i128` and `f64` doing the plain operation a
//! decimal's is built on, the cost no decimal goes below. The oracle does not check them.

use core::{fmt, str};

use crate::input::Width;
use crate::oracle::Mode;

pub mod bigdecimal;
pub mod ddust;
pub mod decimal_rs;
pub mod fastnum;
pub mod fin_decimal;
pub mod fixdec;
pub mod fixed;
pub mod fixnum;
pub mod floors;
pub mod nexus_decimal;
pub mod primitive_fixed_point_decimal;
pub mod rust_decimal;

/// What a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// ddust.
    Ddust,
    /// Another decimal crate.
    Decimal,
    /// A floor: an integer or float operation, which the oracle does not check.
    Floor,
    /// A binary fixed point, beside the decimals for contrast: its values are not decimal, so the
    /// oracle does not check them.
    Binary,
}

/// A crate's type at one width: how a set's steps become one of its values, and back.
pub trait Contender {
    /// The row's name: the crate, its version, and its type where the crate has several.
    const NAME: &'static str;
    /// What the row is.
    const KIND: Kind;
    /// The width the type is compared at.
    const WIDTH: Width;
    /// The value.
    type Value: Clone;

    /// The value of `steps` steps at the width's decimals, or `None` where the type cannot hold it.
    fn from_steps(steps: i128) -> Option<Self::Value>;

    /// The steps of `value` at the width's decimals, exactly; `None` for a value with digits past
    /// them, or that a floor holds only approximately.
    fn to_steps(value: &Self::Value) -> Option<i128>;

    /// A few low bits of `value`, cheap to read, which a latency chain mixes into the index of its
    /// next operands, so each operation waits for the one before.
    fn fingerprint(value: &Self::Value) -> usize;
}

/// The checked sum: `None` past the type's range.
pub trait CheckedAdd: Contender {
    /// Whether every sum is the oracle's.
    const EXACT: bool = true;

    /// `a + b`, or `None` past the range.
    fn checked_add(a: &Self::Value, b: &Self::Value) -> Option<Self::Value>;
}

/// The order of two values.
pub trait Compare: Contender {
    /// `a < b`.
    fn is_less(a: &Self::Value, b: &Self::Value) -> bool;
}

/// The product of two values at the width's decimals, at those decimals, rounded.
pub trait MulRound: Contender {
    /// The rounding the call uses.
    const MODE: Mode;
    /// Whether every product is the oracle's.
    const EXACT: bool = true;

    /// `a × b`, rounded by [`MODE`](Self::MODE), or `None` past the range.
    fn checked_mul_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value>;
}

/// The quotient of two values at the width's decimals, at those decimals, rounded.
pub trait DivRound: Contender {
    /// The rounding the call uses.
    const MODE: Mode;
    /// Whether every quotient is the oracle's.
    const EXACT: bool = true;

    /// `a / b`, rounded by [`MODE`](Self::MODE), or `None` for a zero divisor or past the range.
    fn checked_div_round(a: &Self::Value, b: &Self::Value) -> Option<Self::Value>;
}

/// A rounded quotient by a divisor prepared once for many: ddust's `Divisor`.
pub trait DivRoundPrepared: DivRound {
    /// A divisor prepared.
    type Prepared: Copy;

    /// `b` prepared, or `None` for zero.
    fn prepare(b: &Self::Value) -> Option<Self::Prepared>;

    /// `a / b` by `b` prepared, rounded by [`MODE`](DivRound::MODE), or `None` past the range.
    fn checked_div_round_prepared(a: &Self::Value, b: &Self::Prepared) -> Option<Self::Value>;
}

/// A value rounded to 2 decimals: a price to cents.
pub trait RescaleRound: Contender {
    /// The rounding the call uses.
    const MODE: Mode;
    /// Whether every rounded value is the oracle's.
    const EXACT: bool = true;
    /// The rounded value, which may be another type.
    type Rounded;

    /// `value` at 2 decimals, rounded by [`MODE`](Self::MODE), or `None` where it fails.
    fn rescale_round(value: &Self::Value) -> Option<Self::Rounded>;

    /// The steps of `rounded` at 2 decimals.
    fn rounded_steps(rounded: &Self::Rounded) -> Option<i128>;
}

/// A value read from text, exactly: a digit past the width's decimals is refused, never rounded.
pub trait Parse: Contender {
    /// Whether every value read is the oracle's.
    const EXACT: bool = true;

    /// The value `text` spells, or `None`.
    fn parse(text: &str) -> Option<Self::Value>;
}

/// A value written as text, into a buffer the caller reuses.
pub trait Format: Contender {
    /// Whether every text reads back as its value.
    const EXACT: bool = true;

    /// Writes `value` into `buffer`, which it clears first.
    fn format(value: &Self::Value, buffer: &mut Buffer);
}

/// A value as the nearest `f64`.
pub trait ToF64: Contender {
    /// Whether every double is the nearest, as the oracle's is.
    const EXACT: bool = true;

    /// `value` as an `f64`.
    fn to_f64(value: &Self::Value) -> f64;
}

/// An `f64` as a value at the width's decimals, rounded.
pub trait FromF64: Contender {
    /// The rounding the call uses.
    const MODE: Mode;
    /// Whether every value is the oracle's rounding of the double's exact binary value.
    const EXACT: bool = true;

    /// `x` at the width's decimals, or `None` for a NaN, an infinity, or past the range.
    fn from_f64(x: f64) -> Option<Self::Value>;
}

/// The exact product of a price at 2 decimals and a quantity at 5: a notional at 7.
pub trait MulExact {
    /// The row's name.
    const NAME: &'static str;
    /// What the row is.
    const KIND: Kind;
    /// The width the types are compared at.
    const WIDTH: Width;
    /// A price, at 2 decimals.
    type Price: Clone;
    /// A quantity, at 5 decimals.
    type Quantity: Clone;
    /// Their product, at 7 decimals.
    type Product;

    /// The price of `steps` steps at 2 decimals.
    fn price(steps: i128) -> Option<Self::Price>;

    /// The quantity of `steps` steps at 5 decimals.
    fn quantity(steps: i128) -> Option<Self::Quantity>;

    /// `price × quantity`, exactly, or `None` past the range.
    fn checked_mul(price: &Self::Price, quantity: &Self::Quantity) -> Option<Self::Product>;

    /// The steps of `product` at 7 decimals.
    fn product_steps(product: &Self::Product) -> Option<i128>;
}

/// The text a [`Format`] writes, in a buffer the caller reuses.
///
/// Its 64 bytes are more than any value at either width spells, and the text is anywhere in them,
/// as a writer that fills from the front or from the back leaves it.
#[derive(Clone)]
pub struct Buffer {
    /// The bytes.
    bytes: [u8; 64],
    /// Where the text starts.
    start: usize,
    /// Where it ends.
    end: usize,
}

impl Buffer {
    /// An empty buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: [0; 64], start: 0, end: 0 }
    }

    /// Empties the buffer.
    pub const fn clear(&mut self) {
        (self.start, self.end) = (0, 0);
    }

    /// The text's bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(self.start..self.end).unwrap_or_default()
    }

    /// The text, or `None` for bytes that are not UTF-8.
    #[must_use]
    pub fn to_str(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes()).ok()
    }

    /// The bytes and the text's end, for a writer that fills from the front and returns its length.
    pub const fn fill_from_front(&mut self) -> (&mut [u8; 64], &mut usize) {
        self.start = 0;
        (&mut self.bytes, &mut self.end)
    }

    /// The bytes and the text's start, for a writer that fills from the back.
    pub const fn fill_from_back(&mut self) -> (&mut [u8; 64], &mut usize) {
        self.end = 64;
        (&mut self.bytes, &mut self.start)
    }
}

/// An empty buffer.
impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

/// The text written.
impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.to_str().unwrap_or_default(), f)
    }
}

/// Appends text, failing past 64 bytes.
impl fmt::Write for Buffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.end.checked_add(text.len()).ok_or(fmt::Error)?;
        self.bytes.get_mut(self.end..end).ok_or(fmt::Error)?.copy_from_slice(text.as_bytes());
        self.end = end;
        Ok(())
    }
}

/// The steps at 2 decimals of a value rounded to 2 but held at `decimals`: `steps / 10^(decimals -
/// 2)`, or `None` where digits below the cents remain.
#[must_use]
pub fn cents(steps: i128, decimals: u8) -> Option<i128> {
    let power = 10_i128.checked_pow(u32::from(decimals.checked_sub(2)?))?;
    if steps.checked_rem(power)? == 0 { steps.checked_div(power) } else { None }
}

/// The steps at `decimals` of `digits × 10^-scale`, a value whose scale it carries: exact, or
/// `None` where a digit below `decimals` is not zero, or past an `i128`.
#[must_use]
pub fn steps_of(digits: i128, scale: i64, decimals: u8) -> Option<i128> {
    let decimals = i64::from(decimals);
    if scale <= decimals {
        let lift = 10_i128.checked_pow(u32::try_from(decimals.checked_sub(scale)?).ok()?)?;
        return digits.checked_mul(lift);
    }
    let drop = 10_i128.checked_pow(u32::try_from(scale.checked_sub(decimals)?).ok()?)?;
    if digits.checked_rem(drop)? == 0 { digits.checked_div(drop) } else { None }
}

/// The low byte of `bits`, as a latency chain's fingerprint.
#[must_use]
pub fn low_byte(bits: u128) -> usize {
    usize::from(bits.to_le_bytes().first().copied().unwrap_or(0))
}

/// `value` written by `Display` into `buffer`, which a [`Format`] calls for a crate with no faster
/// writer.
pub fn format_by_display<T: fmt::Display>(value: &T, buffer: &mut Buffer) {
    use fmt::Write as _;

    buffer.clear();
    let _fits = write!(buffer, "{value}");
}

/// Calls `$apply!` with each contender that has the operation named, by its path, and `$function`.
///
/// It is the one list of who does what, for the benches, the probe and the equivalence test alike,
/// in the order a table lists them: the 64-bit width, then the 128-bit, each with its floors first,
/// then ddust, then the other crates by name; the rounded operations then add ddust in the modes
/// the others round by, truncating and half up.
///
/// `$function` is one token tree handed to every call, as the function to run for the contender.
///
/// The operations: `add`, `compare` and `mul_round`, which every crate has; `div_round` and
/// `format`, which the binary `fixed` lacks; `div_round_prepared`, by a prepared divisor, which
/// only ddust has; `rescale_round`, which `fixnum` lacks too; `parse`, which the integer floors
/// lack; the conversions `to_f64` and `from_f64`, which the `f64` floor does not need; and
/// `mul_exact`, the price-times-quantity product, a row of its own types.
#[macro_export]
macro_rules! for_each_contender {
    (add, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::I64 floors::F64Narrow ddust::Narrow bigdecimal::Narrow decimal_rs::Narrow
            fastnum::Narrow fin_decimal::Narrow fixdec::Narrow fixed::Narrow fixnum::Narrow
            nexus_decimal::Narrow primitive_fixed_point_decimal::Narrow rust_decimal::Narrow
            floors::I128 floors::F64Wide ddust::Wide bigdecimal::Wide decimal_rs::Wide
            fastnum::Wide fin_decimal::Wide fixnum::Wide nexus_decimal::Wide
            primitive_fixed_point_decimal::Wide rust_decimal::Wide
        ]);
    };
    (compare, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(add, $apply, $function);
    };
    (mul_round, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(add, $apply, $function);
        $crate::for_each_contender!(@modes $apply $function);
    };
    (div_round, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(format, $apply, $function);
        $crate::for_each_contender!(@modes $apply $function);
    };
    (div_round_prepared, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [ddust::Narrow ddust::Wide]);
        $crate::for_each_contender!(@modes $apply $function);
    };
    // ddust in the modes the other contenders round by, for the rounded rows.
    (@modes $apply:ident $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            ddust::NarrowTrunc ddust::NarrowHalfExpand ddust::WideTrunc ddust::WideHalfExpand
        ]);
    };
    (format, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::I64 floors::F64Narrow ddust::Narrow bigdecimal::Narrow decimal_rs::Narrow
            fastnum::Narrow fin_decimal::Narrow fixdec::Narrow fixnum::Narrow
            nexus_decimal::Narrow primitive_fixed_point_decimal::Narrow rust_decimal::Narrow
            floors::I128 floors::F64Wide ddust::Wide bigdecimal::Wide decimal_rs::Wide
            fastnum::Wide fin_decimal::Wide fixnum::Wide nexus_decimal::Wide
            primitive_fixed_point_decimal::Wide rust_decimal::Wide
        ]);
    };
    (rescale_round, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::I64 floors::F64Narrow ddust::Narrow bigdecimal::Narrow decimal_rs::Narrow
            fastnum::Narrow fin_decimal::Narrow fixdec::Narrow
            nexus_decimal::Narrow primitive_fixed_point_decimal::Narrow rust_decimal::Narrow
            floors::I128 floors::F64Wide ddust::Wide bigdecimal::Wide decimal_rs::Wide
            fastnum::Wide fin_decimal::Wide nexus_decimal::Wide
            primitive_fixed_point_decimal::Wide rust_decimal::Wide
        ]);
        $crate::for_each_contender!(@modes $apply $function);
    };
    (parse, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::F64Narrow ddust::Narrow bigdecimal::Narrow decimal_rs::Narrow
            fastnum::Narrow fin_decimal::Narrow fixdec::Narrow fixnum::Narrow
            nexus_decimal::Narrow primitive_fixed_point_decimal::Narrow rust_decimal::Narrow
            floors::F64Wide ddust::Wide bigdecimal::Wide decimal_rs::Wide
            fastnum::Wide fin_decimal::Wide fixnum::Wide nexus_decimal::Wide
            primitive_fixed_point_decimal::Wide rust_decimal::Wide
        ]);
    };
    (to_f64, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::I64 ddust::Narrow bigdecimal::Narrow decimal_rs::Narrow
            fastnum::Narrow fin_decimal::Narrow fixdec::Narrow fixnum::Narrow
            nexus_decimal::Narrow primitive_fixed_point_decimal::Narrow rust_decimal::Narrow
            floors::I128 ddust::Wide bigdecimal::Wide decimal_rs::Wide
            fastnum::Wide fin_decimal::Wide fixnum::Wide nexus_decimal::Wide
            primitive_fixed_point_decimal::Wide rust_decimal::Wide
        ]);
    };
    (from_f64, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(to_f64, $apply, $function);
    };
    (mul_exact, $apply:ident, $function:tt) => {
        $crate::for_each_contender!(@each $apply $function [
            floors::Notional ddust::Notional bigdecimal::Notional decimal_rs::Notional
            fastnum::Notional primitive_fixed_point_decimal::Notional rust_decimal::Notional
        ]);
    };
    (@each $apply:ident $function:tt [$($module:ident::$contender:ident)*]) => {
        $($apply!($crate::contender::$module::$contender, $function);)*
    };
}
