//! Fixed-point decimals that leave no dust: exact, typed, as fast as the integers beneath them.

#![no_std]
// Const traits: the arithmetic, the rounding and the literal parser are `const fn` over every
// integer, so a constant computes with decimals.
#![feature(const_trait_impl, const_ops, const_cmp, const_convert, derive_const)]
// `u128::carrying_mul` in a `const fn`: the 256-bit product of two 128-bit magnitudes.
#![feature(const_unsigned_bigint_helpers)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(test)]
extern crate alloc;

mod int;
mod kernel;
pub mod round;
pub mod scale;
mod word;

#[doc(hidden)]
pub use crate::int::Outcome;
pub use crate::int::{Int, Signed};
pub use crate::scale::{Dynamic, Fixed, Scale, StaticScale};
