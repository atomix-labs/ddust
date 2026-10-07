//! Reading a decimal: what `f64::from_str` reads, without its infinities and NaN, straight to the
//! steps, exactly, or rounded by a mode when asked.

use core::ops::Range;
use core::str::FromStr;

use super::simd::read_plain;
use super::swar::{DOTS, HIGHS, ONES, ZEROS, all_digits, eight_digits};
use crate::decimal::Decimal;
use crate::errors::{ParseError, ParseErrorKind};
use crate::int::Int;
use crate::round::RoundingMode;
use crate::scale::{Dynamic, MAX_DECIMALS, Scale, StaticScale};
use crate::word::{pow10_u128, rounds_up_by_class};

/// Where a number's parts are in its text: the integer digits, the fraction's, and the exponent.
struct Parts<'a> {
    /// The digits before the point.
    integer: &'a [u8],
    /// The digits after it.
    fraction: &'a [u8],
    /// The power of ten the exponent says, saturating.
    exponent: i64,
}

/// The parts of `digits` (no sign): `[digits][.digits][(e|E)[+|-]digits]` with a digit somewhere
/// before the exponent, or `None` for any other text.
fn parts(digits: &[u8]) -> Option<Parts<'_>> {
    let run = |from: usize| {
        digits.get(from..).map_or(0, |rest| rest.iter().take_while(|b| b.is_ascii_digit()).count())
    };
    let integer_len = run(0);
    let mut position = integer_len;
    let mut fraction_len = 0;
    if digits.get(position) == Some(&b'.') {
        fraction_len = run(position.checked_add(1)?);
        position = position.checked_add(1)?.checked_add(fraction_len)?;
    }
    if integer_len == 0 && fraction_len == 0 {
        return None;
    }
    let integer = digits.get(..integer_len)?;
    let fraction = digits
        .get(integer_len.checked_add(1)?..integer_len.checked_add(1)?.checked_add(fraction_len)?)
        .unwrap_or_default();
    let mut exponent = 0_i64;
    if let Some(b'e' | b'E') = digits.get(position) {
        let rest = digits.get(position.checked_add(1)?..)?;
        let (negative, rest) = match rest {
            [b'-', rest @ ..] => (true, rest),
            [b'+', rest @ ..] | rest => (false, rest),
        };
        if rest.is_empty() || !rest.iter().all(u8::is_ascii_digit) {
            return None;
        }
        for &byte in rest {
            exponent =
                exponent.saturating_mul(10).saturating_add(i64::from(byte.wrapping_sub(b'0')));
        }
        if negative {
            exponent = exponent.saturating_neg();
        }
        position = digits.len();
    }
    (position == digits.len()).then_some(Parts { integer, fraction, exponent })
}

/// Reads `digits` (no sign) at `decimals`: exactly, or, given a table, rounded by it for a
/// number of sign `negative`. Each digit's place at `decimals` is known from the exponent
/// before it is read, so only the digits worth a step or more are accumulated; of those below a
/// step, the first and whether any after it is non-zero settle the rounding. Only the steps
/// outgrowing a `u128` overflow, however long the text.
fn read_general(
    digits: &[u8], decimals: u8, round: Option<(bool, u16)>,
) -> Result<u128, ParseErrorKind> {
    let Parts { integer, fraction, exponent } =
        parts(digits).ok_or(ParseErrorKind::InvalidDigit)?;
    // The place of the first digit, as a power of ten at `decimals`: 0 is one step.
    let first = i64::try_from(integer.len())
        .unwrap_or(i64::MAX)
        .saturating_sub(1)
        .saturating_add(exponent)
        .saturating_add(i64::from(decimals));
    let (mut value, mut place) = (0_u128, first);
    let (mut below, mut sticky) = (0_u8, false);
    for &byte in integer.iter().chain(fraction) {
        let digit = byte.wrapping_sub(b'0');
        match place {
            0.. => {
                value = value
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(u128::from(digit)))
                    .ok_or(ParseErrorKind::PosOverflow)?;
            },
            -1 => below = digit,
            _ => sticky |= digit != 0,
        }
        place = place.saturating_sub(1);
    }
    // The digits ran out above one step: the places left are zeros, which zero keeps at any place.
    if place >= 0 && value != 0 {
        let power = u32::try_from(place.saturating_add(1))
            .map_err(|_too_far| ParseErrorKind::PosOverflow)?;
        value = 10_u128
            .checked_pow(power)
            .and_then(|power| value.checked_mul(power))
            .ok_or(ParseErrorKind::PosOverflow)?;
    }
    if below == 0 && !sticky {
        return Ok(value);
    }
    let Some((negative, table)) = round else {
        return Err(ParseErrorKind::TooManyDecimals);
    };
    let class = match below {
        0..=4 => 1,
        5 if !sticky => 2,
        _ => 3,
    };
    let away = rounds_up_by_class(class, value & 1 == 1, negative, table);
    value.checked_add(u128::from(away)).ok_or(ParseErrorKind::PosOverflow)
}

/// Reads `text` at `decimals`: its sign and its magnitude at those decimals, exactly,
/// or, given a table, rounded by it.
///
/// The plain shape is read inline, where a static scale's decimals are a constant and its
/// groups fold; any other text, the general reader's, out of line.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: out of line, a static scale's decimals are no constant to the reader"
)]
fn read_or_round(
    text: &[u8], decimals: u8, table: Option<u16>,
) -> Result<(bool, u128), ParseErrorKind> {
    let (negative, digits) = match text {
        [] => return Err(ParseErrorKind::Empty),
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] => (false, rest),
        _ => (false, text),
    };
    read_plain(digits, decimals).map_or_else(
        || read_signed(negative, digits, decimals, table),
        |magnitude| Ok((negative, magnitude)),
    )
}

/// Reads the `digits` of a number of sign `negative` at `decimals` by the general reader,
/// whatever their shape: out of line, off the plain shape's path.
#[inline(never)]
fn read_signed(
    negative: bool, digits: &[u8], decimals: u8, table: Option<u16>,
) -> Result<(bool, u128), ParseErrorKind> {
    let round = table.map(|table| (negative, table));
    match read_general(digits, decimals, round) {
        Err(ParseErrorKind::PosOverflow) if negative => Err(ParseErrorKind::NegOverflow),
        magnitude => magnitude.map(|magnitude| (negative, magnitude)),
    }
}

/// Reads `text` at `decimals`: its sign and its magnitude, exactly.
#[inline]
pub(crate) fn read(text: &[u8], decimals: u8) -> Result<(bool, u128), ParseErrorKind> {
    read_or_round(text, decimals, None)
}

/// Bytes `0..n` set, for `n` in `0..=9`.
const LOW: [u64; 10] = [
    0,
    0xFF,
    0xFFFF,
    0x00FF_FFFF,
    0xFFFF_FFFF,
    0x00FF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF,
    0x00FF_FFFF_FFFF_FFFF,
    u64::MAX,
    u64::MAX,
];

/// The eight bytes of `window` from `position`, little-endian.
#[inline]
fn word(window: &[u8; 32], position: usize) -> Option<u64> {
    let bytes: [u8; 8] = window.get(position..position.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

/// The path with no loop over the digits: a number of at most eight integer and eight
/// fraction digits whose magnitude fits a `u64`, read eight bytes at a time from the 32 bytes
/// around it, or `None` for anything else, which the byte loop then reads.
#[inline]
fn read_window(
    buffer: &[u8], start: usize, len: usize, decimals: u8,
) -> Option<Result<(bool, u128), ParseErrorKind>> {
    let window: &[u8; 32] =
        buffer.get(start.checked_sub(8)?..start.checked_add(24)?)?.try_into().ok()?;
    if len == 0 || len > 17 {
        return None;
    }
    let first = word(window, 8)?;
    let negative = first & 0xFF == u64::from(b'-');
    let sign = usize::from(negative);
    let probe = first ^ DOTS;
    let dots = probe.wrapping_sub(ONES) & !probe & HIGHS & *LOW.get(len.min(8))?;
    let dot = if dots != 0 {
        usize::try_from(dots.trailing_zeros().wrapping_shr(3)).ok()?
    } else if len > 8 && window.get(16) == Some(&b'.') {
        8
    } else {
        len
    };
    let fraction_len = len.saturating_sub(dot.checked_add(1)?);
    if dot > 8 || fraction_len > 8 || dot == sign || (dot < len && fraction_len == 0) {
        return None;
    }
    let fill = *LOW.get(8_usize.checked_sub(dot)?.checked_add(sign)?)?;
    let integer_word = (word(window, dot)? & !fill) | (ZEROS & fill);
    let keep = *LOW.get(fraction_len)?;
    let fraction_word = (word(window, dot.checked_add(9)?)? & keep) | (ZEROS & !keep);
    if !(all_digits(integer_word) && all_digits(fraction_word)) {
        return None;
    }
    let (integer, fraction) = (eight_digits(integer_word), eight_digits(fraction_word));
    let pow = |k: u8| pow10_u128(k).and_then(|power| u64::try_from(power).ok());
    let magnitude = if decimals >= 8 {
        let scaled = fraction.checked_mul(pow(decimals.saturating_sub(8))?)?;
        integer.checked_mul(pow(decimals)?).and_then(|integer| integer.checked_add(scaled))
    } else {
        let drop = pow(8_u8.saturating_sub(decimals))?;
        if fraction.checked_rem(drop)? != 0 {
            return Some(Err(ParseErrorKind::TooManyDecimals));
        }
        integer
            .checked_mul(pow(decimals)?)
            .and_then(|integer| integer.checked_add(fraction.checked_div(drop)?))
    };
    magnitude.map(|magnitude| Ok((negative, u128::from(magnitude))))
}

/// Reads the number at `range` of `buffer` at `decimals`: with no loop over the digits
/// when the 32 bytes around it are in the buffer, by the byte loop otherwise.
#[inline]
pub(crate) fn read_at(
    buffer: &[u8], range: Range<usize>, decimals: u8,
) -> Result<(bool, u128), ParseErrorKind> {
    let len = range.end.saturating_sub(range.start);
    match read_window(buffer, range.start, len, decimals) {
        Some(read) => read,
        None => read(buffer.get(range).ok_or(ParseErrorKind::RangeOutsideBuffer)?, decimals),
    }
}

/// How many bytes at the front of `bytes` are a number: a sign, digits with an optional
/// point, and an exponent when its letter is followed by digits. Zero when there are no digits.
#[expect(clippy::arithmetic_side_effects, reason = "offsets within the slice")]
fn number_len(bytes: &[u8]) -> usize {
    let digits_from = |position: usize| {
        bytes
            .get(position..)
            .map_or(0, |rest| rest.iter().take_while(|b| b.is_ascii_digit()).count())
    };
    let mut position = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    let integer = digits_from(position);
    position += integer;
    let mut fraction = 0;
    if bytes.get(position) == Some(&b'.') {
        fraction = digits_from(position + 1);
        if integer + fraction > 0 {
            position += 1 + fraction;
        }
    }
    if integer + fraction == 0 {
        return 0;
    }
    if let Some(b'e' | b'E') = bytes.get(position) {
        let signed = usize::from(matches!(bytes.get(position + 1), Some(b'-' | b'+')));
        let exponent = digits_from(position + 1 + signed);
        if exponent > 0 {
            position += 1 + signed + exponent;
        }
    }
    position
}

/// The scale `text` spells: its fraction's digits less its exponent, at least none, and
/// at most 38 when the digits past 38 are zeros.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "counts within the text, the exponent saturating"
)]
fn spelled_scale(text: &[u8]) -> Result<Dynamic, ParseErrorKind> {
    let len = number_len(text);
    if text.is_empty() {
        return Err(ParseErrorKind::Empty);
    }
    if len != text.len() {
        return Err(ParseErrorKind::InvalidDigit);
    }
    let body = match text.first() {
        Some(b'-' | b'+') => text.get(1..).unwrap_or_default(),
        _ => text,
    };
    let (mantissa, exponent) = body.iter().position(|&byte| byte == b'e' || byte == b'E').map_or(
        (body, &[][..]),
        |position| {
            (body.get(..position).unwrap_or_default(), body.get(position + 1..).unwrap_or_default())
        },
    );
    let fraction = mantissa
        .iter()
        .position(|&byte| byte == b'.')
        .map_or(&[][..], |position| mantissa.get(position + 1..).unwrap_or_default());
    let exponent = match exponent {
        [b'-', digits @ ..] => -digits
            .iter()
            .fold(0_i64, |e, &d| e.saturating_mul(10).saturating_add(i64::from(d - b'0'))),
        [b'+', digits @ ..] | digits => digits
            .iter()
            .fold(0_i64, |e, &d| e.saturating_mul(10).saturating_add(i64::from(d - b'0'))),
    };
    let written = i64::try_from(fraction.len()).unwrap_or(i64::MAX).saturating_sub(exponent);
    // The decimals the value needs: those written, less every trailing zero of its digits, the
    // integer's as well as the fraction's; a value of zeros needs none.
    let significant = mantissa.iter().any(|&byte| byte.is_ascii_digit() && byte != b'0');
    let zeros = mantissa
        .iter()
        .rev()
        .filter(|&&byte| byte != b'.')
        .take_while(|&&byte| byte == b'0')
        .count();
    let needed = if significant {
        written.saturating_sub(i64::try_from(zeros).unwrap_or(i64::MAX))
    } else {
        0
    };
    let decimals = written.clamp(0, i64::from(MAX_DECIMALS));
    if needed > i64::from(MAX_DECIMALS) {
        return Err(ParseErrorKind::TooManyDecimals);
    }
    Dynamic::new(u8::try_from(decimals).unwrap_or(MAX_DECIMALS))
        .ok_or(ParseErrorKind::TooManyDecimals)
}

/// The integer of what was read, or the overflow it is.
#[inline]
fn steps<I: Int>(read: Result<(bool, u128), ParseErrorKind>) -> Result<I, ParseError> {
    let (negative, magnitude) = read.map_err(ParseError::new)?;
    I::from_magnitude(negative, magnitude).ok_or_else(|| {
        ParseError::new(if negative {
            ParseErrorKind::NegOverflow
        } else {
            ParseErrorKind::PosOverflow
        })
    })
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The decimal at `scale` that `text` spells, exactly: zeros past the scale's decimals
    /// are allowed, a non-zero digit there is refused.
    ///
    /// # Errors
    /// [`ParseError`]: empty, not a number, past the range, or a non-zero digit past the scale.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Fixed, ParseErrorKind};
    ///
    /// assert_eq!(D64::<2>::from_ascii(b"60000.50", Fixed)?.steps(), 6_000_050, "exact");
    /// assert_eq!(D64::<2>::from_ascii(b"+.5e1", Fixed)?.steps(), 500, "as f64 reads it");
    /// let refused = D64::<2>::from_ascii(b"0.125", Fixed).map_err(ddust::ParseError::kind);
    /// assert_eq!(refused, Err(ParseErrorKind::TooManyDecimals), "never rounded");
    /// # Ok::<(), ddust::ParseError>(())
    /// ```
    #[inline]
    pub fn from_ascii(text: &[u8], scale: S) -> Result<Self, ParseError> {
        Ok(Self::from_steps(steps(read(text, scale.decimals()))?, scale))
    }

    /// The decimal at `range` of `buffer`: as [`from_ascii`](Self::from_ascii), with no loop
    /// over the digits when the 32 bytes around the number are in the buffer.
    ///
    /// # Errors
    /// As [`from_ascii`](Self::from_ascii), and `RangeOutsideBuffer` for a `range` that is not
    /// within `buffer`.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Fixed};
    ///
    /// let message = br#"{"price":"60000.5","size":"0.25","pad":"................"}"#;
    /// let price = D64::<7>::from_ascii_at(message, 10..17, Fixed)?;
    /// assert_eq!(price.to_string(), "60000.5", "read in place");
    /// # Ok::<(), ddust::ParseError>(())
    /// ```
    #[inline]
    pub fn from_ascii_at(buffer: &[u8], range: Range<usize>, scale: S) -> Result<Self, ParseError> {
        Ok(Self::from_steps(steps(read_at(buffer, range, scale.decimals()))?, scale))
    }

    /// The decimal at `scale` that `text` spells, rounded by `mode` when it has more decimals
    /// than the scale.
    ///
    /// # Errors
    /// [`ParseError`]: empty, not a number, or past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, Fixed};
    ///
    /// let rounded = D64::<2>::from_ascii_round(b"2.675", Fixed, HalfEven)?;
    /// assert_eq!(rounded.to_string(), "2.68", "a tie, to the even cent");
    /// # Ok::<(), ddust::ParseError>(())
    /// ```
    #[inline]
    pub fn from_ascii_round<R: RoundingMode>(
        text: &[u8], scale: S, mode: R,
    ) -> Result<Self, ParseError> {
        Ok(Self::from_steps(
            steps(read_or_round(text, scale.decimals(), Some(mode.table())))?,
            scale,
        ))
    }

    /// The decimal at `scale` at the front of `bytes`, and how many bytes it took: for
    /// a reader that has not found where the number ends.
    ///
    /// # Errors
    /// [`ParseError`]: no number at the front, past the range, or a non-zero digit past the scale.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Fixed};
    ///
    /// let (price, len) = D64::<2>::from_ascii_prefix(b"60000.5,0.25", Fixed)?;
    /// assert_eq!((price.to_string().as_str(), len), ("60000.5", 7), "up to the comma");
    /// # Ok::<(), ddust::ParseError>(())
    /// ```
    #[inline]
    pub fn from_ascii_prefix(bytes: &[u8], scale: S) -> Result<(Self, usize), ParseError> {
        if bytes.is_empty() {
            return Err(ParseError::new(ParseErrorKind::Empty));
        }
        let len = number_len(bytes);
        if len == 0 {
            return Err(ParseError::new(ParseErrorKind::InvalidDigit));
        }
        let number = bytes.get(..len).unwrap_or_default();
        Ok((Self::from_ascii(number, scale)?, len))
    }
}

/// The text exactly, at the scale: what `f64::from_str` reads, without its infinities and
/// NaN.
impl<I: Int, S: StaticScale> FromStr for Decimal<I, S> {
    type Err = ParseError;

    #[inline]
    fn from_str(text: &str) -> Result<Self, ParseError> {
        Self::from_ascii(text.as_bytes(), S::INSTANCE)
    }
}

/// The text exactly, at the scale it spells: `"1.50"` is 1.50 at two decimals.
impl<I: Int> FromStr for Decimal<I, Dynamic> {
    type Err = ParseError;

    #[inline]
    fn from_str(text: &str) -> Result<Self, ParseError> {
        let scale = spelled_scale(text.as_bytes()).map_err(ParseError::new)?;
        Self::from_ascii(text.as_bytes(), scale)
    }
}

/// Reads `text` at `decimals`, rounded by `mode`, for the tests: [`read_or_round`] given a mode.
#[cfg(test)]
fn read_rounded<R: RoundingMode>(
    text: &[u8], decimals: u8, mode: R,
) -> Result<(bool, u128), ParseErrorKind> {
    read_or_round(text, decimals, Some(mode.table()))
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "offsets into a test buffer")]
mod tests {
    use alloc::format;
    use alloc::string::String;

    use proptest::prelude::*;
    use rstest::rstest;

    use super::{read, read_general, read_plain, read_rounded};
    use crate::round::{Ceil, Floor, HalfEven, Rounding, RoundingMode};
    use crate::{D64, D128, Decimal, Dynamic, Fixed, ParseError, ParseErrorKind, UD64, UD128};

    /// Seven decimals.
    type Price = D64<7>;

    #[rstest]
    #[case::integer("60000", 7, Ok((false, 600_000_000_000)))]
    #[case::fraction("60000.5", 7, Ok((false, 600_005_000_000)))]
    #[case::padded("60000.50000000", 7, Ok((false, 600_005_000_000)))]
    #[case::many_zeros("1.0000000000000000000000000000000000000000", 7, Ok((false, 10_000_000)))]
    #[case::negative("-37.63", 7, Ok((true, 376_300_000)))]
    #[case::plus("+37.63", 7, Ok((false, 376_300_000)))]
    #[case::leading_point(".5", 7, Ok((false, 5_000_000)))]
    #[case::trailing_point("5.", 7, Ok((false, 50_000_000)))]
    #[case::exponent("1.2e-5", 17, Ok((false, 1_200_000_000_000)))]
    #[case::upper_exponent("12E+2", 0, Ok((false, 1200)))]
    #[case::point_exponent(".5e1", 0, Ok((false, 5)))]
    #[case::zero("-0.000", 7, Ok((true, 0)))]
    #[case::too_fine("0.12345678", 7, Err(ParseErrorKind::TooManyDecimals))]
    #[case::empty("", 7, Err(ParseErrorKind::Empty))]
    #[case::sign_only("-", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::point_only(".", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::two_points("1.2.3", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::sign_inside("1.-5", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::space(" 1", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::two_signs("--1", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::separator("1_000", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::bare_exponent("1e", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::infinity("inf", 7, Err(ParseErrorKind::InvalidDigit))]
    #[case::many_digits_below_a_step(
        "0.1234567890123456789012345678901234567891",
        2,
        Err(ParseErrorKind::TooManyDecimals)
    )]
    #[case::a_digit_far_below_a_step(concat!("1.", "0000000000000000000000000000000000000000", "1"), 7, Err(ParseErrorKind::TooManyDecimals))]
    #[case::zero_at_any_place("0e5000000000", 7, Ok((false, 0)))]
    #[case::past_u128("1e40", 7, Err(ParseErrorKind::PosOverflow))]
    #[case::past_u128_negative("-1e40", 7, Err(ParseErrorKind::NegOverflow))]
    fn a_number_reads_exactly_or_says_why_not(
        #[case] text: &str, #[case] decimals: u8,
        #[case] expected: Result<(bool, u128), ParseErrorKind>,
    ) {
        assert_eq!(read(text.as_bytes(), decimals), expected, "{text:?} at {decimals}");
    }

    #[rstest]
    #[case::down("0.123456785", Floor, Ok((false, 1_234_567)))]
    #[case::up("0.123456781", Ceil, Ok((false, 1_234_568)))]
    #[case::tie_to_even("0.12345675", HalfEven, Ok((false, 1_234_568)))]
    #[case::negative_floor("-0.123456781", Floor, Ok((true, 1_234_568)))]
    #[case::far_below_a_step("1e-30", Ceil, Ok((false, 1)))]
    #[case::exact_needs_none("0.5", Floor, Ok((false, 5_000_000)))]
    #[case::many_digits_below_a_step("0.1234567890123456789012345678901234567891", Floor, Ok((false, 1_234_567)))]
    #[case::half_with_a_digit_far_after("0.12345675000000000000000000000000000000001", HalfEven, Ok((false, 1_234_568)))]
    fn a_number_with_more_decimals_rounds_when_asked<R: RoundingMode>(
        #[case] text: &str, #[case] mode: R, #[case] expected: Result<(bool, u128), ParseErrorKind>,
    ) {
        assert_eq!(read_rounded(text.as_bytes(), 7, mode), expected, "{text:?}");
    }

    #[test]
    fn a_value_past_its_type_says_which_end() {
        let above = "922337203686".parse::<Price>().expect_err("above 9.2e11 at 7 decimals");
        assert_eq!(above.kind(), ParseErrorKind::PosOverflow, "above");
        let below = "-922337203686".parse::<Price>().expect_err("below");
        assert_eq!(below.kind(), ParseErrorKind::NegOverflow, "below");
        let negative = "-0.5".parse::<UD64<11>>().expect_err("a negative");
        assert_eq!(negative.kind(), ParseErrorKind::NegOverflow, "unsigned refuses a negative");
        let rounded = Price::from_ascii_round(b"60000.123456789", Fixed, Rounding::HalfEven);
        assert_eq!(rounded.map(Price::steps), Ok(600_001_234_568), "rounded");
    }

    #[test]
    fn a_run_time_scale_is_the_one_the_text_spells() {
        let scale = |text: &str| text.parse::<Decimal<i64, Dynamic>>().map(Decimal::decimals);
        assert_eq!(scale("1.50"), Ok(2), "trailing zeros are spelled");
        assert_eq!(scale("15"), Ok(0), "none");
        assert_eq!(scale("1.5e1"), Ok(0), "the exponent lifts the point");
        assert_eq!(scale("1e-3"), Ok(3), "or lowers it");
        let wide = |text: &str| text.parse::<Decimal<i128, Dynamic>>().map(Decimal::decimals);
        assert_eq!(wide("1000e-41"), Ok(38), "one step at 38, its zeros spelled in the integer");
        assert_eq!(wide("0e-50"), Ok(38), "zero needs no decimals");
        let long = format!("1.{}", "0".repeat(45));
        assert_eq!(
            long.parse::<Decimal<i128, Dynamic>>().map(Decimal::decimals),
            Ok(38),
            "capped at 38"
        );
        let fine = format!("0.{}1", "0".repeat(40));
        assert_eq!(
            fine.parse::<Decimal<i128, Dynamic>>().map_err(ParseError::kind),
            Err(ParseErrorKind::TooManyDecimals)
        );
    }

    #[test]
    fn a_long_run_of_zeros_is_counted_not_overflowed() {
        let zeros = "0".repeat(100_000);
        assert_eq!(
            read(format!("1{zeros}").as_bytes(), 7),
            Err(ParseErrorKind::PosOverflow),
            "past a u128"
        );
        assert_eq!(
            read(format!("0.{zeros}1e100000").as_bytes(), 0),
            Err(ParseErrorKind::TooManyDecimals)
        );
        assert_eq!(
            read(format!("1.5{zeros}").as_bytes(), 7),
            Ok((false, 15_000_000)),
            "padding is allowed"
        );
        assert_eq!(
            read(format!("0.{zeros}1e100002").as_bytes(), 0),
            Ok((false, 10)),
            "brought back"
        );
        assert_eq!(
            read(format!("1e{}", "9".repeat(40)).as_bytes(), 0),
            Err(ParseErrorKind::PosOverflow)
        );
    }

    #[test]
    fn a_range_outside_the_buffer_is_the_callers() {
        let refused = Price::from_ascii_at(b"60000.5", 3..12, Fixed).map_err(ParseError::kind);
        assert_eq!(refused, Err(ParseErrorKind::RangeOutsideBuffer), "past its end");
    }

    #[test]
    fn a_number_at_the_front_reads_up_to_its_end() {
        let read = |bytes: &[u8]| {
            Price::from_ascii_prefix(bytes, Fixed).map(|(price, len)| (price.steps(), len))
        };
        assert_eq!(read(b"60000.5,"), Ok((600_005_000_000, 7)), "up to the comma");
        assert_eq!(read(b"1e5x"), Ok((1_000_000_000_000, 3)), "with its exponent");
        assert_eq!(read(b"1ex"), Ok((10_000_000, 1)), "a letter with no digits is not an exponent");
        assert_eq!(
            read(b"x1").map_err(ParseError::kind),
            Err(ParseErrorKind::InvalidDigit),
            "nothing at the front"
        );
    }

    proptest! {
        #[test]
        fn the_plain_reader_agrees_with_the_general_one(
            text in "[0-9.eE+x-]{0,24}|[0-9]{0,24}\\.?[0-9]{0,24}", decimals in 0_u8..=38,
        ) {
            // Wherever the eight-at-a-time reader answers, it answers as the byte loop does; and on
            // the plain shape it refuses only what the byte loop refuses or rounds.
            let bytes = text.as_bytes();
            let general = read_general(bytes, decimals, None);
            let points: usize = bytes.iter().map(|&byte| usize::from(byte == b'.')).sum();
            let plain = bytes.iter().all(|&byte| byte.is_ascii_digit() || byte == b'.') && points <= 1;
            match read_plain(bytes, decimals) {
                Some(magnitude) => prop_assert_eq!(general, Ok(magnitude), "{}", text),
                None => prop_assert!(general.is_err() || !plain || bytes.len() > 38, "{} read as {:?}", text, general),
            }
        }

        #[test]
        fn every_value_reads_back_what_it_writes(steps: i64) {
            let price = Price::from_steps(steps, Fixed);
            prop_assert_eq!(format!("{price}").parse::<Price>(), Ok(price));
        }

        #[test]
        fn every_wide_value_reads_back_what_it_writes(steps: i128) {
            let amount = D128::<18>::from_steps(steps, Fixed);
            prop_assert_eq!(format!("{amount}").parse::<D128<18>>(), Ok(amount));
        }

        #[test]
        fn every_unsigned_wide_value_reads_back_what_it_writes(steps: u128) {
            let amount = UD128::<18>::from_steps(steps, Fixed);
            prop_assert_eq!(format!("{amount}").parse::<UD128<18>>(), Ok(amount));
        }

        #[test]
        fn a_run_time_value_reads_back_at_its_scale(steps: i64, decimals in 0_u8..=18) {
            let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
            let back: Decimal<i64, Dynamic> = format!("{value:#}").parse().expect("a number");
            prop_assert_eq!((back.steps(), back.decimals()), (steps, decimals));
        }

        #[test]
        fn the_windowed_read_agrees_with_the_byte_loop(text in "-?[0-9]{1,9}(\\.[0-9]{1,9})?", decimals in 0_u8..=18) {
            let mut buffer = String::from("........");
            buffer.push_str(&text);
            buffer.push_str("\",\"....................");
            let range = 8..8 + text.len();
            prop_assert_eq!(super::read_at(buffer.as_bytes(), range, decimals), read(text.as_bytes(), decimals));
        }
    }
}
