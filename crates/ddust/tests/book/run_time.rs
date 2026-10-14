//! The listings of the chapter Run-Time Scales.

#[test]
// ANCHOR: spelled
fn a_run_time_scale_is_the_one_its_text_spells() {
    use ddust::{Decimal, Dynamic};

    let price: Decimal<i64, Dynamic> = "1.50".parse().expect("a decimal");
    assert_eq!((price.steps(), price.decimals()), (150, 2));
    assert_eq!(size_of::<Decimal<i64, Dynamic>>(), 16, "the steps, and a byte of scale");
}
// ANCHOR_END: spelled

#[test]
// ANCHOR: lines_up
fn two_run_time_scales_line_up_at_the_finer() {
    use ddust::{Decimal, Dynamic};

    let read = |text: &str| -> Decimal<i64, Dynamic> { text.parse().expect("a decimal") };
    let sum = read("1.5") + read("1.25");
    assert_eq!((sum.to_string(), sum.decimals()), ("2.75".into(), 2));
    assert_eq!(read("1.5"), read("1.50"), "equal, whatever their scales");
}
// ANCHOR_END: lines_up

#[test]
// ANCHOR: product
fn a_run_time_product_takes_the_sum_of_its_factors_decimals() {
    use ddust::{Decimal, Dynamic};

    let read = |text: &str| -> Decimal<i64, Dynamic> { text.parse().expect("a decimal") };
    let product = read("1.5") * read("1.25");
    assert_eq!((product.to_string(), product.decimals()), ("1.875".into(), 3));
}
// ANCHOR_END: product

#[test]
// ANCHOR: venue
fn a_scale_read_from_a_venue_rounds_on_purpose() {
    use ddust::round::Floor;
    use ddust::{ConvertErrorKind, Decimal, Dynamic};

    let size_step = Dynamic::new(3).expect("at most 38 decimals");
    let wanted =
        Decimal::<i64, Dynamic>::from_ascii(b"0.12345", Dynamic::new(5).expect("at most 38"))
            .expect("a decimal");
    let refused = wanted.rescale(size_step).expect_err("two decimals past the venue's");
    assert_eq!(refused.kind(), ConvertErrorKind::TooManyDecimals);
    let traded = wanted.rescale_round(size_step, Floor).expect("in range");
    assert_eq!(traded.to_string(), "0.123", "down to the venue's step");
}
// ANCHOR_END: venue

#[test]
// ANCHOR: static
fn a_static_scale_converts_to_a_run_time_one_and_back() {
    use ddust::{ConvertError, ConvertErrorKind, D64, Decimal, Dynamic, dec};

    let price: D64<2> = dec!(1.25);
    let carried: Decimal<i64, Dynamic> = price.into();
    assert_eq!(carried.decimals(), 2);
    assert_eq!(D64::<2>::try_from(carried), Ok(price));
    assert_eq!(D64::<4>::try_from(carried), Ok(dec!(1.25)), "zeros added");
    let refused = D64::<1>::try_from(carried).map_err(ConvertError::kind);
    assert_eq!(refused, Err(ConvertErrorKind::TooManyDecimals), "a digit past one decimal");
}
// ANCHOR_END: static
