# Compared with Others

Each crate below is described from its own source and documentation, at the
version the benchmarks build, so every claim can be checked there. Each speed is
from the run [Performance](performance.md) cites, where every crate is measured
in one binary on one machine, on values every crate holds, and every result is
checked against an exact reference.

## Speed

Nanoseconds per operation on the unpredictable values, ddust's rounding half to
even and each other crate's in its own mode, as [Performance](performance.md)
names it with ddust beside it in each:

| Crate                                          |  Sum | Rounded product | Rounded quotient | To cents |            Read | Write |         The mix |
| ---------------------------------------------- | ---: | --------------: | ---------------: | -------: | --------------: | ----: | --------------: |
| **ddust** `D64<8>`                             | 0.77 |            3.93 |             5.38 |     1.30 |            9.59 |  9.97 |            1.74 |
| fixdec 0.1.0 `D64`                             | 0.72 | 4.53, *inexact* |             8.17 |     3.82 |            23.0 |  40.5 | 3.24, *inexact* |
| nexus-decimal 1.2.2 `<i64, 8>`                 | 0.72 |            5.64 |             8.12 |     4.27 |            19.1 |  23.4 |            3.42 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | 0.73 |            3.70 |             11.0 |     3.22 |            20.8 |  33.2 |            3.63 |
| fin_decimal 0.4.0 `Decimal<8>`                 | 0.75 |            9.80 |             11.4 |     5.00 | 21.0, *inexact* |  26.5 |            4.87 |
| fastnum 0.7.5 `D64`                            | 16.7 |            33.7 |             86.1 |     18.2 |            24.0 |  79.0 |            31.1 |
| rust_decimal 1.43.0                            | 9.82 |            30.8 |              114 |     21.9 |            19.1 |  65.8 |            35.1 |
| decimal-rs 0.2.0                               | 7.65 |            40.9 |              198 |     6.50 |            43.1 |  44.0 |            51.0 |
| bigdecimal 0.4.11                              | 36.8 |             125 |            4,394 |     76.5 |            91.3 |   110 |             931 |
| fixnum 0.9.5 `<i64, U8>`                       | 0.76 |            13.4 |             14.2 |          |            28.6 |  29.0 |                 |

| Crate                                            |  Sum | Rounded product | Rounded quotient | To cents |            Read | Write | The mix |
| ------------------------------------------------ | ---: | --------------: | ---------------: | -------: | --------------: | ----: | ------: |
| **ddust** `D128<18>`                             | 1.06 |            8.07 |             15.9 |     4.19 |            10.0 |  23.4 |    5.68 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | 1.09 |            7.30 |             29.9 |     8.40 |            39.6 |  77.3 |    8.40 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | 1.07 |            8.68 |             22.3 |     14.7 |            20.9 |  49.3 |    8.52 |
| fin_decimal 0.4.0 `Decimal128<18>`               | 1.08 |            12.9 |             27.4 |     8.92 | 35.7, *inexact* |  45.8 |    9.77 |
| rust_decimal 1.43.0                              | 10.6 |            57.8 |             98.5 |     43.7 |            30.2 |  91.0 |    42.2 |
| decimal-rs 0.2.0                                 | 7.61 |            52.6 |              198 |     9.47 |            62.1 |  48.4 |    53.3 |
| fastnum 0.7.5 `D128`                             | 18.3 |            44.7 |              206 |     24.0 |            32.0 |   104 |    60.0 |
| bigdecimal 0.4.11                                | 47.0 |             172 |           10,117 |     91.2 |             118 |   137 |   2,075 |
| fixnum 0.9.5 `<i128, U18>`                       | 1.09 |            17.4 |             34.7 |          |            65.9 |  42.6 |         |

The fixed-point crates add as their integers do, and so do they all; past the
sum, ddust is the fastest at every operation but the narrow rounded product,
where primitive_fixed_point_decimal's half away from zero is 3.70 ns and ddust's
3.73 in the same mode, and the wide one, 7.30 against 7.20. The decimals that
carry a scale in each value take ten to a hundred times as long, and bigdecimal,
which allocates, longer still.

## rust_decimal

[rust_decimal](https://docs.rs/rust_decimal/1.43.0) 1.43.0 is the decimal most
Rust programs use. Its `Decimal` is a 96-bit magnitude, a sign and a scale from
0 to 28, in 16 bytes; each value carries its own scale, and an operation chooses
its result's.

| rust_decimal 1.43.0                                                                                     | ddust                                                                           |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| a scale per value, 0 to 28; `+` lines two up                                                            | a scale in the type, 0 to 38, or carried at run time and lined up               |
| 28 or 29 significant digits, in 16 bytes                                                                | the integer's: 18 in 8 bytes, 38 in 16, down to 2 in 1                          |
| `*` exact while the product fits, then rounded half to even                                             | `*` exact, its scale the sum of the factors', or an overflow                    |
| `/` rounded half to even at the last digit that fits                                                    | `/` truncated at the dividend's scale, or rounded by a mode when asked          |
| seven rounding strategies, taken by `round_dp_with_strategy` and `round_sf_with_strategy`               | nine modes, taken by every operation that can lose a digit                      |
| operators that panic past the range in every build; `checked_*` and `saturating_add`, `_sub` and `_mul` | operators that panic or wrap as the integer's; all four families                |
| `FromStr` rounds a digit past 28 decimals or 96 bits; `from_str_exact` refuses it                       | every reader exact or refused; `from_ascii_round` rounds by a mode              |
| `Display` keeps the trailing zeros a value carries                                                      | `Display` writes the shortest exact decimal; `{:#}` every decimal               |
| `from_f64` from the double's shortest digits; `to_f64` not correctly rounded                            | `from_f64` from the double's exact value, by a mode; `to_f64` correctly rounded |
| `no_std` with `alloc`                                                                                   | `no_std` with no allocator                                                      |
| stable Rust, from 1.67.1                                                                                | nightly Rust                                                                    |

rust_decimal has what ddust has not: square roots, exponentials, logarithms,
powers and trigonometry, under its `maths` feature; rounding to significant
figures; reading text in any radix; and integrations with diesel, postgres,
tokio-postgres, borsh, ndarray and rocket. Take it on stable Rust, where values
of many scales meet and each result should keep as many digits as fit, or where
those integrations or functions are needed.

## fastnum

[fastnum](https://docs.rs/fastnum/0.7.5) 0.7.5 is decimal floating point, after
IEEE 754 and the General Decimal Arithmetic specification: a coefficient of 64
to 1,024 bits and an exponent, with NaN, the infinities, signed zero and
signals. A value carries a context, which holds its rounding mode, one of seven.
A coefficient that fills rounds away its last digits and signals that it did; an
exponent past the range gives an infinity, and a trapped signal panics only with
`debug_assertions` on. It has square roots, exponentials, logarithms, powers and
trigonometry, and integrations with diesel, sqlx and tokio-postgres; it needs
`alloc`, and builds on stable Rust from 1.94. Take it for IEEE decimal
semantics, for precision past 38 digits, or for those functions.

## bigdecimal

[bigdecimal](https://docs.rs/bigdecimal/0.4.11) 0.4.11 is arbitrary precision: a
heap-allocated integer of any length and a 64-bit scale, so no value overflows.
A quotient is rounded to 100 significant digits unless a build-time setting says
otherwise, and a `Context` sets the precision of the functions that take one:
square and cube roots, exponentials and powers. Take it where a value can
outgrow 38 digits and allocation is acceptable.

## The Fixed-Point Crates

These crates are decimals of a fixed scale, as ddust's static ones are, each in
one of Rust's integers:

| Crate                                                                                      | Scale                                     | Rounding                        | `*` and `/`                       | An operator past the range                                       | Text past the scale               | `no_std` |
| ------------------------------------------------------------------------------------------ | ----------------------------------------- | ------------------------------- | --------------------------------- | ---------------------------------------------------------------- | --------------------------------- | -------- |
| ddust                                                                                      | a const generic, or at run time           | nine modes, as types or a value | exact; truncated                  | as the integer's                                                 | refused                           | yes      |
| [primitive_fixed_point_decimal](https://docs.rs/primitive_fixed_point_decimal/1.5.0) 1.5.0 | a const generic, or passed with each call | five modes, by an argument      | rounded half away from zero       | `+` and `-` as the integer's; `*` and `/` panic                  | refused                           | yes      |
| [nexus-decimal](https://docs.rs/nexus-decimal/1.2.2) 1.2.2                                 | a const generic                           | four, each a method             | truncated                         | panics in every build                                            | refused                           | yes      |
| [fixnum](https://docs.rs/fixnum/0.9.5) 0.9.5                                               | a type parameter                          | three modes, by an argument     | no operators: methods with a mode | no operators: its methods return an error                        | rounded; `from_str_exact` refuses | yes      |
| [fin_decimal](https://docs.rs/fin_decimal/0.4.0) 0.4.0                                     | a const generic                           | five modes, by an argument      | rounded by its `HalfUp`           | `+` and `-` as the integer's, but on 256 bits; `*` and `/` panic | rounded by its `HalfUp`           | yes      |
| [const-decimal](https://docs.rs/const-decimal/0.4.0) 0.4.0                                 | a const generic                           | truncation alone                | truncated                         | panics in every build                                            | refused                           | no       |

Each builds on stable Rust, and has what ddust has not:
primitive_fixed_point_decimal, negative scales and a scale kept outside the
value; nexus-decimal, helpers for ticks, basis points and spreads; fixnum, a
macro for typed wrappers' arithmetic, and parity-scale-codec, rkyv and schemars;
fin_decimal, `split_evenly`, a correctly rounded square root and 256-bit values;
const-decimal, borsh. Take one of them on stable Rust, choosing by the columns
above.

decimal-rs 0.2.0, also measured, is a 38-digit decimal with a floating exponent,
in 20 bytes, that rounds half up wherever it rounds and needs `std`.

## fixed

[fixed](https://docs.rs/fixed/1.31.0) 1.31.0 is binary fixed point: a number of
fraction bits, not of decimals, so it holds no decimal fraction exactly, as its
own docs say. Its `*` and `/` overflow by `debug_assertions` rather than
`overflow-checks`, and it has the integer's families and more. Take it for
signal processing and embedded arithmetic, where a binary fraction is the point;
the benchmarks time its multiply beside the decimals' for contrast.

## fixdec

fixdec 0.1.0 is measured with its results marked inexact. Its `D64` multiply
divides by $10^8$ through an approximate reciprocal, which is wrong once the
product of the steps reaches $2^{64}$: 1,000 × 1,000 is 1,000,000.0000049.
Parsing an integer part too large for its word wraps silently, and its `D96`
writes and divides some large values wrongly.

## What ddust Gives Up

- **Stable Rust.** ddust needs nightly, for const traits.
- **A scale each value carries and each operation chooses**, as rust_decimal's
  and the decimal floats' do. ddust's run-time scale lines up exactly but never
  grows a quotient's decimals.
- **Arbitrary precision**, and decimal floating point: 38 digits is its limit.
- **NaN, the infinities and negative zero.**
- **Square roots, exponentials, logarithms, powers and trigonometry.**
- **The integrations only time brings**: diesel, sqlx, postgres, borsh, rkyv,
  and the rest of a crate that has been used for years.
