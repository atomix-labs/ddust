# Floats

A double is a binary fraction, so most decimals fall between two doubles: the
double nearest 0.1 is 0.1000000000000000055511151231257827…, and Rust reads and
writes it as `0.1` only because no shorter decimal is nearer to it. A decimal
converts to and from a double at the edges of a program, where a library, a
chart or a model speaks in floats, and each way says how it rounds.

## To a Double

[`to_f64`][ddust::Decimal::to_f64] gives the nearest double, a tie to the even
one: exact whenever the value is, and correctly rounded otherwise, as a decimal
read from text is, so it is the double Rust reads the decimal's text as:

```rs
{{#include ../../crates/ddust/tests/book/floats.rs:to_f64}}
```

## From a Double

[`from_f64`][ddust::Decimal::from_f64] takes the double's exact binary value to
the nearest step at a scale by a rounding mode, or returns `None` for NaN, an
infinity, or a value past the range:

```rs
{{#include ../../crates/ddust/tests/book/floats.rs:from_f64}}
```

Rounded half to even, the nearest step of a double is the decimal it was written
from, while the steps stay below $2^{52}$: the double 0.1 is 0.10 at two
decimals. A directed mode sees the double's binary digits past the decimal ones:
rounded up by `Ceil`, the double 0.1 is 0.11 at two decimals.

`from_f64` is a `const fn`. [Serialization](serialization.md) reads and writes a
decimal as a double through `ddust::serde::float`, and num-traits'
`FromPrimitive` reads one, both at the nearest step, a tie to the even one.
