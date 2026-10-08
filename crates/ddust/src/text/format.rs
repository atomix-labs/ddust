//! Writing a decimal: its shortest exact digits, eight at a time, into a stack buffer.

use core::fmt::Write as _;
use core::{fmt, str};

use super::swar::{GROUP, ZEROS};
use crate::decimal::Decimal;
use crate::int::Int;
use crate::reciprocal;
use crate::scale::Scale;
use crate::word::pow10_u128;

/// The longest text a decimal writes: a sign, 39 digits and a point. [`Decimal::write_ascii`]
/// writes into any buffer that holds its text, so one of this length holds every value's.
pub const MAX_ASCII_LEN: usize = 41;

/// The room the eight-byte writer stores into: the text, and up to seven bytes past it.
const EIGHT_BYTE_ROOM: usize = 48;

/// `10^k` as a `u64`, or `None` past `10^19`.
#[inline]
fn u64_pow10(k: u8) -> Option<u64> {
    pow10_u128(k).and_then(|power| u64::try_from(power).ok())
}

/// The eight digits of `n`, below `10^8`, as byte values, the most significant in the lowest
/// byte: two multiply-shifts per halving, not a division per digit.
#[expect(clippy::arithmetic_side_effects, reason = "n is below 10^8: every step stays in range")]
const fn digits8(n: u64) -> u64 {
    let x = (n / 10_000) | ((n % 10_000) << 32);
    let q = ((x * 10_486) >> 20) & 0x0000_007F_0000_007F;
    let y = q | ((x - q * 100) << 16);
    let q = ((y * 103) >> 10) & 0x000F_000F_000F_000F;
    q | ((y - q * 10) << 8)
}

/// How many significant digits `n`, below `10^8`, has; zero has none.
fn significant(n: u64) -> usize {
    let digits = digits8(n);
    let leading = usize::try_from(digits.trailing_zeros() / 8).unwrap_or(8);
    if digits == 0 { 0 } else { 8_usize.saturating_sub(leading) }
}

/// A decimal number built left to right in a stack buffer: room for a sign, 39 digits, a point,
/// 38 decimals and an exponent. Zeros a precision asks for past those are written after it.
struct Text {
    /// The bytes written so far.
    bytes: [u8; 96],
    /// How many are written.
    len: usize,
}

impl Text {
    /// An empty buffer.
    const fn new() -> Self {
        Self { bytes: [0; 96], len: 0 }
    }

    /// The text written.
    fn as_str(&self) -> &str {
        self.bytes.get(..self.len).and_then(|bytes| str::from_utf8(bytes).ok()).unwrap_or_default()
    }

    /// Appends `bytes`.
    fn push(&mut self, bytes: &[u8]) {
        let end = self.len.saturating_add(bytes.len());
        if let Some(slot) = self.bytes.get_mut(self.len..end) {
            slot.copy_from_slice(bytes);
            self.len = end;
        }
    }

    /// Appends the last `width` of the eight digits of `n`, below `10^8`.
    fn push_digits(&mut self, n: u64, width: usize) {
        let ascii = (digits8(n) | ZEROS).to_le_bytes();
        self.push(ascii.get(8_usize.saturating_sub(width)..).unwrap_or_default());
    }

    /// Appends `n` with no leading zeros, `0` for zero.
    fn push_integer(&mut self, n: u64) {
        let (high, rest) = (n / (GROUP * GROUP), n % (GROUP * GROUP));
        let (middle, low) = (rest / GROUP, rest % GROUP);
        if high > 0 {
            self.push_digits(high, significant(high));
            self.push_digits(middle, 8);
            self.push_digits(low, 8);
        } else if middle > 0 {
            self.push_digits(middle, significant(middle));
            self.push_digits(low, 8);
        } else {
            self.push_digits(low, significant(low).max(1));
        }
    }

    /// Appends `n` with no leading zeros, for values past a `u64`.
    #[expect(clippy::arithmetic_side_effects, reason = "division by non-zero powers of ten")]
    fn push_wide_integer(&mut self, n: u128) {
        let split = u128::from(GROUP * GROUP * 1_000);
        if let Ok(narrow) = u64::try_from(n) {
            self.push_integer(narrow);
        } else {
            self.push_wide_integer(n / split);
            let rest = u64::try_from(n % split).unwrap_or_default();
            self.push_digits(rest / (GROUP * GROUP), 3);
            self.push_digits(rest / GROUP % GROUP, 8);
            self.push_digits(rest % GROUP, 8);
        }
    }

    /// Appends `fraction`, below `10^width`, as exactly `width` digits, eight at a time from
    /// the most significant.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "division by non-zero powers of ten; widths below 40"
    )]
    fn push_fraction(&mut self, fraction: u128, width: u8) {
        let width = usize::from(width);
        if width == 0 {
            return;
        }
        let chunks = width.div_ceil(8);
        let mut power = (1..chunks).fold(1_u128, |power, _| power * u128::from(GROUP));
        let first = u64::try_from(fraction / power).unwrap_or_default();
        self.push_digits(first, width - 8 * (chunks - 1));
        let mut rest = fraction % power;
        while power > 1 {
            power /= u128::from(GROUP);
            self.push_digits(u64::try_from(rest / power).unwrap_or_default(), 8);
            rest %= power;
        }
    }

    /// Drops the fraction's trailing zeros, and the point when nothing is left after it.
    fn trim(&mut self) {
        while self.bytes.get(self.len.saturating_sub(1)) == Some(&b'0') {
            self.len = self.len.saturating_sub(1);
        }
        if self.bytes.get(self.len.saturating_sub(1)) == Some(&b'.') {
            self.len = self.len.saturating_sub(1);
        }
    }

    /// Appends `magnitude × 10^-decimals`: the shortest exact spelling, or with `precision`
    /// decimals, rounded half to even when it is fewer. Returns how many zeros a longer precision
    /// pads it with, which the caller writes.
    #[must_use = "the zeros a longer precision asks for are the caller's to write"]
    #[expect(clippy::arithmetic_side_effects, reason = "division by non-zero powers of ten")]
    fn push_decimal_unpadded(
        &mut self, magnitude: u128, decimals: u8, precision: Option<usize>,
    ) -> usize {
        let pow = |k: u8| pow10_u128(k).unwrap_or(1);
        let (magnitude, decimals, zeros) = match precision {
            Some(precision) if precision < usize::from(decimals) => {
                let precision = u8::try_from(precision).unwrap_or(decimals);
                let step = pow(decimals - precision);
                let (quotient, remainder) = (magnitude / step, magnitude % step);
                let half = step / 2;
                let up = remainder > half || (remainder == half && quotient % 2 == 1);
                (quotient.saturating_add(u128::from(up)), precision, Some(0))
            },
            Some(precision) => {
                (magnitude, decimals, Some(precision.saturating_sub(usize::from(decimals))))
            },
            None => (magnitude, decimals, None),
        };
        let unit = pow(decimals);
        self.push_wide_integer(magnitude / unit);
        let fraction = magnitude % unit;
        match zeros {
            Some(0) if decimals == 0 => {},
            Some(_) => {
                self.push(b".");
                self.push_fraction(fraction, decimals);
            },
            None if fraction == 0 => {},
            None => {
                self.push(b".");
                self.push_fraction(fraction, decimals);
                self.trim();
            },
        }
        zeros.unwrap_or(0)
    }

    /// Appends `magnitude × 10^-decimals` in scientific notation's digits: one, the point and
    /// the rest when there are any; with `precision`, exactly that many after the point, a tie
    /// rounded to even. Returns how many zeros a longer precision pads the digits with, and the
    /// exponent, which the caller writes after them.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "digit counts below 40, and exponents below 80"
    )]
    fn push_exponent_unpadded(
        &mut self, magnitude: u128, decimals: u8, precision: Option<usize>,
    ) -> (usize, i32) {
        let mut digits = Self::new();
        digits.push_wide_integer(magnitude);
        let total = digits.len;
        let mut kept = total;
        while kept > 1 && digits.bytes.get(kept - 1) == Some(&b'0') {
            kept -= 1;
        }
        let total = i32::try_from(total).unwrap_or_default();
        let mut exponent = if magnitude == 0 { 0 } else { total - 1 - i32::from(decimals) };
        let mut zeros = 0;
        if let Some(precision) = precision {
            let wanted = precision.saturating_add(1);
            if wanted < kept {
                let next = digits.bytes.get(wanted).copied().unwrap_or(b'0');
                let beyond = digits
                    .bytes
                    .get(wanted + 1..kept)
                    .is_some_and(|rest| rest.iter().any(|&d| d != b'0'));
                let odd = digits.bytes.get(wanted - 1).is_some_and(|&d| d % 2 == 1);
                kept = wanted;
                if (next > b'5' || (next == b'5' && (beyond || odd))) && digits.increment(kept) {
                    exponent += 1;
                }
            } else {
                zeros = wanted - kept;
            }
        }
        if let Some((first, rest)) = digits.bytes.get(..kept).and_then(<[u8]>::split_first) {
            self.push(&[*first]);
            if !rest.is_empty() || zeros > 0 {
                self.push(b".");
                self.push(rest);
            }
        }
        (zeros, exponent)
    }

    /// Adds one to the number the first `len` digits spell, carrying; whether the carry passed
    /// the first digit, which then becomes `1` and the rest `0`, the exponent one higher.
    #[expect(clippy::arithmetic_side_effects, reason = "a digit below 9 plus one")]
    fn increment(&mut self, len: usize) -> bool {
        let mut i = len;
        while i > 0 {
            i -= 1;
            if let Some(digit) = self.bytes.get_mut(i) {
                if *digit < b'9' {
                    *digit += 1;
                    return false;
                }
                *digit = b'0';
            }
        }
        if let Some(first) = self.bytes.get_mut(0) {
            *first = b'1';
        }
        true
    }
}

/// Writes `zeros` zeros, thirty-two at a time.
fn write_zeros(f: &mut fmt::Formatter<'_>, zeros: usize) -> fmt::Result {
    const RUN: &str = "00000000000000000000000000000000";
    let mut left = zeros;
    while left > 0 {
        let run = left.min(RUN.len());
        f.write_str(RUN.get(..run).unwrap_or_default())?;
        left = left.saturating_sub(run);
    }
    Ok(())
}

/// Writes `digits`, `zeros` zeros, then `suffix`, as one number, padded as an integer is: by
/// `pad_integral` itself when there are no zeros, and by its rules otherwise, since the zeros are
/// not in the buffer.
fn pad_number(
    f: &mut fmt::Formatter<'_>, nonnegative: bool, digits: &str, zeros: usize, suffix: &str,
) -> fmt::Result {
    if zeros == 0 && suffix.is_empty() {
        return f.pad_integral(nonnegative, "", digits);
    }
    let sign = if !nonnegative {
        "-"
    } else if f.sign_plus() {
        "+"
    } else {
        ""
    };
    let len =
        sign.len().saturating_add(digits.len()).saturating_add(zeros).saturating_add(suffix.len());
    let fill = f.width().unwrap_or(0).saturating_sub(len);
    if f.sign_aware_zero_pad() {
        f.write_str(sign)?;
        write_zeros(f, fill)?;
        f.write_str(digits)?;
        write_zeros(f, zeros)?;
        return f.write_str(suffix);
    }
    let (before, after) = match f.align() {
        Some(fmt::Alignment::Left) => (0, fill),
        Some(fmt::Alignment::Center) => (fill / 2, fill.div_ceil(2)),
        Some(fmt::Alignment::Right) | None => (fill, 0),
    };
    let character = f.fill();
    (0..before).try_for_each(|_| f.write_char(character))?;
    f.write_str(sign)?;
    f.write_str(digits)?;
    write_zeros(f, zeros)?;
    f.write_str(suffix)?;
    (0..after).try_for_each(|_| f.write_char(character))
}

/// Puts the eight bytes of `word` at `position` in `out`; `None` when they do not fit.
#[inline]
fn put(out: &mut [u8], position: usize, word: u64) -> Option<()> {
    out.get_mut(position..position.checked_add(8)?)?.copy_from_slice(&word.to_le_bytes());
    Some(())
}

/// Puts `n`, below `10^8`, with no leading zeros (`0` for zero); the end it wrote to.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "a byte count below 8 times 8")]
fn put_short(out: &mut [u8], position: usize, n: u64) -> Option<usize> {
    let digits = digits8(n);
    let leading = if n == 0 { 7 } else { digits.trailing_zeros() / 8 };
    put(out, position, (digits | ZEROS) >> (leading * 8))?;
    position.checked_add(8_usize.checked_sub(usize::try_from(leading).ok()?)?)
}

/// Puts `n` with no leading zeros, eight digits at a time; the end it wrote to.
#[inline]
fn put_integer(out: &mut [u8], position: usize, n: u64) -> Option<usize> {
    if n < GROUP {
        return put_short(out, position, n);
    }
    let (high, low) = (n / GROUP, n % GROUP);
    let position = if high < GROUP {
        put_short(out, position, high)?
    } else {
        let position = put_short(out, position, high / GROUP)?;
        put(out, position, digits8(high % GROUP) | ZEROS)?;
        position.checked_add(8)?
    };
    put(out, position, digits8(low) | ZEROS)?;
    position.checked_add(8)
}

/// Puts `fraction`, below `10^decimals`, as its digits with the trailing zeros dropped, eight
/// at a time; the end it wrote to.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "division by non-zero powers of ten")]
fn put_fraction(out: &mut [u8], position: usize, fraction: u64, decimals: u8) -> Option<usize> {
    // The fraction as up to three 8-digit chunks, left-aligned: the last padded with zeros.
    let (first, second, third) = match decimals {
        0..=8 => (fraction * u64_pow10(8 - decimals)?, 0, 0),
        9..=16 => {
            let rest = u64_pow10(decimals - 8)?;
            (fraction / rest, fraction % rest * u64_pow10(16 - decimals)?, 0)
        },
        _ => {
            let (rest, last) = (u64_pow10(decimals - 8)?, u64_pow10(decimals - 16)?);
            (fraction / rest, fraction % rest / last, fraction % last * u64_pow10(24 - decimals)?)
        },
    };
    let mut position = position;
    for (chunk, more) in [(first, second | third != 0), (second, third != 0), (third, false)] {
        let digits = digits8(chunk);
        put(out, position, digits | ZEROS)?;
        if !more {
            let trailing = usize::try_from(digits.leading_zeros() / 8).ok()?;
            return position.checked_add(8 - trailing);
        }
        position += 8;
    }
    Some(position)
}

/// Writes `magnitude × 10^-decimals` into `out`, a `-` first when `negative`: the shortest exact
/// decimal, with no division but the one by the unit, eight bytes at a time. The length
/// written, or `None` when the text and the eight bytes past it do not fit.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "division by a non-zero power of ten")]
pub(crate) fn write_ascii_narrow(
    negative: bool, magnitude: u64, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    let unit = u64_pow10(decimals)?;
    let (integer, fraction) = (magnitude / unit, magnitude % unit);
    let position = if negative && magnitude != 0 {
        *out.first_mut()? = b'-';
        1
    } else {
        0
    };
    let mut position = put_integer(out, position, integer)?;
    if fraction != 0 {
        *out.get_mut(position)? = b'.';
        position = put_fraction(out, position.checked_add(1)?, fraction, decimals)?;
    }
    Some(position)
}

/// [`write_ascii_narrow`] for a magnitude past a `u64`, at most 19 decimals: the integer and the
/// fraction split by two Möller–Granlund steps on the scale's reciprocal, and an integer past a
/// word split once more by `10^16`, where the [`Text`] writer divides in a `u128` for each. `None`
/// past what that reaches, or when the text and the eight bytes past it do not fit. Out of line:
/// inline, it led LLVM to call `put_integer` out of line from the narrow writer as well.
#[inline(never)]
fn write_ascii_split(
    negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    let (integer, fraction, _) = reciprocal::divide_u128_past_a_word(magnitude, decimals)?;
    let position = if negative && magnitude != 0 {
        *out.first_mut()? = b'-';
        1
    } else {
        0
    };
    let mut position = match u64::try_from(integer) {
        Ok(integer) => put_integer(out, position, integer)?,
        Err(_past_a_word) => {
            let (high, low, _) = reciprocal::divide_u128(integer, 16)?;
            let position = put_integer(out, position, high)?;
            put(out, position, digits8(low / GROUP) | ZEROS)?;
            put(out, position.checked_add(8)?, digits8(low % GROUP) | ZEROS)?;
            position.checked_add(16)?
        },
    };
    if fraction != 0 {
        *out.get_mut(position)? = b'.';
        position = put_fraction(out, position.checked_add(1)?, fraction, decimals)?;
    }
    Some(position)
}

/// [`write_ascii_narrow`] for a magnitude past a `u64` or more than 19 decimals, through a
/// [`Text`]: `None` when `out` is too short.
fn write_ascii_wide(
    negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    let mut text = Text::new();
    if negative && magnitude != 0 {
        text.push(b"-");
    }
    let _no_zeros = text.push_decimal_unpadded(magnitude, decimals, None);
    let written = text.as_str().as_bytes();
    out.get_mut(..written.len())?.copy_from_slice(written);
    Some(written.len())
}

/// Writes `steps` at `decimals`, padded as an integer is, with `precision` decimals when one
/// is asked: by the eight-digit writers at up to 19 decimals when no precision is asked, the
/// narrow one for a `u64` and the split one past it, through a [`Text`] otherwise.
fn write<I: Int>(
    f: &mut fmt::Formatter<'_>, steps: I, decimals: u8, precision: Option<usize>,
) -> fmt::Result {
    let (negative, magnitude) = steps.sign_and_magnitude();
    if decimals <= 19 && precision.is_none() {
        let plain = f.width().is_none() && !f.sign_plus();
        let mut bytes = [0; EIGHT_BYTE_ROOM];
        let written = match u64::try_from(magnitude) {
            Ok(narrow) => write_ascii_narrow(negative && plain, narrow, decimals, &mut bytes),
            Err(_past_a_word) => {
                write_ascii_split(negative && plain, magnitude, decimals, &mut bytes)
            },
        };
        if let Some(text) =
            written.and_then(|len| bytes.get(..len)).and_then(|text| str::from_utf8(text).ok())
        {
            return if plain {
                f.write_str(text)
            } else {
                f.pad_integral(!negative || magnitude == 0, "", text)
            };
        }
    }
    let mut text = Text::new();
    let zeros = text.push_decimal_unpadded(magnitude, decimals, precision);
    pad_number(f, !negative || magnitude == 0, text.as_str(), zeros, "")
}

/// Writes `steps` at `decimals` in scientific notation, with `letter` before the exponent.
fn write_exponent<I: Int>(
    f: &mut fmt::Formatter<'_>, steps: I, decimals: u8, letter: u8,
) -> fmt::Result {
    let (negative, magnitude) = steps.sign_and_magnitude();
    let mut text = Text::new();
    let (zeros, exponent) = text.push_exponent_unpadded(magnitude, decimals, f.precision());
    let mut suffix = Text::new();
    suffix.push(&[letter]);
    suffix.push(exponent_digits(exponent).as_bytes());
    pad_number(f, !negative || magnitude == 0, text.as_str(), zeros, suffix.as_str())
}

/// An exponent's digits, with its sign.
struct Exponent {
    /// The text.
    bytes: [u8; 12],
    /// Its length.
    len: usize,
}

impl Exponent {
    /// The text's bytes.
    fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }
}

/// The digits of an exponent, below 100 in magnitude, with a `-` for a negative one.
#[expect(clippy::arithmetic_side_effects, reason = "an exponent below 100 in magnitude")]
fn exponent_digits(exponent: i32) -> Exponent {
    let mut text = Exponent { bytes: [0; 12], len: 0 };
    let mut push = |byte: u8| {
        if let Some(slot) = text.bytes.get_mut(text.len) {
            *slot = byte;
            text.len += 1;
        }
    };
    if exponent < 0 {
        push(b'-');
    }
    let magnitude = exponent.unsigned_abs();
    if magnitude >= 10 {
        push(b'0' + u8::try_from(magnitude / 10).unwrap_or_default());
    }
    push(b'0' + u8::try_from(magnitude % 10).unwrap_or_default());
    text
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// Writes the value's shortest exact decimal into `out`, as `Display` does, with no formatter:
    /// the length written, or `None` when the text does not fit; no byte past the text is touched.
    /// [`MAX_ASCII_LEN`] bytes hold every value's.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, MAX_ASCII_LEN, dec};
    ///
    /// let price: D64<7> = dec!(60000.5);
    /// let mut out = [0; MAX_ASCII_LEN];
    /// let len = price.write_ascii(&mut out).expect("room enough");
    /// assert_eq!(&out[..len], b"60000.5");
    /// ```
    #[inline]
    pub fn write_ascii(self, out: &mut [u8]) -> Option<usize> {
        let (negative, magnitude) = self.steps().sign_and_magnitude();
        let decimals = self.decimals();
        match u64::try_from(magnitude) {
            // Stored eight bytes at a time, past the text, so through the stack: `out` is written
            // with the text alone, and nothing past it.
            Ok(narrow) if decimals <= 19 => {
                let mut room = [0; EIGHT_BYTE_ROOM];
                let len = write_ascii_narrow(negative, narrow, decimals, &mut room)?;
                out.get_mut(..len)?.copy_from_slice(room.get(..len)?);
                Some(len)
            },
            Err(_past_a_word) if decimals <= 19 => {
                let mut room = [0; EIGHT_BYTE_ROOM];
                match write_ascii_split(negative, magnitude, decimals, &mut room) {
                    Some(len) => {
                        out.get_mut(..len)?.copy_from_slice(room.get(..len)?);
                        Some(len)
                    },
                    None => write_ascii_wide(negative, magnitude, decimals, out),
                }
            },
            Ok(_) | Err(_) => write_ascii_wide(negative, magnitude, decimals, out),
        }
    }
}

/// The shortest exact decimal; `{:#}` writes every decimal of the scale; `{:.N}` exactly `N`
/// decimals, a tie rounded to even as `f64`'s are; width, fill and sign as an integer's.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// let price: D64<4> = dec!(60000.5);
/// assert_eq!(format!("{price}"), "60000.5", "the shortest exact decimal");
/// assert_eq!(format!("{price:#}"), "60000.5000", "every decimal of the scale");
/// assert_eq!(format!("{price:.0}"), "60000", "a tie, to the even whole");
/// assert_eq!(format!("{price:>+10.2}"), " +60000.50", "width, sign and precision");
/// ```
impl<I: Int, S: Scale> fmt::Display for Decimal<I, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = self.decimals();
        let precision = f.precision().or_else(|| f.alternate().then_some(usize::from(decimals)));
        write(f, self.steps(), decimals, precision)
    }
}

/// As `{:#}`: every decimal of the scale, so a run-time value's scale shows.
impl<I: Int, S: Scale> fmt::Debug for Decimal<I, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let decimals = self.decimals();
        let all = usize::from(decimals);
        write(f, self.steps(), decimals, Some(f.precision().unwrap_or(all)))
    }
}

/// Scientific notation, as `f64`'s: the shortest exact digits, or `{:.N}` of them after the point,
/// a tie rounded to even.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// let x: D64<4> = dec!(1234.5);
/// assert_eq!(format!("{x:e}"), "1.2345e3");
/// assert_eq!(format!("{x:.2e}"), "1.23e3", "rounded to two decimals");
/// ```
impl<I: Int, S: Scale> fmt::LowerExp for Decimal<I, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_exponent(f, self.steps(), self.decimals(), b'e')
    }
}

/// As [`LowerExp`](fmt::LowerExp), with `E`.
impl<I: Int, S: Scale> fmt::UpperExp for Decimal<I, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_exponent(f, self.steps(), self.decimals(), b'E')
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::str;

    use proptest::prelude::*;
    use rstest::rstest;

    use super::MAX_ASCII_LEN;
    use crate::{D64, D128, Decimal, Dynamic, Fixed};

    /// Seven decimals.
    type Price = D64<7>;

    #[rstest]
    #[case::narrow(1_500_000_000_000_000_000_i128, "1.5")]
    #[case::negative(-1, "-0.000000000000000001")]
    #[case::largest(i128::MAX, "170141183460469231731.687303715884105727")]
    #[case::smallest(i128::MIN, "-170141183460469231731.687303715884105728")]
    fn a_wide_value_writes_ascii_as_display_does(#[case] steps: i128, #[case] text: &str) {
        let amount = D128::<18>::from_steps(steps, Fixed);
        let mut out = [0; MAX_ASCII_LEN];
        let len = amount.write_ascii(&mut out).expect("room enough");
        assert_eq!(
            (str::from_utf8(&out[..len]), format!("{amount}").as_str()),
            (Ok(text), text),
            "{steps}"
        );
    }

    #[rstest]
    #[case::whole(600_000_000_000_i64, "60000")]
    #[case::fraction(600_005_000_000, "60000.5")]
    #[case::smallest(1, "0.0000001")]
    #[case::negative(-376_300_000, "-37.63")]
    #[case::zero(0, "0")]
    #[case::largest(i64::MAX, "922337203685.4775807")]
    #[case::smallest_negative(i64::MIN, "-922337203685.4775808")]
    fn a_value_writes_its_shortest_exact_decimal(#[case] steps: i64, #[case] text: &str) {
        assert_eq!(format!("{}", Price::from_steps(steps, Fixed)), text, "{steps}");
    }

    #[test]
    fn nothing_past_the_text_is_written() {
        let mut frame = [b'#'; 64];
        let len =
            Price::from_steps(15_000_000, Fixed).write_ascii(&mut frame).expect("room enough");
        assert_eq!(
            (&frame[..len], frame[len]),
            (&b"1.5"[..], b'#'),
            "the byte after the text, kept"
        );
    }

    #[test]
    fn a_buffer_the_text_fits_is_enough() {
        let price = Price::from_steps(600_005_000_000, Fixed);
        let mut exact = [0; 7];
        assert_eq!(price.write_ascii(&mut exact), Some(7), "seven bytes for `60000.5`");
        assert_eq!(&exact, b"60000.5");
        assert_eq!(price.write_ascii(&mut [0; 6]), None, "but not six");
        let longest = D128::<38>::from_steps(i128::MIN, Fixed);
        let mut out = [0; MAX_ASCII_LEN];
        assert_eq!(longest.write_ascii(&mut out), Some(MAX_ASCII_LEN), "the longest text fills it");
    }

    #[test]
    fn a_value_honours_the_format_spec() {
        let price = Price::from_steps(600_005_000_000, Fixed);
        assert_eq!(format!("{price:.3}"), "60000.500", "pads to the precision");
        assert_eq!(format!("{price:.0}"), "60000", "a half rounds to the even digit");
        assert_eq!(format!("{price:>12}"), "     60000.5", "a width");
        assert_eq!(format!("{price:012.2}"), "000060000.50", "zero-padded");
        assert_eq!(format!("{price:+}"), "+60000.5", "a sign");
        assert_eq!(format!("{price:#}"), "60000.5000000", "every decimal of the scale");
        assert_eq!(format!("{price:?}"), "60000.5000000", "debug, as the alternate form");
        assert_eq!(format!("{price:.200}"), format!("60000.5{}", "0".repeat(199)), "200 decimals");
    }

    #[test]
    fn a_run_time_value_shows_its_scale_in_debug() {
        let value = Decimal::<i64, Dynamic>::from_steps(150, Dynamic::new(2).expect("at most 38"));
        assert_eq!((format!("{value}"), format!("{value:?}")), ("1.5".into(), "1.50".into()));
    }

    #[test]
    fn zeros_past_the_decimals_are_padded_as_digits() {
        let price = Price::from_steps(-600_005_000_000, Fixed);
        assert_eq!(format!("{price:>19.9}"), "   -60000.500000000", "right, as a number");
        assert_eq!(format!("{price:<19.9}"), "-60000.500000000   ", "left");
        assert_eq!(format!("{price:^19.9}"), " -60000.500000000  ", "centred, the odd one after");
        assert_eq!(format!("{price:*^19.9}"), "*-60000.500000000**", "a fill");
        assert_eq!(format!("{price:019.9}"), "-00060000.500000000", "zeros after the sign");
    }

    #[rstest]
    #[case::fraction(12_345_000, 4, "1.2345e3", "1.2345E3")]
    #[case::small(5, 4, "5e-4", "5E-4")]
    #[case::zero(0, 4, "0e0", "0E0")]
    #[case::negative(-12, 0, "-1.2e1", "-1.2E1")]
    fn a_value_writes_scientific_notation(
        #[case] steps: i64, #[case] decimals: u8, #[case] lower: &str, #[case] upper: &str,
    ) {
        let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
        assert_eq!((format!("{value:e}"), format!("{value:E}")), (lower.into(), upper.into()));
    }

    #[test]
    fn scientific_notation_rounds_to_its_precision() {
        let x = D64::<4>::from_steps(12_345_000, Fixed);
        assert_eq!(format!("{x:.2e}"), "1.23e3", "1.2345, to two");
        assert_eq!(format!("{x:.3e}"), "1.234e3", "a tie, to the even digit");
        assert_eq!(format!("{x:.6e}"), "1.234500e3", "padded");
        assert_eq!(
            format!("{:.0e}", D64::<0>::from_steps(95, Fixed)),
            "1e2",
            "the carry moves the exponent"
        );
    }

    proptest! {
        #[test]
        fn the_eight_digit_writer_agrees_with_the_general_one(magnitude: u64, decimals in 0_u8..=18) {
            let mut bytes = [0; super::EIGHT_BYTE_ROOM];
            let len = super::write_ascii_narrow(false, magnitude, decimals, &mut bytes).expect("room enough");
            let mut text = super::Text::new();
            let _no_zeros = text.push_decimal_unpadded(u128::from(magnitude), decimals, None);
            prop_assert_eq!(str::from_utf8(&bytes[..len]).expect("ASCII"), text.as_str());
        }

        #[test]
        fn the_split_writer_agrees_with_the_general_one(raw: u128, bits in 1_u32..=128, decimals in 0_u8..=19, negative: bool) {
            let magnitude = raw.unbounded_shr(128_u32.wrapping_sub(bits));
            let mut bytes = [0; super::EIGHT_BYTE_ROOM];
            let mut text = super::Text::new();
            if negative && magnitude != 0 {
                text.push(b"-");
            }
            let _no_zeros = text.push_decimal_unpadded(magnitude, decimals, None);
            if let Some(len) = super::write_ascii_split(negative, magnitude, decimals, &mut bytes) {
                prop_assert_eq!(str::from_utf8(&bytes[..len]).expect("ASCII"), text.as_str());
            } else {
                // Only an integer past 10^16 words is left to the general writer.
                let integer = magnitude.checked_div(10_u128.pow(u32::from(decimals))).unwrap_or(0);
                prop_assert!((integer >> 64) >= 10_u128.pow(16), "{}", text.as_str());
            }
        }

        #[test]
        fn scientific_notation_reads_back_as_the_value(steps: i64, decimals in 0_u8..=18) {
            let value = Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"));
            let text = format!("{value:e}");
            let back = Decimal::<i64, Dynamic>::from_ascii(text.as_bytes(), Dynamic::new(decimals).expect("at most 38"));
            prop_assert_eq!(back, Ok(value), "{}", text);
        }
    }
}
