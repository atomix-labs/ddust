# Performance

Every figure on this page is from one run, committed under
`crates/ddust/bench/results/`: each pass's transcript and figures, the code size
of each operation, and a `manifest.toml` that says how they were taken. The run
was taken on an AWS Graviton4 (Arm Neoverse V2, a c8g.8xlarge) under Linux
6.12.53, built by rustc 1.101.0-nightly of 2026-09-27 in the bench profile
(release's settings with fat LTO, one codegen unit, `panic = "abort"` and no
overflow checks), pinned with `taskset` to a core the kernel keeps other work
off. Each figure is the median of five passes' medians.

| Run                                         | What it measures                                                   |
| ------------------------------------------- | ------------------------------------------------------------------ |
| `2026-10-12T14-44Z-pr15-baseline-graviton4` | every operation of ddust beside every contender, time and counters |

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
| `f64`                                          |            0.27 |              0.38 |   1.06 |         3.50 |          0.00 |
| fixdec 0.1.0 `D64`                             |            0.68 |              0.72 |   2.02 |         7.00 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            0.68 |              0.72 |   2.02 |         7.00 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            0.68 |              0.73 |   2.04 |         7.00 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            0.68 |              0.75 |   2.09 |         7.00 |          0.00 |
| `i64`                                          |            0.68 |              0.75 |   2.09 |         7.00 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`                       |            0.68 |              0.76 |   2.12 |         7.00 |          0.00 |
| **ddust** `D64<8>`                             |            0.68 |              0.77 |   2.16 |         9.00 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            0.95 |              1.04 |   2.92 |         14.0 |          0.00 |
| decimal-rs 0.2.0                               |            6.67 |              7.65 |   21.4 |         54.7 |          0.77 |
| rust_decimal 1.43.0                            |            8.15 |              9.82 |   27.5 |         67.5 |          1.21 |
| fastnum 0.7.5 `D64`                            |            7.80 |              16.7 |   46.7 |          126 |          1.49 |
| bigdecimal 0.4.11                              |            27.8 |              36.8 |    103 |          317 |          1.76 |

**The checked sum, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            |            0.28 |              0.39 |   1.08 |         3.50 |          0.00 |
| **ddust** `D128<18>`                             |            0.95 |              1.06 |   2.96 |         14.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            0.95 |              1.07 |   3.00 |         14.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            0.98 |              1.08 |   3.01 |         14.0 |          0.00 |
| `i128`                                           |            0.98 |              1.08 |   3.03 |         14.0 |          0.00 |
| fixnum 0.9.5 `<i128, U18>`                       |            0.95 |              1.09 |   3.04 |         14.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            0.95 |              1.09 |   3.05 |         14.0 |          0.00 |
| decimal-rs 0.2.0                                 |            6.67 |              7.61 |   21.3 |         54.7 |          0.77 |
| rust_decimal 1.43.0                              |            8.04 |              10.6 |   29.6 |         67.1 |          1.52 |
| fastnum 0.7.5 `D128`                             |            8.93 |              18.3 |   51.2 |          141 |          1.57 |
| bigdecimal 0.4.11                                |            49.8 |              47.0 |    132 |          451 |          2.24 |

**The order, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| fixnum 0.9.5 `<i64, U8>`                       |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| `i64`                                          |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| **ddust** `D64<8>`                             |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fixdec 0.1.0 `D64`                             |            0.15 |              0.21 |   0.59 |         1.69 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            0.15 |              0.21 |   0.60 |         1.69 |          0.00 |
| `f64`                                          |            0.15 |              0.21 |   0.60 |         1.69 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            0.57 |              0.60 |   1.67 |         7.63 |          0.00 |
| decimal-rs 0.2.0                               |            1.83 |              4.24 |   11.9 |         25.0 |          0.50 |
| bigdecimal 0.4.11                              |            3.60 |              5.58 |   15.6 |         43.9 |          0.48 |
| rust_decimal 1.43.0                            |            3.73 |              6.55 |   18.3 |         45.5 |          0.83 |
| fastnum 0.7.5 `D64`                            |            8.82 |              7.67 |   21.4 |         43.0 |          1.07 |

**The order, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            |            0.15 |              0.22 |   0.60 |         1.69 |          0.00 |
| fixnum 0.9.5 `<i128, U18>`                       |            0.55 |              0.57 |   1.59 |         7.63 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            0.55 |              0.57 |   1.60 |         7.63 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            0.54 |              0.57 |   1.60 |         7.63 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            0.54 |              0.58 |   1.61 |         7.63 |          0.00 |
| `i128`                                           |            0.57 |              0.59 |   1.66 |         7.63 |          0.00 |
| **ddust** `D128<18>`                             |            0.54 |              0.60 |   1.69 |         7.63 |          0.00 |
| decimal-rs 0.2.0                                 |            1.83 |              4.18 |   11.7 |         25.0 |          0.49 |
| bigdecimal 0.4.11                                |            3.59 |              6.95 |   19.4 |         40.8 |          0.84 |
| rust_decimal 1.43.0                              |            3.75 |              7.80 |   21.8 |         44.7 |          1.02 |
| fastnum 0.7.5 `D128`                             |            15.3 |              9.98 |   27.9 |         56.4 |          1.34 |

The fixed-point crates are within a few percent of each other and of the integer
here, since each is the integer underneath; ddust's checked add takes 9.00
instructions where the `i64`'s takes 7.00. The crates that carry a scale in each
value, rust_decimal, fastnum, decimal-rs and bigdecimal, line two scales up
before they add, at the cost of branches the unpredictable set mispredicts.

## Multiplying

An exact product of a price at 2 decimals and a quantity at 5 is a notional at
7: the integer's multiply with its overflow check, and nothing for the scale:

**The exact product, narrow, 64 bits at 8 decimals:**

| Contender                                                 | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| primitive_fixed_point_decimal 1.5.0 `<i64, 2> × <i64, 5>` |            0.77 |              0.86 |   2.39 |         10.0 |          0.00 |
| `i64`                                                     |            0.78 |              0.86 |   2.42 |         10.0 |          0.00 |
| **ddust** `D64<2> × D64<5>`                               |            0.80 |              0.89 |   2.50 |         11.0 |          0.00 |
| fastnum 0.7.5 `D64`                                       |            7.17 |              7.19 |   20.1 |          122 |          0.00 |
| rust_decimal 1.43.0                                       |            7.23 |              7.24 |   20.3 |         60.0 |          0.00 |
| bigdecimal 0.4.11                                         |            12.4 |              20.3 |   56.6 |          196 |          1.11 |
| decimal-rs 0.2.0                                          |            33.8 |              33.8 |   94.5 |          231 |          0.00 |

A product rounded back to the factors' scale is the exact product in an integer
twice as wide, divided by $10^8$ or $10^{18}$ through a multiply by its
reciprocal, and the remainder read against half the divisor for the mode; no
division instruction is left in it, which `just bench-assembly` checks on
aarch64 and `x86_64`:

**The rounded product, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          | HalfEven   |            0.29 |              0.38 |   1.06 |         3.50 |          0.00 |
| `i64`                                          | Trunc      |            0.77 |              0.85 |   2.39 |         10.0 |          0.00 |
| fixed 1.31.0 `I64F64`                          | Floor      |            2.63 |              2.63 |   7.35 |         38.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            2.96 |              3.21 |   8.96 |         35.5 |          0.15 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            4.05 |              3.70 |   10.3 |         36.4 |          0.18 |
| **ddust** `D64<8>`                             | HalfExpand |            3.29 |              3.73 |   10.4 |         42.9 |          0.15 |
| **ddust** `D64<8>`                             | HalfEven   |            4.01 |              3.93 |   11.0 |         46.9 |          0.15 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc      |            1.73 |              4.53 |   12.7 |         25.5 |          0.50 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            3.04 |              5.64 |   15.8 |         41.5 |          0.50 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            4.83 |              9.80 |   27.4 |         64.3 |          0.57 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            13.9 |              13.4 |   37.4 |          101 |          0.71 |
| rust_decimal 1.43.0                            | HalfEven   |            29.9 |              30.8 |   86.1 |          270 |          0.93 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            70.3 |              33.7 |   94.2 |          253 |          1.22 |
| decimal-rs 0.2.0                               | HalfExpand |            41.1 |              40.9 |    114 |          314 |          0.32 |
| bigdecimal 0.4.11                              | HalfEven   |             163 |               125 |    350 |        1,329 |          4.03 |

**The rounded product, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.27 |              0.39 |   1.09 |         3.75 |          0.00 |
| `i128`                                           | Trunc      |            3.45 |              3.83 |   10.7 |         27.1 |          0.32 |
| **ddust** `D128<18>`                             | Trunc      |            8.61 |              6.64 |   18.6 |         77.9 |          0.09 |
| **ddust** `D128<18>`                             | HalfExpand |            10.1 |              7.20 |   20.2 |         83.6 |          0.09 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            11.7 |              7.30 |   20.4 |         81.8 |          0.06 |
| **ddust** `D128<18>`                             | HalfEven   |            10.1 |              8.07 |   22.6 |         92.0 |          0.08 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            10.1 |              8.68 |   24.3 |         73.8 |          0.55 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            11.2 |              12.9 |   35.9 |         98.6 |          0.80 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            20.1 |              17.4 |   48.5 |          141 |          1.26 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             132 |              44.7 |    125 |          347 |          1.68 |
| decimal-rs 0.2.0                                 | HalfExpand |             130 |              52.6 |    147 |          372 |          0.80 |
| rust_decimal 1.43.0                              | HalfEven   |            70.3 |              57.8 |    162 |          389 |          2.04 |
| bigdecimal 0.4.11                                | HalfEven   |             247 |               172 |    480 |        1,771 |          4.86 |

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
| `f64`                                          | HalfEven   |            0.89 |              0.90 |   2.51 |         3.75 |          0.00 |
| `i64`                                          | Trunc      |            2.19 |              2.13 |   5.97 |         12.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            3.79 |              4.61 |   12.9 |         46.7 |          0.24 |
| **ddust** `D64<8>`                             | HalfExpand |            4.61 |              5.38 |   15.1 |         52.2 |          0.24 |
| **ddust** `D64<8>`                             | HalfEven   |            4.70 |              5.38 |   15.1 |         52.2 |          0.23 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            8.00 |              8.12 |   22.7 |         72.8 |          0.45 |
| fixdec 0.1.0 `D64`                             | Trunc      |            8.10 |              8.17 |   22.8 |         72.8 |          0.45 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            11.9 |              11.0 |   30.6 |         71.4 |          1.34 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            12.8 |              11.4 |   31.8 |         66.4 |          0.74 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            9.73 |              14.2 |   39.6 |          100 |          0.85 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            67.5 |              86.1 |    241 |          525 |          2.67 |
| rust_decimal 1.43.0                            | HalfEven   |             108 |               114 |    318 |          763 |          2.98 |
| decimal-rs 0.2.0                               | HalfExpand |             207 |               198 |    554 |        1,154 |          2.74 |
| bigdecimal 0.4.11                              | HalfEven   |           4,281 |             4,394 | 12,287 |       49,099 |         29.48 |

**The rounded quotient, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.89 |              0.90 |   2.52 |         3.75 |          0.00 |
| `i128`                                           | Trunc      |            6.94 |              6.16 |   17.2 |         69.3 |          0.40 |
| **ddust** `D128<18>`                             | Trunc      |            12.6 |              13.5 |   37.7 |          129 |          0.49 |
| **ddust** `D128<18>`                             | HalfExpand |            13.6 |              14.4 |   40.4 |          138 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            15.0 |              15.9 |   44.6 |          140 |          0.49 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            36.5 |              22.3 |   62.3 |          172 |          1.60 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            37.1 |              27.4 |   76.7 |          201 |          1.47 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            49.6 |              29.9 |   83.6 |          240 |          1.79 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            39.3 |              34.7 |   96.9 |          256 |          2.41 |
| rust_decimal 1.43.0                              | HalfEven   |            91.1 |              98.5 |    276 |          637 |          4.32 |
| decimal-rs 0.2.0                                 | HalfExpand |             226 |               198 |    553 |        1,127 |          3.07 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             200 |               206 |    577 |        1,324 |          6.65 |
| bigdecimal 0.4.11                                | HalfEven   |          14,823 |            10,117 | 28,289 |      138,450 |         48.47 |

The same quotients, each dividend by one divisor, as a book is divided by one
price:

**The rounded quotient by one divisor, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                          | HalfEven   |            0.89 |              0.90 |   2.51 |         3.75 |          0.00 |
| `i64`                                          | Trunc      |            2.17 |              2.40 |   6.71 |         12.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            2.67 |              4.44 |   12.4 |         30.3 |          0.33 |
| **ddust** `D64<8>`                             | HalfExpand |            3.33 |              4.83 |   13.5 |         36.7 |          0.31 |
| **ddust** `D64<8>`                             | HalfEven   |            4.72 |              6.02 |   16.8 |         52.9 |          0.32 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            7.80 |              6.78 |   19.0 |         69.9 |          0.20 |
| fixdec 0.1.0 `D64`                             | Trunc      |            7.74 |              6.80 |   19.0 |         69.9 |          0.20 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            11.2 |              9.06 |   25.3 |         67.9 |          0.80 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            13.4 |              11.3 |   31.7 |         66.6 |          0.66 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            10.3 |              14.0 |   39.2 |         97.8 |          0.67 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            68.2 |              79.9 |    223 |          501 |          2.47 |
| rust_decimal 1.43.0                            | HalfEven   |             110 |               109 |    306 |          716 |          2.46 |
| decimal-rs 0.2.0                               | HalfExpand |             210 |               208 |    581 |        1,248 |          2.61 |
| bigdecimal 0.4.11                              | HalfEven   |           4,253 |             4,375 | 12,234 |       47,378 |         23.51 |

**The rounded quotient by one divisor, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            0.89 |              0.90 |   2.51 |         3.50 |          0.00 |
| `i128`                                           | Trunc      |            6.88 |              7.54 |   21.1 |         71.9 |          0.65 |
| **ddust** `D128<18>`                             | Trunc      |            12.6 |              13.5 |   37.8 |          129 |          0.50 |
| **ddust** `D128<18>`                             | HalfExpand |            13.6 |              14.5 |   40.4 |          138 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            15.0 |              15.9 |   44.6 |          140 |          0.50 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            37.8 |              22.3 |   62.4 |          174 |          1.38 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            33.2 |              23.1 |   64.8 |          154 |          0.93 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            49.7 |              30.0 |   84.0 |          243 |          1.37 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            42.7 |              34.9 |   97.7 |          261 |          1.77 |
| rust_decimal 1.43.0                              | HalfEven   |            92.8 |              86.9 |    243 |          610 |          3.03 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             196 |               171 |    480 |        1,151 |          4.52 |
| decimal-rs 0.2.0                                 | HalfExpand |             227 |               208 |    581 |        1,183 |          2.66 |
| bigdecimal 0.4.11                                | HalfEven   |          14,899 |             4,381 | 12,251 |       50,875 |         37.67 |

A [`Divisor`] prepared from that one divisor divides by two multiplications and
a comparison, with no division at all; no other crate has one:

**The rounded quotient by a prepared divisor, narrow, 64 bits at 8 decimals:**

| Contender          | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>` | Trunc      |            2.07 |              2.07 |   5.78 |         29.0 |          0.00 |
| **ddust** `D64<8>` | HalfExpand |            2.70 |              2.70 |   7.55 |         35.0 |          0.00 |
| **ddust** `D64<8>` | HalfEven   |            2.97 |              2.98 |   8.33 |         37.0 |          0.00 |

**The rounded quotient by a prepared divisor, wide, 128 bits at 18 decimals:**

| Contender            | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| -------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>` | Trunc      |            7.89 |              9.33 |   26.1 |         91.0 |          0.49 |
| **ddust** `D128<18>` | HalfExpand |            10.4 |              11.5 |   32.2 |          111 |          0.49 |
| **ddust** `D128<18>` | HalfEven   |            11.1 |              12.4 |   34.6 |          120 |          0.48 |

A narrow quotient by a prepared divisor takes 2.98 ns where by the decimal it
takes 6.02, and a wide one 12.4 where by the decimal it takes 15.9. Preparing it
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
| **ddust** `D64<8>`                             | HalfEven   |            1.30 |              1.30 |   3.62 |         16.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            1.13 |              3.22 |   9.00 |         17.0 |          0.48 |
| fixdec 0.1.0 `D64`                             | HalfEven   |            1.19 |              3.82 |   10.7 |         18.7 |          0.34 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | HalfEven   |            1.87 |              4.27 |   11.9 |         23.8 |          0.32 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            1.86 |              5.00 |   14.0 |         27.0 |          0.35 |
| decimal-rs 0.2.0                               | HalfExpand |            5.32 |              6.50 |   18.1 |         61.1 |          0.31 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            12.8 |              18.2 |   51.0 |          135 |          1.08 |
| rust_decimal 1.43.0                            | HalfEven   |            21.8 |              21.9 |   61.3 |          190 |          0.36 |
| bigdecimal 0.4.11                              | HalfEven   |            87.8 |              76.5 |    214 |          917 |          2.11 |

**To cents, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfExpand |            0.92 |              0.82 |   2.30 |         4.00 |          0.00 |
| **ddust** `D128<18>`                             | Trunc      |            3.89 |              3.97 |   11.1 |         31.1 |          0.26 |
| **ddust** `D128<18>`                             | HalfExpand |            3.89 |              3.98 |   11.1 |         31.1 |          0.26 |
| **ddust** `D128<18>`                             | HalfEven   |            4.12 |              4.19 |   11.7 |         33.1 |          0.25 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            12.4 |              8.40 |   23.5 |         58.9 |          0.97 |
| `i128`                                           | Trunc      |            10.1 |              8.70 |   24.3 |         68.5 |          0.96 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            12.5 |              8.92 |   24.9 |         51.3 |          0.53 |
| decimal-rs 0.2.0                                 | HalfExpand |            10.9 |              9.47 |   26.5 |         66.8 |          0.71 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | HalfEven   |            18.8 |              14.7 |   41.2 |          114 |          1.31 |
| fastnum 0.7.5 `D128`                             | HalfExpand |            23.3 |              24.0 |   67.3 |          184 |          1.11 |
| rust_decimal 1.43.0                              | HalfEven   |            43.1 |              43.7 |    122 |          354 |          0.23 |
| bigdecimal 0.4.11                                | HalfEven   |             126 |              91.2 |    255 |        1,010 |          2.37 |

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
| **ddust** `D64<8>`                             |            9.63 |              9.59 |   26.8 |          101 |          0.16 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            11.8 |              19.1 |   53.6 |          173 |          1.61 |
| rust_decimal 1.43.0                            |            17.4 |              19.1 |   53.5 |          175 |          1.75 |
| `f64`                                          |            14.3 |              19.5 |   54.5 |          169 |          1.73 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            18.3 |              20.8 |   58.3 |          202 |          1.64 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*      |            16.4 |              21.0 |   58.7 |          216 |          1.69 |
| fixdec 0.1.0 `D64`                             |            19.7 |              23.0 |   64.2 |          241 |          1.51 |
| fastnum 0.7.5 `D64`                            |            19.8 |              24.0 |   67.2 |          267 |          1.68 |
| fixnum 0.9.5 `<i64, U8>`                       |            26.3 |              28.6 |   80.1 |          360 |          1.57 |
| decimal-rs 0.2.0                               |            37.5 |              43.1 |    121 |          384 |          2.90 |
| bigdecimal 0.4.11                              |            93.9 |              91.3 |    255 |        1,373 |          1.42 |

**Reading, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>`                             |            9.42 |              10.0 |   28.1 |         95.5 |          0.48 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            16.9 |              20.9 |   58.3 |          230 |          1.37 |
| rust_decimal 1.43.0                              |            39.6 |              30.2 |   84.4 |          365 |          1.60 |
| `f64`                                            |            48.4 |              31.9 |   89.2 |          329 |          1.32 |
| fastnum 0.7.5 `D128`                             |            30.8 |              32.0 |   89.5 |          429 |          1.45 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*    |            36.1 |              35.7 |   99.7 |          425 |          1.57 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            44.0 |              39.6 |    111 |          402 |          1.58 |
| decimal-rs 0.2.0                                 |            59.5 |              62.1 |    174 |          577 |          2.09 |
| fixnum 0.9.5 `<i128, U18>`                       |            68.1 |              65.9 |    184 |          762 |          1.77 |
| bigdecimal 0.4.11                                |             131 |               118 |    330 |        1,751 |          1.34 |

Writing splits the steps into their 32 digits by multiplies, and makes each
window of the text by one lookup into them; `Display`, through a formatter, is
measured where a crate has nothing faster:

**Writing, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>`                             |            11.0 |              9.97 |   27.9 |          114 |          0.54 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            18.1 |              23.4 |   65.3 |          198 |          2.25 |
| `i64`                                          |            14.3 |              25.8 |   72.3 |          215 |          2.27 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            17.4 |              26.5 |   74.1 |          275 |          1.84 |
| fixnum 0.9.5 `<i64, U8>`                       |            21.7 |              29.0 |   81.0 |          268 |          1.63 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            23.8 |              33.2 |   93.0 |          333 |          2.61 |
| fixdec 0.1.0 `D64`                             |            34.2 |              40.5 |    113 |          408 |          2.12 |
| decimal-rs 0.2.0                               |            34.2 |              44.0 |    123 |          517 |          2.21 |
| rust_decimal 1.43.0                            |            76.4 |              65.8 |    184 |          641 |          1.44 |
| `f64`                                          |            63.7 |              67.3 |    188 |          650 |          2.99 |
| fastnum 0.7.5 `D64`                            |            64.3 |              79.0 |    221 |          889 |          2.23 |
| bigdecimal 0.4.11                              |             109 |               110 |    306 |        1,391 |          1.89 |

**Writing, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>`                             |            28.3 |              23.4 |   65.5 |          254 |          1.05 |
| `i128`                                           |            20.5 |              31.5 |   88.1 |          263 |          2.61 |
| fixnum 0.9.5 `<i128, U18>`                       |            43.9 |              42.6 |    119 |          429 |          1.76 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            57.8 |              45.8 |    128 |          611 |          1.37 |
| decimal-rs 0.2.0                                 |            37.8 |              48.4 |    135 |          551 |          2.62 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            68.3 |              49.3 |    138 |          439 |          2.29 |
| `f64`                                            |            65.9 |              73.9 |    207 |          706 |          3.50 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            95.9 |              77.3 |    216 |          569 |          2.23 |
| rust_decimal 1.43.0                              |             116 |              91.0 |    254 |          956 |          1.47 |
| fastnum 0.7.5 `D128`                             |            83.1 |               104 |    290 |        1,148 |          2.65 |
| bigdecimal 0.4.11                                |             148 |               137 |    384 |        1,646 |          2.39 |

## Floats

A decimal becomes the nearest double. Steps below $2^{53}$ at up to 22 decimals
take one conversion and one division by an exact power of ten, Clinger's fast
path; past them, a division of the steps by a power of five makes the double's
53 bits and the bits that round them. The predictable set's steps have 17 and 28
digits, past $2^{53}$, so it is the slower one here. The crates marked inexact
compute the double in floating point, rounding more than once:

**To a double, narrow, 64 bits at 8 decimals:**

| Contender                                                 | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                                     |            0.12 |              0.15 |   0.43 |         2.00 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*                 |            0.89 |              0.89 |   2.50 |         2.50 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>`, *inexact* |            0.89 |              0.89 |   2.50 |         2.50 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`, *inexact*                       |            1.23 |              1.23 |   3.44 |         11.5 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`, *inexact*                 |            1.23 |              1.23 |   3.44 |         11.5 |          0.00 |
| **ddust** `D64<8>`                                        |            4.76 |              1.48 |   4.15 |         15.6 |          0.05 |
| fixdec 0.1.0 `D64`, *inexact*                             |            1.61 |              1.61 |   4.51 |         19.0 |          0.00 |
| fastnum 0.7.5 `D64`                                       |            6.21 |              2.86 |   7.99 |         47.7 |          0.07 |
| decimal-rs 0.2.0                                          |            41.1 |              10.6 |   29.7 |          141 |          0.20 |
| rust_decimal 1.43.0, *inexact*                            |            21.7 |              23.9 |   66.7 |          246 |          0.58 |
| bigdecimal 0.4.11                                         |             110 |               108 |    301 |        1,234 |          1.57 |

**To a double, wide, 128 bits at 18 decimals:**

| Contender                                                   | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ----------------------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i128`                                                      |            3.55 |              3.55 |   9.93 |         43.5 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>`, *inexact* |            3.60 |              3.60 |   10.1 |         44.3 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*               |            3.60 |              3.60 |   10.1 |         44.3 |          0.00 |
| **ddust** `D128<18>`                                        |            7.43 |              7.56 |   21.1 |         60.7 |          0.40 |
| fastnum 0.7.5 `D128`                                        |            32.3 |              15.6 |   43.6 |          136 |          0.44 |
| nexus-decimal 1.2.2 `<i128, 18>`, *inexact*                 |            24.6 |              17.0 |   47.5 |          153 |          0.70 |
| fixnum 0.9.5 `<i128, U18>`, *inexact*                       |            24.8 |              17.0 |   47.5 |          153 |          0.71 |
| rust_decimal 1.43.0, *inexact*                              |            33.7 |              27.3 |   76.4 |          274 |          0.77 |
| decimal-rs 0.2.0                                            |            80.9 |              42.6 |    119 |          496 |          1.46 |
| bigdecimal 0.4.11                                           |             181 |               144 |    404 |        1,601 |          1.99 |

A double becomes the decimal nearest its exact binary value, by the mode. The
product of the double and the power of ten, and its residual, which a fused
multiply-add computes exactly, decide the rounding below $2^{52}$; past it, the
double's bits are scaled exactly. The crates marked inexact multiply in floating
point, or round the double's shortest text:

**From a double, narrow, 64 bits at 8 decimals:**

| Contender                                                 | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| --------------------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                                     | Trunc      |            0.27 |              0.29 |   0.82 |         3.00 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`, *inexact*                 | Trunc      |            0.44 |              0.48 |   1.33 |         4.75 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>`, *inexact* | HalfExpand |            0.62 |              0.63 |   1.77 |         5.75 |          0.00 |
| fixdec 0.1.0 `D64`, *inexact*                             | HalfExpand |            1.02 |              1.04 |   2.90 |         15.0 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`, *inexact*                 | HalfExpand |            1.02 |              1.04 |   2.91 |         15.0 |          0.00 |
| **ddust** `D64<8>`                                        | HalfEven   |            5.14 |              3.91 |   10.9 |         52.1 |          0.08 |
| fixnum 0.9.5 `<i64, U8>`, *inexact*                       | HalfExpand |            35.4 |              92.1 |    258 |          555 |          6.31 |
| fastnum 0.7.5 `D64`                                       | HalfExpand |            63.0 |               176 |    491 |        1,193 |          3.58 |
| decimal-rs 0.2.0, *inexact*                               | HalfExpand |             113 |               230 |    644 |        1,304 |          7.46 |
| rust_decimal 1.43.0, *inexact*                            | HalfEven   |             119 |               260 |    727 |        1,274 |         20.49 |
| bigdecimal 0.4.11                                         | HalfEven   |             218 |               344 |    959 |        3,855 |          5.03 |

**From a double, wide, 128 bits at 18 decimals:**

| Contender                                                   | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ----------------------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>`, *inexact* | HalfExpand |            2.87 |              2.87 |   8.02 |         42.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`, *inexact*               | Trunc      |            3.07 |              3.08 |   8.61 |         49.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`, *inexact*                 | HalfExpand |            3.42 |              3.44 |   9.61 |         53.0 |          0.00 |
| `i128`                                                      | Trunc      |            2.78 |              4.21 |   11.8 |         30.2 |          0.36 |
| **ddust** `D128<18>`                                        | HalfEven   |            4.94 |              6.32 |   17.7 |         66.0 |          0.49 |
| fixnum 0.9.5 `<i128, U18>`, *inexact*                       | HalfExpand |            36.6 |               138 |    386 |          743 |         12.36 |
| fastnum 0.7.5 `D128`                                        | HalfExpand |             105 |               179 |    502 |        1,348 |          4.20 |
| decimal-rs 0.2.0, *inexact*                                 | HalfExpand |            95.1 |               275 |    769 |        1,459 |         13.82 |
| rust_decimal 1.43.0, *inexact*                              | HalfEven   |             108 |               298 |    835 |        1,485 |         26.87 |
| bigdecimal 0.4.11                                           | HalfEven   |             202 |               410 |  1,146 |        4,434 |          6.08 |

## Latency

A chain makes each operation wait for the one before, by taking its operands at
an index the previous result decides: what a calculation that feeds on its own
results costs, where throughput is what a loop over independent values costs.

**The checked sum, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          |            3.57 |              3.68 |   10.3 |         14.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` |            3.70 |              3.68 |   10.3 |         14.0 |          0.00 |
| fixnum 0.9.5 `<i64, U8>`                       |            3.58 |              3.68 |   10.3 |         14.0 |          0.00 |
| nexus-decimal 1.2.2 `<i64, 8>`                 |            3.59 |              3.68 |   10.3 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             |            3.58 |              3.69 |   10.3 |         14.0 |          0.00 |
| fixdec 0.1.0 `D64`                             |            3.57 |              3.69 |   10.3 |         14.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal<8>`                 |            3.78 |              3.70 |   10.3 |         14.0 |          0.00 |
| fixed 1.31.0 `I64F64`                          |            4.48 |              4.56 |   12.8 |         18.0 |          0.00 |
| `f64`                                          |            4.94 |              5.01 |   14.0 |         14.0 |          0.00 |
| decimal-rs 0.2.0                               |            6.88 |              11.6 |   32.3 |         65.8 |          0.77 |
| rust_decimal 1.43.0                            |            12.3 |              17.3 |   48.3 |         80.5 |          1.20 |
| fastnum 0.7.5 `D64`                            |            20.2 |              29.8 |   83.2 |          136 |          1.43 |
| bigdecimal 0.4.11                              |            36.5 |              44.7 |    125 |          330 |          1.75 |

**The checked sum, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | --------------: | ----------------: | -----: | -----------: | ------------: |
| fixnum 0.9.5 `<i128, U18>`                       |            4.49 |              4.56 |   12.8 |         18.0 |          0.00 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` |            4.48 |              4.56 |   12.8 |         18.0 |          0.00 |
| fin_decimal 0.4.0 `Decimal128<18>`               |            4.48 |              4.56 |   12.8 |         18.0 |          0.00 |
| `i128`                                           |            4.48 |              4.56 |   12.8 |         18.0 |          0.00 |
| nexus-decimal 1.2.2 `<i128, 18>`                 |            4.48 |              4.56 |   12.8 |         18.0 |          0.00 |
| **ddust** `D128<18>`                             |            4.77 |              4.85 |   13.6 |         18.0 |          0.00 |
| `f64`                                            |            4.94 |              5.01 |   14.0 |         14.0 |          0.00 |
| decimal-rs 0.2.0                                 |            6.90 |              11.6 |   32.2 |         65.8 |          0.77 |
| rust_decimal 1.43.0                              |            12.9 |              19.0 |   53.0 |         80.1 |          1.51 |
| fastnum 0.7.5 `D128`                             |            17.3 |              25.9 |   72.4 |          152 |          1.56 |
| bigdecimal 0.4.11                                |            56.8 |              54.4 |    152 |          462 |          2.22 |

**The rounded product, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.62 |              4.65 |   13.0 |         16.0 |          0.00 |
| `f64`                                          | HalfEven   |            5.29 |              5.36 |   15.0 |         14.0 |          0.00 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc      |            5.23 |              7.64 |   21.4 |         26.0 |          0.50 |
| **ddust** `D64<8>`                             | Trunc      |            10.1 |              8.11 |   22.7 |         31.9 |          0.14 |
| fixed 1.31.0 `I64F64`                          | Floor      |            8.59 |              8.64 |   24.2 |         40.0 |          0.00 |
| **ddust** `D64<8>`                             | HalfExpand |            10.6 |              9.03 |   25.3 |         34.8 |          0.15 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            11.0 |              9.14 |   25.6 |         34.2 |          0.17 |
| **ddust** `D64<8>`                             | HalfEven   |            12.4 |              9.47 |   26.5 |         39.3 |          0.15 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            10.6 |              12.9 |   36.0 |         33.5 |          0.44 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            12.0 |              14.0 |   39.3 |         55.0 |          0.57 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            22.1 |              16.4 |   45.7 |         96.5 |          0.76 |
| rust_decimal 1.43.0                            | HalfEven   |            35.3 |              38.1 |    107 |          283 |          0.86 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            84.6 |              43.8 |    123 |          263 |          1.24 |
| decimal-rs 0.2.0                               | HalfExpand |            52.2 |              44.7 |    125 |          293 |          0.32 |
| bigdecimal 0.4.11                              | HalfEven   |             162 |               125 |    350 |        1,328 |          4.05 |

**The rounded product, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            5.29 |              5.36 |   15.0 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            3.76 |              7.05 |   19.7 |         27.5 |          0.28 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            20.8 |              13.0 |   36.5 |         67.9 |          0.05 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            20.3 |              15.1 |   42.4 |         63.4 |          0.55 |
| **ddust** `D128<18>`                             | Trunc      |            17.8 |              15.4 |   43.0 |         86.9 |          0.10 |
| **ddust** `D128<18>`                             | HalfExpand |            18.8 |              15.8 |   44.1 |         92.6 |          0.09 |
| **ddust** `D128<18>`                             | HalfEven   |            19.9 |              17.8 |   49.7 |          101 |          0.07 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            23.2 |              19.6 |   55.0 |          109 |          0.81 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            30.8 |              23.4 |   65.5 |          151 |          1.19 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             148 |              53.1 |    149 |          358 |          1.68 |
| decimal-rs 0.2.0                                 | HalfExpand |             148 |              57.5 |    161 |          352 |          0.83 |
| rust_decimal 1.43.0                              | HalfEven   |            85.7 |              63.5 |    178 |          402 |          2.01 |
| bigdecimal 0.4.11                                | HalfEven   |             247 |               172 |    480 |        1,770 |          4.87 |

**The rounded quotient, in a chain, narrow, 64 bits at 8 decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.97 |              4.95 |   13.9 |         15.0 |          0.00 |
| `f64`                                          | HalfEven   |            8.62 |              8.74 |   24.5 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            14.6 |              10.5 |   29.3 |         33.5 |          0.22 |
| **ddust** `D64<8>`                             | HalfExpand |            15.2 |              12.0 |   33.7 |         40.0 |          0.23 |
| **ddust** `D64<8>`                             | HalfEven   |            16.2 |              12.6 |   35.1 |         39.9 |          0.23 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            20.4 |              15.9 |   44.5 |         59.7 |          0.77 |
| fixdec 0.1.0 `D64`                             | Trunc      |            17.5 |              16.0 |   44.8 |         74.8 |          0.47 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            17.5 |              16.0 |   44.9 |         74.8 |          0.45 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            21.8 |              16.1 |   45.1 |         63.8 |          1.29 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            16.9 |              18.2 |   51.0 |         93.9 |          0.91 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            82.9 |              95.3 |    267 |          535 |          2.67 |
| rust_decimal 1.43.0                            | HalfEven   |             109 |               117 |    327 |          761 |          3.00 |
| decimal-rs 0.2.0                               | HalfExpand |             219 |               208 |    581 |        1,142 |          2.68 |
| bigdecimal 0.4.11                              | HalfEven   |           4,254 |             4,393 | 12,288 |       49,180 |         28.24 |

**The rounded quotient, in a chain, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            8.62 |              8.74 |   24.5 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            13.9 |              9.61 |   26.9 |         76.3 |          0.40 |
| **ddust** `D128<18>`                             | Trunc      |            30.1 |              30.4 |   85.0 |          143 |          0.49 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            50.5 |              31.2 |   87.4 |          182 |          1.61 |
| **ddust** `D128<18>`                             | HalfExpand |            31.4 |              31.6 |   88.5 |          152 |          0.50 |
| **ddust** `D128<18>`                             | HalfEven   |            32.4 |              33.2 |   92.9 |          154 |          0.49 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            46.9 |              37.3 |    104 |          216 |          1.51 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            60.8 |              39.7 |    111 |          255 |          1.82 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            43.7 |              41.0 |    115 |          271 |          2.44 |
| rust_decimal 1.43.0                              | HalfEven   |            99.8 |               103 |    289 |          635 |          4.30 |
| decimal-rs 0.2.0                                 | HalfExpand |             239 |               207 |    579 |        1,116 |          3.08 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             213 |               218 |    608 |        1,335 |          6.62 |
| bigdecimal 0.4.11                                | HalfEven   |          14,813 |             9,947 | 27,819 |      138,625 |         48.14 |

**The rounded quotient by one divisor, in a chain, narrow, 64 bits at 8
decimals:**

| Contender                                      | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ---------------------------------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `i64`                                          | Trunc      |            4.97 |              5.24 |   14.7 |         15.0 |          0.00 |
| `f64`                                          | HalfEven   |            8.62 |              8.73 |   24.5 |         14.0 |          0.00 |
| **ddust** `D64<8>`                             | Trunc      |            14.3 |              11.3 |   31.7 |         34.4 |          0.27 |
| **ddust** `D64<8>`                             | HalfExpand |            15.4 |              13.0 |   36.3 |         40.7 |          0.27 |
| **ddust** `D64<8>`                             | HalfEven   |            16.4 |              13.2 |   36.8 |         40.6 |          0.29 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand |            21.3 |              15.8 |   44.2 |         60.5 |          0.78 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc      |            17.1 |              15.9 |   44.4 |         71.9 |          0.20 |
| fixdec 0.1.0 `D64`                             | Trunc      |            17.1 |              15.9 |   44.4 |         71.9 |          0.21 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven   |            20.8 |              16.7 |   46.6 |         59.8 |          0.69 |
| fixnum 0.9.5 `<i64, U8>`                       | HalfExpand |            17.1 |              18.3 |   51.2 |         91.3 |          0.68 |
| fastnum 0.7.5 `D64`                            | HalfExpand |            82.6 |              90.4 |    253 |          511 |          2.47 |
| rust_decimal 1.43.0                            | HalfEven   |             114 |               112 |    314 |          714 |          2.47 |
| decimal-rs 0.2.0                               | HalfExpand |             222 |               220 |    615 |        1,237 |          2.61 |
| bigdecimal 0.4.11                              | HalfEven   |           4,242 |             4,388 | 12,273 |       47,477 |         22.71 |

**The rounded quotient by one divisor, in a chain, wide, 128 bits at 18
decimals:**

| Contender                                        | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------------------------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| `f64`                                            | HalfEven   |            8.61 |              8.73 |   24.5 |         14.0 |          0.00 |
| `i128`                                           | Trunc      |            14.1 |              11.2 |   31.2 |         77.9 |          0.64 |
| **ddust** `D128<18>`                             | Trunc      |            30.1 |              30.4 |   85.0 |          143 |          0.49 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc      |            50.8 |              31.5 |   88.1 |          184 |          1.37 |
| **ddust** `D128<18>`                             | HalfExpand |            31.5 |              31.6 |   88.5 |          152 |          0.49 |
| **ddust** `D128<18>`                             | HalfEven   |            32.3 |              33.2 |   92.9 |          154 |          0.49 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven   |            44.1 |              33.6 |   94.0 |          169 |          0.94 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand |            61.0 |              40.8 |    114 |          258 |          1.36 |
| fixnum 0.9.5 `<i128, U18>`                       | HalfExpand |            46.6 |              42.0 |    117 |          276 |          1.79 |
| rust_decimal 1.43.0                              | HalfEven   |            99.0 |              92.4 |    258 |          619 |          3.00 |
| fastnum 0.7.5 `D128`                             | HalfExpand |             208 |               182 |    510 |        1,162 |          4.43 |
| decimal-rs 0.2.0                                 | HalfExpand |             239 |               220 |    614 |        1,180 |          2.66 |
| bigdecimal 0.4.11                                | HalfEven   |          14,893 |             4,361 | 12,195 |       50,969 |         37.07 |

**The rounded quotient by a prepared divisor, in a chain, narrow, 64 bits at 8
decimals:**

| Contender          | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| ------------------ | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D64<8>` | Trunc      |            6.27 |              6.23 |   17.4 |         34.0 |          0.00 |
| **ddust** `D64<8>` | HalfExpand |            7.59 |              7.59 |   21.2 |         40.0 |          0.00 |
| **ddust** `D64<8>` | HalfEven   |            8.54 |              8.56 |   24.0 |         43.0 |          0.00 |

**The rounded quotient by a prepared divisor, in a chain, wide, 128 bits at 18
decimals:**

| Contender            | Mode       | ns, predictable | ns, unpredictable | Cycles | Instructions | Branch misses |
| -------------------- | ---------- | --------------: | ----------------: | -----: | -----------: | ------------: |
| **ddust** `D128<18>` | Trunc      |            14.2 |              14.6 |   40.9 |         96.0 |          0.48 |
| **ddust** `D128<18>` | HalfExpand |            17.3 |              17.9 |   50.0 |          116 |          0.49 |
| **ddust** `D128<18>` | HalfEven   |            19.4 |              19.7 |   55.1 |          126 |          0.48 |

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
| **ddust** `D64<8>`                             | Trunc                |            1.18 |              1.42 |      0.000 |             0.40 |
| **ddust** `D64<8>`                             | HalfExpand           |            1.41 |              1.63 |      0.000 |             0.36 |
| **ddust** `D64<8>`                             | HalfEven             |            1.66 |              1.74 |      0.000 |             0.37 |
| fixdec 0.1.0 `D64`, *inexact*                  | Trunc, HalfEven      |            2.26 |              3.24 |      0.000 |             1.27 |
| nexus-decimal 1.2.2 `<i64, 8>`                 | Trunc, HalfEven      |            2.57 |              3.42 |      0.000 |             1.21 |
| primitive_fixed_point_decimal 1.5.0 `<i64, 8>` | HalfExpand           |            3.54 |              3.63 |      0.000 |             1.88 |
| fin_decimal 0.4.0 `Decimal<8>`                 | HalfEven             |            3.27 |              4.87 |      0.000 |             1.31 |
| fastnum 0.7.5 `D64`                            | HalfExpand           |            35.4 |              31.1 |      0.001 |             7.33 |
| rust_decimal 1.43.0                            | HalfEven             |            33.3 |              35.1 |      0.001 |             6.30 |
| decimal-rs 0.2.0                               | HalfExpand           |            51.7 |              51.0 |      0.001 |             4.03 |
| bigdecimal 0.4.11                              | HalfEven             |             927 |               931 |      0.061 |             81.3 |

**The mix, wide, 128 bits at 18 decimals:**

| Contender                                        | Mode                 | ns, predictable | ns, unpredictable | L1i misses | Front-end stalls |
| ------------------------------------------------ | -------------------- | --------------: | ----------------: | ---------: | ---------------: |
| `f64`                                            | HalfEven, HalfExpand |            0.49 |              0.48 |      0.000 |             0.00 |
| `i128`                                           | Trunc                |            3.54 |              3.49 |      0.000 |             1.05 |
| **ddust** `D128<18>`                             | Trunc                |            5.21 |              4.75 |      0.000 |             1.17 |
| **ddust** `D128<18>`                             | HalfExpand           |            5.96 |              5.28 |      0.000 |             1.35 |
| **ddust** `D128<18>`                             | HalfEven             |            6.38 |              5.68 |      0.000 |             1.25 |
| primitive_fixed_point_decimal 1.5.0 `<i128, 18>` | HalfExpand           |            13.8 |              8.40 |      0.000 |             2.78 |
| nexus-decimal 1.2.2 `<i128, 18>`                 | Trunc, HalfEven      |            11.7 |              8.52 |      0.000 |             2.89 |
| fin_decimal 0.4.0 `Decimal128<18>`               | HalfEven             |            11.0 |              9.77 |      0.000 |             2.49 |
| rust_decimal 1.43.0                              | HalfEven             |            46.6 |              42.2 |      0.001 |             9.24 |
| decimal-rs 0.2.0                                 | HalfExpand           |            74.3 |              53.3 |      0.001 |             4.99 |
| fastnum 0.7.5 `D128`                             | HalfExpand           |            80.2 |              60.0 |      0.004 |             13.3 |
| bigdecimal 0.4.11                                | HalfEven             |           3,097 |             2,075 |      0.455 |              130 |

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
| format             |                197 |                1,509 |           333 |                           129 |       1,248 |    129 |          129 |     129 |        129 |        132 |
| to-f64             |                 70 |                  131 |            19 |                             7 |           7 |     19 |          377 |     177 |      1,994 |     10,163 |
| from-f64           |                141 |                  101 |            18 |                            15 |          11 |    488 |          467 | 130,345 |        530 |     10,916 |

A call through a pointer, which `core::fmt` makes, is not followed: a crate that
writes text through `Display` counts only the code around the formatter. ddust's
reader and writer count their general paths, for a number past 32 bytes or 30
decimals, beside their vector ones.

## What to Keep off a Hot Path

- **A wide division where a narrow one does.** A `D128` quotient costs about
  three times a `D64`'s, and a chain of them about twice; a quantity, a price or
  a cash amount fits a `D64`.
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
instruction and call no division routine, on aarch64 and `x86_64`. The run's
five passes agree within 2% in time and cycles, and 0.5% in instructions. It
runs with:

```sh
just bench-run 15 baseline graviton4 --passes 5
```
