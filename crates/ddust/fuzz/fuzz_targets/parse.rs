//! What every parser target checks of one decimal type over any bytes.

/// Checks one type over `$data`: `from_ascii` against `FromStr`, a read in place, a read at the
/// front and the rounding reads; and `write_ascii` and `Display` against each other and read back.
#[macro_export]
macro_rules! check {
    ($data:expr, $ty:ty) => {{
        use ddust::round::Rounding;
        use ddust::{Fixed, ParseErrorKind};
        let data: &[u8] = $data;
        let value = <$ty>::from_ascii(data, Fixed);
        if let Ok(text) = core::str::from_utf8(data) {
            assert_eq!(text.parse::<$ty>(), value, "a &str reads as its bytes: {text:?}");
        }
        let mut buffer = b"9.9".to_vec();
        buffer.extend_from_slice(data);
        buffer.extend_from_slice(b"99");
        assert_eq!(<$ty>::from_ascii_at(&buffer, 3..3 + data.len(), Fixed), value, "a range reads as the bytes alone");
        if let Ok(value) = value {
            assert_eq!(<$ty>::from_ascii_prefix(data, Fixed), Ok((value, data.len())), "a whole number reads at the front");
        }
        let round = |mode: Rounding| <$ty>::from_ascii_round(data, Fixed, mode);
        let (floor, ceil, even) = (round(Rounding::Floor), round(Rounding::Ceil), round(Rounding::HalfEven));
        match value {
            Ok(value) => {
                assert_eq!((floor, ceil, even), (Ok(value), Ok(value), Ok(value)), "an exact read needs no mode");
                let mut out = [0; ddust::MAX_ASCII_LEN];
                let len = value.write_ascii(&mut out).expect("MAX_ASCII_LEN holds any value");
                assert_eq!(<$ty>::from_ascii(&out[..len], Fixed), Ok(value), "the spelling of {data:?} reads back");
                assert_eq!(value.to_string().as_bytes(), &out[..len], "Display writes the same");
                assert_eq!(format!("{value:#}").parse::<$ty>(), Ok(value), "so does the alternate form");
                assert_eq!(format!("{value:e}").parse::<$ty>(), Ok(value), "and scientific notation");
            },
            Err(error) if error.kind() == ParseErrorKind::TooManyDecimals => {
                if let (Ok(floor), Ok(ceil)) = (floor, ceil) {
                    let step = <$ty>::from_steps(1, Fixed);
                    assert!(floor < ceil && floor.checked_add(step) == Some(ceil), "a step apart: {data:?}");
                    let even = even.expect("between the two");
                    assert!(even == floor || even == ceil, "the nearest is one of them");
                }
            },
            Err(_) => {},
        }
    }};
}
