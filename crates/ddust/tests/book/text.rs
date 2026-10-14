//! The listings of the chapter Text.

#[test]
// ANCHOR: parse
fn text_is_read_exactly_or_refused() {
    use ddust::{D64, ParseError, ParseErrorKind, dec};

    let read = |text: &str| text.parse::<D64<2>>().map_err(ParseError::kind);
    assert_eq!(read("12.30"), Ok(dec!(12.3)));
    assert_eq!(read("1.2300"), read("1.23"), "zeros past the scale are no digits");
    assert_eq!(read("-.5"), read("-0.50"));
    assert_eq!(read("1.5e3"), read("1500"), "an exponent, as Rust's floats are written");
    assert_eq!(read("1.234"), Err(ParseErrorKind::TooManyDecimals), "never rounded unasked");
    assert_eq!(read("1,000"), Err(ParseErrorKind::InvalidDigit));
    assert_eq!(read("NaN"), Err(ParseErrorKind::InvalidDigit), "a decimal has no NaN");
    assert_eq!(read("1e17"), Err(ParseErrorKind::PosOverflow));
    assert_eq!(read(""), Err(ParseErrorKind::Empty));
}
// ANCHOR_END: parse

#[test]
// ANCHOR: round
fn text_with_more_decimals_rounds_by_a_mode_when_asked() {
    use ddust::round::HalfEven;
    use ddust::{D64, Fixed, dec};

    let price = D64::from_ascii_round(b"64250.745", Fixed::<2>, HalfEven).expect("in range");
    assert_eq!(price, dec!(64250.74), "a tie, to the even cent");
}
// ANCHOR_END: round

#[test]
// ANCHOR: message
fn a_number_is_read_in_place_from_a_message() {
    use ddust::{D64, Fixed, dec};

    let message = br#"{"p":"64250.75","q":"0.0153"}"#;
    let price = D64::from_ascii_at(message, 6..14, Fixed::<2>).expect("a price");
    assert_eq!(price, dec!(64250.75));
    let (quantity, length) = D64::from_ascii_prefix(&message[21..], Fixed::<4>).expect("a size");
    assert_eq!((quantity, length), (dec!(0.0153), 6), "the number, and the bytes it took");
}
// ANCHOR_END: message

#[test]
// ANCHOR: display
fn a_value_is_written_as_its_format_asks() {
    use ddust::{D64, dec};

    let price: D64<4> = dec!(1.2500);
    assert_eq!(format!("{price}"), "1.25", "the shortest exact decimal");
    assert_eq!(format!("{price:#}"), "1.2500", "every decimal of the scale");
    assert_eq!(format!("{price:.1}"), "1.2", "a tie, to the even digit");
    assert_eq!(format!("{price:>+8}"), "   +1.25", "width, fill and sign as an integer's");
    assert_eq!(format!("{price:e}"), "1.25e0");
    assert_eq!(format!("{price:?}"), "1.2500");
}
// ANCHOR_END: display

#[test]
// ANCHOR: write
fn a_value_is_written_into_bytes_with_no_formatter() {
    use ddust::{D128, MAX_ASCII_LEN, dec};

    let mut buffer = [0_u8; MAX_ASCII_LEN];
    let value: D128<18> = dec!(-0.000000000000000001);
    let length = value.write_ascii(&mut buffer).expect("room for every value");
    assert_eq!(&buffer[..length], b"-0.000000000000000001");
    assert_eq!(value.write_ascii(&mut [0; 8]), None, "a buffer too short is refused");
}
// ANCHOR_END: write
