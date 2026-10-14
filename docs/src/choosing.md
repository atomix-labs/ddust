# Choosing a Decimal

A decimal is three choices: the integer, which sets the range; the scale, which
sets the decimals and whether the compiler knows them; and, for each operation
that rounds, the mode.

## The Integer and the Scale

| Need                                                       | Take                          | Why                                                                                 |
| ---------------------------------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------- |
| an amount to the cent: a balance, a payment, an invoice    | `D64<2>`                      | eight bytes, up to about ±9.2 × 10¹⁶                                                |
| a price or a quantity to eight decimals                    | `D64<8>`                      | eight bytes, up to about ±9.2 × 10¹⁰                                                |
| a token's amount at eighteen decimals, or a large notional | `D128<18>`                    | sixteen bytes, up to about ±1.7 × 10²⁰                                              |
| the exact product of two `D64`s                            | `widen` them first            | a `D128` at the sum of their scales holds every product of two `i64`s               |
| a value that is never negative                             | `UD64` and its siblings       | the type refuses a negative value, and a `u64` holds twice an `i64`'s largest value |
| millions of values in memory                               | the narrowest that holds them | a `D32<2>` holds ±21 million in four bytes, sixteen to a 64-byte cache line         |

The scale:

| Need                                                       | Take       | Why                                                                                 |
| ---------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------- |
| a scale known when the code is written                     | `Fixed<D>` | no bytes, two scales that never mix by accident, and every operation at its fastest |
| a scale learned as the program runs: a venue's, a column's | `Dynamic`  | a byte beside the steps, and two scales that line up exactly                        |

A program that learns its scales at run time can still keep each in a type where
the values of one scale gather: a book of one instrument, or a column of one
table. [Run-Time Scales](run-time-scales.md) converts between the two.

## The Rounding Mode

| Need                                                  | Take                                                                           |
| ----------------------------------------------------- | ------------------------------------------------------------------------------ |
| a mode the code names                                 | its type: `HalfEven`, `Floor` and the rest                                     |
| a mode a configuration, a venue or a protocol chooses | a [`Rounding`][ddust::round::Rounding]                                         |
| a rule that names the nearest cent                    | `HalfEven`, banker's rounding, or `HalfExpand`, the school's, as the rule says |
| a fee or a margin a business must not undercharge     | `Ceil`, or `Expand` for both signs                                             |
| a size an account must be able to pay for             | `Floor`, or `Trunc` for both signs                                             |

## Division

| Need                                  | Take                                                  |
| ------------------------------------- | ----------------------------------------------------- |
| a quotient truncated, as an integer's | `/`                                                   |
| a quotient rounded                    | [`div_round`][ddust::Decimal::div_round], with a mode |
| a quotient at another scale           | [`div_round_to`][ddust::Decimal::div_round_to]        |
| many quotients by one divisor         | a [`Divisor`], prepared once                          |
| a share: `a × b / c`, rounded once    | [`mul_div_round`][ddust::Decimal::mul_div_round]      |

[Performance](performance.md) measures what each choice costs.
