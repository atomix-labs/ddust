# Platforms and Features

ddust is `#![no_std]`, allocates nothing, and has no dependencies unless a
feature asks for one. It builds on nightly Rust, from `nightly-2026-09-28`,
since its arithmetic, rounding and literals are `const fn` over every integer
through const traits, which are not stable yet.

## Where It Is Tested

| Target                                        | What runs                                                                  |
| --------------------------------------------- | -------------------------------------------------------------------------- |
| `x86_64` Linux, at `x86-64-v2`                | every test and doctest, with every feature, and the book's listings        |
| arm64 Linux                                   | every test, with every feature, with none, and in release                  |
| macOS on Apple silicon, and on Intel          | the same                                                                   |
| `x86_64` Linux, at plain `x86-64`             | every test, debug and release, with no feature and with `runtime-dispatch` |
| `x86_64` Linux, at `x86-64-v3`                | every test, debug and release                                              |
| `aarch64-unknown-none`, `x86_64-unknown-none` | the build, with no feature and with every feature but `arbitrary`'s        |
| `i686-unknown-linux-gnu`                      | `cargo check`, with every feature                                          |

A release build runs the tests where overflow wraps rather than panics, and the
bare-metal builds fail on any use of `std`. Every night, each feature is linted
alone. The fuzz targets, the readers at each width, the arithmetic and the float
conversions, run over the committed corpus, then for a minute each, on every
change.

## Text in Vector Lanes

Text is read in 32 vector lanes at once, through `core::simd`, when it is up to
32 bytes long, and written in them when its steps are below $10^{31}$, at up to
30 decimals, where the build has the instructions:

| Build                                                           | Text is read and written                                    |
| --------------------------------------------------------------- | ----------------------------------------------------------- |
| aarch64, little-endian, with NEON: every one but the soft-float | in NEON's lanes                                             |
| `x86_64` with SSSE3: `-C target-cpu=x86-64-v2` or newer         | in SSSE3's lanes, with no check at run time                 |
| `x86_64` without SSSE3, with `runtime-dispatch`                 | in SSSE3's lanes where a check of the CPU, once, finds them |
| anything else, and any other text                               | eight bytes at a time in a 64-bit word                      |

The repository's own builds take `x86-64-v2` as their floor, which every
`x86_64` CPU since 2009 meets. On aarch64 with NEON, whose floating-point
division is pipelined, a quotient within a double's reach is taken from a
double's estimate, then corrected to the exact one.

## Crate Features

| Feature            | What it adds                                                                                                                                                                                                                                                                                    |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `arbitrary`        | arbitrary's `Arbitrary`, for fuzzing: any steps at any scale, and any rounding mode; needs `std`                                                                                                                                                                                                |
| `bytemuck`         | bytemuck's `Pod` for a decimal of a `Fixed` scale, [`D8`] to [`UD128`], read from and written as its steps' bytes; `Zeroable` for a decimal of a `Dynamic` one; and `NoUninit` and `Zeroable` for `Dynamic`, to write a scale's byte                                                            |
| `defmt`            | defmt's `Format`, to log a decimal from a device as its text, written without `core::fmt`                                                                                                                                                                                                       |
| `num-traits-02`    | num-traits 0.2's `Zero`, `ConstZero`, `Bounded`, `Checked*`, `Saturating*` and `Wrapping*` traits, `FromPrimitive` and `ToPrimitive`                                                                                                                                                            |
| `proptest`         | proptest's `Arbitrary`, for property tests: any steps at any scale, and any rounding mode                                                                                                                                                                                                       |
| `rand-09`          | rand 0.9's `SampleUniform`, to draw a decimal uniformly from a range, through `ddust::rand_09::UniformDecimal`                                                                                                                                                                                  |
| `runtime-dispatch` | On an `x86_64` build without SSSE3, a check of the CPU, once, so text is read and written with SSSE3 where the CPU has it                                                                                                                                                                       |
| `serde`            | serde's `Serialize` and `Deserialize`: the text where a person reads the format, the steps where none does, and the modules in `ddust::serde` that choose the form at a field                                                                                                                   |
| `zerocopy-08`      | zerocopy 0.8's `FromBytes` and `IntoBytes` for a decimal of a `Fixed` scale, [`D8`] to [`UD128`], read from and written as its steps' bytes; `KnownLayout` and `Immutable` for every decimal; and `IntoBytes` and `Unaligned` for `Dynamic`, to write its byte, and for `Fixed`, which has none |

A feature that names a crate's version, `num-traits-02`, `rand-09` and
`zerocopy-08`, follows that version, so a new major version of the crate comes
as a feature of its own, beside the old one. [Interop](interop.md) shows each
integration at work.
