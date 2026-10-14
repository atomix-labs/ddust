# Interop

Each crate ddust works with is behind a feature, so a program that does not use
it builds none of it. [Serialization](serialization.md) covers serde, and
[Testing](testing.md) proptest, arbitrary and rand.

## Numeric Traits

With `num-traits-02`, a decimal implements num-traits 0.2's traits, so generic
numeric code takes it:

| Trait                                                          | For                                      |
| -------------------------------------------------------------- | ---------------------------------------- |
| `CheckedAdd`, `CheckedSub`, `CheckedDiv`, `CheckedRem`         | every decimal                            |
| `CheckedNeg`, `WrappingNeg`                                    | every signed decimal                     |
| `SaturatingAdd`, `SaturatingSub`, `WrappingAdd`, `WrappingSub` | a static scale, and `Dynamic`            |
| `CheckedMul`                                                   | `Dynamic`, whose product is its own type |
| `Zero`, `ConstZero`, `Bounded`, `FromPrimitive`                | a static scale                           |
| `ToPrimitive`                                                  | every decimal                            |

```rs
{{#include ../../crates/ddust/tests/book/interop.rs:num_traits}}
```

Each trait's method is the decimal's own method of that name. A product of a
static scale is at the sum of its factors' scales, another type, so `CheckedMul`
is `Dynamic`'s alone; and the saturating and wrapping traits never panic, so a
scale of another crate, whose values may refuse to mix, has none of them.
`FromPrimitive` reads a whole number exactly and a float at the nearest step, a
tie to the even one, and returns `None` past the range or for a float that is
not finite. `ToPrimitive` gives the whole part, truncated toward zero as a
float's `as` is, or `None` past the integer's range, and the nearest `f64`, from
which an `f32` is then rounded: rounded twice, so in rare cases a step from the
`f32` nearest the decimal.

## bytemuck and zerocopy

A decimal of a `Fixed` scale is its integer and nothing else, so its bytes are
its steps'. With `bytemuck`, it is bytemuck's `Pod`, and a slice of decimals
casts to bytes and back with no copy:

```rs
{{#include ../../crates/ddust/tests/book/interop.rs:bytemuck}}
```

With `zerocopy-08`, it is zerocopy 0.8's `FromBytes` and `IntoBytes`, and every
decimal is `KnownLayout` and `Immutable`:

```rs
{{#include ../../crates/ddust/tests/book/interop.rs:zerocopy}}
```

The bytes are the steps alone, so the scale comes from the type: a reader of the
bytes knows them as a `D64<2>` because its schema says so. A decimal of a
run-time scale is never read from bytes, since its scale's byte must be 0 to 38,
nor written as bytes, since its padding holds no value; bytemuck can zero it,
and a `Dynamic` alone is written as its byte. The compiler refuses the rest:

```rs
{{#include ../../crates/ddust/tests/compile_fail_bytes/a_run_time_scale_is_never_read_from_bytes_by_zerocopy.rs}}
```

```text
{{#include ../../crates/ddust/tests/compile_fail_bytes/a_run_time_scale_is_never_read_from_bytes_by_zerocopy.stderr}}
```

## defmt

With `defmt`, a decimal implements defmt's `Format`, so firmware logs it as its
shortest exact decimal, the text `Display` writes, without `core::fmt`.
`Fixed<D>` logs as its `Debug` writes it, with its decimals.
