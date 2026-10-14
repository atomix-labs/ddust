# Serialization

With the `serde` feature, a decimal implements serde's `Serialize` and
`Deserialize`. Where a person reads the format, as in JSON, TOML or YAML, a
decimal is its text; where none does, as in bincode or postcard, it is its
steps. Text is read exactly, as [Text](text.md) reads it, and a float is
refused, since it has already lost the digits a decimal keeps:

```rs
{{#include ../../crates/ddust/tests/book/serialization.rs:text}}
```

## Choosing a Field's Form

A field takes another form through `#[serde(with = …)]` and one of the modules
in `ddust::serde`, each of which has an `option` twin for an `Option` field:

| Form              | Where a person reads the format | Where none does |
| ----------------- | ------------------------------- | --------------- |
| the decimal's own | its text                        | its steps       |
| `text`            | its text                        | its text        |
| `steps`           | its steps, as a number          | its steps       |
| `float`           | its nearest double              | the same        |

Each reads what it writes; `text` and the decimal's own form read a whole number
too, and `steps` a string of digits, where a person reads the format:

```rs
{{#include ../../crates/ddust/tests/book/serialization.rs:fields}}
```

`steps` suits a format whose numbers are integers and whose scale the schema
already states. `float` suits a consumer that takes only doubles: a double is
rarely exact at a decimal scale, and is read back at the nearest step, as
[Floats](floats.md) says.

## Run-Time Scales

A decimal of a run-time scale is written as every decimal of its scale, so
`1.50` stays at two decimals, and is read at the scale its text spells. Where no
person reads the format, it is its steps and its decimals, and more than 38
decimals are refused:

```rs
{{#include ../../crates/ddust/tests/book/serialization.rs:dynamic}}
```

The modules take a decimal of a static scale, whose scale the type already
gives.
