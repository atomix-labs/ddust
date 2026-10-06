//! One function an operation and contender, out of line, so each kernel's code can be measured
//! and read: `nm` sizes it, `objdump` prints it, and the assembly gate checks it. A probe is named
//! by its operation and its contender, as
//! `probe::mul_round::<ddust_bench::contenders::ddust::Narrow>`.
//!
//! ```text
//! cargo build --profile bench --example probe
//! python3 scripts/code.py sizes target/release/examples/probe
//! ```

use core::hint::black_box;

use ddust_bench::contenders;
use ddust_bench::contenders::{
    Add, Buffer, Compare, DivRound, Format, FromF64, MulExact, MulRound, Parse, Rescale, ToF64,
};

/// The checked sum.
#[inline(never)]
fn add<C: Add>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::add(a, b)
}

/// The order.
#[inline(never)]
fn compare<C: Compare>(a: &C::Value, b: &C::Value) -> bool {
    C::less(a, b)
}

/// The rounded product.
#[inline(never)]
fn mul_round<C: MulRound>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::mul_round(a, b)
}

/// The rounded quotient.
#[inline(never)]
fn div_round<C: DivRound>(a: &C::Value, b: &C::Value) -> Option<C::Value> {
    C::div_round(a, b)
}

/// The value to cents.
#[inline(never)]
fn rescale<C: Rescale>(value: &C::Value) -> Option<C::Rounded> {
    C::rescale(value)
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
    C::mul(price, quantity)
}

/// Hands every probe's address to `black_box`, so the linker keeps each.
fn main() {
    macro_rules! keep {
        ($contender:ty, $probe:ident) => {
            black_box($probe::<$contender> as *const ());
        };
    }
    contenders!(add, keep, add);
    contenders!(compare, keep, compare);
    contenders!(mul_round, keep, mul_round);
    contenders!(div_round, keep, div_round);
    contenders!(rescale, keep, rescale);
    contenders!(parse, keep, parse);
    contenders!(format, keep, format);
    contenders!(to_f64, keep, to_f64);
    contenders!(from_f64, keep, from_f64);
    contenders!(mul_exact, keep, mul_exact);
}
