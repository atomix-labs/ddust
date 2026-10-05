# Working in `ddust`

What an agent needs to work here: what the repository is, how to check a change,
and the rules a change keeps.

<!-- >>> devset: agents >>> -->

## Before You Commit

Run `just check`: CI runs the same checks, and names each that fails. `just fix`
fixes what a formatter or linter can, and `just --list` shows every recipe.

## Before You Finish

A change beyond a line or two is ready when its passes have run and `just check`
passes:

- `/humanize` on what the change says to a reader: its comments, docs, names and
  messages.
- `/review-rust` on a change to Rust code.
- `/review-names` on a change that adds, renames or changes what a public item
  does.
- `/review-security` on a change that reads input from outside the process.

Each runs in a fresh context and reports: `/humanize` edits, then says what it
changed and what it left, and a review says what it found. Fix what a pass
leaves or finds, or say why a finding does not hold.

## Managed Files

Profiles, applied by devset, manage some of the files here. `devset status`
names each, and whether a local change to it is kept or is drift; `devset
explain <file>` says which profile owns what in it. What a profile owns changes
with the profile, on `devset update`. Never edit `.devset/`.

<!-- <<< devset: agents <<< -->

## The Repository

A Cargo workspace for `ddust`, a crate of exact fixed-point decimals: a whole
number of steps of `10^-decimals` in an integer from `i8` to `i128`, the scale a
type or a value carried at run time. The crate is `crates/ddust`, and inherits
its version, edition, licence and lints from the root `Cargo.toml`. The book is
under `docs/`. The toolchain is the nightly `rust-toolchain.toml` pins, and the
crate builds only on nightly; the tools are the versions `.config/mise/` pins.

The design and what is left for later are in `.notes/`, which git ignores.

## Rules

What a change here keeps, beyond what the checks hold it to.

### Code

- No better way is left: before code is written, std, the crate's own helpers
  and the ecosystem are searched for what does it more neatly, and the most
  concise form that measures as fast is the one taken. A derivable trait is
  derived; ddust has no dependencies by default, so a trait std cannot derive is
  written by hand. A shape that repeats is one macro or helper.
- Imports, never paths: neither a body nor an attribute names `core::`,
  `crate::` or another crate's path. A doc link may; a `macro_rules!` body names
  `$crate::` and `::core::`.
- Every name is whole words, never a fragment such as `at`, `by` or `held`.
- The crate is `#![no_std]` and allocates nothing outside its tests.
- No `unsafe`, but in the SIMD modules: each `unsafe` block holds one operation
  and a `// SAFETY:` comment, and its kernel has a SWAR twin that the tests and
  the fuzz targets hold it to, byte for byte.
- The crate builds on the nightly `rust-toolchain.toml` pins, and only there.
  `lib.rs` lists each `#![feature]` with what it is for; adding one is a change
  of its own. `rust-version` is the pinned nightly's version, raised by hand
  when a `devset update` moves the nightly.

### Decimals, Scales and Rounding

- Every operation is written once, on `Decimal`, generic over `Int` and `Scale`,
  as a `const fn` whose bounds are `[const]` per method: a crate that enables
  const traits computes with it in a constant, and every crate will once they
  are stable. `dec!`'s constructor and the constants take always-const bounds
  instead: a crate that enables no nightly feature has them in a constant too.
- Arithmetic that can outgrow its integer is a kernel in `kernel.rs`: on
  magnitudes, exact in a word wide enough for the result (the integer's double,
  or a `U256`), and rounded once, by the mode's table. The integer's four
  families read what it came to; no operation rounds twice.
- What can lose digits takes a rounding mode as its last argument, generic over
  `RoundingMode`, so a mode type is compiled in and a `Rounding` decides at run
  time; an exact operation's rounding twin ends in `_round`. The operators never
  take a mode: `*` is exact, and `/` truncates, as the integer's does.
- Every operator behaves as the integer's: overflow panics with overflow checks
  on and wraps otherwise, decided by `cfg!(overflow_checks)`, and each operator
  has the integer's `checked_*`, `saturating_*`, `wrapping_*` and
  `overflowing_*` methods.
- Two static scales never mix: they are two types. Two run-time scales line up
  exactly at the finer one; a scale of another crate says whether its values
  line up or never mix, by `Scale::LINES_UP`.
- Every value has one spelling: `Display` writes its shortest exact decimal, and
  the readers take what `f64::from_str` takes, less its infinities and NaN,
  exactly or refused. Nothing on that path goes through a float, and the `f64`
  conversions are correctly rounded both ways.

### Docs

- Headings are in Title Case, `# Crate Features`, and an example sits under `#
  Examples`.
- Siblings are documented alike: every public method has an example, and every
  refusal one that shows it.
- What the types refuse has a fixture in `crates/ddust/tests/compile_fail/`; a
  check made when a constant is evaluated, past `cargo check`, is a
  `compile_fail` doctest instead. A new toolchain may reword a message;
  `TRYBUILD=overwrite cargo test -p ddust --test trybuild` writes it again, to
  be read before it is committed.

### Checks Beyond `just check`

- A new operation joins `crates/ddust/tests/exhaustive.rs`: every pair of 8-bit
  values, signed and unsigned, against the exact reference.
- A change to a parser or a kernel fuzzes it: `cargo fuzz run <target> --
  -max_total_time=300` in `crates/ddust/fuzz/`, for each target it touches; what
  it adds to the corpus is kept small with `cargo fuzz cmin`, and a crash it
  finds, shrunk with `cargo fuzz tmin`, becomes a unit test.

<!-- >>> devset: cargo-deny >>> -->

## Dependencies

`just check-cargo-deny` holds every dependency, with every feature on, to
`deny.toml`: its advisories, its licence, its source, and the bans. A failure
names a choice for the maintainer, between a newer version, another crate, and
an exception with its reason: ask before adding an exception or allowing another
licence. Each goes in a key devset leaves to the repository: `skip` in `[bans]`,
`ignore` in `[advisories]`, and `exceptions` in `[licenses]`, which allows a
licence for one crate.

<!-- <<< devset: cargo-deny <<< -->

<!-- >>> devset: git-commits >>> -->

## Commits

A pull request lands squashed, as one commit its title names: the title follows
Conventional Commits, `type(scope): subject`, the subject imperative and lower
case, with no closing period, since it is the line the changelog shows; the
workflow `title` checks it. Each commit on a branch keeps the same rules, which
`just check-git-commits` checks. A breaking change adds `!` after the scope, and
a footer that starts `BREAKING CHANGE:` and says what to do.

<!-- <<< devset: git-commits <<< -->

<!-- >>> devset: mdbook >>> -->

## The Book

`just check-mdbook` lints the book, builds it and runs its examples. Its pages
are Markdown under the `src/` of its directory, each listed in `SUMMARY.md`, and
a preview rebuilds on every save:

```sh
mdbook serve docs
```

<!-- <<< devset: mdbook <<< -->
