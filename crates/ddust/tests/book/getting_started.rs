//! The listings of the chapter Getting Started.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the chapter shows the operators, which overflow as an integer's do"
)]

#[test]
// ANCHOR: first
fn a_notional_is_exact_and_rounds_only_when_asked() {
    use ddust::round::HalfEven;
    use ddust::{D64, dec};

    let price: D64<2> = "64250.75".parse().expect("a price to the cent");
    let quantity: D64<4> = dec!(0.0153);

    let notional: D64<6> = (price * quantity).into();
    assert_eq!(notional.to_string(), "983.036475", "exact: two decimals and four make six");

    let billed = price.mul_round(quantity, HalfEven);
    assert_eq!(billed, dec!(983.04), "to the cent, as asked");
}
// ANCHOR_END: first
