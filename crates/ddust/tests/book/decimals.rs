//! The listings of the chapter Decimals and Scales.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the operators are what the chapter shows; overflow checks decide, as for an integer"
)]

#[test]
// ANCHOR: steps
fn a_decimal_is_a_whole_number_of_steps() {
    use ddust::{D64, Fixed, dec};

    let price: D64<2> = dec!(12.34);
    assert_eq!(price.steps(), 1_234, "1,234 steps of 0.01");
    assert_eq!(price.decimals(), 2);
    assert_eq!(D64::from_steps(1_234, Fixed::<2>), price);
    assert_eq!(size_of::<D64<2>>(), size_of::<i64>(), "the scale takes no room");
}
// ANCHOR_END: steps

#[test]
// ANCHOR: literal
fn a_literal_is_checked_when_the_constant_is_evaluated() {
    use ddust::{D64, D128, dec};

    const FEE: D64<4> = dec!(0.0025);
    const RATE: D128<18> = dec!(1.2e-5);
    assert_eq!(FEE.steps(), 25);
    assert_eq!(RATE.to_string(), "0.000012");
    assert_eq!(dec!(-7.5: D64<1>).steps(), -75, "a type named after a colon");
}
// ANCHOR_END: literal

#[test]
// ANCHOR: new
fn digits_and_decimals_make_a_decimal_exactly() {
    use ddust::round::HalfEven;
    use ddust::{ConvertErrorKind, D64, Fixed, dec};

    let half: D64<1> = D64::new(600_005, 1, Fixed).expect("one decimal fits one");
    assert_eq!(half, dec!(60_000.5));
    let refused = D64::new(600_005, 1, Fixed::<0>).expect_err("a decimal past the scale");
    assert_eq!(refused.kind(), ConvertErrorKind::TooManyDecimals);
    assert_eq!(D64::new_round(600_005, 1, Fixed::<0>, HalfEven), Ok(dec!(60_000: D64<0>)));
    assert_eq!(half.to_digits(4), Ok(600_005_000), "back to digits, at four decimals");
}
// ANCHOR_END: new

#[test]
// ANCHOR: widths
fn a_value_moves_to_a_wider_integer_freely_and_back_by_a_check() {
    use ddust::{ConvertErrorKind, D32, D64, dec};

    let small: D32<2> = dec!(19.99);
    let wide: D64<2> = small.into();
    assert_eq!(D32::try_from(wide), Ok(small), "it fits again");
    let large: D64<2> = dec!(30_000_000);
    let refused = D32::<2>::try_from(large).expect_err("past an i32's 21,474,836.47");
    assert_eq!(refused.kind(), ConvertErrorKind::PosOverflow);
}
// ANCHOR_END: widths

#[test]
// ANCHOR: product
fn a_product_takes_the_sum_of_its_factors_scales() {
    use ddust::{D64, dec};

    let (price, quantity): (D64<2>, D64<4>) = (dec!(1.25), dec!(0.0003));
    let notional: D64<6> = (price * quantity).into();
    assert_eq!(notional.steps(), 375, "1.25 × 0.0003 is 0.000375, exactly");
}
// ANCHOR_END: product
