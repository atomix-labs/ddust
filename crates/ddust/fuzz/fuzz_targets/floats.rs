//! Any double to a decimal and any decimal to a double, against core's own correctly rounded
//! conversions through text.

#![no_main]

use ddust::round::Rounding;
use ddust::{D64, Decimal, Dynamic, Fixed};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (u64, i64, u8)| {
    let (bits, steps, decimals) = input;
    let decimals = decimals % 19;
    // A decimal to its nearest double, as core reads its text.
    let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
    let text: f64 = format!("{steps}e-{decimals}").parse().expect("a number");
    assert_eq!(value.to_f64().to_bits(), text.to_bits(), "{steps}e-{decimals}");
    // A double to the decimal its exact value truncates to, as its full expansion spells it.
    let x = f64::from_bits(bits);
    if x.is_finite() && x.abs() < 9e11 {
        let expansion = format!("{x:.1100}");
        let (whole, fraction) = expansion.split_once('.').expect("a point");
        let truncated: D64<7> = format!("{whole}.{}", &fraction[..7]).parse().expect("in range");
        assert_eq!(D64::<7>::from_f64(x, Fixed, Rounding::Trunc), Some(truncated), "{x:e}");
    }
});
