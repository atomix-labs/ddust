# Getting Started

Add the crate:

```sh
cargo add ddust
```

ddust builds on nightly Rust, from `nightly-2026-09-28`, since its arithmetic is
`const fn` through const traits, which are not stable yet. A
`rust-toolchain.toml` beside the project's `Cargo.toml` pins it:

```toml
[toolchain]
channel = "nightly-2026-09-28"
```

It needs no feature, and builds without `std` or an allocator.
[Platforms and Features](platforms-and-features.md) lists what each feature
adds.

## A First Program

A price read exactly from its text and a quantity written as a literal make a
notional, first exactly and then rounded to the cent:

```rs
{{#include ../../crates/ddust/tests/book/getting_started.rs:first}}
```

Each type names its scale: the price has two decimals, the quantity four, and
their product six, so the product is exact. Rounding happens only where a mode
is named, here half to even: 983.036475 is billed as 983.04.
[Decimals and Scales](decimals-and-scales.md) shows what a scale is, and
[Rounding](rounding.md) each mode.

Every listing in this book runs as a test:

```sh
cargo test -p ddust --all-features --test book
```

From there, the crate's page, [`ddust`], lists every type.
