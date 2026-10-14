# Changelog

Every release, newest first, written by [git-cliff](https://git-cliff.org) from the commits.
[BREAKING-CHANGES.md](BREAKING-CHANGES.md) says how to move across a breaking change.

## [0.1.0](https://github.com/atomix-labs/ddust/releases/tag/v0.1.0) - 2026-10-14

### Features

- [90dc647](https://github.com/atomix-labs/ddust/commit/90dc64734983f82a9de0142bc9702ecddaeb3074) Add Divisor, a divisor prepared once that divides by multiplication ([#10](https://github.com/atomix-labs/ddust/pull/10))
- [b2feeff](https://github.com/atomix-labs/ddust/commit/b2feeff745f3d7867c4d9daa9a9ab35357b83748) Add the integrations with serde, num-traits, rand, zerocopy, bytemuck, defmt, arbitrary and proptest ([#7](https://github.com/atomix-labs/ddust/pull/7))
- [5715aaa](https://github.com/atomix-labs/ddust/commit/5715aaafdafe565781603ee132e0a239464c4697) *(bench)* Add the bench suite, every contender beside ddust, and a Graviton4 baseline ([#3](https://github.com/atomix-labs/ddust/pull/3))
- [7e52bcc](https://github.com/atomix-labs/ddust/commit/7e52bcc43e4e002770c81e7db082e9bf63e36ad9) Add the decimal, its scales, rounding and text ([#2](https://github.com/atomix-labs/ddust/pull/2))

### Performance

- [111b3fe](https://github.com/atomix-labs/ddust/commit/111b3feb4fd3ebdc26bfc79f3cfb293299afd3f4) Sum a static scale's checked values as their integer does, and fingerprint chains by the whole result ([#17](https://github.com/atomix-labs/ddust/pull/17))
- [d8d6d52](https://github.com/atomix-labs/ddust/commit/d8d6d52e2e25bd558fc579d291e781d9d4ed9884) Divide a narrow numerator within a word by one udiv ([#14](https://github.com/atomix-labs/ddust/pull/14))
- [4a51e68](https://github.com/atomix-labs/ddust/commit/4a51e68538ed18d03a942afc3c798b14f01b9ad7) Write text through one vector writer, written once in core::simd ([#13](https://github.com/atomix-labs/ddust/pull/13))
- [85e937d](https://github.com/atomix-labs/ddust/commit/85e937d96e6eb1d0e84c1824c1a30eebb372acb0) Divide by an f64 estimate, corrected exactly, on aarch64 ([#12](https://github.com/atomix-labs/ddust/pull/12))
- [a9b83ff](https://github.com/atomix-labs/ddust/commit/a9b83ff6037df4bde31d6fd8acdb81796168f542) Read text of up to 32 bytes in one vector reader, written once in core::simd ([#11](https://github.com/atomix-labs/ddust/pull/11))
- [71030e1](https://github.com/atomix-labs/ddust/commit/71030e122c519c1bd0f81ab816960b7843952295) Divide wide products by a two-word inverse, and small ones by one multiply-high ([#9](https://github.com/atomix-labs/ddust/pull/9))
- [7d3553b](https://github.com/atomix-labs/ddust/commit/7d3553b23c4b6f8993980d2b5b6ce1ed0c50c9a4) Round products by a bias, divide by short powers in two steps, and time every mode like for like ([#8](https://github.com/atomix-labs/ddust/pull/8))
- [82fb341](https://github.com/atomix-labs/ddust/commit/82fb341c9ce141a6342ad7fb7c9560be120ccee4) Read and write decimals eight bytes at a time, and short texts in vector registers ([#5](https://github.com/atomix-labs/ddust/pull/5))
- [476e7c1](https://github.com/atomix-labs/ddust/commit/476e7c105dd573e938fa2107b4acba2291f263e0) Divide through reciprocals, round by one comparison, and convert floats exactly ([#4](https://github.com/atomix-labs/ddust/pull/4))

### Documentation

- [d425a01](https://github.com/atomix-labs/ddust/commit/d425a01fc304d852c7c477e4cb9e400827be2d62) Write the book, draw the logo and complete the README for the first release ([#18](https://github.com/atomix-labs/ddust/pull/18))

### Miscellaneous

- [57c1562](https://github.com/atomix-labs/ddust/commit/57c156287ab24af78294522bdfe9b5f5e5bf9155) Scaffold the workspace from the atxp/rust profile ([#1](https://github.com/atomix-labs/ddust/pull/1))
