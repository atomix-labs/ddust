# Run-Time Scales

[`Dynamic`] is a scale known only at run time: read from a venue's rules, a
database column's type or the text of a number. A decimal of it carries its
scale in a byte beside its steps, so a `Decimal<i64, Dynamic>` is 16 bytes where
a `D64<2>` is 8. Text read into one takes the scale the text spells:

```rs
{{#include ../../crates/ddust/tests/book/run_time.rs:spelled}}
```

[`Dynamic::new`][ddust::Dynamic::new] makes a scale of up to 38 decimals, and
returns `None` past them.

## Lining Up

Two values at different run-time scales line up exactly at the finer one, as
SQL's `DECIMAL` does: the coarser is multiplied by a power of ten, and the
operation is done at the finer scale. A sum, a difference, a remainder, a
comparison and equality all line up, so `1.5 == 1.50`, and the two hash alike. A
product takes the sum of the factors' decimals, as a static one does:

```rs
{{#include ../../crates/ddust/tests/book/run_time.rs:lines_up}}
```

```rs
{{#include ../../crates/ddust/tests/book/run_time.rs:product}}
```

Lining up never rounds. A comparison is exact whatever the two scales are; a sum
or a difference that does not fit at the finer scale overflows as any other
does, panicking with overflow checks on and wrapping otherwise, and its
`checked_*` method returns `None`. A product past 38 decimals, a scale no
integer holds a step of, panics in every build, and
[`checked_mul`][ddust::Decimal::checked_mul] returns `None` for it. A quotient
is at the dividend's scale, as a static one is.

## Rescaling

[`rescale`][ddust::Decimal::rescale] moves a value to another scale exactly, and
refuses a non-zero digit past it with `TooManyDecimals`;
[`rescale_round`][ddust::Decimal::rescale_round] rounds instead. A size read at
five decimals, for a venue whose step is a thousandth:

```rs
{{#include ../../crates/ddust/tests/book/run_time.rs:venue}}
```

## Static and Run-Time Scales

A value of a static scale converts into one of a run-time scale by `From`, at
the same decimals, and back by `TryFrom`, exactly or refused:

```rs
{{#include ../../crates/ddust/tests/book/run_time.rs:static}}
```

A product of a static scale and a run-time one is at a run-time scale. A decimal
of a static scale is the faster and smaller of the two, and a run-time one is
for the values whose scale the program learns as it runs;
[Choosing a Decimal](choosing.md) weighs them.

## Another Crate's Scale

[`Scale`] is a trait a crate built on ddust can implement, for a scale its own
types give, such as a price's scale that its assets decide.
[`LINES_UP`][ddust::Scale::LINES_UP] says what two of its values at different
decimals do: line up at the finer one, as `Dynamic`'s do, or never mix, in which
case an operator panics in every build and a `checked_*` method returns `None`.
