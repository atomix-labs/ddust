//! Every operation, on every contender, on both input sets: each by throughput, independent
//! operations over the set, and the binary ones by latency too, a chain where each waits for the
//! one before.
//!
//! A measurement is named `operation/width/inputs/contender`, as
//! `mul-round/64/unpredictable/ddust D64<8>`, and any part of a name filters the run:
//!
//! ```text
//! cargo bench --bench ops -- mul-round/64 --quick
//! ```

use core::hint::black_box;
use std::io;

use ddust_bench::contenders::{
    Add, Buffer, Compare, Contender, DivRound, Format, FromF64, MulExact, MulRound, Parse, Rescale,
    ToF64,
};
use ddust_bench::inputs::{self, Inputs, Pairs};
use ddust_bench::{Harness, contenders};

fn main() -> io::Result<()> {
    let mut harness = Harness::from_args();
    run(&mut harness)?;
    harness.finish()
}

/// Every measurement: each operation's throughput on both sets, then the chains' latency.
fn run(harness: &mut Harness) -> io::Result<()> {
    for inputs in Inputs::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, inputs)?;
            };
        }
        contenders!(add, bench, add);
        contenders!(compare, bench, compare);
        contenders!(mul_exact, bench, mul_exact);
        contenders!(mul_round, bench, mul_round);
        contenders!(div_round, bench, div_round);
        contenders!(rescale, bench, rescale);
        contenders!(parse, bench, parse);
        contenders!(format, bench, format);
        contenders!(to_f64, bench, to_f64);
        contenders!(from_f64, bench, from_f64);
    }
    for inputs in Inputs::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, inputs)?;
            };
        }
        contenders!(add, bench, add_chain);
        contenders!(mul_round, bench, mul_round_chain);
        contenders!(div_round, bench, div_round_chain);
    }
    Ok(())
}

/// The measurement's name: `operation/width/inputs/contender`.
fn name<C: Contender>(operation: &str, inputs: Inputs) -> String {
    format!("{operation}/{}/{}/{}", C::WIDTH.name(), inputs.name(), C::NAME)
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
    if !harness.runs(name) {
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
    if !harness.runs(name) {
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

/// Times `operation` over each of the inputs `build` makes, unless the filters pass the name over,
/// the results kept in a vector it reuses.
fn each_one<I, R>(
    harness: &mut Harness, name: &str, build: impl FnOnce() -> Vec<I>,
    mut operation: impl FnMut(&I) -> R,
) -> io::Result<()> {
    if !harness.runs(name) {
        return Ok(());
    }
    let inputs = build();
    let inputs = inputs.as_slice();
    let mut out: Vec<R> = inputs.iter().map(&mut operation).collect();
    harness.measure(name, u64::try_from(out.len()).unwrap_or(0), || {
        for (input, slot) in black_box(inputs).iter().zip(out.iter_mut()) {
            *slot = operation(input);
        }
        black_box(&mut out);
    })
}

/// The checked sum.
fn add<C: Add>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("add", inputs),
        || inputs::sums(C::WIDTH, inputs),
        C::add,
    )
}

/// The checked sum, as a chain.
fn add_chain<C: Add>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("add-chain", inputs),
        || inputs::sums(C::WIDTH, inputs),
        C::add,
    )
}

/// The order of two values.
fn compare<C: Compare>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("compare", inputs),
        || inputs::sums(C::WIDTH, inputs),
        C::less,
    )
}

/// The product at one scale, rounded.
fn mul_round<C: MulRound>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("mul-round", inputs),
        || inputs::products(C::WIDTH, inputs),
        C::mul_round,
    )
}

/// The product at one scale, rounded, as a chain.
fn mul_round_chain<C: MulRound>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("mul-round-chain", inputs),
        || inputs::products(C::WIDTH, inputs),
        C::mul_round,
    )
}

/// The quotient at one scale, rounded.
fn div_round<C: DivRound>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    throughput::<C, _>(
        harness,
        &name::<C>("div-round", inputs),
        || inputs::quotients(C::WIDTH, inputs),
        C::div_round,
    )
}

/// The quotient at one scale, rounded, as a chain.
fn div_round_chain<C: DivRound>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    latency::<C>(
        harness,
        &name::<C>("div-round-chain", inputs),
        || inputs::quotients(C::WIDTH, inputs),
        C::div_round,
    )
}

/// A value rounded to cents.
fn rescale<C: Rescale>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let build = || values::<C>(&inputs::values(C::WIDTH, inputs));
    each_one(harness, &name::<C>("rescale", inputs), build, C::rescale)
}

/// A value read from its shortest text.
fn parse<C: Parse>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let build = || inputs::texts(C::WIDTH, inputs);
    each_one(harness, &name::<C>("parse", inputs), build, |text| C::parse(text))
}

/// A value written into a reused buffer.
fn format<C: Format>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let build = || values::<C>(&inputs::values(C::WIDTH, inputs));
    let mut buffer = Buffer::new();
    each_one(harness, &name::<C>("format", inputs), build, |value| {
        C::format(value, &mut buffer);
        buffer.as_bytes().len()
    })
}

/// A value as the nearest double.
fn to_f64<C: ToF64>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let build = || values::<C>(&inputs::values(C::WIDTH, inputs));
    each_one(harness, &name::<C>("to-f64", inputs), build, C::to_f64)
}

/// A double as a value, rounded.
fn from_f64<C: FromF64>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let build = || inputs::doubles(C::WIDTH, inputs);
    each_one(harness, &name::<C>("from-f64", inputs), build, |&x| C::from_f64(x))
}

/// A price times a quantity, exactly.
fn mul_exact<C: MulExact>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let name = format!("mul-exact/{}/{}/{}", C::WIDTH.name(), inputs.name(), C::NAME);
    if !harness.runs(&name) {
        return Ok(());
    }
    let set = inputs::prices_and_quantities(inputs);
    let prices: Vec<C::Price> = set.left.iter().filter_map(|&steps| C::price(steps)).collect();
    let quantities: Vec<C::Quantity> =
        set.right.iter().filter_map(|&steps| C::quantity(steps)).collect();
    let mut out: Vec<Option<C::Product>> =
        prices.iter().zip(&quantities).map(|(a, b)| C::mul(a, b)).collect();
    harness.measure(&name, u64::try_from(out.len()).unwrap_or(0), || {
        let (prices, quantities) = (black_box(prices.as_slice()), black_box(quantities.as_slice()));
        for ((price, quantity), slot) in prices.iter().zip(quantities).zip(out.iter_mut()) {
            *slot = C::mul(price, quantity);
        }
        black_box(&mut out);
    })
}
