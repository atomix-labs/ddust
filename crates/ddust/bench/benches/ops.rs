//! Every operation, on every contender, on predictable values and unpredictable ones: each by
//! throughput, independent operations over the values, and the sum, the rounded product and the
//! rounded quotient by latency too, a chain where each waits for the one before.
//!
//! A measurement is named `operation/width/predictability/contender`, as
//! `mul-round/narrow/unpredictable/ddust D64<8>`, and any part of a name filters the run:
//!
//! ```text
//! cargo bench --bench ops -- mul-round/narrow --quick
//! ```

use core::hint::black_box;
use std::io;

use ddust_bench::contender::{
    Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, MulExact, MulRound, Parse,
    RescaleRound, ToF64,
};
use ddust_bench::input::{self, Pairs, Predictability};
use ddust_bench::{Harness, for_each_contender};

fn main() -> io::Result<()> {
    let mut harness = Harness::from_args()?;
    run(&mut harness)?;
    harness.finish()
}

/// Every measurement: each operation's throughput on both sets, then the chains' latency.
fn run(harness: &mut Harness) -> io::Result<()> {
    for predictability in Predictability::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, predictability)?;
            };
        }
        for_each_contender!(add, bench, add);
        for_each_contender!(compare, bench, compare);
        for_each_contender!(mul_exact, bench, mul_exact);
        for_each_contender!(mul_round, bench, mul_round);
        for_each_contender!(div_round, bench, div_round);
        for_each_contender!(rescale_round, bench, rescale_round);
        for_each_contender!(parse, bench, parse);
        for_each_contender!(format, bench, format);
        for_each_contender!(to_f64, bench, to_f64);
        for_each_contender!(from_f64, bench, from_f64);
    }
    for predictability in Predictability::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, predictability)?;
            };
        }
        for_each_contender!(add, bench, add_chain);
        for_each_contender!(mul_round, bench, mul_round_chain);
        for_each_contender!(div_round, bench, div_round_chain);
    }
    Ok(())
}

/// The measurement's name: `operation/width/predictability/contender`.
fn name<C: Contender>(operation: &str, predictability: Predictability) -> String {
    format!("{operation}/{}/{}/{}", C::WIDTH.name(), predictability.name(), C::NAME)
}

/// The values of `steps`, built before anything is timed.
fn values<C: Contender>(steps: &[i128]) -> Vec<C::Value> {
    steps.iter().filter_map(|&steps| C::from_steps(steps)).collect()
}

/// Times `operation` over each pair of the values `set` builds, unless the filters pass the name
/// over, the results kept in a vector it reuses.
fn throughput<C: Contender, R>(
    harness: &mut Harness, name: &str, set: impl FnOnce() -> Pairs,
    operation: impl Fn(&C::Value, &C::Value) -> R,
) -> io::Result<()> {
    if !harness.is_selected(name) {
        return Ok(());
    }
    let set = set();
    let (left, right) = (values::<C>(&set.left), values::<C>(&set.right));
    let mut out: Vec<R> = left.iter().zip(&right).map(|(a, b)| operation(a, b)).collect();
    harness.measure(name, u64::try_from(out.len()).unwrap_or(0), || {
        let (left, right) = (black_box(left.as_slice()), black_box(right.as_slice()));
        for ((a, b), slot) in left.iter().zip(right).zip(out.iter_mut()) {
            *slot = operation(a, b);
        }
        black_box(&mut out);
    })
}

/// Times `operation` as a chain over the values `set` builds: each pair's index mixes in the
/// fingerprint of the result before, through a zero the compiler cannot see, so each waits for
/// the last.
fn latency<C: Contender>(
    harness: &mut Harness, name: &str, set: impl FnOnce() -> Pairs,
    operation: impl Fn(&C::Value, &C::Value) -> Option<C::Value>,
) -> io::Result<()> {
    if !harness.is_selected(name) {
        return Ok(());
    }
    let set = set();
    let (left, right) = (values::<C>(&set.left), values::<C>(&set.right));
    let mask = left.len().saturating_sub(1);
    let zero = black_box(0_usize);
    harness.measure(name, u64::try_from(left.len()).unwrap_or(0), || {
        let (left, right) = (black_box(left.as_slice()), black_box(right.as_slice()));
        let mut previous = 0_usize;
        for position in 0..left.len() {
            let index = (position ^ (previous & zero)) & mask;
            if let (Some(a), Some(b)) = (left.get(index), right.get(index)) {
                previous = operation(a, b).as_ref().map_or(0, C::fingerprint);
            }
        }
        black_box(previous);
    })
}

/// Times `operation` over each of the operands `build` makes, unless the filters pass the
/// name over, the results kept in a vector it reuses.
fn each_one<I, R>(
    harness: &mut Harness, name: &str, build: impl FnOnce() -> Vec<I>,
    mut operation: impl FnMut(&I) -> R,
) -> io::Result<()> {
    if !harness.is_selected(name) {
        return Ok(());
    }
    let operands = build();
    let operands = operands.as_slice();
    let mut out: Vec<R> = operands.iter().map(&mut operation).collect();
    harness.measure(name, u64::try_from(out.len()).unwrap_or(0), || {
        for (operand, slot) in black_box(operands).iter().zip(out.iter_mut()) {
            *slot = operation(operand);
        }
        black_box(&mut out);
    })
}

/// The checked sum.
fn add<C: CheckedAdd>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("add", predictability),
        || input::addends(C::WIDTH, predictability),
        C::checked_add,
    )
}

/// The checked sum, as a chain.
fn add_chain<C: CheckedAdd>(
    harness: &mut Harness, predictability: Predictability,
) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("add-chain", predictability),
        || input::addends(C::WIDTH, predictability),
        C::checked_add,
    )
}

/// The order of two values.
fn compare<C: Compare>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("compare", predictability),
        || input::addends(C::WIDTH, predictability),
        C::is_less,
    )
}

/// The product at one scale, rounded.
fn mul_round<C: MulRound>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("mul-round", predictability),
        || input::factors(C::WIDTH, predictability),
        C::checked_mul_round,
    )
}

/// The product at one scale, rounded, as a chain.
fn mul_round_chain<C: MulRound>(
    harness: &mut Harness, predictability: Predictability,
) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("mul-round-chain", predictability),
        || input::factors(C::WIDTH, predictability),
        C::checked_mul_round,
    )
}

/// The quotient at one scale, rounded.
fn div_round<C: DivRound>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("div-round", predictability),
        || input::dividends_and_divisors(C::WIDTH, predictability),
        C::checked_div_round,
    )
}

/// The quotient at one scale, rounded, as a chain.
fn div_round_chain<C: DivRound>(
    harness: &mut Harness, predictability: Predictability,
) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("div-round-chain", predictability),
        || input::dividends_and_divisors(C::WIDTH, predictability),
        C::checked_div_round,
    )
}

/// A value rounded to cents.
fn rescale_round<C: RescaleRound>(
    harness: &mut Harness, predictability: Predictability,
) -> io::Result<()> {
    let build = || values::<C>(&input::steps(C::WIDTH, predictability));
    each_one(harness, &name::<C>("rescale-round", predictability), build, C::rescale_round)
}

/// A value read from its shortest text.
fn parse<C: Parse>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let build = || input::texts(C::WIDTH, predictability);
    each_one(harness, &name::<C>("parse", predictability), build, |text| C::parse(text))
}

/// A value written into a reused buffer.
fn format<C: Format>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let build = || values::<C>(&input::steps(C::WIDTH, predictability));
    let mut buffer = Buffer::new();
    each_one(harness, &name::<C>("format", predictability), build, |value| {
        C::format(value, &mut buffer);
        buffer.as_bytes().len()
    })
}

/// A value as the nearest double.
fn to_f64<C: ToF64>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let build = || values::<C>(&input::steps(C::WIDTH, predictability));
    each_one(harness, &name::<C>("to-f64", predictability), build, C::to_f64)
}

/// A double as a value, rounded.
fn from_f64<C: FromF64>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let build = || input::doubles(C::WIDTH, predictability);
    each_one(harness, &name::<C>("from-f64", predictability), build, |&x| C::from_f64(x))
}

/// A price times a quantity, exactly.
fn mul_exact<C: MulExact>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let name = format!("mul-exact/{}/{}/{}", C::WIDTH.name(), predictability.name(), C::NAME);
    if !harness.is_selected(&name) {
        return Ok(());
    }
    let set = input::prices_and_quantities(predictability);
    let prices: Vec<C::Price> = set.left.iter().filter_map(|&steps| C::price(steps)).collect();
    let quantities: Vec<C::Quantity> =
        set.right.iter().filter_map(|&steps| C::quantity(steps)).collect();
    let mut out: Vec<Option<C::Product>> =
        prices.iter().zip(&quantities).map(|(a, b)| C::checked_mul(a, b)).collect();
    harness.measure(&name, u64::try_from(out.len()).unwrap_or(0), || {
        let (prices, quantities) = (black_box(prices.as_slice()), black_box(quantities.as_slice()));
        for ((price, quantity), slot) in prices.iter().zip(quantities).zip(out.iter_mut()) {
            *slot = C::checked_mul(price, quantity);
        }
        black_box(&mut out);
    })
}
