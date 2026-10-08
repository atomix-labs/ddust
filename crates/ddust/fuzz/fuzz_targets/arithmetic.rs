//! Products and quotients of any two 64- and 128-bit decimals, at any scales and in any mode,
//! against an exact reference in arbitrary precision: each mode by its definition. Quotients are
//! taken by the second decimal both as it is and prepared as a `Divisor`.

#![no_main]

use ddust::round::Rounding;
use ddust::{Decimal, Divisor, Dynamic};
use libfuzzer_sys::arbitrary::{self, Arbitrary};
use libfuzzer_sys::fuzz_target;
use num_bigint::BigInt;
use num_integer::Integer;

/// Two values, their scales, the result's, and a mode.
#[derive(Arbitrary, Debug)]
struct Input {
    a: i128,
    b: i128,
    scales: [u8; 3],
    mode: u8,
}

/// Every mode, indexed by the input's byte.
const MODES: [Rounding; 9] = [
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::Trunc,
    Rounding::Expand,
    Rounding::HalfFloor,
    Rounding::HalfCeil,
    Rounding::HalfTrunc,
    Rounding::HalfExpand,
    Rounding::HalfEven,
];

/// `n / d` rounded by `mode`, by its definition.
fn divide(n: &BigInt, d: &BigInt, mode: Rounding) -> BigInt {
    let (q, r) = (n / d, n % d);
    if r == BigInt::ZERO {
        return q;
    }
    let negative = (n.sign() == num_bigint::Sign::Minus) != (d.sign() == num_bigint::Sign::Minus);
    let twice = (&r * 2_u8).magnitude().clone();
    let whole = d.magnitude().clone();
    let away = match mode {
        Rounding::Floor => negative,
        Rounding::Ceil => !negative,
        Rounding::Trunc => false,
        Rounding::Expand => true,
        Rounding::HalfFloor => twice > whole || (twice == whole && negative),
        Rounding::HalfCeil => twice > whole || (twice == whole && !negative),
        Rounding::HalfTrunc => twice > whole,
        Rounding::HalfExpand => twice >= whole,
        Rounding::HalfEven => twice > whole || (twice == whole && q.is_odd()),
    };
    if away { q + if negative { -1 } else { 1 } } else { q }
}

/// `10^k`.
fn pow10(k: u8) -> BigInt {
    BigInt::from(10_u8).pow(u32::from(k))
}

fuzz_target!(|input: Input| {
    let [a, b, to] = input.scales.map(|scale| scale % 39);
    let mode = MODES[usize::from(input.mode) % MODES.len()];
    let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
    let (x, y) = (BigInt::from(input.a), BigInt::from(input.b));
    let fits = |value: BigInt| i128::try_from(value).ok();

    let (left, right) = (Decimal::from_steps(input.a, scale(a)), Decimal::from_steps(input.b, scale(b)));
    let from = a + b;
    let product = if from >= to { divide(&(&x * &y), &pow10(from - to), mode) } else { &x * &y * pow10(to - from) };
    assert_eq!(left.checked_mul_round_to(right, scale(to), mode).map(Decimal::steps), fits(product), "{input:?}");
    if input.b != 0 {
        let (up, down) = ((to + b).saturating_sub(a), a.saturating_sub(to + b));
        let quotient = divide(&(&x * pow10(up)), &(&y * pow10(down)), mode);
        assert_eq!(left.checked_div_round_to(right, scale(to), mode).map(Decimal::steps), fits(quotient), "{input:?}");
        // The same divisor prepared, at the dividend's scale.
        let quotient = divide(&(&x * pow10(b)), &y, mode);
        let prepared = Divisor::new(right).expect("not zero");
        assert_eq!(left.checked_div_round(prepared, mode).map(Decimal::steps), fits(quotient), "{input:?}");
    }
    let finer = a.max(b);
    let sum = &x * pow10(finer - a) + &y * pow10(finer - b);
    assert_eq!(left.checked_add(right).map(Decimal::steps), fits(sum), "{input:?}");

    // The same at 64 bits, from the low halves.
    let (a64, b64) = (input.a as i64, input.b as i64);
    let (x, y) = (BigInt::from(a64), BigInt::from(b64));
    let (left, right) = (Decimal::from_steps(a64, scale(a)), Decimal::from_steps(b64, scale(b)));
    let product = if from >= to { divide(&(&x * &y), &pow10(from - to), mode) } else { &x * &y * pow10(to - from) };
    let fits = |value: BigInt| i64::try_from(value).ok();
    assert_eq!(left.checked_mul_round_to(right, scale(to), mode).map(Decimal::steps), fits(product), "{input:?}");
    if b64 != 0 {
        let quotient = divide(&(&x * pow10(b)), &y, mode);
        let prepared = Divisor::new(right).expect("not zero");
        assert_eq!(left.checked_div_round(prepared, mode).map(Decimal::steps), fits(quotient), "{input:?}");
    }
});
