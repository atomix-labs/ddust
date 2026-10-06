//! One function an operation and contender, out of line, so each kernel's code can be measured
//! and read: `nm` sizes it, `objdump` prints it, and the assembly gate checks it. A probe is named
//! by its operation and its contender, as
//! `probe::mul_round::<ddust_bench::contender::ddust::Narrow>`.
//!
//! ```text
//! cargo build --profile bench --example probe
//! python3 scripts/assembly.py sizes target/release/examples/probe
//! ```

use core::hint::black_box;

use ddust_bench::contender::{
    Buffer, CheckedAdd, Compare, DivRound, Format, FromF64, MulExact, MulRound, Parse,
    RescaleRound, ToF64,
};
use ddust_bench::for_each_contender;

/// The checked sum.
#[inline(never)]
fn add<C: CheckedAdd>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::checked_add(a, b)
}

/// The order.
#[inline(never)]
fn compare<C: Compare>(a: &C::Value, b: &C::Value) -> bool {
    C::is_less(a, b)
}

/// The rounded product.
#[inline(never)]
fn mul_round<C: MulRound>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::checked_mul_round(a, b)
}

/// The rounded quotient.
#[inline(never)]
fn div_round<C: DivRound>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::checked_div_round(a, b)
}

/// The value to cents.
#[inline(never)]
fn rescale_round<C: RescaleRound>(value: &C::Value) -> Option<C::Rounded> {
    C::rescale_round(value)
}

/// The value read from text.
#[inline(never)]
fn parse<C: Parse>(text: &str) -> Option<C::Value> {
    C::parse(text)
}

/// The value written.
#[inline(never)]
fn format<C: Format>(value: &C::Value, buffer: &mut Buffer) {
    C::format(value, buffer);
}

/// The value as a double.
#[inline(never)]
fn to_f64<C: ToF64>(value: &C::Value) -> f64 {
    C::to_f64(value)
}

/// The double as a value.
#[inline(never)]
fn from_f64<C: FromF64>(x: f64) -> Option<C::Value> {
    C::from_f64(x)
}

/// The price times the quantity.
#[inline(never)]
fn mul_exact<C: MulExact>(price: &C::Price, quantity: &C::Quantity) -> Option<C::Product> {
    C::checked_mul(price, quantity)
}

/// Hands every probe's address to `black_box`, so the linker keeps each.
fn main() {
    macro_rules! keep {
        ($contender:ty, $probe:ident) => {
            black_box($probe::<$contender> as *const ());
        };
    }
    for_each_contender!(add, keep, add);
    for_each_contender!(compare, keep, compare);
    for_each_contender!(mul_round, keep, mul_round);
    for_each_contender!(div_round, keep, div_round);
    for_each_contender!(rescale_round, keep, rescale_round);
    for_each_contender!(parse, keep, parse);
    for_each_contender!(format, keep, format);
    for_each_contender!(to_f64, keep, to_f64);
    for_each_contender!(from_f64, keep, from_f64);
    for_each_contender!(mul_exact, keep, mul_exact);
}
