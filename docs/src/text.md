# Text

Every value has one spelling. `Display` writes its shortest exact decimal, and
the readers take it back exactly or refuse it. Nothing on that path goes through
a float.

## Reading

`FromStr`, [`from_ascii`][ddust::Decimal::from_ascii] for bytes, and the other
readers take what `f64::from_str` takes, without its infinities and NaN: a sign,
digits with an optional point, and an optional exponent. Zeros past the scale's
decimals are allowed; a non-zero digit there is refused, never rounded:

```rs
{{#include ../../crates/ddust/tests/book/text.rs:parse}}
```

A refusal is a [`ParseError`], whose [`kind`][ddust::ParseError::kind] says why:

| [`ParseErrorKind`]   | The text                                              |
| -------------------- | ----------------------------------------------------- |
| `Empty`              | is empty                                              |
| `InvalidDigit`       | is not a number: a sign, digits, a point, an exponent |
| `PosOverflow`        | is above the decimal's range                          |
| `NegOverflow`        | is below it                                           |
| `TooManyDecimals`    | has a non-zero digit past the decimal's scale         |
| `RangeOutsideBuffer` | was asked for at a range its buffer does not hold     |

A decimal of a run-time scale reads at the scale its text spells, so `"1.50"` is
1.50 at two decimals; `from_ascii` takes a scale to read at instead.

Text with more decimals than the scale is rounded only when asked, by
[`from_ascii_round`][ddust::Decimal::from_ascii_round]:

```rs
{{#include ../../crates/ddust/tests/book/text.rs:round}}
```

## Reading Inside a Message

A number inside a message is read where it lies, with no copy.
[`from_ascii_at`][ddust::Decimal::from_ascii_at] takes the buffer and the range
the number fills, and [`from_ascii_prefix`][ddust::Decimal::from_ascii_prefix]
reads the number at the front of a slice and says how many bytes it took, for a
reader that has not found where it ends:

```rs
{{#include ../../crates/ddust/tests/book/text.rs:message}}
```

On a build with a vector unit, a number of up to 32 bytes is read in 32 lanes at
once, NEON's on aarch64 and SSSE3's on `x86_64`. `from_ascii` reads only its
slice: the first 16 bytes and the last 16 of a number that long, and of a
shorter one its first eight and last eight, or the one word it fits in.
`from_ascii_at` reads the 16 bytes from the number's start and the 16 before its
end wherever the buffer holds them, so it reads every number the same way,
whatever its length. [Platforms and Features](platforms-and-features.md) says
which builds have the lanes.

## Writing

`Display` writes the shortest exact decimal. `{:#}` writes every decimal of the
scale, `{:.N}` exactly `N` decimals, a tie rounded to even as `f64`'s is, and
width, fill and sign work as an integer's do. `Debug` writes as `{:#}`, so a
run-time value's scale shows, and `LowerExp` and `UpperExp` write scientific
notation:

```rs
{{#include ../../crates/ddust/tests/book/text.rs:display}}
```

[`write_ascii`][ddust::Decimal::write_ascii] writes the same text as `Display`
into a byte buffer, with no formatter, and returns its length, or `None` when
the buffer is too short. It touches no byte past the text, and [`MAX_ASCII_LEN`]
bytes, 41, hold every value's: a sign, 39 digits and a point.

```rs
{{#include ../../crates/ddust/tests/book/text.rs:write}}
```
