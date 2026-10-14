<!-- >>> devset: project >>> -->
<!-- dprint-ignore-start -->

<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/atomix-labs/ddust/main/docs/src/media/logo-dark.svg">
    <img alt="ddust" src="https://raw.githubusercontent.com/atomix-labs/ddust/main/docs/src/media/logo-light.svg" height="56">
  </picture>
</h1>

<p align="center">Fixed-point decimals that leave no dust: exact, typed, as fast as the integers beneath them.</p>

<p align="center">
  <a href="https://github.com/atomix-labs/ddust/actions/workflows/check.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/atomix-labs/ddust/check.yml?branch=main&amp;style=flat-square&amp;label=check"></a>
  <a href="https://crates.io/crates/ddust"><img alt="crates.io" src="https://img.shields.io/crates/v/ddust?style=flat-square"></a>
  <a href="https://docs.rs/ddust"><img alt="docs.rs" src="https://img.shields.io/docsrs/ddust?style=flat-square"></a>
  <a href="https://atomix-labs.github.io/ddust/"><img alt="Book" src="https://img.shields.io/badge/book-read-blue?style=flat-square"></a>
  <a href="https://github.com/atomix-labs/devset"><img alt="managed with devset" src="https://img.shields.io/badge/managed_with-devset-0969da?style=flat-square&amp;logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHRpdGxlPmRldnNldDwvdGl0bGU+PHBhdGggZmlsbD0iI2YwZjZmYyIgZD0ibTE2IDMgMTMgNi41TDE2IDE2IDMgOS41WiIvPjxwYXRoIGZpbGw9Im5vbmUiIHN0cm9rZT0iI2YwZjZmYyIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbGluZWpvaW49InJvdW5kIiBzdHJva2Utd2lkdGg9IjIuNSIgZD0ibTMgMTYgMTMgNi41TDI5IDE2TTMgMjIuNSAxNiAyOWwxMy02LjUiLz48L3N2Zz4K"></a>
</p>

<!-- dprint-ignore-end -->
<!-- <<< devset: project <<< -->

ddust's decimal is a whole number of steps of a power of ten, held in an integer
from `i8` to `u128`, at a scale the compiler knows or one carried at run time. A
static scale takes no room, so a `D64<2>` is an `i64` and adds as one. Nothing
loses a digit unless a rounding mode says so, two static scales never mix, and
every operator behaves as the integer's, overflow included. It is not arbitrary
precision and not a decimal float: a value has at most 38 decimals, its
integer's range, and no NaN or infinity.

The name reads as decimal dust: the digits a rounding leaves behind, which ddust
never drops without being asked.

## Install

```sh
cargo add ddust
```

ddust builds on nightly Rust, from `nightly-2026-09-28`, since its arithmetic is
`const fn` through const traits, which are not stable yet. A
`rust-toolchain.toml` beside the project's `Cargo.toml` pins it:

```toml
[toolchain]
channel = "nightly-2026-09-28"
```

## Quick Start

```rust
use ddust::round::{Ceil, HalfEven};
use ddust::{D64, ParseError, dec};

fn main() -> Result<(), ParseError> {
    let amount: D64<2> = "100.10".parse()?;
    assert_eq!(amount.div_int_round(3, HalfEven), dec!(33.37), "a third, to the cent");

    let fee: D64<4> = dec!(0.0025);
    assert_eq!(amount.mul_round(fee, Ceil), dec!(0.26), "0.25025, up to the cent");

    let exact: D64<6> = (amount * fee).into();
    assert_eq!(
        exact.to_string(),
        "0.25025",
        "the product, exact: its scale the sum of the factors'"
    );
    Ok(())
}
```

## Types and Sizes

| Decimal                                | Holds 12.34 as                             | Size     |
| -------------------------------------- | ------------------------------------------ | -------- |
| `D32<2>`, `Decimal<i32, Fixed<2>>`     | 1,234 steps of 0.01                        | 4 bytes  |
| `D64<2>`, `Decimal<i64, Fixed<2>>`     | 1,234 steps of 0.01                        | 8 bytes  |
| `D128<18>`, `Decimal<i128, Fixed<18>>` | 12,340,000,000,000,000,000 steps of 10^-18 | 16 bytes |
| `Decimal<i64, Dynamic>`                | 1,234 steps, and a scale of 2              | 16 bytes |

Every integer has an alias, `D8` to `D128` and `UD8` to `UD128`, and `dec!`
writes a literal checked when the constant is evaluated.

## Rounding Modes

An operation that can lose a digit takes one of nine modes as its last argument,
each a zero-sized type, or a `Rounding` chosen at run time: `Floor`, `Ceil`,
`Trunc` and `Expand`, which round toward −∞, +∞, zero and away from zero; and
`HalfFloor`, `HalfCeil`, `HalfTrunc`, `HalfExpand` and `HalfEven`, which round
to the nearer step and break a tie as their names say. `*` is exact, and `/`
truncates, as the integer's does.

## Speed

Nanoseconds per operation on a Graviton4, over 64-bit values at eight decimals
of random signs and lengths; in a rounded row, ddust rounds in the mode of the
fastest other crate. [Performance][performance] has every operation, both
widths, and how each was measured.

| Operation        | ddust | rust_decimal | The fastest other                   |
| ---------------- | ----: | -----------: | ----------------------------------- |
| Checked sum      |  0.76 |         9.79 | 0.73, primitive_fixed_point_decimal |
| Rounded product  |  3.73 |         30.7 | 3.68, primitive_fixed_point_decimal |
| Rounded quotient |  4.49 |          114 | 8.18, nexus-decimal                 |
| To cents         |  1.14 |         21.8 | 3.21, primitive_fixed_point_decimal |
| Read text        |  9.75 |         19.1 | 19.1, rust_decimal                  |
| Write text       |  9.84 |         67.0 | 23.3, nexus-decimal                 |

## Features

| Feature            | What it adds                                                                                                                                                                                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `arbitrary`        | arbitrary's `Arbitrary`, for fuzzing: any steps at any scale, and any rounding mode; needs `std`                                                                                                                                                                                            |
| `bytemuck`         | bytemuck's `Pod` for a decimal of a `Fixed` scale, `D8` to `UD128`, read from and written as its steps' bytes; `Zeroable` for a decimal of a `Dynamic` one; and `NoUninit` and `Zeroable` for `Dynamic`, to write a scale's byte                                                            |
| `defmt`            | defmt's `Format`, to log a decimal from a device as its text, written without `core::fmt`                                                                                                                                                                                                   |
| `num-traits-02`    | num-traits 0.2's `Zero`, `ConstZero`, `Bounded`, `Checked*`, `Saturating*` and `Wrapping*` traits, `FromPrimitive` and `ToPrimitive`                                                                                                                                                        |
| `proptest`         | proptest's `Arbitrary`, for property tests: any steps at any scale, and any rounding mode                                                                                                                                                                                                   |
| `rand-09`          | rand 0.9's `SampleUniform`, to draw a decimal uniformly from a range, through `ddust::rand_09::UniformDecimal`                                                                                                                                                                              |
| `runtime-dispatch` | a check of the CPU, once, on an `x86_64` build without SSSE3, so text is read and written with SSSE3 where the CPU has it                                                                                                                                                                   |
| `serde`            | serde's `Serialize` and `Deserialize`: the text where a person reads the format, the steps where none does, and the modules in `ddust::serde` that choose the form at a field                                                                                                               |
| `zerocopy-08`      | zerocopy 0.8's `FromBytes` and `IntoBytes` for a decimal of a `Fixed` scale, `D8` to `UD128`, read from and written as its steps' bytes; `KnownLayout` and `Immutable` for every decimal; and `IntoBytes` and `Unaligned` for `Dynamic`, to write its byte, and for `Fixed`, which has none |

None is on by default, and ddust has no dependencies without one. A feature is
turned on as it is added:

```sh
cargo add ddust --features serde
```

## Documentation

- [The book][book]: decimals and scales, rounding, text and floats, what each
  operation costs, and how ddust compares with the other decimal crates.
- [The API on docs.rs][docs.rs], with an example for each method.
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first, and report a vulnerability as [SECURITY.md][security] says.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[book]: https://atomix-labs.github.io/ddust/
[performance]: https://atomix-labs.github.io/ddust/performance.html
[docs.rs]: https://docs.rs/ddust
[changelog]: https://github.com/atomix-labs/ddust/blob/main/CHANGELOG.md
[contributing]: https://github.com/atomix-labs/ddust/blob/main/CONTRIBUTING.md
[security]: https://github.com/atomix-labs/ddust/blob/main/SECURITY.md
[mit]: https://github.com/atomix-labs/ddust/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/ddust/blob/main/LICENSE-APACHE
