//! The listings of the chapter Serialization.

#![cfg(feature = "serde")]

#[test]
// ANCHOR: text
fn a_decimal_is_its_text_where_a_person_reads_the_format() {
    use ddust::{D64, dec};
    use serde_json::{from_str, to_string};

    let price: D64<2> = dec!(64250.75);
    assert_eq!(to_string(&price).expect("a decimal writes as a JSON string"), r#""64250.75""#);
    let read = |json: &str| from_str::<D64<2>>(json).map_err(|error| error.to_string());
    assert_eq!(read(r#""64250.75""#), Ok(price));
    assert_eq!(read("64250"), Ok(dec!(64250)), "a whole number, exactly");
    let float = r#"invalid type: floating point `64250.75`, expected a decimal as a string, as "60000.5", or a whole number at line 1 column 8"#;
    assert_eq!(
        read("64250.75"),
        Err(float.into()),
        "a float, which has lost the digits, is refused"
    );
    let past = "parse error: the number has more fraction digits than the decimal's scale at line 1 column 11";
    assert_eq!(
        read(r#""64250.755""#),
        Err(past.into()),
        "a digit past the scale, as parsing refuses it"
    );
}
// ANCHOR_END: text

#[test]
// ANCHOR: fields
fn a_field_chooses_its_form() {
    use ddust::{D64, dec};
    use serde::{Deserialize, Serialize};
    use serde_json::{from_str, to_string};

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
    let json = to_string(&fill).expect("each field writes as a JSON value");
    assert_eq!(json, r#"{"price":"64250.75","quantity":153,"fee":0.25,"rebate":null}"#);
    assert_eq!(from_str::<Fill>(&json).expect("each form reads what it writes"), fill);
}
// ANCHOR_END: fields

#[test]
// ANCHOR: dynamic
fn a_run_time_scale_is_read_from_the_text() {
    use ddust::{Decimal, Dynamic};
    use serde_json::{from_str, to_string};

    let price: Decimal<i64, Dynamic> = from_str(r#""1.50""#).expect("1.50 is a decimal");
    assert_eq!((price.steps(), price.decimals()), (150, 2));
    let json = to_string(&price).expect("a decimal writes as a JSON string");
    assert_eq!(json, r#""1.50""#, "its scale kept");
}
// ANCHOR_END: dynamic
