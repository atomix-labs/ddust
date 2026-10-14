//! The listings of the chapter Serialization.

#![cfg(feature = "serde")]

#[test]
// ANCHOR: text
fn a_decimal_is_its_text_where_a_person_reads_the_format() {
    use ddust::{D64, dec};

    let price: D64<2> = dec!(64250.75);
    assert_eq!(serde_json::to_string(&price).expect("written"), r#""64250.75""#);
    let read = |json: &str| serde_json::from_str::<D64<2>>(json).map_err(|error| error.to_string());
    assert_eq!(read(r#""64250.75""#), Ok(price));
    assert_eq!(read("64250"), Ok(dec!(64250)), "a whole number, exactly");
    assert!(read("64250.75").is_err(), "a float, which has lost the digits, is refused");
    assert!(read(r#""64250.755""#).is_err(), "a digit past the scale, as parsing refuses it");
}
// ANCHOR_END: text

#[test]
// ANCHOR: fields
fn a_field_chooses_its_form() {
    use ddust::{D64, dec};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Fill {
        price: D64<2>,
        #[serde(with = "ddust::serde::steps")]
        quantity: D64<4>,
        #[serde(with = "ddust::serde::float")]
        fee: D64<4>,
        #[serde(with = "ddust::serde::text::option", default)]
        rebate: Option<D64<4>>,
    }

    let fill =
        Fill { price: dec!(64250.75), quantity: dec!(0.0153), fee: dec!(0.25), rebate: None };
    let json = serde_json::to_string(&fill).expect("written");
    assert_eq!(json, r#"{"price":"64250.75","quantity":153,"fee":0.25,"rebate":null}"#);
    assert_eq!(serde_json::from_str::<Fill>(&json).expect("read back"), fill);
}
// ANCHOR_END: fields

#[test]
// ANCHOR: dynamic
fn a_run_time_scale_is_read_from_the_text() {
    use ddust::{Decimal, Dynamic};

    let price: Decimal<i64, Dynamic> = serde_json::from_str(r#""1.50""#).expect("read");
    assert_eq!((price.steps(), price.decimals()), (150, 2));
    assert_eq!(serde_json::to_string(&price).expect("written"), r#""1.50""#, "its scale kept");
}
// ANCHOR_END: dynamic
