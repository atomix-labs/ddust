//! How a decimal crosses to a file, a message or a peer.
//!
//! A decimal serializes as its text where a person reads the format, as JSON, TOML or YAML, and as
//! its steps where none does, as bincode or postcard. A decimal of a static scale writes its
//! shortest exact text; one of a [`Dynamic`] scale writes every decimal of its scale, and its
//! decimals beside its steps, so its scale reads back too. Read back, the text is exact or refused,
//! a whole number is read as one, and a float is refused, since it has lost the digits a decimal
//! keeps. A [`Dynamic`] scale on its own is its decimals.
//!
//! A peer that writes a decimal of a static scale another way names it at the field, with one of
//! the modules here, each with an `option` twin for a field that may be absent: [`text`] for the
//! text in every format, [`steps`] for the steps in every format, and [`float`] for a double,
//! rounded to the scale.
//!
//! # Examples
//! ```
//! use ddust::{D64, dec};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Fill {
//!     price: D64<2>,
//!     #[serde(with = "ddust::serde::steps")]
//!     size: D64<8>,
//!     #[serde(with = "ddust::serde::float")]
//!     fee: D64<4>,
//! }
//!
//! let fill: Fill = serde_json::from_str(r#"{"price":"60000.5","size":25000000,"fee":0.0025}"#)?;
//! assert_eq!(fill.price, dec!(60000.5), "the text, exactly");
//! assert_eq!(fill.size, dec!(0.25), "25,000,000 steps of 10^-8");
//! assert_eq!(fill.fee, dec!(0.0025), "the double, rounded to the scale");
//! assert_eq!(
//!     serde_json::to_string(&fill)?,
//!     r#"{"price":"60000.5","size":25000000,"fee":0.0025}"#,
//!     "and back, as it came"
//! );
//! # Ok::<(), serde_json::Error>(())
//! ```

use core::fmt;
use core::marker::PhantomData;
use core::str::{self, FromStr};

use serde_core::de::{Error, Unexpected, Visitor};
use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Decimal, Dynamic, Fixed, Int, MAX_ASCII_LEN, ParseError, Scale, StaticScale};

/// Reads `n` as the text of a whole number, which `T` then reads exactly.
fn whole<T: FromStr<Err = ParseError>, I: Int, E: Error>(n: I) -> Result<T, E> {
    let mut out = [0; MAX_ASCII_LEN];
    let len = Decimal::from_steps(n, Fixed::<0>).write_ascii(&mut out).unwrap_or_default();
    str::from_utf8(out.get(..len).unwrap_or_default())
        .unwrap_or_default()
        .parse()
        .map_err(E::custom)
}

/// Reads a decimal from its text, or from a whole number; a float is refused, since it has lost
/// the digits a decimal keeps.
struct Text<T>(PhantomData<T>);

impl<T: FromStr<Err = ParseError>> Visitor<'_> for Text<T> {
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a decimal as a string, as \"60000.5\", or a whole number")
    }

    fn visit_str<E: Error>(self, text: &str) -> Result<T, E> {
        text.parse().map_err(E::custom)
    }

    fn visit_bytes<E: Error>(self, text: &[u8]) -> Result<T, E> {
        let text = str::from_utf8(text)
            .map_err(|_not_utf8| E::invalid_value(Unexpected::Bytes(text), &self))?;
        self.visit_str(text)
    }

    fn visit_i64<E: Error>(self, n: i64) -> Result<T, E> {
        whole(n)
    }

    fn visit_u64<E: Error>(self, n: u64) -> Result<T, E> {
        whole(n)
    }

    fn visit_i128<E: Error>(self, n: i128) -> Result<T, E> {
        whole(n)
    }

    fn visit_u128<E: Error>(self, n: u128) -> Result<T, E> {
        whole(n)
    }
}

/// Reads a decimal's text where a person reads the format, and whatever `compact` reads where none
/// does, since a format that does not describe itself reads only the type it is asked for.
fn read<'de, T: FromStr<Err = ParseError>, D: Deserializer<'de>>(
    deserializer: D, compact: impl FnOnce(D) -> Result<T, D::Error>,
) -> Result<T, D::Error> {
    if deserializer.is_human_readable() {
        deserializer.deserialize_any(Text(PhantomData))
    } else {
        compact(deserializer)
    }
}

/// The text where a person reads the format, the steps where none does.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// let price: D64<2> = dec!(60000.5);
/// assert_eq!(serde_json::to_string(&price)?, r#""60000.5""#, "the text, for a person");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl<I: Int + Serialize, S: StaticScale> Serialize for Decimal<I, S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            self.steps().serialize(serializer)
        }
    }
}

/// The text or a whole number where a person reads the format, the steps where none does; a float
/// is refused.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// let price: D64<2> = serde_json::from_str(r#""60000.5""#)?;
/// assert_eq!(price, dec!(60000.5), "the text, exactly");
/// assert_eq!(serde_json::from_str::<D64<2>>("60000")?, dec!(60000), "a whole number");
/// assert!(serde_json::from_str::<D64<2>>("60000.5").is_err(), "a float has lost digits");
/// assert!(serde_json::from_str::<D64<2>>(r#""0.125""#).is_err(), "past the scale");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl<'de, I: Int + Deserialize<'de>, S: StaticScale> Deserialize<'de> for Decimal<I, S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read(deserializer, |compact| {
            I::deserialize(compact).map(|steps| Self::from_steps(steps, S::INSTANCE))
        })
    }
}

/// The text where a person reads the format, and the steps and decimals where none does.
///
/// # Examples
/// ```
/// use ddust::{Decimal, Dynamic};
///
/// let price = Decimal::from_steps(6_000_050_i64, Dynamic::new(2).expect("at most 38"));
/// assert_eq!(serde_json::to_string(&price)?, r#""60000.50""#, "its scale in its text");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl<I: Int + Serialize> Serialize for Decimal<I, Dynamic> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(&format_args!("{self:#}"))
        } else {
            (self.steps(), self.decimals()).serialize(serializer)
        }
    }
}

/// The text or a whole number where a person reads the format, at the scale it spells; the steps
/// and decimals where none does, refusing more than 38 decimals.
///
/// # Examples
/// ```
/// use ddust::{Decimal, Dynamic, Scale};
///
/// let price: Decimal<i64, Dynamic> = serde_json::from_str(r#""60000.50""#)?;
/// assert_eq!(price.decimals(), 2, "the scale its text spells");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl<'de, I: Int + Deserialize<'de>> Deserialize<'de> for Decimal<I, Dynamic> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        read(deserializer, |compact| {
            let (steps, decimals) = <(I, u8)>::deserialize(compact)?;
            Ok(Self::from_steps(steps, scale(decimals)?))
        })
    }
}

/// The scale of `decimals`, or an error past 38.
fn scale<E: Error>(decimals: u8) -> Result<Dynamic, E> {
    Dynamic::new(decimals).ok_or_else(|| {
        E::invalid_value(Unexpected::Unsigned(u64::from(decimals)), &"at most 38 decimals")
    })
}

/// Its decimals.
///
/// # Examples
/// ```
/// use ddust::Dynamic;
///
/// let precision = Dynamic::new(4).expect("at most 38");
/// assert_eq!(serde_json::to_string(&precision)?, "4", "four decimals");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl Serialize for Dynamic {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        serializer.serialize_u8(self.decimals())
    }
}

/// Its decimals, refusing more than 38.
///
/// # Examples
/// ```
/// use ddust::{Dynamic, Scale};
///
/// assert_eq!(serde_json::from_str::<Dynamic>("4")?.decimals(), 4, "four decimals");
/// assert!(serde_json::from_str::<Dynamic>("39").is_err(), "past any integer's finest step");
/// # Ok::<(), serde_json::Error>(())
/// ```
impl<'de> Deserialize<'de> for Dynamic {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        scale(u8::deserialize(deserializer)?)
    }
}

/// A `#[serde(with = …)]` module's `option` twin, through a newtype that writes and reads as the
/// module does: `write` and `read` are the bounds on the integer each needs.
macro_rules! option_module {
    (write [$($write:tt)*], read [$($read:tt)*]) => {
        /// The same, for a field that may be absent.
        pub mod option {
            use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

            use super::{deserialize as read, serialize as write};
            use crate::{Decimal, Int, StaticScale};

            /// A decimal, in the shape `Option` writes and reads.
            struct Field<I, S>(Decimal<I, S>);

            impl<I: Int $($write)*, S: StaticScale> Serialize for Field<I, S> {
                fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
                    write(&self.0, serializer)
                }
            }

            impl<'de, I: Int $($read)*, S: StaticScale> Deserialize<'de> for Field<I, S> {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    read(deserializer).map(Self)
                }
            }

            /// Writes the decimal, or nothing.
            ///
            /// # Errors
            /// Whatever the serializer reports.
            pub fn serialize<I: Int $($write)*, S: StaticScale, Ser: Serializer>(
                value: &Option<Decimal<I, S>>, serializer: Ser,
            ) -> Result<Ser::Ok, Ser::Error> {
                value.map(Field).serialize(serializer)
            }

            /// Reads the decimal, or nothing.
            ///
            /// # Errors
            /// As the module's own `deserialize`.
            pub fn deserialize<'de, I: Int $($read)*, S: StaticScale, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Option<Decimal<I, S>>, D::Error> {
                Ok(Option::<Field<I, S>>::deserialize(deserializer)?.map(|field| field.0))
            }
        }
    };
}

/// A decimal of a static scale as its text in every format, read exactly.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Quote {
///     #[serde(with = "ddust::serde::text")]
///     bid: D64<2>,
///     #[serde(with = "ddust::serde::text::option")]
///     ask: Option<D64<2>>,
/// }
///
/// let quote: Quote = serde_json::from_str(r#"{"bid":"60000.5","ask":null}"#)?;
/// assert_eq!((quote.bid, quote.ask), (dec!(60000.5), None), "the text, or nothing");
/// # Ok::<(), serde_json::Error>(())
/// ```
pub mod text {
    use core::marker::PhantomData;

    use serde_core::{Deserializer, Serializer};

    use super::Text;
    use crate::{Decimal, Int, StaticScale};

    /// Writes the text.
    ///
    /// # Errors
    /// Whatever the serializer reports.
    pub fn serialize<I: Int, S: StaticScale, Ser: Serializer>(
        value: &Decimal<I, S>, serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        serializer.collect_str(value)
    }

    /// Reads the text, or a whole number where a person reads the format.
    ///
    /// # Errors
    /// Whatever the deserializer reports, or text that is not a decimal of the type exactly.
    pub fn deserialize<'de, I: Int, S: StaticScale, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Decimal<I, S>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(Text(PhantomData))
        } else {
            deserializer.deserialize_str(Text(PhantomData))
        }
    }

    option_module!(write [], read []);
}

/// A decimal of a static scale as its steps in every format: a number, read as a number or as a
/// string of its digits where a person reads the format.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Trade {
///     #[serde(with = "ddust::serde::steps")]
///     price: D64<8>,
/// }
///
/// let trade: Trade = serde_json::from_str(r#"{"price":"6000050000000"}"#)?;
/// assert_eq!(trade.price, dec!(60000.5), "6,000,050,000,000 steps of 10^-8");
/// assert_eq!(serde_json::to_string(&trade)?, r#"{"price":6000050000000}"#, "written as a number");
/// # Ok::<(), serde_json::Error>(())
/// ```
pub mod steps {
    use core::marker::PhantomData;

    use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

    use super::Text;
    use crate::{Decimal, Fixed, Int, StaticScale};

    /// Writes the steps.
    ///
    /// # Errors
    /// Whatever the serializer reports.
    pub fn serialize<I: Int + Serialize, S: StaticScale, Ser: Serializer>(
        value: &Decimal<I, S>, serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        value.steps().serialize(serializer)
    }

    /// Reads the steps, as a number, or as a string of digits where a person reads the format.
    ///
    /// # Errors
    /// Whatever the deserializer reports, or steps past what the integer holds.
    pub fn deserialize<'de, I: Int + Deserialize<'de>, S: StaticScale, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Decimal<I, S>, D::Error> {
        let steps = if deserializer.is_human_readable() {
            deserializer.deserialize_any(Text::<Decimal<I, Fixed<0>>>(PhantomData))?.steps()
        } else {
            I::deserialize(deserializer)?
        };
        Ok(Decimal::from_steps(steps, S::INSTANCE))
    }

    option_module!(write [+ Serialize], read [+ Deserialize<'de>]);
}

/// A decimal of a static scale as a double in every format: written as its nearest double, and read
/// from one at the nearest step, or from a whole number exactly.
///
/// A double is rarely exact at a decimal scale, and its nearest step, a tie to the even one, is the
/// decimal it was written from: 0.1 is 0.1000000000000000055511151231257827 as a double, and 0.10
/// at two decimals.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Ticker {
///     #[serde(with = "ddust::serde::float")]
///     last: D64<2>,
/// }
///
/// let ticker: Ticker = serde_json::from_str(r#"{"last":0.1}"#)?;
/// assert_eq!(ticker.last, dec!(0.1), "the nearest cent");
/// assert_eq!(serde_json::to_string(&ticker)?, r#"{"last":0.1}"#, "written as its nearest double");
/// # Ok::<(), serde_json::Error>(())
/// ```
pub mod float {
    use core::fmt;
    use core::marker::PhantomData;

    use serde_core::de::{Error, Unexpected, Visitor};
    use serde_core::{Deserializer, Serializer};

    use super::whole;
    use crate::round::HalfEven;
    use crate::{Decimal, Int, StaticScale};

    /// Reads a double at the nearest step, or a whole number exactly.
    struct Float<I, S>(PhantomData<(I, S)>);

    impl<I: Int, S: StaticScale> Visitor<'_> for Float<I, S> {
        type Value = Decimal<I, S>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a number")
        }

        fn visit_f64<E: Error>(self, x: f64) -> Result<Self::Value, E> {
            Decimal::from_f64(x, S::INSTANCE, HalfEven).ok_or_else(|| {
                E::invalid_value(Unexpected::Float(x), &"a finite number the decimal holds")
            })
        }

        fn visit_i64<E: Error>(self, n: i64) -> Result<Self::Value, E> {
            whole(n)
        }

        fn visit_u64<E: Error>(self, n: u64) -> Result<Self::Value, E> {
            whole(n)
        }
    }

    /// Writes the nearest double.
    ///
    /// # Errors
    /// Whatever the serializer reports.
    pub fn serialize<I: Int, S: StaticScale, Ser: Serializer>(
        value: &Decimal<I, S>, serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        serializer.serialize_f64(value.to_f64())
    }

    /// Reads a double at the nearest step, or a whole number exactly.
    ///
    /// # Errors
    /// Whatever the deserializer reports, or a number past what the decimal holds.
    pub fn deserialize<'de, I: Int, S: StaticScale, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Decimal<I, S>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(Float(PhantomData))
        } else {
            deserializer.deserialize_f64(Float(PhantomData))
        }
    }

    option_module!(write [], read []);
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::{String, ToString as _};

    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_test::{Configure, Token, assert_de_tokens, assert_de_tokens_error, assert_tokens};

    use crate::{D64, D128, Decimal, Dynamic, Fixed, UD128, dec};

    /// Every form a static scale's decimal takes, one field each.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Forms {
        default: D64<2>,
        #[serde(with = "super::text")]
        text: D64<2>,
        #[serde(with = "super::steps")]
        steps: D64<2>,
        #[serde(with = "super::float")]
        float: D64<2>,
        #[serde(with = "super::text::option")]
        text_option: Option<D64<2>>,
        #[serde(with = "super::steps::option")]
        steps_option: Option<D64<2>>,
        #[serde(with = "super::float::option")]
        float_option: Option<D64<2>>,
    }

    /// Each field 12.5, but the options' last, which is absent.
    const FORMS: Forms = Forms {
        default: dec!(12.5),
        text: dec!(12.5),
        steps: dec!(12.5),
        float: dec!(12.5),
        text_option: Some(dec!(12.5)),
        steps_option: Some(dec!(12.5)),
        float_option: None,
    };

    /// `FORMS`'s tokens, from the struct's name to its end, with `default` written as `first`.
    fn forms(first: Token) -> [Token; 18] {
        [
            Token::Struct { name: "Forms", len: 7 },
            Token::Str("default"),
            first,
            Token::Str("text"),
            Token::Str("12.5"),
            Token::Str("steps"),
            Token::I64(1_250),
            Token::Str("float"),
            Token::F64(12.5),
            Token::Str("text_option"),
            Token::Some,
            Token::Str("12.5"),
            Token::Str("steps_option"),
            Token::Some,
            Token::I64(1_250),
            Token::Str("float_option"),
            Token::None,
            Token::StructEnd,
        ]
    }

    #[test]
    fn each_form_writes_and_reads_as_its_module_says() {
        assert_tokens(&FORMS.readable(), &forms(Token::Str("12.5")));
        assert_tokens(&FORMS.compact(), &forms(Token::I64(1_250)));
    }

    #[test]
    fn a_run_time_scale_writes_every_decimal_and_its_scale() {
        let price: Decimal<i64, Dynamic> = "60000.50".parse().expect("a decimal");
        assert_tokens(&price.readable(), &[Token::Str("60000.50")]);
        assert_tokens(
            &price.compact(),
            &[Token::Tuple { len: 2 }, Token::I64(6_000_050), Token::U8(2), Token::TupleEnd],
        );
        assert_tokens(&Dynamic::new(38).expect("at most 38"), &[Token::U8(38)]);
    }

    #[test]
    fn a_whole_number_reads_at_the_scale() {
        assert_de_tokens(&D64::<2>::from_steps(-300, Fixed).readable(), &[Token::I64(-3)]);
        assert_de_tokens(&D64::<2>::from_steps(300, Fixed).readable(), &[Token::U64(3)]);
        let widest = D128::<2>::from_steps(i128::MAX / 100 * 100, Fixed);
        assert_de_tokens(
            &widest.readable(),
            &[Token::Str("1701411834604692317316873037158841057")],
        );
        let spelled: Decimal<i64, Dynamic> = "7".parse().expect("a decimal");
        assert_de_tokens(&spelled.readable(), &[Token::U64(7)]);
    }

    #[test]
    fn what_a_decimal_does_not_hold_is_refused() {
        let readable = "a decimal as a string, as \"60000.5\", or a whole number";
        assert_de_tokens_error::<serde_test::Readable<D64<2>>>(
            &[Token::F64(12.5)],
            &format!("invalid type: floating point `12.5`, expected {readable}"),
        );
        assert_de_tokens_error::<serde_test::Readable<D64<2>>>(
            &[Token::Str("0.125")],
            "parse error: the number has more fraction digits than the decimal's scale",
        );
        assert_de_tokens_error::<serde_test::Readable<D64<18>>>(
            &[Token::U64(10)],
            "parse error: the number is above the decimal's range",
        );
        assert_de_tokens_error::<serde_test::Compact<Decimal<i64, Dynamic>>>(
            &[Token::Tuple { len: 2 }, Token::I64(1), Token::U8(39), Token::TupleEnd],
            "invalid value: integer `39`, expected at most 38 decimals",
        );
        assert_de_tokens_error::<Dynamic>(
            &[Token::U8(39)],
            "invalid value: integer `39`, expected at most 38 decimals",
        );
    }

    #[test]
    fn a_float_reads_at_the_nearest_step_or_is_refused() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Last(#[serde(with = "super::float")] D64<2>);
        let last: Last = serde_json::from_str("0.125").expect("a number");
        assert_eq!(last, Last(dec!(0.12)), "a tie, to the even cent");
        let past = serde_json::from_str::<Last>("1e300").map_err(|error| error.to_string());
        let refused = "invalid value: floating point `1e+300`, expected a finite number the decimal holds at line 1 column 5";
        assert_eq!(past, Err(String::from(refused)), "past what an i64 of cents holds");
    }

    #[test]
    fn steps_read_from_a_string_of_digits() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Steps(#[serde(with = "super::steps")] UD128<0>);
        let steps: Steps = serde_json::from_str(&format!("\"{}\"", u128::MAX)).expect("digits");
        assert_eq!(steps.0.steps(), u128::MAX, "past what a JSON number holds");
    }

    proptest! {
        #[test]
        fn json_reads_back_what_it_wrote(steps: i64, decimals in 0_u8..=18) {
            let scale = Dynamic::new(decimals).expect("at most 18");
            let value = Decimal::from_steps(steps, scale);
            let read: Decimal<i64, Dynamic> = serde_json::from_str(&serde_json::to_string(&value).expect("writes")).expect("reads");
            prop_assert_eq!((read.steps(), read.decimals()), (steps, decimals));
            let fixed = D64::<7>::from_steps(steps, Fixed);
            let read: D64<7> = serde_json::from_str(&serde_json::to_string(&fixed).expect("writes")).expect("reads");
            prop_assert_eq!(read, fixed);
        }
    }
}
