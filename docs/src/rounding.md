# Rounding

An operation that can lose a digit takes a rounding mode as its last argument,
so it never rounds unless asked to. A mode is one of nine zero-sized types in
[`round`][ddust::round], which the compiler resolves where the operation is
inlined, or a [`Rounding`][ddust::round::Rounding], the same nine as a value
chosen at run time.

## The Nine Modes

| Mode                                     | Rounds                                         |
| ---------------------------------------- | ---------------------------------------------- |
| [`Floor`][ddust::round::Floor]           | toward −∞                                      |
| [`Ceil`][ddust::round::Ceil]             | toward +∞                                      |
| [`Trunc`][ddust::round::Trunc]           | toward zero                                    |
| [`Expand`][ddust::round::Expand]         | away from zero                                 |
| [`HalfFloor`][ddust::round::HalfFloor]   | to the nearer step, and a tie toward −∞        |
| [`HalfCeil`][ddust::round::HalfCeil]     | to the nearer step, and a tie toward +∞        |
| [`HalfTrunc`][ddust::round::HalfTrunc]   | to the nearer step, and a tie toward zero      |
| [`HalfExpand`][ddust::round::HalfExpand] | to the nearer step, and a tie away from zero   |
| [`HalfEven`][ddust::round::HalfEven]     | to the nearer step, and a tie to the even step |

Each takes six values to a whole number here:

```rs
{{#include ../../crates/ddust/tests/book/rounding.rs:modes}}
```

`HalfExpand` is the rounding taught at school, and `HalfEven`, banker's
rounding, is IEEE 754's default and the one that does not drift up over many
ties.

## What Takes a Mode

| Method                                                                                         | What it rounds                                                                  |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| [`mul_round`][ddust::Decimal::mul_round]                                                       | a product, at this value's scale                                                |
| [`mul_round_to`][ddust::Decimal::mul_round_to]                                                 | a product, at a scale named                                                     |
| [`div_round`][ddust::Decimal::div_round]                                                       | a quotient by a decimal or a [`Divisor`][ddust::Divisor], at this value's scale |
| [`div_round_to`][ddust::Decimal::div_round_to]                                                 | a quotient, at a scale named                                                    |
| [`div_int_round`][ddust::Decimal::div_int_round]                                               | one of `n` equal parts                                                          |
| [`mul_div_round`][ddust::Decimal::mul_div_round]                                               | `self × numerator / denominator`, rounded once                                  |
| [`round_to`][ddust::Decimal::round_to]                                                         | to a multiple of a step: a tick, a lot, a coin                                  |
| [`round`][ddust::Decimal::round]                                                               | to a whole number, at this value's scale                                        |
| [`to_int`][ddust::Decimal::to_int]                                                             | to a whole number, as an integer                                                |
| [`rescale_round`][ddust::Decimal::rescale_round]                                               | to another scale                                                                |
| [`new_round`][ddust::Decimal::new_round], [`to_digits_round`][ddust::Decimal::to_digits_round] | digits, to and from a decimal                                                   |
| [`from_ascii_round`][ddust::Decimal::from_ascii_round]                                         | text with more decimals than the scale                                          |
| [`from_f64`][ddust::Decimal::from_f64]                                                         | a double                                                                        |

Each of the arithmetic ones has a `checked_*` twin that returns `None` past the
range, or for a zero divisor. [`trunc`][ddust::Decimal::trunc],
[`floor`][ddust::Decimal::floor] and [`ceil`][ddust::Decimal::ceil] are the
whole-number roundings with their modes named, and
[`fract`][ddust::Decimal::fract] is what `trunc` leaves:

```rs
{{#include ../../crates/ddust/tests/book/rounding.rs:operations}}
```

Every operation rounds once. Its exact result is computed in an integer wide
enough to hold it, twice the decimal's or 256 bits, and the mode settles it from
the remainder: `mul_div_round` takes two thirds of 100.10 with no rounding
between the multiplication and the division.

The compiler refuses an operation that can lose a digit without a mode:

```rs
{{#include ../../crates/ddust/tests/compile_fail/a_lossy_operation_takes_a_mode.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail/a_lossy_operation_takes_a_mode.stderr}}
```

## A Mode Chosen at Run Time

A venue's rules, a configuration or a protocol may choose the mode. A
[`Rounding`][ddust::round::Rounding] is that choice as a value, and every
operation takes it where it takes a mode type:

```rs
{{#include ../../crates/ddust/tests/book/rounding.rs:run_time}}
```

Each mode is a table of sixteen bits, one for each way a division can end: the
result's sign, whether its quotient is odd, and whether the remainder is zero,
below half the divisor, at half or above it. Rounding reads one bit of the
table, so a `Rounding` costs no `match`: a mode is data, whether it is a type, a
constant or a value read at run time. The trait every mode implements,
[`RoundingMode`][ddust::round::RoundingMode], is sealed, so the nine are all
there are:

```rs
{{#include ../../crates/ddust/tests/compile_fail/a_rounding_mode_is_one_of_the_nine.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail/a_rounding_mode_is_one_of_the_nine.stderr}}
```

## Many Values by One Divisor

A [`Divisor`][ddust::Divisor] is a divisor prepared once, for dividing many
values by it: a book's notionals by one price, or amounts by one rate.
[`div_round`][ddust::Decimal::div_round] and
[`checked_div_round`][ddust::Decimal::checked_div_round] take it where they take
a decimal, and give the same quotients:

```rs
{{#include ../../crates/ddust/tests/book/rounding.rs:divisor}}
```

Preparing it costs a division; each quotient then costs two multiplications and
a comparison. [Performance](performance.md#division) measures both.
