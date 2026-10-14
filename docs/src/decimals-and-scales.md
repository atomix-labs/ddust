# Decimals and Scales

A [`Decimal<I, S>`][ddust::Decimal] is a whole number of steps, held in an
integer `I`, at a scale `S` that says how many decimals a step is. 12.34 at two
decimals is 1,234 steps of 0.01:

```rs
{{#include ../../crates/ddust/tests/book/decimals.rs:steps}}
```

The steps are the decimal's whole state. Adding two values of one scale adds
their steps, comparing them compares their steps, and nothing is normalized or
rescaled behind the caller's back.

## The Integer

`I` is one of Rust's ten integers, which [`Int`] names; the five signed ones are
[`Signed`] too. Each has an alias at a static scale of `D` decimals:

| Signed    | Unsigned   | Bytes | Digits every value has room for |
| --------- | ---------- | ----- | ------------------------------- |
| `D8<D>`   | `UD8<D>`   | 1     | 2                               |
| `D16<D>`  | `UD16<D>`  | 2     | 4                               |
| `D32<D>`  | `UD32<D>`  | 4     | 9                               |
| `D64<D>`  | `UD64<D>`  | 8     | 18, and 19 unsigned             |
| `D128<D>` | `UD128<D>` | 16    | 38                              |

The scale splits those digits between the whole part and the fraction: a
`D64<8>` has room for ten whole digits and eight decimals, and a `D128<18>` for
twenty and eighteen. The integer's range is the decimal's, so a `D64<2>` runs
from −92,233,720,368,547,758.08 to 92,233,720,368,547,758.07.

## The Scale

[`Fixed<D>`][ddust::Fixed] is a scale the compiler knows: zero-sized, so a
decimal of it is its integer alone, and two values of it never differ in scale.
`D` is at most [`MAX_DECIMALS`][ddust::scale::MAX_DECIMALS], 38, since $10^{38}$
is the largest power of ten a 128-bit integer holds; a `Fixed<39>` fails the
build. [`Dynamic`] carries its scale at run time, in a byte beside the steps,
and has a chapter of its own, [Run-Time Scales](run-time-scales.md).

Two static scales are two types, so their values never meet until one is
converted:

```rs
{{#include ../../crates/ddust/tests/compile_fail/two_static_scales_never_mix.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail/two_static_scales_never_mix.stderr}}
```

## Making a Decimal

A program reads a decimal from text, as [Text](text.md) shows, or writes it as a
literal. [`dec!`] takes its type from where it goes, or from a type named after
a colon, and is checked when the constant is evaluated: a literal with more
decimals than its scale, past its range, or negative for an unsigned decimal
fails the build. The value is the literal exactly, never rounded.

```rs
{{#include ../../crates/ddust/tests/book/decimals.rs:literal}}
```

From digits known at run time, [`new`][ddust::Decimal::new] takes the digits,
their decimals and the scale, and makes
$\text{digits} \times 10^{-\text{decimals}}$ exactly, or refuses it with a
[`ConvertError`]: `TooManyDecimals` for a non-zero digit past the scale,
`PosOverflow` or `NegOverflow` past the range.
[`new_round`][ddust::Decimal::new_round] rounds where `new` refuses, and
[`to_digits`][ddust::Decimal::to_digits] goes back:

```rs
{{#include ../../crates/ddust/tests/book/decimals.rs:new}}
```

[`from_steps`][ddust::Decimal::from_steps] and [`steps`][ddust::Decimal::steps]
are the raw representation, for storage and for types built on a decimal.
[`from_int`][ddust::Decimal::from_int] makes a whole number, and
[`from_f64`][ddust::Decimal::from_f64] reads a double, as [Floats](floats.md)
shows.

## Widths

A value moves to a wider integer at the same scale by `From`, since every value
fits, and back by `TryFrom`, which refuses one that does not.
[`widen`][ddust::Decimal::widen] moves to the integer twice as wide, from `D8`
to `D16` up to `D64` to `D128`, and the unsigned alike, where a product or more
digits have room:

```rs
{{#include ../../crates/ddust/tests/book/decimals.rs:widths}}
```

Values of two widths never mix, as two integers do not:

```rs
{{#include ../../crates/ddust/tests/compile_fail/two_widths_never_mix.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail/two_widths_never_mix.stderr}}
```

## Products

A product's scale is the sum of its factors' decimals, so a product of two
decimals is exact. Its type is a [`Sum`][ddust::scale::Sum] of the two scales,
which converts into the `Fixed` of their sum with its steps unchanged; a `Fixed`
of any other decimals fails the build:

```rs
{{#include ../../crates/ddust/tests/book/decimals.rs:product}}
```

A product's scale could be written in the type, `Fixed<{ A + B }>`, only with
the unstable and incomplete `generic_const_exprs`; `Sum` gives the same result
without it.
