//! Why text is not a decimal, and why a decimal does not convert.

use core::error::Error;
use core::fmt;

#[cfg(feature = "defmt")]
use defmt::Format;

/// Why text is not a decimal: [`kind`](Self::kind) says which way it failed, as std's
/// `ParseIntError` does.
///
/// # Examples
/// ```
/// use ddust::{D64, ParseErrorKind};
///
/// let refused = "0.125".parse::<D64<2>>().expect_err("one digit too many");
/// assert_eq!(refused.kind(), ParseErrorKind::TooManyDecimals, "two decimals, never rounded");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct ParseError {
    /// Which way the text failed.
    kind: ParseErrorKind,
}

impl ParseError {
    /// The refusal of text for `kind`.
    #[inline]
    #[must_use]
    pub(crate) const fn new(kind: ParseErrorKind) -> Self {
        Self { kind }
    }

    /// Which way the text failed.
    #[inline]
    #[must_use]
    pub const fn kind(self) -> ParseErrorKind {
        self.kind
    }
}

/// The refusal of text for `kind`.
impl From<ParseErrorKind> for ParseError {
    #[inline]
    fn from(kind: ParseErrorKind) -> Self {
        Self::new(kind)
    }
}

/// `parse error:` and why.
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: {}", self.kind)
    }
}

impl Error for ParseError {}

/// Which way text failed to be a decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The text is empty.
    Empty,
    /// The text is not a decimal number: a sign, digits with an optional point, and an optional
    /// exponent, as Rust's own numbers are written.
    InvalidDigit,
    /// The number is above the decimal's range.
    PosOverflow,
    /// The number is below the decimal's range.
    NegOverflow,
    /// The number has a non-zero digit past the decimal's scale.
    TooManyDecimals,
    /// The range a number is read at is not within its buffer: the caller's range, not the text.
    RangeOutsideBuffer,
}

/// What failed, in words.
impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "expected a number, found an empty string",
            Self::InvalidDigit => "expected a decimal number, as `-12.34` or `1.5e3`",
            Self::PosOverflow => "the number is above the decimal's range",
            Self::NegOverflow => "the number is below the decimal's range",
            Self::TooManyDecimals => "the number has more fraction digits than the decimal's scale",
            Self::RangeOutsideBuffer => "the range to read is not within the buffer",
        })
    }
}

/// Why a decimal does not convert: [`kind`](Self::kind) says which way.
///
/// # Examples
/// ```
/// use ddust::{ConvertErrorKind, D64, Fixed};
///
/// let refused = D64::<2>::new(1_005, 3, Fixed).expect_err("1.005 has three decimals");
/// assert_eq!(refused.kind(), ConvertErrorKind::TooManyDecimals, "two decimals, never rounded");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct ConvertError {
    /// Which way the value failed to convert.
    kind: ConvertErrorKind,
}

impl ConvertError {
    /// The refusal of a conversion for `kind`.
    #[inline]
    #[must_use]
    pub(crate) const fn new(kind: ConvertErrorKind) -> Self {
        Self { kind }
    }

    /// The refusal of a value of this sign past the target's range.
    #[inline]
    #[must_use]
    pub(crate) const fn overflow(negative: bool) -> Self {
        Self::new(if negative {
            ConvertErrorKind::NegOverflow
        } else {
            ConvertErrorKind::PosOverflow
        })
    }

    /// Which way the value failed to convert.
    #[inline]
    #[must_use]
    pub const fn kind(self) -> ConvertErrorKind {
        self.kind
    }
}

/// The refusal of a conversion for `kind`: for a type built on a decimal, refusing as the decimal
/// does.
impl From<ConvertErrorKind> for ConvertError {
    #[inline]
    fn from(kind: ConvertErrorKind) -> Self {
        Self::new(kind)
    }
}

/// `convert error:` and why.
impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "convert error: {}", self.kind)
    }
}

impl Error for ConvertError {}

/// Which way a value failed to convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[non_exhaustive]
pub enum ConvertErrorKind {
    /// The value is above the target's range.
    PosOverflow,
    /// The value is below the target's range.
    NegOverflow,
    /// The value has a non-zero digit past the target's scale.
    TooManyDecimals,
}

/// What failed, in words.
impl fmt::Display for ConvertErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PosOverflow => "the value is above the target's range",
            Self::NegOverflow => "the value is below the target's range",
            Self::TooManyDecimals => "the value has more fraction digits than the target's scale",
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;

    use rstest::rstest;

    use super::{ConvertError, ConvertErrorKind, ParseError, ParseErrorKind};

    #[rstest]
    #[case::empty(ParseErrorKind::Empty, "parse error: expected a number, found an empty string")]
    #[case::invalid(
        ParseErrorKind::InvalidDigit,
        "parse error: expected a decimal number, as `-12.34` or `1.5e3`"
    )]
    #[case::above(
        ParseErrorKind::PosOverflow,
        "parse error: the number is above the decimal's range"
    )]
    #[case::below(
        ParseErrorKind::NegOverflow,
        "parse error: the number is below the decimal's range"
    )]
    #[case::decimals(
        ParseErrorKind::TooManyDecimals,
        "parse error: the number has more fraction digits than the decimal's scale"
    )]
    #[case::range(
        ParseErrorKind::RangeOutsideBuffer,
        "parse error: the range to read is not within the buffer"
    )]
    fn a_parse_error_says_why(#[case] kind: ParseErrorKind, #[case] message: &str) {
        assert_eq!(ParseError::from(kind).to_string(), message, "{kind:?}");
    }

    #[rstest]
    #[case::above(
        ConvertErrorKind::PosOverflow,
        "convert error: the value is above the target's range"
    )]
    #[case::below(
        ConvertErrorKind::NegOverflow,
        "convert error: the value is below the target's range"
    )]
    #[case::decimals(
        ConvertErrorKind::TooManyDecimals,
        "convert error: the value has more fraction digits than the target's scale"
    )]
    fn a_convert_error_says_why(#[case] kind: ConvertErrorKind, #[case] message: &str) {
        assert_eq!(ConvertError::from(kind).to_string(), message, "{kind:?}");
    }

    #[test]
    fn an_overflow_says_which_end() {
        assert_eq!(ConvertError::overflow(true).kind(), ConvertErrorKind::NegOverflow, "below");
        assert_eq!(ConvertError::overflow(false).kind(), ConvertErrorKind::PosOverflow, "above");
    }
}
