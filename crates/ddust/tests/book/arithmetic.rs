//! The listings of the chapter Arithmetic and Overflow.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the operators are what the chapter shows; overflow checks decide, as for an integer"
)]

#[test]
// ANCHOR: operators
fn values_of_one_scale_add_as_their_steps_do() {
    use ddust::{D64, dec};

    let (bid, ask): (D64<2>, D64<2>) = (dec!(99.98), dec!(100.02));
    assert_eq!(ask - bid, dec!(0.04));
    assert_eq!((bid + ask) / 2, dec!(100), "a mean, by a count");
    assert_eq!(bid * 3, dec!(299.94));
    assert_eq!(dec!(10.00: D64<2>) % dec!(3), dec!(1), "the remainder after a whole quotient");
    assert!(bid < ask);
    let fills: [D64<2>; 3] = [dec!(1.25), dec!(2.50), dec!(0.75)];
    assert_eq!(fills.iter().sum::<D64<2>>(), dec!(4.5));
}
// ANCHOR_END: operators

#[test]
// ANCHOR: division
fn a_quotient_keeps_the_dividends_scale_and_truncates() {
    use ddust::{D64, dec};

    let (budget, price): (D64<2>, D64<2>) = (dec!(100), dec!(3));
    assert_eq!(budget / price, dec!(33.33), "toward zero, as the integer's `/`");
    assert_eq!(-budget / price, dec!(-33.33));
    assert_eq!(dec!(1: D64<2>) / 3, dec!(0.33), "by a count, likewise");
}
// ANCHOR_END: division

#[test]
// ANCHOR: families
fn each_operator_has_the_integers_four_families() {
    use ddust::{D64, dec};

    let (max, cent): (D64<2>, D64<2>) = (D64::MAX, dec!(0.01));
    assert_eq!(max.checked_add(cent), None);
    assert_eq!(max.saturating_add(cent), D64::MAX);
    assert_eq!(max.wrapping_add(cent), D64::MIN);
    assert_eq!(max.overflowing_add(cent), (D64::MIN, true));
    assert_eq!(cent.checked_div(D64::<2>::ZERO), None, "never by zero");
}
// ANCHOR_END: families

#[test]
#[cfg(overflow_checks)]
#[should_panic(expected = "attempt to add with overflow")]
// ANCHOR: overflow
fn past_the_range_an_operator_panics_with_overflow_checks_on() {
    use ddust::{D64, dec};

    let max: D64<2> = D64::MAX;
    let _ = max + dec!(0.01);
}
// ANCHOR_END: overflow

#[test]
// ANCHOR: unsigned
fn an_unsigned_decimal_stops_at_zero() {
    use ddust::{UD64, dec};

    let (balance, withdrawal): (UD64<2>, UD64<2>) = (dec!(10), dec!(12.50));
    assert_eq!(balance.checked_sub(withdrawal), None);
    assert_eq!(balance.saturating_sub(withdrawal), UD64::ZERO);
}
// ANCHOR_END: unsigned
