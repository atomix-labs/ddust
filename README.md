<!-- >>> devset: project >>> -->
<!-- dprint-ignore-start -->

<h1 align="center">ddust</h1>

<p align="center">Fixed-point decimals that leave no dust: exact, typed, as fast as the integers beneath them.</p>

<p align="center">
  <a href="https://github.com/atomix-labs/ddust/actions/workflows/check.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/atomix-labs/ddust/check.yml?branch=main&amp;style=flat-square&amp;label=check"></a>
  <a href="https://crates.io/crates/ddust"><img alt="crates.io" src="https://img.shields.io/crates/v/ddust?style=flat-square"></a>
  <a href="https://docs.rs/ddust"><img alt="docs.rs" src="https://img.shields.io/docsrs/ddust?style=flat-square"></a>
  <a href="https://atomix-labs.github.io/ddust/"><img alt="Book" src="https://img.shields.io/badge/book-read-blue?style=flat-square"></a>
  <a href="https://github.com/atomix-labs/devset"><img alt="managed with devset" src="https://img.shields.io/badge/managed_with-devset-0969da?style=flat-square&amp;logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHRpdGxlPmRldnNldDwvdGl0bGU+PHBhdGggZmlsbD0iI2YwZjZmYyIgZD0ibTE2IDMgMTMgNi41TDE2IDE2IDMgOS41WiIvPjxwYXRoIGZpbGw9Im5vbmUiIHN0cm9rZT0iI2YwZjZmYyIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbGluZWpvaW49InJvdW5kIiBzdHJva2Utd2lkdGg9IjIuNSIgZD0ibTMgMTYgMTMgNi41TDI5IDE2TTMgMjIuNSAxNiAyOWwxMy02LjUiLz48L3N2Zz4K"></a>
</p>

<!-- dprint-ignore-end -->
<!-- <<< devset: project <<< -->

ddust is a Rust crate of exact fixed-point decimals: a value is a whole number
of steps of a power of ten, held in an integer from `i8` to `i128`, with the
scale a type the compiler knows or a value read at run time. Nothing loses a
digit without a rounding mode, scales the compiler knows never mix, and each
operation compiles to the integer arithmetic beneath it. It is in early
development, and no version is released yet; the install line and the first
example land here with the crate.

The name reads as decimal dust, or the dust: the digits a rounding leaves
behind, which ddust never drops without being asked.

## Documentation

- [The book][book]: what ddust is, and how to use it.
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first, and report a vulnerability as [SECURITY.md][security] says.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[book]: https://atomix-labs.github.io/ddust/
[changelog]: https://github.com/atomix-labs/ddust/blob/main/CHANGELOG.md
[contributing]: https://github.com/atomix-labs/ddust/blob/main/CONTRIBUTING.md
[security]: https://github.com/atomix-labs/ddust/blob/main/SECURITY.md
[mit]: https://github.com/atomix-labs/ddust/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/ddust/blob/main/LICENSE-APACHE
