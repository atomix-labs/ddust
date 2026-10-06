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

use ddust_bench::contender::{CheckedAdd, Compare, DivRound, MulRound, RescaleRound};
use ddust_bench::input::{self, Predictability};
use ddust_bench::{Harness, for_each_contender};

/// Every operation the mix holds.
trait Mix: CheckedAdd + Compare + MulRound + DivRound + RescaleRound {}

impl<C: CheckedAdd + Compare + MulRound + DivRound + RescaleRound> Mix for C {}

fn main() -> io::Result<()> {
    let mut harness = Harness::from_args();
    run(&mut harness)?;
    harness.finish()
}

/// Every contender with all five operations, on both sets.
fn run(harness: &mut Harness) -> io::Result<()> {
    for predictability in Predictability::ALL {
        macro_rules! bench {
            ($contender:ty, $function:ident) => {
                $function::<$contender>(harness, predictability)?;
            };
        }
        for_each_contender!(rescale_round, bench, mix);
    }
    Ok(())
}

/// The five operations in turn over a product set's pairs and a quotient set's, five operations
/// a pair.
fn mix<C: Mix>(harness: &mut Harness, predictability: Predictability) -> io::Result<()> {
    let name = format!("mixed/{}/{}/{}", C::WIDTH.name(), predictability.name(), C::NAME);
    if !harness.is_selected(&name) {
        return Ok(());
    }
    let values = |steps: &[i128]| -> Vec<C::Value> {
        steps.iter().filter_map(|&steps| C::from_steps(steps)).collect()
    };
    let factors = input::factors(C::WIDTH, predictability);
    let dividends_and_divisors = input::dividends_and_divisors(C::WIDTH, predictability);
    let (left, right) = (values(&factors.left), values(&factors.right));
    let (dividends, divisors) =
        (values(&dividends_and_divisors.left), values(&dividends_and_divisors.right));
    let operations = u64::try_from(left.len().saturating_mul(5)).unwrap_or(0);
    harness.measure(&name, operations, || {
        let (left, right) = (black_box(left.as_slice()), black_box(right.as_slice()));
        let (dividends, divisors) =
            (black_box(dividends.as_slice()), black_box(divisors.as_slice()));
        for (((a, b), dividend), divisor) in left.iter().zip(right).zip(dividends).zip(divisors) {
            black_box(C::checked_add(a, b));
            black_box(C::is_less(a, b));
            black_box(C::checked_mul_round(a, b));
            black_box(C::checked_div_round(dividend, divisor));
            black_box(C::rescale_round(a));
        }
    })
}
