# Testing

Code that uses ddust is tested as code that uses integers is: a decimal is a
value, with no clock, no allocator and no global state, so a test that builds
its decimals from literals is deterministic. Three features make more of them.

## Property Tests

With `proptest`, a decimal implements proptest's `Arbitrary`: any steps, at any
scale its scale type takes, from 0 to 38 decimals for `Dynamic`. A rounding mode
is `Arbitrary` too, any of the nine:

```rs
{{#include ../../crates/ddust/tests/book/testing.rs:proptest}}
```

Any steps means the ends of the range too, so a property that adds two values
meets an overflow, and is written in the `checked_*` forms, or over a narrower
strategy of its own.

## Fuzzing

With `arbitrary`, a decimal implements arbitrary's `Arbitrary`, the trait
cargo-fuzz and libFuzzer targets draw their inputs through: a fuzz target takes
decimals from the fuzzer's bytes as it takes integers. The feature needs `std`,
as arbitrary does:

```rs
{{#include ../../crates/ddust/tests/book/testing.rs:arbitrary}}
```

## Random Values

With `rand-09`, a decimal implements rand 0.9's `SampleUniform`, through
[`UniformDecimal`][ddust::rand_09::UniformDecimal], so it is drawn uniformly
from a range as an integer is: its steps from the bounds' steps. Seeded, the
draws are the same on every run:

```rs
{{#include ../../crates/ddust/tests/book/testing.rs:rand}}
```

## Overflow in Tests

Cargo's test profile has overflow checks on, so an operator past the range
panics in a test even where it wraps in the release build. A test of what
happens past the range calls the `checked_*`, `saturating_*`, `wrapping_*` or
`overflowing_*` method, which does the same in every profile, and a test of the
operator itself is gated on the build's checks, as
[Arithmetic and Overflow](arithmetic-and-overflow.md#overflow) shows.
