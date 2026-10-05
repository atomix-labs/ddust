//! An operation that can lose digits takes a rounding mode, so it never rounds unasked.

use ddust::{D64, dec};

fn main() {
    let (amount, rate): (D64<2>, D64<4>) = (dec!(100.10), dec!(0.0025));
    let _rounded = amount.mul_round(rate);
}
