# Performance

Every figure on this page is from one run, committed under
`crates/ddust/bench/results/`: each pass's transcript and figures, the code size
of each operation, and a `manifest.toml` that says how they were taken. The run
was taken on an AWS Graviton4 (Arm Neoverse V2, a c8g.8xlarge) under Linux
6.12.53, built by rustc 1.101.0-nightly of 2026-09-27 in the bench profile
(release's settings with fat LTO, one codegen unit, `panic = "abort"` and no
overflow checks), pinned with `taskset` to a core the kernel keeps other work
off; the manifest records the host's load before and after.

| Run                                         | What it measures                                                   |
| ------------------------------------------- | ------------------------------------------------------------------ |
| `2026-10-14T06-08Z-pr18-baseline-graviton4` | every operation of ddust beside every contender, time and counters |

The name is the time the run was taken, in UTC, the pull request whose code it
measured, what it measured, and the machine.

## How It Is Measured

Every crate is built into one binary and measured in one run, since code layout
alone moves a figure between builds by a few percent. Each operation is each
crate's fastest correct call, with the rounding mode the crate uses, and ddust
is measured again in that mode where a crate's differs from half to even. The
benchmarks' equivalence test checks every result against an exact reference; a
row whose results differ is marked *inexact*, and is timed all the same.

Each operation runs over two sets of values:

- **Predictable**: 1,024 values of one sign and one count of digits, which stay
  in the level 1 cache: the best case for a branch on the value.
- **Unpredictable**: 65,536 values of random signs and counts of digits, more
  than a branch predictor learns, so a branch on the value mispredicts as it
  would in a program.

Narrow values are 64-bit at 8 decimals, a price or a quantity as most venues
send one; wide values are 128-bit at 18 decimals, a token's amount as a chain
holds it. A crate with one type, as rust_decimal has, is measured at both.
Beside the decimals stand the floors, `i64`, `i128` and `f64` doing the plain
operation a decimal's is built on: the cost no decimal goes below, which the
reference does not check.

Each table lists the crates fastest first on the unpredictable set, with the
nanoseconds per operation on each set, and the cycles, instructions and branch
misses per operation on the unpredictable set. An operation is timed by
throughput, independent operations over the values; the sum, the rounded product
and the rounded quotients are timed by latency too, in [a chain](#latency) where
each waits for the one before.

## Adding and Comparing

A decimal of a static scale is its integer, so adding two is the integer's add
with its overflow check, and comparing two is the integer's compare:

**The checked sum, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          |            0.27 |              0.39 |   1.08 |         3.50 |          0.00 |
| fixdec 0.1.0 `D64`                             |            0.68 |              0.73 |   2.03 |         7.00 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            0.68 |              0.73 |   2.03 |         7.00 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            0.68 |              0.73 |   2.04 |         7.00 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`                       |            0.68 |              0.75 |   2.09 |         7.00 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            0.68 |              0.75 |   2.10 |         7.00 |          0.00 |
| `i64`                                          |            0.68 |              0.75 |   2.11 |         7.00 |          0.00 |
| **ddust** `D64<8>`                             |            0.69 |              0.76 |   2.13 |         9.00 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            0.95 |              1.02 |   2.86 |         14.0 |          0.00 |
| decimal-rs 0.2.0                               |            6.64 |              7.61 |   21.3 |         54.7 |          0.76 |
| rust_decimal 1.43.0                            |            7.88 |              9.79 |   27.4 |         67.5 |          1.23 |
| fastnum 0.7.5 `D64`                            |            7.75 |              16.6 |   46.4 |          126 |          1.48 |
| bigdecimal 0.4.11                              |            28.6 |              38.0 |    106 |          317 |          1.78 |

**The checked sum, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            |            0.28 |              0.39 |   1.09 |         3.50 |          0.00 |
| fixnum 0.9.5 `<i128, U18>`                       |            0.94 |              1.03 |   2.88 |         14.0 |          0.00 |
| `i128`                                           |            0.95 |              1.04 |   2.91 |         14.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            0.94 |              1.05 |   2.93 |         14.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            0.94 |              1.05 |   2.94 |         14.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            0.94 |              1.05 |   2.93 |         14.0 |          0.00 |
| **ddust** `D128<18>`                             |            0.95 |              1.06 |   2.94 |         14.0 |          0.00 |
| decimal-rs 0.2.0                                 |            6.68 |              7.65 |   21.4 |         54.8 |          0.77 |
| rust_decimal 1.43.0                              |            8.06 |              10.6 |   29.7 |         67.1 |          1.52 |
| fastnum 0.7.5 `D128`                             |            8.86 |              18.2 |   50.9 |          141 |          1.57 |
| bigdecimal 0.4.11                                |            50.5 |              47.8 |    134 |          451 |          2.27 |

**The order, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            0.15 |              0.21 |   0.58 |         1.69 |          0.00 |
| **ddust** `D64<8>`                             |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fixdec 0.1.0 `D64`                             |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`                       |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| `i64`                                          |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| `f64`                                          |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            0.55 |              0.60 |   1.69 |         7.63 |          0.00 |
| decimal-rs 0.2.0                               |            1.81 |              4.17 |   11.7 |         25.0 |          0.49 |
| bigdecimal 0.4.11                              |            3.61 |              5.62 |   15.7 |         43.9 |          0.49 |
| rust_decimal 1.43.0                            |            3.75 |              6.58 |   18.4 |         45.5 |          0.83 |
| fastnum 0.7.5 `D64`                            |            8.30 |              7.64 |   21.4 |         43.0 |          1.07 |

**The order, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            0.53 |              0.56 |   1.58 |         7.63 |          0.00 |
| fixnum 0.9.5 `<i128, U18>`                       |            0.54 |              0.58 |   1.62 |         7.63 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            0.55 |              0.58 |   1.62 |         7.63 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            0.53 |              0.58 |   1.63 |         7.63 |          0.00 |
| **ddust** `D128<18>`                             |            0.54 |              0.60 |   1.68 |         7.63 |          0.00 |
| `i128`                                           |            0.53 |              0.61 |   1.69 |         7.63 |          0.00 |
| decimal-rs 0.2.0                                 |            1.83 |              4.21 |   11.8 |         25.0 |          0.49 |
| bigdecimal 0.4.11                                |            3.63 |              6.83 |   19.1 |         40.8 |          0.80 |
| rust_decimal 1.43.0                              |            3.75 |              7.84 |   21.9 |         44.7 |          1.02 |
| fastnum 0.7.5 `D128`                             |            15.3 |              10.0 |   28.0 |         56.4 |          1.34 |

The fixed-point crates are within a few percent of each other and of the integer
here, since each is the integer underneath; ddust's checked add takes 9.00
instructions where the `i64`'s takes 7.00. The two more are the cost of
`Decimal`'s `repr(C)` layout, which keeps a decimal its integer's bytes for
zerocopy and bytemuck: Rust builds an `Option` of such a struct in memory, where
it returns an `Option` of a bare integer in two registers. The crates that carry
a scale in each value, rust_decimal, fastnum, decimal-rs and bigdecimal, line
two scales up before they add, at the cost of branches the unpredictable set
mispredicts.

## Multiplying

An exact product of a price at 2 decimals and a quantity at 5 is a notional at
7: the integer's multiply with its overflow check, and nothing for the scale:

**The exact product, narrow, 64 bits at 8 decimals:**

| Contender                                                 | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                                     |            0.77 |              0.87 |   2.42 |         10.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 2> × <i64, 5>` |            0.77 |              0.87 |   2.43 |         10.0 |          0.00 |
| **ddust** `D64<2> × D64<5>`                               |            0.80 |              0.89 |   2.50 |         11.0 |          0.00 |
| fastnum 0.7.5 `D64`                                       |            7.17 |              7.20 |   20.1 |          122 |          0.00 |
| rust_decimal 1.43.0                                       |            7.27 |              7.29 |   20.4 |         60.0 |          0.00 |
| bigdecimal 0.4.11                                         |            12.3 |              20.4 |   56.9 |          196 |          1.12 |
| decimal-rs 0.2.0                                          |            33.8 |              33.8 |   94.6 |          231 |          0.00 |

A product rounded back to the factors' scale is the exact product in an integer
twice as wide, divided by $10^8$ or $10^{18}$ through a multiply by its
reciprocal, and the remainder read against half the divisor for the mode; no
division instruction is left in it, which `just bench-assembly` checks on
aarch64 and `x86_64`:

**The rounded product, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          | HalfEven   |            0.27 |              0.41 |   1.13 |         3.50 |          0.00 |
| `i64`                                          | Trunc      |            0.77 |              0.86 |   2.42 |         10.0 |          0.00 |
| fixed 1.31.0 `I64F64`                          | Floor      |            2.65 |              2.63 |   7.36 |         38.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            2.87 |              3.31 |   9.25 |         35.5 |          0.15 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            4.02 |              3.68 |   10.3 |         36.4 |          0.18 |
| **ddust** `D64<8>`                             | HalfExpand |            3.29 |              3.73 |   10.4 |         42.9 |          0.15 |
| **ddust** `D64<8>`                             | HalfEven   |            3.96 |              3.97 |   11.1 |         46.9 |          0.15 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc      |            1.69 |              4.49 |   12.5 |         25.5 |          0.49 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            3.04 |              5.57 |   15.6 |         41.5 |          0.50 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            4.85 |              9.63 |   26.9 |         64.3 |          0.56 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            14.3 |              13.3 |   37.3 |          101 |          0.71 |
| rust_decimal 1.43.0                            | HalfEven   |            27.8 |              30.7 |   85.8 |          270 |          0.92 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            69.9 |              33.8 |   94.6 |          253 |          1.23 |
| decimal-rs 0.2.0                               | HalfExpand |            41.2 |              40.9 |    114 |          314 |          0.32 |
| bigdecimal 0.4.11                              | HalfEven   |             163 |               124 |    348 |        1,329 |          4.00 |

**The rounded product, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.27 |              0.41 |   1.14 |         3.75 |          0.00 |
| `i128`                                           | Trunc      |            3.46 |              3.70 |   10.4 |         27.1 |          0.30 |
| **ddust** `D128<18>`                             | Trunc      |            8.83 |              6.62 |   18.5 |         77.9 |          0.08 |
| **ddust** `D128<18>`                             | HalfExpand |            10.1 |              7.22 |   20.2 |         83.6 |          0.09 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            11.7 |              7.30 |   20.4 |         81.8 |          0.07 |
| **ddust** `D128<18>`                             | HalfEven   |            10.1 |              8.12 |   22.7 |         92.0 |          0.09 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            10.1 |              8.63 |   24.1 |         73.8 |          0.54 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            11.2 |              12.9 |   36.0 |         98.6 |          0.79 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            20.0 |              17.4 |   48.8 |          141 |          1.27 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             132 |              44.7 |    125 |          347 |          1.68 |
| decimal-rs 0.2.0                                 | HalfExpand |             130 |              52.5 |    147 |          372 |          0.81 |
| rust_decimal 1.43.0                              | HalfEven   |            69.7 |              57.8 |    162 |          389 |          2.03 |
| bigdecimal 0.4.11                                | HalfEven   |             246 |               171 |    478 |        1,771 |          4.82 |

## Division

A rounded quotient is $a \times 10^{d} / b$ in an integer twice as wide. A
narrow numerator that fits in 64 bits is divided by one `udiv`. On aarch64, a
wider one is taken from a double's estimate, which the core's pipelined `fdiv`
makes in a few cycles, then corrected to the exact quotient by a comparison, or
a step and a comparison, wherever a proof bounds the estimate's error; a wide
quotient, 256 bits by 128, is taken the same way:

**The rounded quotient, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          | HalfEven   |            0.89 |              0.93 |   2.60 |         3.75 |          0.00 |
| `i64`                                          | Trunc      |            2.17 |              2.13 |   5.94 |         12.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            3.82 |              4.49 |   12.6 |         46.7 |          0.22 |
| **ddust** `D64<8>`                             | HalfExpand |            4.54 |              5.31 |   14.9 |         52.2 |          0.23 |
| **ddust** `D64<8>`                             | HalfEven   |            4.70 |              5.40 |   15.1 |         52.2 |          0.24 |
| fixdec 0.1.0 `D64`                             | Trunc      |            8.00 |              8.12 |   22.7 |         72.8 |          0.45 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            8.12 |              8.18 |   22.9 |         72.8 |          0.46 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            11.7 |              10.9 |   30.5 |         71.4 |          1.34 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            13.6 |              11.4 |   31.8 |         66.4 |          0.73 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            9.99 |              14.3 |   40.0 |          100 |          0.88 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            68.7 |              86.2 |    241 |          525 |          2.67 |
| rust_decimal 1.43.0                            | HalfEven   |             104 |               114 |    318 |          763 |          2.98 |
| decimal-rs 0.2.0                               | HalfExpand |             209 |               199 |    555 |        1,154 |          2.68 |
| bigdecimal 0.4.11                              | HalfEven   |           4,278 |             4,416 | 12,348 |       49,100 |         29.78 |

**The rounded quotient, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.89 |              0.93 |   2.59 |         3.75 |          0.00 |
| `i128`                                           | Trunc      |            6.97 |              6.29 |   17.6 |         69.3 |          0.41 |
| **ddust** `D128<18>`                             | Trunc      |            12.7 |              13.5 |   37.9 |          129 |          0.49 |
| **ddust** `D128<18>`                             | HalfExpand |            13.6 |              14.4 |   40.1 |          138 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            15.0 |              15.8 |   44.2 |          140 |          0.50 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            36.3 |              22.2 |   62.2 |          172 |          1.58 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            35.2 |              27.4 |   76.6 |          201 |          1.47 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            49.6 |              29.9 |   83.6 |          240 |          1.80 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            39.3 |              34.7 |   97.0 |          256 |          2.41 |
| rust_decimal 1.43.0                              | HalfEven   |            84.9 |              98.8 |    276 |          637 |          4.33 |
| decimal-rs 0.2.0                                 | HalfExpand |             234 |               198 |    554 |        1,127 |          3.07 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             206 |               206 |    576 |        1,324 |          6.70 |
| bigdecimal 0.4.11                                | HalfEven   |          14,834 |            10,127 | 28,315 |      138,450 |         49.13 |

The same quotients, each dividend by one divisor, as a book is divided by one
price:

**The rounded quotient by one divisor, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          | HalfEven   |            0.89 |              0.92 |   2.58 |         3.75 |          0.00 |
| `i64`                                          | Trunc      |            2.17 |              2.40 |   6.72 |         12.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            2.71 |              4.29 |   12.0 |         30.3 |          0.29 |
| **ddust** `D64<8>`                             | HalfExpand |            3.38 |              4.88 |   13.7 |         36.7 |          0.32 |
| **ddust** `D64<8>`                             | HalfEven   |            4.72 |              5.97 |   16.7 |         52.9 |          0.30 |
| fixdec 0.1.0 `D64`                             | Trunc      |            7.81 |              6.81 |   19.1 |         69.9 |          0.20 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            7.80 |              6.81 |   19.1 |         69.9 |          0.20 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            11.2 |              8.81 |   24.6 |         67.9 |          0.77 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            13.2 |              11.2 |   31.3 |         66.6 |          0.64 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            10.5 |              14.0 |   39.1 |         97.8 |          0.67 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            70.4 |              80.1 |    224 |          501 |          2.48 |
| rust_decimal 1.43.0                            | HalfEven   |             106 |               109 |    306 |          716 |          2.46 |
| decimal-rs 0.2.0                               | HalfExpand |             210 |               208 |    582 |        1,248 |          2.61 |
| bigdecimal 0.4.11                              | HalfEven   |           4,251 |             4,411 | 12,335 |       47,381 |         23.30 |

**The rounded quotient by one divisor, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.89 |              0.93 |   2.60 |         3.50 |          0.00 |
| `i128`                                           | Trunc      |            6.85 |              7.52 |   21.0 |         71.9 |          0.65 |
| **ddust** `D128<18>`                             | Trunc      |            12.8 |              13.6 |   38.0 |          129 |          0.50 |
| **ddust** `D128<18>`                             | HalfExpand |            13.6 |              14.5 |   40.5 |          138 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            15.0 |              16.0 |   44.7 |          140 |          0.50 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            38.0 |              22.3 |   62.4 |          174 |          1.39 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            32.8 |              23.2 |   64.8 |          154 |          0.93 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            49.7 |              30.0 |   84.1 |          243 |          1.37 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            44.2 |              34.9 |   97.6 |          261 |          1.77 |
| rust_decimal 1.43.0                              | HalfEven   |            86.3 |              86.9 |    243 |          610 |          3.05 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             198 |               171 |    479 |        1,151 |          4.51 |
| decimal-rs 0.2.0                                 | HalfExpand |             227 |               208 |    581 |        1,183 |          2.66 |
| bigdecimal 0.4.11                                | HalfEven   |          14,917 |             4,376 | 12,236 |       50,876 |         37.99 |

A [`Divisor`] prepared from that one divisor divides by two multiplications and
a comparison, with no division at all; no other crate has one:

**The rounded quotient by a prepared divisor, narrow, 64 bits at 8 decimals:**

| Contender          | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>` | Trunc      |            2.07 |              2.07 |   5.78 |         29.0 |          0.00 |
| **ddust** `D64<8>` | HalfExpand |            2.70 |              2.70 |   7.55 |         35.0 |          0.00 |
| **ddust** `D64<8>` | HalfEven   |            3.01 |              3.00 |   8.37 |         37.0 |          0.00 |

**The rounded quotient by a prepared divisor, wide, 128 bits at 18 decimals:**

| Contender            | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| -------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>` | Trunc      |            7.88 |              9.32 |   26.1 |         91.0 |          0.48 |
| **ddust** `D128<18>` | HalfExpand |            10.4 |              11.5 |   32.2 |          111 |          0.48 |
| **ddust** `D128<18>` | HalfEven   |            11.1 |              12.4 |   34.7 |          120 |          0.49 |

A narrow quotient by a prepared divisor takes 3.00 ns where by the decimal it
takes 5.97, and a wide one 12.4 where by the decimal it takes 16.0. Preparing it
costs a division, so it pays from the second quotient on.

## Rescaling

A value rounded from 8 or 18 decimals to 2, a price to cents, divides by a power
of ten the compiler knows, through a multiply by its reciprocal:

**To cents, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            0.72 |              0.72 |   2.02 |         6.75 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            0.84 |              0.84 |   2.36 |         10.0 |          0.00 |
| `f64`                                          | HalfExpand |            0.92 |              0.89 |   2.49 |         4.00 |          0.00 |
| **ddust** `D64<8>`                             | HalfExpand |            1.14 |              1.14 |   3.19 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             | HalfEven   |            1.30 |              1.30 |   3.63 |         16.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            1.13 |              3.21 |   8.98 |         17.0 |          0.48 |
| fixdec 0.1.0 `D64`                             | HalfEven   |            1.19 |              3.82 |   10.7 |         18.7 |          0.34 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | HalfEven   |            1.88 |              4.32 |   12.1 |         23.8 |          0.33 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            1.86 |              4.99 |   13.9 |         27.0 |          0.35 |
| decimal-rs 0.2.0                               | HalfExpand |            5.32 |              6.57 |   18.4 |         61.1 |          0.32 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            12.9 |              18.3 |   51.3 |          135 |          1.08 |
| rust_decimal 1.43.0                            | HalfEven   |            20.2 |              21.8 |   61.0 |          190 |          0.36 |
| bigdecimal 0.4.11                              | HalfEven   |            87.8 |              75.5 |    211 |          917 |          2.06 |

**To cents, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfExpand |            0.92 |              0.82 |   2.30 |         4.00 |          0.00 |
| **ddust** `D128<18>`                             | HalfExpand |            3.89 |              3.91 |   10.9 |         31.1 |          0.25 |
| **ddust** `D128<18>`                             | Trunc      |            3.89 |              4.02 |   11.2 |         31.1 |          0.26 |
| **ddust** `D128<18>`                             | HalfEven   |            4.12 |              4.15 |   11.6 |         33.1 |          0.25 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            12.4 |              8.39 |   23.5 |         58.9 |          0.98 |
| `i128`                                           | Trunc      |            10.1 |              8.66 |   24.2 |         68.5 |          0.95 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            12.4 |              8.89 |   24.9 |         51.3 |          0.52 |
| decimal-rs 0.2.0                                 | HalfExpand |            10.9 |              9.58 |   26.6 |         66.8 |          0.71 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | HalfEven   |            18.5 |              14.7 |   40.8 |          114 |          1.28 |
| fastnum 0.7.5 `D128`                             | HalfExpand |            22.0 |              24.0 |   67.2 |          184 |          1.05 |
| rust_decimal 1.43.0                              | HalfEven   |            42.9 |              43.7 |    122 |          354 |          0.23 |
| bigdecimal 0.4.11                                | HalfEven   |             126 |              90.5 |    253 |        1,010 |          2.35 |

## Text

Reading takes a number of up to 32 bytes into 32 vector lanes at once, puts its
digits in their steps' places by one lookup, and makes the number by multiplies
in the lanes, with no branch on its length, so the unpredictable set costs
nearly what the predictable one does. Each crate reads the text exactly, and
refuses a digit past its scale; fin_decimal rounds one instead, and is marked
so:

**Reading, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>`                             |            9.56 |              9.75 |   27.3 |          101 |          0.17 |
| rust_decimal 1.43.0                            |            17.5 |              19.1 |   53.5 |          175 |          1.74 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            11.6 |              19.2 |   53.8 |          173 |          1.66 |
| `f64`                                          |            14.2 |              19.5 |   54.6 |          169 |          1.78 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            18.8 |              20.5 |   57.4 |          202 |          1.69 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*      |            15.8 |              20.7 |   57.8 |          216 |          1.68 |
| fixdec 0.1.0 `D64`                             |            21.4 |              23.5 |   65.7 |          241 |          1.51 |
| fastnum 0.7.5 `D64`                            |            20.4 |              24.2 |   67.6 |          267 |          1.68 |
| fixnum 0.9.5 `<i64, U8>`                       |            26.7 |              29.4 |   82.3 |          360 |          1.55 |
| decimal-rs 0.2.0                               |            37.8 |              43.6 |    122 |          384 |          2.96 |
| bigdecimal 0.4.11                              |            92.8 |              91.1 |    255 |        1,373 |          1.42 |

**Reading, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>`                             |            9.32 |              10.0 |   28.1 |         95.5 |          0.50 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            16.9 |              20.8 |   58.3 |          230 |          1.38 |
| rust_decimal 1.43.0                              |            39.8 |              30.0 |   83.9 |          365 |          1.56 |
| `f64`                                            |            48.4 |              31.8 |   89.1 |          329 |          1.33 |
| fastnum 0.7.5 `D128`                             |            30.8 |              31.9 |   89.3 |          429 |          1.45 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*    |            35.5 |              35.2 |   98.4 |          425 |          1.54 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            44.0 |              39.6 |    110 |          402 |          1.58 |
| decimal-rs 0.2.0                                 |            59.7 |              62.1 |    174 |          577 |          2.13 |
| fixnum 0.9.5 `<i128, U18>`                       |            68.1 |              65.9 |    184 |          762 |          1.78 |
| bigdecimal 0.4.11                                |             129 |               117 |    326 |        1,751 |          1.34 |

Writing splits the steps into their 32 digits by multiplies, and makes each
window of the text by one lookup into them; `Display`, through a formatter, is
measured where a crate has nothing faster:

**Writing, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>`                             |            9.84 |              9.84 |   27.5 |         99.0 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            18.4 |              23.3 |   65.1 |          198 |          2.25 |
| `i64`                                          |            14.7 |              26.5 |   74.2 |          215 |          2.25 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            17.4 |              26.6 |   74.4 |          275 |          1.83 |
| fixnum 0.9.5 `<i64, U8>`                       |            21.2 |              28.5 |   79.8 |          268 |          1.62 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            23.9 |              33.4 |   93.6 |          333 |          2.54 |
| fixdec 0.1.0 `D64`                             |            34.2 |              40.4 |    113 |          408 |          2.12 |
| decimal-rs 0.2.0                               |            34.4 |              44.0 |    123 |          517 |          2.20 |
| rust_decimal 1.43.0                            |            77.7 |              67.0 |    187 |          641 |          1.45 |
| `f64`                                          |            63.8 |              67.7 |    189 |          650 |          2.98 |
| fastnum 0.7.5 `D64`                            |            64.9 |              80.0 |    224 |          889 |          2.22 |
| bigdecimal 0.4.11                              |             108 |               111 |    312 |        1,391 |          1.89 |

**Writing, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>`                             |            18.0 |              16.4 |   45.9 |          177 |          0.32 |
| `i128`                                           |            20.9 |              31.5 |   88.2 |          263 |          2.62 |
| fixnum 0.9.5 `<i128, U18>`                       |            44.1 |              43.0 |    120 |          429 |          1.74 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            58.3 |              46.2 |    129 |          611 |          1.34 |
| decimal-rs 0.2.0                                 |            37.9 |              48.4 |    135 |          551 |          2.62 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            68.4 |              49.3 |    138 |          439 |          2.29 |
| `f64`                                            |            65.8 |              74.4 |    208 |          706 |          3.50 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            96.3 |              77.6 |    217 |          569 |          2.22 |
| rust_decimal 1.43.0                              |             118 |              91.7 |    256 |          956 |          1.48 |
| fastnum 0.7.5 `D128`                             |            83.1 |               103 |    289 |        1,148 |          2.66 |
| bigdecimal 0.4.11                                |             148 |               139 |    389 |        1,646 |          2.40 |

## Floats

A decimal becomes the nearest double. Steps below $2^{53}$ at up to 22 decimals
take one conversion and one division by an exact power of ten, Clinger's fast
path; past them, a division of the steps by a power of five makes the double's
53 bits and the bits that round them. The predictable set's steps have 17 and 28
digits, past $2^{53}$, so at narrow width it is the slower one here. The crates
marked inexact compute the double in floating point, rounding more than once:

**To a double, narrow, 64 bits at 8 decimals:**

| Contender                                                 | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                                     |            0.12 |              0.15 |   0.43 |         2.00 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>`, *inexact* |            0.89 |              0.89 |   2.50 |         2.50 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*                 |            0.89 |              0.89 |   2.50 |         2.50 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`, *inexact*                       |            1.23 |              1.23 |   3.44 |         11.5 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`, *inexact*                 |            1.23 |              1.23 |   3.44 |         11.5 |          0.00 |
| **ddust** `D64<8>`                                        |            4.79 |              1.38 |   3.87 |         15.6 |          0.04 |
| fixdec 0.1.0 `D64`, *inexact*                             |            1.70 |              1.61 |   4.52 |         19.0 |          0.00 |
| fastnum 0.7.5 `D64`                                       |            6.21 |              2.87 |   8.02 |         47.7 |          0.07 |
| decimal-rs 0.2.0                                          |            41.1 |              10.6 |   29.7 |          141 |          0.20 |
| rust_decimal 1.43.0, *inexact*                            |            21.7 |              23.9 |   66.8 |          246 |          0.59 |
| bigdecimal 0.4.11                                         |             109 |               108 |    301 |        1,234 |          1.56 |

**To a double, wide, 128 bits at 18 decimals:**

| Contender                                                   | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ----------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i128`                                                      |            3.55 |              3.55 |   9.94 |         43.5 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>`, *inexact* |            3.61 |              3.60 |   10.1 |         44.3 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*               |            3.60 |              3.60 |   10.1 |         44.3 |          0.00 |
| **ddust** `D128<18>`                                        |            7.42 |              7.73 |   21.6 |         60.7 |          0.43 |
| fastnum 0.7.5 `D128`                                        |            32.3 |              15.5 |   43.5 |          136 |          0.44 |
| nexus-decimal 1.2.2 `<i128, 18>`, *inexact*                 |            24.7 |              17.0 |   47.5 |          153 |          0.71 |
| fixnum 0.9.5 `<i128, U18>`, *inexact*                       |            24.8 |              17.0 |   47.5 |          153 |          0.71 |
| rust_decimal 1.43.0, *inexact*                              |            33.8 |              27.3 |   76.4 |          274 |          0.78 |
| decimal-rs 0.2.0                                            |            81.2 |              42.7 |    119 |          496 |          1.46 |
| bigdecimal 0.4.11                                           |             184 |               144 |    401 |        1,601 |          1.98 |

A double becomes the decimal nearest its exact binary value, by the mode. The
product of the double and the power of ten, and its residual, which a fused
multiply-add computes exactly, decide the rounding below $2^{52}$; past it, the
double's bits are scaled exactly. The crates marked inexact multiply in floating
point, or round the double's shortest text:

**From a double, narrow, 64 bits at 8 decimals:**

| Contender                                                 | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                                     | Trunc      |            0.25 |              0.30 |   0.84 |         3.00 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*                 | Trunc      |            0.43 |              0.47 |   1.32 |         4.75 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>`, *inexact* | HalfExpand |            0.62 |              0.62 |   1.74 |         5.75 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`, *inexact*                 | HalfExpand |            1.02 |              1.03 |   2.88 |         15.0 |          0.00 |
| fixdec 0.1.0 `D64`, *inexact*                             | HalfExpand |            1.02 |              1.03 |   2.88 |         15.0 |          0.00 |
| **ddust** `D64<8>`                                        | HalfEven   |            5.14 |              3.91 |   10.9 |         52.1 |          0.09 |
| fixnum 0.9.5 `<i64, U8>`, *inexact*                       | HalfExpand |            35.7 |              92.2 |    258 |          555 |          6.28 |
| fastnum 0.7.5 `D64`                                       | HalfExpand |            63.2 |               176 |    492 |        1,193 |          3.62 |
| decimal-rs 0.2.0, *inexact*                               | HalfExpand |             113 |               231 |    645 |        1,304 |          7.46 |
| rust_decimal 1.43.0, *inexact*                            | HalfEven   |             119 |               260 |    727 |        1,274 |         20.49 |
| bigdecimal 0.4.11                                         | HalfEven   |             217 |               343 |    956 |        3,854 |          5.08 |

**From a double, wide, 128 bits at 18 decimals:**

| Contender                                                   | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ----------------------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>`, *inexact* | HalfExpand |            2.87 |              2.87 |   8.02 |         42.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*               | Trunc      |            3.07 |              3.08 |   8.60 |         49.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`, *inexact*                 | HalfExpand |            3.42 |              3.44 |   9.61 |         53.0 |          0.00 |
| `i128`                                                      | Trunc      |            2.78 |              4.29 |   12.0 |         30.2 |          0.36 |
| **ddust** `D128<18>`                                        | HalfEven   |            4.94 |              6.31 |   17.7 |         66.0 |          0.49 |
| fixnum 0.9.5 `<i128, U18>`, *inexact*                       | HalfExpand |            36.8 |               138 |    387 |          743 |         12.37 |
| fastnum 0.7.5 `D128`                                        | HalfExpand |             104 |               179 |    502 |        1,348 |          4.17 |
| decimal-rs 0.2.0, *inexact*                                 | HalfExpand |            95.7 |               274 |    766 |        1,459 |         13.83 |
| rust_decimal 1.43.0, *inexact*                              | HalfEven   |             108 |               299 |    835 |        1,485 |         26.92 |
| bigdecimal 0.4.11                                           | HalfEven   |             202 |               407 |  1,136 |        4,435 |          6.12 |

## Latency

A chain makes each operation wait for the one before, by taking its operands at
an index the previous result decides: what a calculation that feeds on its own
results costs, where throughput is what a loop over independent values costs.
The index takes a byte that depends on both halves of each result, so a compiler
that inlines the operation into the chain still computes all of it.

**The checked sum, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>`                             |            3.57 |              3.67 |   10.3 |         14.0 |          0.00 |
| `i64`                                          |            3.57 |              3.67 |   10.3 |         14.0 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            3.56 |              3.68 |   10.3 |         14.0 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`                       |            3.56 |              3.68 |   10.3 |         14.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            3.56 |              3.68 |   10.3 |         14.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            3.57 |              3.68 |   10.3 |         14.0 |          0.00 |
| fixdec 0.1.0 `D64`                             |            3.56 |              3.69 |   10.3 |         14.0 |          0.00 |
| `f64`                                          |            4.94 |              5.00 |   14.0 |         14.0 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            5.12 |              5.20 |   14.6 |         19.0 |          0.00 |
| decimal-rs 0.2.0                               |            7.48 |              11.6 |   32.5 |         65.8 |          0.78 |
| rust_decimal 1.43.0                            |            13.0 |              17.8 |   49.7 |         84.5 |          1.20 |
| fastnum 0.7.5 `D64`                            |            20.2 |              29.7 |   83.2 |          136 |          1.43 |
| bigdecimal 0.4.11                              |            37.6 |              45.6 |    128 |          330 |          1.75 |

**The checked sum, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            |            4.94 |              5.00 |   14.0 |         14.0 |          0.00 |
| `i128`                                           |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| fixnum 0.9.5 `<i128, U18>`                       |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| **ddust** `D128<18>`                             |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            5.12 |              5.20 |   14.5 |         19.0 |          0.00 |
| decimal-rs 0.2.0                                 |            7.49 |              11.6 |   32.4 |         65.8 |          0.77 |
| rust_decimal 1.43.0                              |            15.7 |              20.5 |   57.3 |         83.1 |          1.50 |
| fastnum 0.7.5 `D128`                             |            18.4 |              26.7 |   74.6 |          155 |          1.57 |
| bigdecimal 0.4.11                                |            56.6 |              55.5 |    155 |          462 |          2.22 |

**The rounded product, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.62 |              4.65 |   13.0 |         16.0 |          0.00 |
| `f64`                                          | HalfEven   |            5.29 |              5.36 |   15.0 |         14.0 |          0.00 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc      |            5.20 |              7.53 |   21.1 |         26.0 |          0.47 |
| **ddust** `D64<8>`                             | Trunc      |            10.2 |              8.08 |   22.6 |         31.9 |          0.14 |
| fixed 1.31.0 `I64F64`                          | Floor      |            8.67 |              8.66 |   24.2 |         41.0 |          0.00 |
| **ddust** `D64<8>`                             | HalfExpand |            10.6 |              9.00 |   25.2 |         34.8 |          0.14 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            10.9 |              9.10 |   25.4 |         34.2 |          0.17 |
| **ddust** `D64<8>`                             | HalfEven   |            12.4 |              9.46 |   26.5 |         39.3 |          0.15 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            10.6 |              12.9 |   36.0 |         33.5 |          0.45 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            11.7 |              14.2 |   39.7 |         55.0 |          0.58 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            22.1 |              16.3 |   45.7 |         96.5 |          0.77 |
| rust_decimal 1.43.0                            | HalfEven   |            36.7 |              39.1 |    109 |          286 |          0.87 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            84.6 |              43.9 |    123 |          263 |          1.24 |
| decimal-rs 0.2.0                               | HalfExpand |            52.7 |              45.0 |    126 |          295 |          0.33 |
| bigdecimal 0.4.11                              | HalfEven   |             162 |               124 |    347 |        1,328 |          4.02 |

**The rounded product, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            5.29 |              5.36 |   15.0 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            3.76 |              7.73 |   21.6 |         29.1 |          0.30 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            21.9 |              14.1 |   39.5 |         72.9 |          0.06 |
| **ddust** `D128<18>`                             | Trunc      |            17.9 |              15.3 |   42.9 |         86.9 |          0.10 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            20.1 |              15.7 |   43.9 |         66.7 |          0.51 |
| **ddust** `D128<18>`                             | HalfExpand |            19.1 |              15.7 |   44.1 |         92.6 |          0.10 |
| **ddust** `D128<18>`                             | HalfEven   |            19.9 |              17.7 |   49.5 |          101 |          0.09 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            23.1 |              19.5 |   54.7 |          109 |          0.81 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            30.8 |              23.5 |   65.7 |          151 |          1.19 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             150 |              53.7 |    150 |          361 |          1.70 |
| decimal-rs 0.2.0                                 | HalfExpand |             148 |              57.7 |    161 |          354 |          0.83 |
| rust_decimal 1.43.0                              | HalfEven   |            85.4 |              64.5 |    180 |          405 |          1.99 |
| bigdecimal 0.4.11                                | HalfEven   |             246 |               171 |    478 |        1,770 |          4.84 |

**The rounded quotient, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.97 |              4.95 |   13.9 |         15.0 |          0.00 |
| `f64`                                          | HalfEven   |            8.61 |              8.73 |   24.5 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            14.6 |              10.5 |   29.4 |         33.5 |          0.22 |
| **ddust** `D64<8>`                             | HalfExpand |            15.3 |              12.2 |   34.0 |         40.0 |          0.22 |
| **ddust** `D64<8>`                             | HalfEven   |            16.1 |              12.4 |   34.7 |         39.9 |          0.24 |
| fixdec 0.1.0 `D64`                             | Trunc      |            17.5 |              16.0 |   44.8 |         74.8 |          0.47 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            17.5 |              16.0 |   44.8 |         74.8 |          0.47 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            21.8 |              16.1 |   45.0 |         63.8 |          1.30 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            20.8 |              16.2 |   45.2 |         59.7 |          0.78 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            16.9 |              18.2 |   50.9 |         93.9 |          0.91 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            82.7 |              95.6 |    267 |          535 |          2.68 |
| rust_decimal 1.43.0                            | HalfEven   |             109 |               118 |    329 |          764 |          2.98 |
| decimal-rs 0.2.0                               | HalfExpand |             220 |               207 |    580 |        1,141 |          2.67 |
| bigdecimal 0.4.11                              | HalfEven   |           4,269 |             4,410 | 12,332 |       49,179 |         30.47 |

**The rounded quotient, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            8.61 |              8.73 |   24.5 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            14.5 |              10.1 |   28.2 |         77.3 |          0.36 |
| **ddust** `D128<18>`                             | Trunc      |            30.3 |              30.6 |   85.5 |          143 |          0.49 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            50.6 |              31.1 |   87.0 |          182 |          1.59 |
| **ddust** `D128<18>`                             | HalfExpand |            31.4 |              31.7 |   88.6 |          152 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            33.1 |              33.2 |   92.8 |          154 |          0.50 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            46.1 |              37.2 |    104 |          216 |          1.50 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            60.8 |              39.7 |    111 |          255 |          1.82 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            43.9 |              40.9 |    115 |          271 |          2.45 |
| rust_decimal 1.43.0                              | HalfEven   |            93.3 |               105 |    292 |          638 |          4.31 |
| decimal-rs 0.2.0                                 | HalfExpand |             243 |               208 |    580 |        1,115 |          3.08 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             217 |               218 |    609 |        1,338 |          6.62 |
| bigdecimal 0.4.11                                | HalfEven   |          14,829 |             9,958 | 27,849 |      138,627 |         48.67 |

**The rounded quotient by one divisor, in a chain, narrow, 64 bits at 8
decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.97 |              5.24 |   14.7 |         15.0 |          0.00 |
| `f64`                                          | HalfEven   |            8.62 |              8.74 |   24.5 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            14.5 |              11.5 |   32.1 |         34.4 |          0.29 |
| **ddust** `D64<8>`                             | HalfExpand |            15.3 |              13.0 |   36.3 |         40.7 |          0.26 |
| **ddust** `D64<8>`                             | HalfEven   |            16.2 |              13.3 |   37.3 |         40.6 |          0.28 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            21.4 |              15.8 |   44.3 |         60.5 |          0.78 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            17.1 |              16.0 |   44.9 |         71.9 |          0.20 |
| fixdec 0.1.0 `D64`                             | Trunc      |            17.1 |              16.0 |   44.9 |         71.9 |          0.20 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            20.5 |              16.6 |   46.5 |         59.8 |          0.70 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            17.0 |              18.3 |   51.2 |         91.3 |          0.67 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            83.3 |              90.5 |    253 |          511 |          2.48 |
| rust_decimal 1.43.0                            | HalfEven   |             109 |               113 |    316 |          717 |          2.46 |
| decimal-rs 0.2.0                               | HalfExpand |             222 |               220 |    616 |        1,236 |          2.61 |
| bigdecimal 0.4.11                              | HalfEven   |           4,251 |             4,392 | 12,285 |       47,476 |         22.86 |

**The rounded quotient by one divisor, in a chain, wide, 128 bits at 18
decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            8.61 |              8.80 |   24.6 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            14.5 |              11.6 |   32.4 |         78.9 |          0.60 |
| **ddust** `D128<18>`                             | Trunc      |            30.3 |              30.6 |   85.5 |          143 |          0.49 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            51.2 |              31.4 |   87.9 |          184 |          1.37 |
| **ddust** `D128<18>`                             | HalfExpand |            31.3 |              31.6 |   88.4 |          152 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            33.1 |              33.2 |   92.7 |          154 |          0.49 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            42.8 |              33.4 |   93.5 |          169 |          0.95 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            60.8 |              40.8 |    114 |          258 |          1.36 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            47.9 |              41.9 |    117 |          276 |          1.79 |
| rust_decimal 1.43.0                              | HalfEven   |            92.5 |              93.1 |    260 |          622 |          3.01 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             211 |               183 |    512 |        1,165 |          4.44 |
| decimal-rs 0.2.0                                 | HalfExpand |             240 |               219 |    614 |        1,179 |          2.68 |
| bigdecimal 0.4.11                                | HalfEven   |          14,907 |             4,352 | 12,171 |       50,970 |         37.22 |

**The rounded quotient by a prepared divisor, in a chain, narrow, 64 bits at 8
decimals:**

| Contender          | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>` | Trunc      |            6.13 |              6.13 |   17.2 |         34.0 |          0.00 |
| **ddust** `D64<8>` | HalfExpand |            7.59 |              7.60 |   21.3 |         40.0 |          0.00 |
| **ddust** `D64<8>` | HalfEven   |            8.63 |              8.64 |   24.2 |         43.0 |          0.00 |

**The rounded quotient by a prepared divisor, in a chain, wide, 128 bits at 18
decimals:**

| Contender            | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| -------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>` | Trunc      |            13.8 |              14.4 |   40.2 |         94.0 |          0.49 |
| **ddust** `D128<18>` | HalfExpand |            17.2 |              17.7 |   49.5 |          114 |          0.49 |
| **ddust** `D128<18>` | HalfEven   |            19.4 |              19.7 |   55.1 |          123 |          0.50 |

## A Mix, and Code Size

A program mixes operations, and what each brings into the instruction cache is
then a cost of its own. The mix runs a sum, an order, a rounded product, a
rounded quotient and a value to cents in turn over the same values, with the
level 1 instruction cache's misses and the front end's stalled cycles per
operation:

**The mix, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode                 | ns, predictable | ns, unpredictable | L1i misses | Front-end stalls |
| ---------------------------------------------- | -------------------- | --------------: | ----------------: | ---------: | ---------------: |
| `i64`                                          | Trunc                |            0.48 |              0.47 |      0.000 |             0.00 |
| `f64`                                          | HalfEven, HalfExpand |            0.49 |              0.48 |      0.000 |             0.00 |
| **ddust** `D64<8>`                             | Trunc                |            1.18 |              1.41 |      0.000 |             0.40 |
| **ddust** `D64<8>`                             | HalfExpand           |            1.40 |              1.61 |      0.000 |             0.36 |
| **ddust** `D64<8>`                             | HalfEven             |            1.66 |              1.73 |      0.000 |             0.37 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc, HalfEven      |            2.29 |              3.21 |      0.000 |             1.30 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc, HalfEven      |            2.57 |              3.38 |      0.000 |             1.21 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand           |            3.53 |              3.60 |      0.000 |             1.88 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven             |            3.28 |              4.85 |      0.000 |             1.31 |
| fastnum 0.7.5 `D64`                            | HalfExpand           |            35.5 |              31.1 |      0.001 |             7.33 |
| rust_decimal 1.43.0                            | HalfEven             |            33.3 |              35.1 |      0.001 |             6.29 |
| decimal-rs 0.2.0                               | HalfExpand           |            51.8 |              51.1 |      0.001 |             4.03 |
| bigdecimal 0.4.11                              | HalfEven             |             927 |               931 |      0.058 |             81.5 |

**The mix, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode                 | ns, predictable | ns, unpredictable | L1i misses | Front-end stalls |
| ------------------------------------------------ | -------------------- | --------------: | ----------------: | ---------: | ---------------: |
| `f64`                                            | HalfEven, HalfExpand |            0.49 |              0.48 |      0.000 |             0.00 |
| `i128`                                           | Trunc                |            3.54 |              3.49 |      0.000 |             1.05 |
| **ddust** `D128<18>`                             | Trunc                |            5.21 |              4.75 |      0.000 |             1.17 |
| **ddust** `D128<18>`                             | HalfExpand           |            5.96 |              5.27 |      0.000 |             1.33 |
| **ddust** `D128<18>`                             | HalfEven             |            6.38 |              5.68 |      0.000 |             1.24 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand           |            13.8 |              8.40 |      0.000 |             2.78 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc, HalfEven      |            11.7 |              8.52 |      0.000 |             2.90 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven             |            10.9 |              9.77 |      0.000 |             2.48 |
| rust_decimal 1.43.0                              | HalfEven             |            46.6 |              42.2 |      0.001 |             9.23 |
| decimal-rs 0.2.0                                 | HalfExpand           |            74.2 |              53.3 |      0.001 |             4.99 |
| fastnum 0.7.5 `D128`                             | HalfExpand           |            79.9 |              60.0 |      0.003 |             13.3 |
| bigdecimal 0.4.11                                | HalfEven             |           3,096 |             2,074 |      0.452 |              130 |

Each operation's code, in instructions, with every function it calls but the
panics and ddust's cold paths, which a value past 19 decimals or a result past
its integer reaches; read from the release build's disassembly on aarch64:

| Operation          | **ddust** `D64<8>` | **ddust** `D128<18>` | nexus-decimal | primitive_fixed_point_decimal | fin_decimal | fixnum | rust_decimal | fastnum | decimal-rs | bigdecimal |
| ------------------ | -----------------: | -------------------: | ------------: | ----------------------------: | ----------: | -----: | -----------: | ------: | ---------: | ---------: |
| add                |                 10 |                   12 |             5 |                             5 |           5 |      5 |          955 |   3,184 |      1,609 |      9,888 |
| compare            |                  5 |                    6 |             5 |                             5 |           5 |      5 |          142 |     322 |        103 |     10,189 |
| mul-round          |                103 |                  215 |            69 |                            64 |         101 |    281 |          587 |   3,295 |      1,295 |     11,060 |
| div-round          |                373 |                  674 |           248 |                           330 |         164 |    280 |        1,267 |   4,603 |      1,854 |     12,338 |
| div-round-prepared |                 48 |                  132 |               |                               |             |        |              |         |            |            |
| rescale-round      |                 23 |                   91 |            43 |                            28 |          33 |        |          221 |   3,214 |        262 |     10,359 |
| parse              |              1,186 |                1,189 |           361 |                           616 |         379 |    598 |        1,051 |     535 |        511 |      1,356 |
| format             |                343 |                1,724 |           333 |                           129 |       1,248 |    129 |          129 |     129 |        129 |        132 |
| to-f64             |                 70 |                  130 |            19 |                             7 |           7 |     19 |          376 |     177 |      1,993 |     10,163 |
| from-f64           |                141 |                  101 |            18 |                            15 |          11 |    488 |          467 | 130,345 |        530 |     10,916 |

A call through a pointer, which `core::fmt` makes, is not followed: a crate that
writes text through `Display` counts only the code around the formatter. ddust's
reader and writer count their general paths, for a number past 32 bytes or 30
decimals, beside their vector ones.

## What to Keep off a Hot Path

- **A wide division where a narrow one does.** A `D128` quotient costs about
  three times a `D64`'s, and a chain of them two to three times; a quantity, a
  price or a cash amount fits a `D64`.
- **Dividing many values by one decimal.** A [`Divisor`] prepared once divides
  with no division.
- **A conversion to a double inside a calculation.** It is correctly rounded and
  cheap, but the double is rarely the decimal, and a calculation that goes
  through one loses the exactness ddust keeps.

## How It Was Checked

The benchmarks' equivalence test checks every result against an exact reference,
on both sets and at both widths. The kernels that divide only by constants, the
add, the compare, the rounded product, the rescale, the reader, the writer and
both float conversions, are held by `just bench-assembly` to contain no division
instruction and call no division routine, on aarch64 and `x86_64`. Each figure
is the median of the five passes' medians: 124 of the run's 782 times spread
past 2% across the passes, half of them operations under two nanoseconds, where
2% is under a twentieth of one, and the run's `summary.toml` keeps each one's
range. It runs with:

```sh
just bench-run 18 baseline graviton4 --passes 5
```
