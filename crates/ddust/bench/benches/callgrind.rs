//! ddust's kernels under callgrind, through gungraun: each one's instructions, and its simulated
//! caches and branches, over 4,096 values of the unpredictable sets.
//!
//! The counts are the same on any machine for one binary, so CI compares a change's with main's
//! and fails one whose instructions rise by more than 1%. They weigh a division as an addition, so
//! they are an alarm, not a speed: the speed is the harness's, on real counters.
//!
//! ```text
//! cargo bench --bench callgrind     # with valgrind, and gungraun-runner 0.20.0 installed
//! ```

#![cfg_attr(
    test,
    expect(
        missing_docs,
        reason = "gungraun's macros generate public modules, constants and functions with no docs"
    )
)]

use core::hint::black_box;

use ddust_bench::contender::{
    Buffer, CheckedAdd as _, Compare as _, Contender, DivRound as _, DivRoundPrepared, Format,
    FromF64, MulExact, MulRound as _, Parse, RescaleRound as _, ToF64 as _, ddust,
};
use ddust_bench::input::{self, Pairs, Predictability};
use ddust_bench::oracle;
use gungraun::{
    Callgrind, EventKind, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main,
};

/// The values each kernel runs over: enough of every digit count and sign, and few enough that
/// valgrind finishes in seconds.
const COUNT: usize = 4096;

/// The first [`COUNT`] of `steps`, as `C`'s values.
fn values<C: Contender>(steps: &[i128]) -> Vec<C::Value> {
    steps.iter().take(COUNT).filter_map(|&steps| C::from_steps(steps)).collect()
}

/// A pair set's values.
fn pairs<C: Contender>(set: &Pairs) -> (Vec<C::Value>, Vec<C::Value>) {
    (values::<C>(&set.left), values::<C>(&set.right))
}

/// Operands of a sum.
fn addends<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&input::addends(C::WIDTH, Predictability::Unpredictable))
}

/// Operands of a product.
fn factors<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&input::factors(C::WIDTH, Predictability::Unpredictable))
}

/// Operands of a quotient.
fn dividends_and_divisors<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&input::dividends_and_divisors(C::WIDTH, Predictability::Unpredictable))
}

/// Dividends, and their one divisor prepared, once for each.
fn dividends_and_prepared<C: DivRoundPrepared>() -> (Vec<C::Value>, Vec<C::Prepared>) {
    let set = input::dividends_and_one_divisor(C::WIDTH, Predictability::Unpredictable);
    let (dividends, divisors) = pairs::<C>(&set);
    let prepared = divisors.first().and_then(C::prepare);
    let prepared = prepared.map_or_default(|divisor| vec![divisor; dividends.len()]);
    (dividends, prepared)
}

/// Values to round, write or convert.
fn singles<C: Contender>() -> Vec<C::Value> {
    values::<C>(&input::steps(C::WIDTH, Predictability::Unpredictable))
}

/// Texts to read: only the [`COUNT`] it needs, since glibc defers the work of freeing many small
/// allocations to the next `malloc`, which would land in the count.
fn texts<C: Contender>() -> Vec<String> {
    let decimals = C::WIDTH.decimals();
    let values = input::steps(C::WIDTH, Predictability::Unpredictable);
    values.iter().take(COUNT).map(|&steps| oracle::text(steps, decimals)).collect()
}

/// Doubles to convert.
fn doubles<C: Contender>() -> Vec<f64> {
    input::doubles(C::WIDTH, Predictability::Unpredictable).into_iter().take(COUNT).collect()
}

/// Prices and quantities, at 2 and 5 decimals.
fn notionals()
-> (Vec<<ddust::Notional as MulExact>::Price>, Vec<<ddust::Notional as MulExact>::Quantity>) {
    let set = input::prices_and_quantities(Predictability::Unpredictable);
    (
        set.left.iter().take(COUNT).filter_map(|&steps| ddust::Notional::price(steps)).collect(),
        set.right
            .iter()
            .take(COUNT)
            .filter_map(|&steps| ddust::Notional::quantity(steps))
            .collect(),
    )
}

/// `operation` over each pair, out of line: inlined into a bench function, its loop reads to
/// callgrind as the function calling itself, which stops the count after the first call.
#[inline(never)]
fn over_pairs<V, W, R>(left: &[V], right: &[W], operation: impl Fn(&V, &W) -> R) -> Vec<R> {
    left.iter().zip(right).map(|(a, b)| operation(a, b)).collect()
}

/// `operation` over each operand, out of line, for the same reason.
#[inline(never)]
fn over_each<I, R>(operands: &[I], operation: impl FnMut(&I) -> R) -> Vec<R> {
    operands.iter().map(operation).collect()
}

/// Reads each text, out of line.
///
/// A closure would do, but one written in a bench function is named inside it, and callgrind's
/// count, which toggles on that name, would stop where it starts.
#[inline(never)]
fn read<C: Parse>(texts: &[String]) -> Vec<Option<C::Value>> {
    texts.iter().map(|text| C::parse(text)).collect()
}

/// Converts each double, out of line, for the same reason.
#[inline(never)]
fn converted<C: FromF64>(doubles: &[f64]) -> Vec<Option<C::Value>> {
    doubles.iter().map(|&x| C::from_f64(x)).collect()
}

/// Writes each value into one buffer, out of line, and the lengths.
#[inline(never)]
fn written<C: Format>(values: &[C::Value]) -> Vec<usize> {
    let mut buffer = Buffer::new();
    values
        .iter()
        .map(|value| {
            C::format(value, &mut buffer);
            buffer.as_bytes().len()
        })
        .collect()
}

/// A bench function `$name`: `$operation` of `$contender` over `$setup`'s values, out of line.
///
/// Each bench function hands its inputs back, so gungraun drops them after the count stops:
/// dropped inside, freeing 4,096 strings would outweigh the parse it measures. gungraun's attribute
/// takes no doc comment, so the bench functions carry plain comments.
macro_rules! kernel {
    ($name:ident : $contender:ty,pairs $setup:ident, $operation:expr) => {
        #[library_benchmark]
        #[bench::values(setup = $setup::<$contender>)]
        fn $name(
            pairs: (Vec<<$contender as Contender>::Value>, Vec<<$contender as Contender>::Value>),
        ) -> (Vec<<$contender as Contender>::Value>, Vec<<$contender as Contender>::Value>) {
            black_box(over_pairs(&pairs.0, &pairs.1, $operation));
            pairs
        }
    };
    ($name:ident : $contender:ty,prepared $setup:ident, $operation:expr) => {
        #[library_benchmark]
        #[bench::values(setup = $setup::<$contender>)]
        fn $name(
            (dividends, divisors): (
                Vec<<$contender as Contender>::Value>,
                Vec<<$contender as DivRoundPrepared>::Prepared>,
            ),
        ) -> (Vec<<$contender as Contender>::Value>, Vec<<$contender as DivRoundPrepared>::Prepared>)
        {
            black_box(over_pairs(&dividends, &divisors, $operation));
            (dividends, divisors)
        }
    };
    ($name:ident : $contender:ty,each $setup:ident, $operation:expr) => {
        #[library_benchmark]
        #[bench::values(setup = $setup::<$contender>)]
        fn $name(
            operands: Vec<<$contender as Contender>::Value>,
        ) -> Vec<<$contender as Contender>::Value> {
            black_box(over_each(&operands, $operation));
            operands
        }
    };
}

kernel!(add_narrow: ddust::Narrow, pairs addends, ddust::Narrow::checked_add);
kernel!(compare_narrow: ddust::Narrow, pairs addends, ddust::Narrow::is_less);
kernel!(mul_round_narrow: ddust::Narrow, pairs factors, ddust::Narrow::checked_mul_round);
kernel!(div_round_narrow: ddust::Narrow, pairs dividends_and_divisors, ddust::Narrow::checked_div_round);
kernel!(rescale_round_narrow: ddust::Narrow, each singles, ddust::Narrow::rescale_round);
kernel!(to_f64_narrow: ddust::Narrow, each singles, ddust::Narrow::to_f64);
kernel!(add_wide: ddust::Wide, pairs addends, ddust::Wide::checked_add);
kernel!(compare_wide: ddust::Wide, pairs addends, ddust::Wide::is_less);
kernel!(mul_round_wide: ddust::Wide, pairs factors, ddust::Wide::checked_mul_round);
kernel!(div_round_wide: ddust::Wide, pairs dividends_and_divisors, ddust::Wide::checked_div_round);
kernel!(rescale_round_wide: ddust::Wide, each singles, ddust::Wide::rescale_round);
kernel!(to_f64_wide: ddust::Wide, each singles, ddust::Wide::to_f64);

// Reads each text.
#[library_benchmark]
#[bench::narrow(setup = texts::<ddust::Narrow>)]
fn parse_narrow(texts: Vec<String>) -> Vec<String> {
    black_box(read::<ddust::Narrow>(&texts));
    texts
}

// Reads each text.
#[library_benchmark]
#[bench::wide(setup = texts::<ddust::Wide>)]
fn parse_wide(texts: Vec<String>) -> Vec<String> {
    black_box(read::<ddust::Wide>(&texts));
    texts
}

// Writes each value.
#[library_benchmark]
#[bench::narrow(setup = singles::<ddust::Narrow>)]
fn format_narrow(
    values: Vec<<ddust::Narrow as Contender>::Value>,
) -> Vec<<ddust::Narrow as Contender>::Value> {
    black_box(written::<ddust::Narrow>(&values));
    values
}

// Writes each value.
#[library_benchmark]
#[bench::wide(setup = singles::<ddust::Wide>)]
fn format_wide(
    values: Vec<<ddust::Wide as Contender>::Value>,
) -> Vec<<ddust::Wide as Contender>::Value> {
    black_box(written::<ddust::Wide>(&values));
    values
}

// Converts each double.
#[library_benchmark]
#[bench::narrow(setup = doubles::<ddust::Narrow>)]
fn from_f64_narrow(doubles: Vec<f64>) -> Vec<f64> {
    black_box(converted::<ddust::Narrow>(&doubles));
    doubles
}

// Converts each double.
#[library_benchmark]
#[bench::wide(setup = doubles::<ddust::Wide>)]
fn from_f64_wide(doubles: Vec<f64>) -> Vec<f64> {
    black_box(converted::<ddust::Wide>(&doubles));
    doubles
}

kernel!(div_round_prepared_narrow: ddust::Narrow, prepared dividends_and_prepared, ddust::Narrow::checked_div_round_prepared);
kernel!(div_round_prepared_wide: ddust::Wide, prepared dividends_and_prepared, ddust::Wide::checked_div_round_prepared);

// Multiplies each price by its quantity, exactly.
#[library_benchmark]
#[bench::narrow(setup = notionals)]
fn mul_exact_narrow(
    (prices, quantities): (
        Vec<<ddust::Notional as MulExact>::Price>,
        Vec<<ddust::Notional as MulExact>::Quantity>,
    ),
) -> (Vec<<ddust::Notional as MulExact>::Price>, Vec<<ddust::Notional as MulExact>::Quantity>) {
    black_box(over_pairs(&prices, &quantities, ddust::Notional::checked_mul));
    (prices, quantities)
}

library_benchmark_group!(
    name = narrow,
    benchmarks = [
        add_narrow,
        compare_narrow,
        mul_exact_narrow,
        mul_round_narrow,
        div_round_narrow,
        div_round_prepared_narrow,
        rescale_round_narrow,
        parse_narrow,
        format_narrow,
        to_f64_narrow,
        from_f64_narrow,
    ]
);

library_benchmark_group!(
    name = wide,
    benchmarks = [
        add_wide,
        compare_wide,
        mul_round_wide,
        div_round_wide,
        div_round_prepared_wide,
        rescale_round_wide,
        parse_wide,
        format_wide,
        to_f64_wide,
        from_f64_wide,
    ]
);

main!(
    config = LibraryBenchmarkConfig::default().tool(
        Callgrind::with_args(["--branch-sim=yes", "--cache-sim=yes"]).soft_limits([(EventKind::Ir, 1.0)])
    );
    library_benchmark_groups = narrow, wide
);
