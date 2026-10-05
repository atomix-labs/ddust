//! Reading a decimal: what `f64::from_str` reads, without its infinities and NaN, straight to the
//! steps, exactly, or rounded by a mode when asked.

use core::ops::Range;
use core::str::FromStr;

use crate::decimal::Decimal;
use crate::errors::{ParseError, ParseErrorKind};
use crate::int::Int;
use crate::round::RoundingMode;
use crate::scale::{Dynamic, MAX_DECIMALS, Scale, StaticScale};
use crate::word::pow10_u128;

/// Whether a quotient of `quotient` and `remainder` over `divisor`, for a value of sign
/// `negative`, moves one step away from zero by `table`.
#[inline]
fn is_rounded_away(
    table: u16, negative: bool, quotient: u128, remainder: u128, divisor: u128,
) -> bool {
    let rest = divisor.wrapping_sub(remainder);
    let class = u32::from(remainder != 0)
        .wrapping_add(u32::from(remainder >= rest))
        .wrapping_add(u32::from(remainder > rest));
    let index = (u32::from(negative) << 3) | (u32::from(quotient & 1 == 1) << 2) | class;
    (table >> index) & 1 == 1
}

/// The digits of a number, accumulated: its significant digits with their trailing zeros held
/// apart, so a long run of zeros never overflows what follows.
macro_rules! digits {
    ($name:ident, $acc:ty, $overflow:expr) => {
        /// Reads `digits` (no sign) at `decimals`: exactly, or, given a table, rounded by it for a
        /// number of sign `negative`. The magnitude, or `None` when the significant digits outgrow
        /// the accumulator.
        #[inline]
        #[expect(clippy::arithmetic_side_effects, reason = "every step is checked or bounded")]
        fn $name(
            digits: &[u8], decimals: u8, round: Option<(bool, u16)>,
        ) -> Option<Result<$acc, ParseErrorKind>> {
            let invalid = Some(Err(ParseErrorKind::InvalidDigit));
            let mut value: $acc = 0;
            // Counted in 64 bits: a slice holds fewer than 2^63 digits, so no count wraps.
            let (mut zeros, mut fraction, mut at) = (0_u64, 0_i64, 0_usize);
            let push = |digit: u8, value: &mut $acc, zeros: &mut u64| -> Option<()> {
                if digit == 0 {
                    *zeros += u64::from(*value != 0);
                } else {
                    let power = u32::try_from(zeros.saturating_add(1)).ok()?;
                    let scale = <$acc>::from(10_u8).checked_pow(power)?;
                    *value = value.checked_mul(scale)?.checked_add(<$acc>::from(digit))?;
                    *zeros = 0;
                }
                Some(())
            };
            let integer = digits.iter().take_while(|byte| byte.is_ascii_digit()).count();
            for &byte in digits.get(..integer)? {
                push(byte - b'0', &mut value, &mut zeros)?;
            }
            at += integer;
            let mut count = 0;
            if digits.get(at) == Some(&b'.') {
                let rest = digits.get(at + 1..)?;
                count = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
                for &byte in rest.get(..count)? {
                    push(byte - b'0', &mut value, &mut zeros)?;
                }
                fraction = i64::try_from(count).unwrap_or(i64::MAX);
                at += count + 1;
            }
            if integer == 0 && count == 0 {
                return invalid;
            }
            let mut exponent = 0_i64;
            if let Some(b'e' | b'E') = digits.get(at) {
                let (negative, rest) = match digits.get(at + 1..)? {
                    [b'-', rest @ ..] => (true, rest),
                    [b'+', rest @ ..] => (false, rest),
                    rest => (false, rest),
                };
                if rest.is_empty() || !rest.iter().all(u8::is_ascii_digit) {
                    return invalid;
                }
                for &byte in rest {
                    exponent = exponent.saturating_mul(10).saturating_add(i64::from(byte - b'0'));
                }
                if negative {
                    exponent = -exponent;
                }
                at = digits.len();
            }
            if at != digits.len() {
                return invalid;
            }
            if value == 0 {
                return Some(Ok(0));
            }
            // Saturating: a sum past the range is a power no accumulator holds either way.
            let power = i64::try_from(zeros)
                .unwrap_or(i64::MAX)
                .saturating_add(exponent)
                .saturating_add(i64::from(decimals))
                .saturating_sub(fraction);
            if power >= 0 {
                return match u32::try_from(power)
                    .ok()
                    .and_then(|power| <$acc>::from(10_u8).checked_pow(power))
                    .and_then(|scale| value.checked_mul(scale))
                {
                    Some(magnitude) => Some(Ok(magnitude)),
                    None => $overflow,
                };
            }
            // More decimals than the value carries: refused, or rounded by the table.
            let Some((negative, table)) = round else {
                return Some(Err(ParseErrorKind::TooManyDecimals));
            };
            let Some(divisor) = u32::try_from(power.unsigned_abs())
                .ok()
                .and_then(|power| <$acc>::from(10_u8).checked_pow(power))
            else {
                // Below one step at `decimals`, and below half of one.
                return Some(Ok(<$acc>::from(is_rounded_away(table, negative, 0, 1, 3))));
            };
            let (quotient, remainder) = (value / divisor, value % divisor);
            let away = is_rounded_away(
                table,
                negative,
                u128::from(quotient),
                u128::from(remainder),
                u128::from(divisor),
            );
            Some(Ok(quotient + <$acc>::from(away)))
        }
    };
}

digits!(digits_narrow, u64, None);
digits!(digits_wide, u128, Some(Err(ParseErrorKind::PosOverflow)));

/// The common shape, `[digits][.digits]` with at most 19 significant digits, read in one 64-bit
/// pass: the fraction's trailing zeros are dropped first, so no step divides. `None` sends any
/// other shape, an error included, to the exact general loop.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "at most 19 digits: every step stays in a u64")]
fn read_simple(digits: &[u8], decimals: u8) -> Option<u64> {
    let (integer, fraction) = match digits.iter().position(|&byte| byte == b'.') {
        Some(dot) => (digits.get(..dot)?, digits.get(dot + 1..)?),
        None => (digits, &[][..]),
    };
    if integer.is_empty() && fraction.is_empty() {
        return None;
    }
    // Any byte but `0` ends the trimming, a stray one included, which the digit test then refuses.
    let significant = fraction.iter().rposition(|&byte| byte != b'0').map_or(0, |last| last + 1);
    let fraction = fraction.get(..significant)?;
    if integer.len() + fraction.len() > 19 || usize::from(decimals) < fraction.len() {
        return None;
    }
    let mut value = 0_u64;
    for &byte in integer.iter().chain(fraction) {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        value = value * 10 + u64::from(digit);
    }
    value.checked_mul(
        u64::try_from(pow10_u128(decimals - u8::try_from(fraction.len()).ok()?)?).ok()?,
    )
}

/// Reads `text` at `decimals`: its sign and its magnitude at those decimals, exactly, or, given a
/// table, rounded by it.
#[inline]
fn read_or_round(
    text: &[u8], decimals: u8, table: Option<u16>,
) -> Result<(bool, u128), ParseErrorKind> {
    let (negative, digits) = match text {
        [] => return Err(ParseErrorKind::Empty),
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] => (false, rest),
        _ => (false, text),
    };
    if let Some(magnitude) = read_simple(digits, decimals) {
        return Ok((negative, u128::from(magnitude)));
    }
    let round = table.map(|table| (negative, table));
    let magnitude = digits_narrow(digits, decimals, round).map_or_else(
        || digits_wide(digits, decimals, round).unwrap_or(Err(ParseErrorKind::PosOverflow)),
        |read| read.map(u128::from),
    );
    match magnitude {
        Err(ParseErrorKind::PosOverflow) if negative => Err(ParseErrorKind::NegOverflow),
        magnitude => magnitude.map(|magnitude| (negative, magnitude)),
    }
}

/// Reads `text` at `decimals`: its sign and its magnitude, exactly.
#[inline]
pub(crate) fn read(text: &[u8], decimals: u8) -> Result<(bool, u128), ParseErrorKind> {
    read_or_round(text, decimals, None)
}

/// ASCII `'0'` in every byte.
const ZEROS: u64 = 0x3030_3030_3030_3030;
/// `0x01` in every byte.
const ONES: u64 = 0x0101_0101_0101_0101;
/// `0x80` in every byte.
const HIGHS: u64 = 0x8080_8080_8080_8080;
/// ASCII `'.'` in every byte.
const DOTS: u64 = 0x2E2E_2E2E_2E2E_2E2E;

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

/// Whether all eight bytes are ASCII digits.
#[inline]
const fn all_digits(word: u64) -> bool {
    (word.wrapping_add(0x4646_4646_4646_4646) | word.wrapping_sub(ZEROS)) & HIGHS == 0
}

/// The value of eight ASCII digits, the first in the lowest byte: three multiplies (Lemire,
/// *Quickly parsing eight digits*, 2018).
#[inline]
const fn eight_digits(word: u64) -> u64 {
    let word = (word & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561).wrapping_shr(8);
    let word = (word & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601).wrapping_shr(16);
    (word & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001).wrapping_shr(32)
}

/// The eight bytes of `window` from `at`, little-endian.
#[inline]
fn word(window: &[u8; 32], at: usize) -> Option<u64> {
    let bytes: [u8; 8] = window.get(at..at.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

/// The path with no loop over the digits: a number of at most eight integer and eight fraction
/// digits whose magnitude fits a `u64`, read eight bytes at a time from the 32 bytes around it,
/// or `None` for anything else, which the byte loop then reads.
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

/// Reads the number at `range` of `buffer` at `decimals`: with no loop over the digits when the
/// 32 bytes around it are in the buffer, by the byte loop otherwise.
#[inline]
pub(crate) fn read_at(
    buffer: &[u8], range: Range<usize>, decimals: u8,
) -> Result<(bool, u128), ParseErrorKind> {
    let len = range.end.saturating_sub(range.start);
    match read_window(buffer, range.start, len, decimals) {
        Some(read) => read,
        None => read(buffer.get(range).ok_or(ParseErrorKind::InvalidDigit)?, decimals),
    }
}

/// How many bytes at the front of `bytes` are a number: a sign, digits with an optional point, and
/// an exponent when its letter is followed by digits. Zero when there are no digits.
#[expect(clippy::arithmetic_side_effects, reason = "offsets within the slice")]
fn number_len(bytes: &[u8]) -> usize {
    let digits_from = |at: usize| {
        bytes.get(at..).map_or(0, |rest| rest.iter().take_while(|b| b.is_ascii_digit()).count())
    };
    let mut at = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    let integer = digits_from(at);
    at += integer;
    let mut fraction = 0;
    if bytes.get(at) == Some(&b'.') {
        fraction = digits_from(at + 1);
        if integer + fraction > 0 {
            at += 1 + fraction;
        }
    }
    if integer + fraction == 0 {
        return 0;
    }
    if let Some(b'e' | b'E') = bytes.get(at) {
        let signed = usize::from(matches!(bytes.get(at + 1), Some(b'-' | b'+')));
        let exponent = digits_from(at + 1 + signed);
        if exponent > 0 {
            at += 1 + signed + exponent;
        }
    }
    at
}

/// The scale `text` spells: its fraction's digits less its exponent, at least none, and at most
/// 38 when the digits past 38 are zeros.
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
    let (mantissa, exponent) =
        body.iter().position(|&byte| byte == b'e' || byte == b'E').map_or((body, &[][..]), |at| {
            (body.get(..at).unwrap_or_default(), body.get(at + 1..).unwrap_or_default())
        });
    let fraction = mantissa
        .iter()
        .position(|&byte| byte == b'.')
        .map_or(&[][..], |at| mantissa.get(at + 1..).unwrap_or_default());
    let exponent = match exponent {
        [b'-', digits @ ..] => -digits
            .iter()
            .fold(0_i64, |e, &d| e.saturating_mul(10).saturating_add(i64::from(d - b'0'))),
        [b'+', digits @ ..] | digits => digits
            .iter()
            .fold(0_i64, |e, &d| e.saturating_mul(10).saturating_add(i64::from(d - b'0'))),
    };
    let written = i64::try_from(fraction.len()).unwrap_or(i64::MAX).saturating_sub(exponent);
    let zeros = fraction.iter().rev().take_while(|&&byte| byte == b'0').count();
    let needed = written.saturating_sub(i64::try_from(zeros).unwrap_or(i64::MAX));
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
    /// The decimal at `scale` that `text` spells, exactly: zeros past the scale's decimals are
    /// allowed, a non-zero digit there is refused.
    ///
    /// # Errors
    /// [`ParseError`]: empty, not a number, past the range, or a non-zero digit past the scale.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Fixed, ParseErrorKind};
    ///
    /// assert_eq!(D64::<2>::from_ascii(b"60000.50", Fixed)?.to_bits(), 6_000_050, "exact");
    /// assert_eq!(D64::<2>::from_ascii(b"+.5e1", Fixed)?.to_bits(), 500, "as f64 reads it");
    /// let refused = D64::<2>::from_ascii(b"0.125", Fixed).map_err(ddust::ParseError::kind);
    /// assert_eq!(refused, Err(ParseErrorKind::TooManyDecimals), "never rounded");
    /// # Ok::<(), ddust::ParseError>(())
    /// ```
    #[inline]
    pub fn from_ascii(text: &[u8], scale: S) -> Result<Self, ParseError> {
        Ok(Self::from_bits(steps(read(text, scale.decimals()))?, scale))
    }

    /// The decimal at `range` of `buffer`: as [`from_ascii`](Self::from_ascii), with no loop over
    /// the digits when the 32 bytes around the number are in the buffer.
    ///
    /// # Errors
    /// As [`from_ascii`](Self::from_ascii).
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
        Ok(Self::from_bits(steps(read_at(buffer, range, scale.decimals()))?, scale))
    }

    /// The decimal at `scale` that `text` spells, rounded by `mode` when it has more decimals than
    /// the scale.
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
        Ok(Self::from_bits(
            steps(read_or_round(text, scale.decimals(), Some(mode.table())))?,
            scale,
        ))
    }

    /// The decimal at `scale` at the front of `bytes`, and how many bytes it took: for a reader
    /// that has not found where the number ends.
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

/// The text exactly, at the scale: what `f64::from_str` reads, without its infinities and NaN.
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

/// Reading with a mode, for the tests: the table's form of [`read_or_round`].
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

    use super::{read, read_rounded};
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
        let short = "-0.5".parse::<UD64<11>>().expect_err("a negative");
        assert_eq!(short.kind(), ParseErrorKind::NegOverflow, "unsigned refuses a negative");
        let rounded = Price::from_ascii_round(b"60000.123456789", Fixed, Rounding::HalfEven);
        assert_eq!(rounded.map(Price::to_bits), Ok(600_001_234_568), "rounded");
    }

    #[test]
    fn a_run_time_scale_is_the_one_the_text_spells() {
        let scale = |text: &str| text.parse::<Decimal<i64, Dynamic>>().map(Decimal::decimals);
        assert_eq!(scale("1.50"), Ok(2), "trailing zeros are spelled");
        assert_eq!(scale("15"), Ok(0), "none");
        assert_eq!(scale("1.5e1"), Ok(0), "the exponent lifts the point");
        assert_eq!(scale("1e-3"), Ok(3), "or lowers it");
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
    fn a_number_at_the_front_reads_up_to_its_end() {
        let read = |bytes: &[u8]| {
            Price::from_ascii_prefix(bytes, Fixed).map(|(price, len)| (price.to_bits(), len))
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
        fn every_value_reads_back_what_it_writes(bits: i64) {
            let price = Price::from_bits(bits, Fixed);
            prop_assert_eq!(format!("{price}").parse::<Price>(), Ok(price));
        }

        #[test]
        fn every_wide_value_reads_back_what_it_writes(bits: i128) {
            let amount = D128::<18>::from_bits(bits, Fixed);
            prop_assert_eq!(format!("{amount}").parse::<D128<18>>(), Ok(amount));
        }

        #[test]
        fn every_unsigned_wide_value_reads_back_what_it_writes(bits: u128) {
            let amount = UD128::<18>::from_bits(bits, Fixed);
            prop_assert_eq!(format!("{amount}").parse::<UD128<18>>(), Ok(amount));
        }

        #[test]
        fn a_run_time_value_reads_back_at_its_scale(bits: i64, decimals in 0_u8..=18) {
            let value = Decimal::from_bits(bits, Dynamic::new(decimals).expect("at most 38"));
            let back: Decimal<i64, Dynamic> = format!("{value:#}").parse().expect("a number");
            prop_assert_eq!((back.to_bits(), back.decimals()), (bits, decimals));
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
