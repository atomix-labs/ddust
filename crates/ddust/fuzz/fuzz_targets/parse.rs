//! What every parser target checks of one decimal type over any bytes.

/// Checks one type over `$data`: `from_ascii` against `FromStr`, the general reader, a read in
/// place, a read at the front and the rounding reads; `write_ascii` and `Display` against each
/// other and read back; and the scale a run-time decimal reads from its spelling.
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
        // The general reader, which reads an exponent, as an oracle for the vector reader and the
        // SWAR one: `e0` changes no number.
        if !data.is_empty() && !data.iter().any(|byte| matches!(byte, b'e' | b'E')) {
            let exponent = [data, b"e0"].concat();
            assert_eq!(<$ty>::from_ascii(&exponent, Fixed), value, "the general reader reads {data:?} alike");
        }
        // Sixteen bytes before the number and twenty-four after, so one of up to 32 bytes is read
        // from the buffer, and digits, points and signs around it, which the read must leave out.
        let (before, after): (&[u8], &[u8]) = (b"9.99999999-99.9-", b"9.99999999999999999-9.99");
        let mut buffer = before.to_vec();
        buffer.extend_from_slice(data);
        buffer.extend_from_slice(after);
        let range = before.len()..before.len() + data.len();
        assert_eq!(<$ty>::from_ascii_at(&buffer, range, Fixed), value, "a range reads as the bytes alone");
        if let Ok(text) = core::str::from_utf8(data)
            && let Ok(spelled) = text.parse::<ddust::Decimal<i128, ddust::Dynamic>>()
        {
            let back: ddust::Decimal<i128, ddust::Dynamic> = format!("{spelled:#}").parse().expect("its own spelling");
            assert_eq!((back.steps(), back.decimals()), (spelled.steps(), spelled.decimals()), "{text:?} keeps its scale");
        }
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
