# Introduction

ddust is a Rust crate of fixed-point decimals. Its [`Decimal`], a `Decimal<I,
S>`, is a whole number of steps of $10^{-d}$, held in one of Rust's ten
integers, at a scale the compiler knows or one carried at run time. It is for
code in which a lost digit is a bug: prices, quantities, balances, fees and
rates, and the messages and files that carry them as text.

A binary float cannot hold 0.1. A sum of prices in `f64` drifts, and writing one
back as text means guessing which decimal it was. A decimal holds 0.1 exactly,
and ddust's keeps three promises:

- **Nothing loses a digit unless a rounding mode says so.** `a * b` is exact,
  its scale the sum of the factors'; `a / b` truncates toward zero, as the
  integer's does; and every other operation that can lose a digit takes a
  rounding mode as its last argument.
- **Scales never mix by accident.** Two static scales are two types, so a
  `D64<2>` plus a `D64<4>` does not compile. Two scales known only at run time
  line up exactly at the finer one, as SQL's `DECIMAL` does.
- **Every operator behaves as the integer's.** A `D64<2>` is an `i64` and adds
  as one. Overflow panics with overflow checks on and wraps otherwise, and each
  operator has the integer's `checked_*`, `saturating_*`, `wrapping_*` and
  `overflowing_*` methods.

## What It Holds

- The decimal, [`Decimal`], with an alias for each integer at a static scale:
  [`D8`] to [`D128`] and [`UD8`] to [`UD128`]. [`dec!`] writes a literal that is
  checked when the constant is evaluated.
- Scales: [`Fixed<D>`][ddust::Fixed], known to the compiler and zero-sized, and
  [`Dynamic`], carried at run time.
- Rounding: nine modes in [`round`], each a zero-sized type, and
  [`Rounding`][ddust::round::Rounding], a mode chosen at run time. [`Divisor`]
  prepares one divisor for many divisions.
- Text: reading that is exact or refused, from a slice or in place inside a
  message, and writing through `Display` or into a byte buffer, with no
  formatter.
- Floats: the nearest `f64`, correctly rounded, and a double to a decimal by a
  rounding mode.
- Integrations, each behind a feature: serde, num-traits, bytemuck, zerocopy,
  rand, proptest, arbitrary and defmt.

The crate is `#![no_std]`, allocates nothing, and has no dependencies unless a
feature asks for one.

## What It Leaves Out

- **Arbitrary precision.** A decimal holds at most 38 decimals and its integer's
  range: about ±9.2 × 10¹⁶ at two decimals in an `i64`, and about ±1.7 × 10²⁰ at
  eighteen in an `i128`.
- **A scale that each operation chooses.** rust_decimal's values carry their own
  scale, up to 28 decimals, and a quotient takes as many decimals as fit. In
  ddust a scale is chosen by the caller; a quotient is at the dividend's scale,
  or at one its rounding names.
- **NaN, the infinities and negative zero**, which a float has and a decimal
  does not need.
- **Square roots, logarithms, powers and trigonometry.**
- **Stable Rust.** ddust builds on nightly, from `nightly-2026-09-28`: its
  arithmetic, rounding and literals are `const fn` through const traits, which
  are not stable yet.

[Compared with Others](comparison.md) says when another crate is the better
choice.

## This Book

The guide comes first: [Getting Started](getting-started.md), then a chapter for
each idea in the order a program meets it, from
[Decimals and Scales](decimals-and-scales.md) to
[Serialization](serialization.md), and [Choosing a Decimal](choosing.md). The
reference follows: what each operation costs, the platforms and features, the
crates ddust works with, testing code that uses it, and how it compares with the
other decimal crates.
