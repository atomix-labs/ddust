//! Fixed-point decimals that leave no dust: exact, typed, as fast as the integers beneath them.
//!
//! A [`Decimal<I, S>`](Decimal) is a whole number of steps of `10^-decimals`, held in an integer
//! `I` from `i8` to `u128`, at a scale `S` the compiler knows, [`Fixed<D>`](Fixed), or one carried
//! at run time, [`Dynamic`]. A static scale takes no room, so the decimal is its integer alone and
//! adds as it does:
//!
//! | Decimal | Holds 12.34 as | Size |
//! | ------- | -------------- | ---- |
//! | [`D32<2>`], `Decimal<i32, Fixed<2>>` | 1,234 steps of 0.01 | 4 bytes |
//! | [`D64<2>`], `Decimal<i64, Fixed<2>>` | 1,234 steps of 0.01 | 8 bytes |
//! | [`D128<18>`], `Decimal<i128, Fixed<18>>` | 12,340,000,000,000,000,000 steps of 10^-18 | 16 bytes |
//! | `Decimal<i64, Dynamic>` | 1,234 steps, and a scale of 2 | 16 bytes |
//!
//! - **Every operator behaves as the integer's.** Overflow panics with overflow checks on and wraps
//!   otherwise, and each operator has the integer's `checked_*`, `saturating_*`, `wrapping_*` and
//!   `overflowing_*` methods.
//! - **Nothing loses a digit unless a rounding mode says so.** `a * b` is exact, its scale the sum
//!   of the factors'; `a / b` truncates toward zero, as the integer's does; every other operation
//!   that can lose a digit takes a [rounding mode](round) as its last argument.
//! - **Scales never mix by accident.** Two static scales are two types; two run-time scales line up
//!   exactly at the finer one, as SQL's `DECIMAL` does.
//!
//! # Types
//!
//! - **The decimal.** [`Decimal`]; an alias for each integer at a static scale, [`D8`] to [`D128`]
//!   and [`UD8`] to [`UD128`]; and [`dec!`], a literal checked when the constant is evaluated.
//! - **Integers.** [`Int`], implemented by the ten primitive integers, and [`Signed`].
//! - **Scales.** [`Scale`]; [`Fixed`] and [`Dynamic`]; [`StaticScale`]; and in [`scale`], a
//!   product's [`Sum`](scale::Sum) and the [`Times`](scale::Times) that gives it.
//! - **Rounding.** The nine modes in [`round`], each a zero-sized type, and
//!   [`Rounding`](round::Rounding), one chosen at run time.
//! - **Refusals.** [`ParseError`] and [`ConvertError`], each with a kind.
//!
//! # Examples
//! ```
//! use ddust::round::{Ceil, HalfEven};
//! use ddust::{D64, dec};
//!
//! let amount: D64<2> = "100.10".parse()?;
//! assert_eq!(amount.div_int_round(3, HalfEven), dec!(33.37), "a third, to the cent");
//!
//! let fee: D64<4> = dec!(0.0025);
//! assert_eq!(amount.mul_round(fee, Ceil), dec!(0.26), "0.25025, up to the cent");
//!
//! let exact: D64<6> = (amount * fee).into();
//! assert_eq!(
//!     exact.to_string(),
//!     "0.25025",
//!     "the product, exact: its scale the sum of the factors'"
//! );
//! # Ok::<(), ddust::ParseError>(())
//! ```
//!
//! # Toolchain
//!
//! ddust builds on nightly Rust, from `nightly-2026-09-28`: its arithmetic, rounding and literals
//! are `const fn` over every integer through const traits, which are not stable yet.

#![no_std]
// Const traits: the arithmetic, the rounding and the literal parser are `const fn` over every
// integer, so a constant computes with decimals.
#![feature(const_trait_impl, const_ops, const_cmp, const_convert, derive_const)]
// `u128::carrying_mul` in a `const fn`: the 256-bit product of two 128-bit magnitudes.
#![feature(const_unsigned_bigint_helpers)]
// Overflow decided by the profile's `overflow-checks`, as the integers' own operators decide it.
#![feature(cfg_overflow_checks)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(test)]
extern crate alloc;

mod cmp;
mod convert;
mod decimal;
mod errors;
mod float;
mod int;
mod kernel;
mod literal;
mod ops;
pub mod round;
mod rounded;
pub mod scale;
mod text;
mod word;

pub use crate::decimal::{D8, D16, D32, D64, D128, Decimal, UD8, UD16, UD32, UD64, UD128};
pub use crate::errors::{ConvertError, ConvertErrorKind, ParseError, ParseErrorKind};
#[doc(hidden)]
pub use crate::int::Outcome;
pub use crate::int::{Int, Signed};
pub use crate::scale::{Dynamic, Fixed, Scale, StaticScale};
pub use crate::text::MAX_ASCII_LEN;
