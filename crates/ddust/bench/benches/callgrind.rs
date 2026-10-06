//! ddust's kernels under callgrind, through gungraun: each one's instructions, and its simulated
//! caches and branches, over 4,096 values of the unpredictable sets. The counts are the same on any
//! machine for one binary, so CI compares a change's with main's and fails one whose instructions
//! rise by more than 1%. They weigh a division as an addition, so they are an alarm, not a speed:
//! the speed is the harness's, on real counters.
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

use ddust_bench::contenders::{
    Add as _, Buffer, Compare as _, Contender, DivRound as _, Format, FromF64, MulExact,
    MulRound as _, Parse, Rescale as _, ToF64 as _, ddust,
};
use ddust_bench::inputs::{self, Inputs, Pairs};
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
fn sums<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&inputs::sums(C::WIDTH, Inputs::Unpredictable))
}

/// Operands of a product.
fn products<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&inputs::products(C::WIDTH, Inputs::Unpredictable))
}

/// Operands of a quotient.
fn quotients<C: Contender>() -> (Vec<C::Value>, Vec<C::Value>) {
    pairs::<C>(&inputs::quotients(C::WIDTH, Inputs::Unpredictable))
}

/// Values to round, write or convert.
fn singles<C: Contender>() -> Vec<C::Value> {
    values::<C>(&inputs::values(C::WIDTH, Inputs::Unpredictable))
}

/// Texts to read: only the [`COUNT`] it needs, since glibc defers the work of freeing many small
/// allocations to the next `malloc`, which would land in the count.
fn texts<C: Contender>() -> Vec<String> {
    let decimals = C::WIDTH.decimals();
    let values = inputs::values(C::WIDTH, Inputs::Unpredictable);
    values.iter().take(COUNT).map(|&steps| oracle::text(steps, decimals)).collect()
}

/// Doubles to convert.
fn doubles<C: Contender>() -> Vec<f64> {
    inputs::doubles(C::WIDTH, Inputs::Unpredictable).into_iter().take(COUNT).collect()
}

/// Prices and quantities, at 2 and 5 decimals.
fn notionals()
-> (Vec<<ddust::Notional as MulExact>::Price>, Vec<<ddust::Notional as MulExact>::Quantity>) {
    let set = inputs::prices_and_quantities(Inputs::Unpredictable);
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

/// `operation` over each input, out of line, for the same reason.
#[inline(never)]
fn over_each<I, R>(inputs: &[I], operation: impl FnMut(&I) -> R) -> Vec<R> {
    inputs.iter().map(operation).collect()
}

/// Reads each text, out of line. A closure would do, but one written in a bench function is named
/// inside it, and callgrind's count, which toggles on that name, would stop where it starts.
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
/// Each bench function hands its inputs back, so gungraun drops them after the count stops: dropped
/// inside, freeing 4,096 strings would outweigh the parse it measures. gungraun's attribute takes
/// no doc comment, so the bench functions carry plain comments.
macro_rules! kernel {
    ($name:ident : $contender:ty,pairs $setup:ident, $operation:expr) => {
        #[library_benchmark]
        #[bench::values(setup = $setup::<$contender>)]
        fn $name(
            (left, right): (
                Vec<<$contender as Contender>::Value>,
                Vec<<$contender as Contender>::Value>,
            ),
        ) -> usize {
            black_box(over_pairs(&left, &right, $operation)).len()
        }
    };
    ($name:ident : $contender:ty,each $setup:ident, $operation:expr) => {
        #[library_benchmark]
        #[bench::values(setup = $setup::<$contender>)]
        fn $name(
            inputs: Vec<<$contender as Contender>::Value>,
        ) -> Vec<<$contender as Contender>::Value> {
            black_box(over_each(&inputs, $operation));
            inputs
        }
    };
}

kernel!(add_narrow: ddust::Narrow, pairs sums, ddust::Narrow::add);
kernel!(compare_narrow: ddust::Narrow, pairs sums, ddust::Narrow::less);
kernel!(mul_round_narrow: ddust::Narrow, pairs products, ddust::Narrow::mul_round);
kernel!(div_round_narrow: ddust::Narrow, pairs quotients, ddust::Narrow::div_round);
kernel!(rescale_narrow: ddust::Narrow, each singles, ddust::Narrow::rescale);
kernel!(to_f64_narrow: ddust::Narrow, each singles, ddust::Narrow::to_f64);
kernel!(add_wide: ddust::Wide, pairs sums, ddust::Wide::add);
kernel!(compare_wide: ddust::Wide, pairs sums, ddust::Wide::less);
kernel!(mul_round_wide: ddust::Wide, pairs products, ddust::Wide::mul_round);
kernel!(div_round_wide: ddust::Wide, pairs quotients, ddust::Wide::div_round);
kernel!(rescale_wide: ddust::Wide, each singles, ddust::Wide::rescale);
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

// Multiplies each price by its quantity, exactly.
#[library_benchmark]
#[bench::narrow(setup = notionals)]
fn mul_exact_narrow(
    (prices, quantities): (
        Vec<<ddust::Notional as MulExact>::Price>,
        Vec<<ddust::Notional as MulExact>::Quantity>,
    ),
) -> (Vec<<ddust::Notional as MulExact>::Price>, Vec<<ddust::Notional as MulExact>::Quantity>) {
    black_box(over_pairs(&prices, &quantities, ddust::Notional::mul));
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
        rescale_narrow,
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
        rescale_wide,
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
