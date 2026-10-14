//! The listings of the chapter Rounding.

#[test]
#[rustfmt::skip]
// ANCHOR: modes
fn each_mode_settles_the_digits_that_do_not_fit() {
    use ddust::round::Rounding;
    use ddust::{D64, dec};

    let values: [D64<2>; 6] = [dec!(-1.5), dec!(-1.25), dec!(-0.5), dec!(0.5), dec!(1.25), dec!(1.5)];
    let whole = |mode| values.map(|value| value.to_int(mode));
    assert_eq!(whole(Rounding::Floor),      [-2, -2, -1, 0, 1, 1]);
    assert_eq!(whole(Rounding::Ceil),       [-1, -1,  0, 1, 2, 2]);
    assert_eq!(whole(Rounding::Trunc),      [-1, -1,  0, 0, 1, 1]);
    assert_eq!(whole(Rounding::Expand),     [-2, -2, -1, 1, 2, 2]);
    assert_eq!(whole(Rounding::HalfFloor),  [-2, -1, -1, 0, 1, 1]);
    assert_eq!(whole(Rounding::HalfCeil),   [-1, -1,  0, 1, 1, 2]);
    assert_eq!(whole(Rounding::HalfTrunc),  [-1, -1,  0, 0, 1, 1]);
    assert_eq!(whole(Rounding::HalfExpand), [-2, -1, -1, 1, 1, 2]);
    assert_eq!(whole(Rounding::HalfEven),   [-2, -1,  0, 0, 1, 2]);
}
// ANCHOR_END: modes

#[test]
// ANCHOR: operations
fn what_can_lose_a_digit_takes_a_mode() {
    use ddust::round::{Ceil, Floor, HalfEven, Trunc};
    use ddust::{D64, dec};

    let (amount, fee): (D64<2>, D64<4>) = (dec!(100.10), dec!(0.0025));
    assert_eq!(amount.mul_round(fee, Ceil), dec!(0.26), "0.25025, up to the cent");
    assert_eq!(amount.div_int_round(3, HalfEven), dec!(33.37), "a third, to the cent");
    assert_eq!(amount.div_round(dec!(0.07: D64<2>), Floor), dec!(1430));
    let share = amount.mul_div_round(dec!(2: D64<0>), dec!(3), HalfEven);
    assert_eq!(share, dec!(66.73), "two thirds, rounded once");
    assert_eq!(dec!(19.97: D64<2>).round_to(dec!(0.05), Floor), dec!(19.95), "to a step");
    assert_eq!(dec!(2.5: D64<1>).round(HalfEven), dec!(2), "to a whole number");
    assert_eq!(dec!(-2.5: D64<1>).to_int(Trunc), -2, "to an integer");
}
// ANCHOR_END: operations

#[test]
// ANCHOR: run_time
fn a_mode_read_at_run_time_is_a_rounding() {
    use ddust::round::{HalfExpand, Rounding};
    use ddust::{D64, Fixed, dec};

    let venue_rounds_half_up = true;
    let mode = if venue_rounds_half_up { Rounding::from(HalfExpand) } else { Rounding::HalfEven };
    let fee: D64<2> = dec!(0.125: D64<3>).rescale_round(Fixed, mode).expect("in range");
    assert_eq!(fee, dec!(0.13));
}
// ANCHOR_END: run_time

#[test]
// ANCHOR: divisor
fn a_divisor_prepared_once_divides_many_values() {
    use ddust::round::HalfEven;
    use ddust::{D64, Divisor, dec};

    let euros_per_dollar: D64<4> = dec!(0.9234);
    let rate = Divisor::new(euros_per_dollar).expect("not zero");
    let euros: [D64<2>; 3] = [dec!(100), dec!(250), dec!(19.99)];
    let dollars = euros.map(|amount| amount.div_round(rate, HalfEven));
    assert_eq!(dollars, [dec!(108.30), dec!(270.74), dec!(21.65)]);
    assert_eq!(dollars[2], euros[2].div_round(euros_per_dollar, HalfEven), "as by the decimal");
}
// ANCHOR_END: divisor
