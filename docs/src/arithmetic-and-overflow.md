# Arithmetic and Overflow

Values of one scale add, subtract, compare and take a remainder as their steps
do, so `+` on two `D64<2>`s is the `i64`'s add and nothing more. A value times
an integer, or divided by one, is a count of it:

```rs
{{#include ../../crates/ddust/tests/book/arithmetic.rs:operators}}
```

| Operation                        | Its scale               | Exact                 |
| -------------------------------- | ----------------------- | --------------------- |
| `a + b`, `a - b`, `a % b`, `-a`  | the operands'           | yes                   |
| `a * n` and `n * a`, `n` an `I`  | `a`'s                   | yes                   |
| `a * b`                          | the sum of the factors' | yes                   |
| `a / n`, `n` an `I`              | `a`'s                   | truncated toward zero |
| `a / b`                          | the dividend's          | truncated toward zero |
| `==`, `<` and the other compares |                         | yes                   |

Each operator has its assigning form, `+=` and the rest, and a decimal sums from
an iterator, starting from zero at its scale.

## Division Truncates

A quotient is at the dividend's scale, truncated toward zero as the integer's
`/` is:

```rs
{{#include ../../crates/ddust/tests/book/arithmetic.rs:division}}
```

The operators never take a rounding mode, since they behave as the integer's. To
round a quotient, or to take it at another scale, call
[`div_round`][ddust::Decimal::div_round] or
[`div_round_to`][ddust::Decimal::div_round_to] with a mode, as
[Rounding](rounding.md) shows. A quotient by zero panics in every build, as the
integer's does.

## Overflow

Past the range, an operator does what the integer's does: it panics with
overflow checks on, as in a debug build, and wraps otherwise, as in a release
build. The profile's `overflow-checks` decides, never `debug-assertions`:

```rs
{{#include ../../crates/ddust/tests/book/arithmetic.rs:overflow}}
```

A release build that should panic instead sets `overflow-checks = true` in its
profile. Where the range can be passed in either build, the integer's four
families say what happens instead:

```rs
{{#include ../../crates/ddust/tests/book/arithmetic.rs:families}}
```

| Family          | Past the range                       |
| --------------- | ------------------------------------ |
| `checked_*`     | `None`                               |
| `saturating_*`  | the end of the range it passes       |
| `wrapping_*`    | wrapped around the range             |
| `overflowing_*` | wrapped around the range, and `true` |

Each family has `add`, `sub`, `mul` (the exact product), `div`, `mul_int` and
`div_int` (by a count), `neg` and `abs`; `rem` has `checked_rem`, `wrapping_rem`
and `overflowing_rem`, as the integer has. A `checked_*` method returns `None`
for a zero divisor too, where its operator panics.

## Unsigned Decimals

An unsigned decimal, [`UD64`] and its siblings, stops at zero as its integer
does:

```rs
{{#include ../../crates/ddust/tests/book/arithmetic.rs:unsigned}}
```

It has no negation, which the compiler refuses:

```rs
{{#include ../../crates/ddust/tests/compile_fail/an_unsigned_decimal_never_negates.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail/an_unsigned_decimal_never_negates.stderr}}
```

[`unsigned_abs`][ddust::Decimal::unsigned_abs] takes a signed value's magnitude
as an unsigned decimal, exact for every value, the minimum included.
