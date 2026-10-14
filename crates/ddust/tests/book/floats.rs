//! The listings of the chapter Floats.

#[test]
// ANCHOR: to_f64
fn a_decimal_becomes_its_nearest_double() {
    use ddust::{D64, D128, dec};

    let tenth: D64<1> = dec!(0.1);
    assert_eq!(tenth.to_f64().to_bits(), 0.1_f64.to_bits(), "the double Rust reads 0.1 as");
    let long: D128<18> = dec!(0.123456789012345678);
    assert_eq!(long.to_f64().to_string(), "0.12345678901234568", "correctly rounded");
}
// ANCHOR_END: to_f64

#[test]
// ANCHOR: from_f64
fn a_double_becomes_a_decimal_by_a_mode() {
    use ddust::round::{Ceil, HalfEven};
    use ddust::{D64, Fixed, dec};

    let price = D64::from_f64(0.1, Fixed::<2>, HalfEven);
    assert_eq!(price, Some(dec!(0.10)), "the nearest cent");
    let up = D64::from_f64(0.1, Fixed::<2>, Ceil);
    assert_eq!(up, Some(dec!(0.11)), "0.1000000000000000055…, up to the cent");
    assert_eq!(D64::from_f64(f64::NAN, Fixed::<2>, HalfEven), None);
    assert_eq!(D64::from_f64(1e17, Fixed::<2>, HalfEven), None, "past the range");
}
// ANCHOR_END: from_f64
