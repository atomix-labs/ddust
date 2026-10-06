//! What a decimal's code costs the instruction cache: a sum, an order, a rounded product, a rounded
//! quotient and a value to cents in turn over the same values, as a program mixes them, with the
//! level 1 instruction cache's misses and the front end's stalls per operation beside the time.
//! Every contender with all five runs: those with a rounding to cents, which the rest lack.
//!
//! ```text
//! cargo bench --bench mixed -- --quick
//! ```

use core::hint::black_box;
use std::io;

use ddust_bench::contenders::{Add, Compare, DivRound, MulRound, Rescale};
use ddust_bench::inputs::{self, Inputs};
use ddust_bench::{Harness, contenders};

/// Every operation the mix holds.
trait Mix: Add + Compare + MulRound + DivRound + Rescale {}

impl<C: Add + Compare + MulRound + DivRound + Rescale> Mix for C {}

fn main() -> io::Result<()> {
    let mut harness = Harness::from_args();
    run(&mut harness)?;
    harness.finish()
}

/// Every contender with all five operations, on both sets.
fn run(harness: &mut Harness) -> io::Result<()> {
    for inputs in Inputs::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, inputs)?;
            };
        }
        contenders!(rescale, bench, mix);
    }
    Ok(())
}

/// The five operations in turn over a product set's pairs and a quotient set's, five operations
/// a pair.
fn mix<C: Mix>(harness: &mut Harness, inputs: Inputs) -> io::Result<()> {
    let name = format!("mixed/{}/{}/{}", C::WIDTH.name(), inputs.name(), C::NAME);
    if !harness.runs(&name) {
        return Ok(());
    }
    let values = |steps: &[i128]| -> Vec<C::Value> {
        steps.iter().filter_map(|&steps| C::from_steps(steps)).collect()
    };
    let products = inputs::products(C::WIDTH, inputs);
    let quotients = inputs::quotients(C::WIDTH, inputs);
    let (left, right) = (values(&products.left), values(&products.right));
    let (dividends, divisors) = (values(&quotients.left), values(&quotients.right));
    let operations = u64::try_from(left.len().saturating_mul(5)).unwrap_or(0);
    harness.measure(&name, operations, || {
        let (left, right) = (black_box(left.as_slice()), black_box(right.as_slice()));
        let (dividends, divisors) =
            (black_box(dividends.as_slice()), black_box(divisors.as_slice()));
        for (((a, b), dividend), divisor) in left.iter().zip(right).zip(dividends).zip(divisors) {
            black_box(C::add(a, b));
            black_box(C::less(a, b));
            black_box(C::mul_round(a, b));
            black_box(C::div_round(dividend, divisor));
            black_box(C::rescale(a));
        }
    })
}
