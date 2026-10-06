//! Any decimal to a double, against core's correctly rounded reading of its text, and any double to
//! a decimal at any scale by any mode, against the double's exact expansion.

#![no_main]

use ddust::round::Rounding;
use ddust::{Decimal, Dynamic};
use libfuzzer_sys::fuzz_target;

/// Every mode, as a value.
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

/// Whether `mode` moves a truncated quotient one step away from zero, by its definition: `class`
/// is 0 for nothing cut, 1 below half a step, 2 at half, 3 above.
fn moves_away(mode: Rounding, negative: bool, odd: bool, class: u32) -> bool {
    let (exact, half) = (class == 0, class.cmp(&2));
    match mode {
        Rounding::Floor => negative && !exact,
        Rounding::Ceil => !negative && !exact,
        Rounding::Trunc => false,
        Rounding::Expand => !exact,
        Rounding::HalfFloor => !exact && (half.is_gt() || (half.is_eq() && negative)),
        Rounding::HalfCeil => !exact && (half.is_gt() || (half.is_eq() && !negative)),
        Rounding::HalfTrunc => !exact && half.is_gt(),
        Rounding::HalfExpand => !exact && half.is_ge(),
        Rounding::HalfEven => !exact && (half.is_gt() || (half.is_eq() && odd)),
    }
}

fuzz_target!(|input: (u64, i128, u8, u8)| {
    let (bits, steps, decimals, mode) = input;
    let decimals = decimals % 39;
    // A decimal to its nearest double, as core reads its text.
    let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
    let text: f64 = format!("{steps}e-{decimals}").parse().expect("a number");
    assert_eq!(value.to_f64().to_bits(), text.to_bits(), "{steps}e-{decimals}");
    // A double to an `i64` decimal at any scale to 18, by any mode, against its exact expansion:
    // the digits kept truncate, and those cut say how the mode moves the step.
    let x = f64::from_bits(bits);
    let (decimals, mode) = (usize::from(decimals % 19), MODES[usize::from(mode % 9)]);
    if x.is_finite() && x.abs() < 1e20 {
        let expansion = format!("{:.1100}", x.abs());
        let (whole, fraction) = expansion.split_once('.').expect("a point");
        let (kept, cut) = fraction.split_at(decimals);
        let truncated: u128 = format!("{whole}{kept}").parse().expect("at most 38 digits");
        let (first, rest) = cut.split_at(1);
        let class = match (first, rest.bytes().all(|digit| digit == b'0')) {
            ("0", true) => 0,
            ("5", true) => 2,
            (first, _) if first < "5" => 1,
            _ => 3,
        };
        let negative = x.is_sign_negative();
        let magnitude = truncated + u128::from(moves_away(mode, negative, truncated % 2 == 1, class));
        let expected = i128::try_from(magnitude)
            .ok()
            .map(|magnitude| if negative { -magnitude } else { magnitude })
            .and_then(|steps| i64::try_from(steps).ok());
        let scale = Dynamic::new(u8::try_from(decimals).expect("at most 18")).expect("at most 38");
        let got = Decimal::<i64, Dynamic>::from_f64(x, scale, mode).map(Decimal::steps);
        assert_eq!(got, expected, "{x:e} at {decimals} decimals, {mode:?}");
    }
});
