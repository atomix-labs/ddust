//! Eight digits at a time in a 64-bit word (SWAR): the portable kernels the reader and the writer
//! are built on, and the twins any SIMD kernel is held to.

use crate::word::pow10_u128;

/// ASCII `'0'` in every byte.
pub(crate) const ZEROS: u64 = 0x3030_3030_3030_3030;
/// `0x01` in every byte.
pub(crate) const ONES: u64 = 0x0101_0101_0101_0101;
/// `0x80` in every byte.
pub(crate) const HIGHS: u64 = 0x8080_8080_8080_8080;
/// ASCII `'.'` in every byte.
pub(crate) const DOTS: u64 = 0x2E2E_2E2E_2E2E_2E2E;
/// `10^8`, one group's worth of places.
const GROUP: u128 = 100_000_000;

/// Whether all eight bytes are ASCII digits.
#[inline]
pub(crate) const fn all_digits(word: u64) -> bool {
    (word.wrapping_add(0x4646_4646_4646_4646) | word.wrapping_sub(ZEROS)) & HIGHS == 0
}

/// The value of eight ASCII digits, the first in the lowest byte: three multiplies (Lemire,
/// *Quickly parsing eight digits*, 2018).
#[inline]
pub(crate) const fn eight_digits(word: u64) -> u64 {
    let word = (word & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561).wrapping_shr(8);
    let word = (word & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601).wrapping_shr(16);
    (word & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001).wrapping_shr(32)
}

/// The low `count` bytes set, for `count` from 0 to 8.
#[inline]
pub(crate) const fn low_bytes(count: usize) -> u64 {
    u64::MAX.unbounded_shr(64_u32.wrapping_sub(bits(count)))
}

/// `count` bytes in bits, for a count of a word's bytes or a little past.
#[inline]
const fn bits(count: usize) -> u32 {
    match u32::try_from(count) {
        Ok(count) => count.wrapping_mul(8),
        Err(_too_many) => u32::MAX,
    }
}

/// The eight bytes of `text` from `start`, the first in the lowest byte, for a `start` at least
/// eight bytes before its end.
#[inline]
pub(crate) fn load(text: &[u8], start: usize) -> u64 {
    text.get(start..start.wrapping_add(8))
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
        .map_or(0, u64::from_le_bytes)
}

/// A text of fewer than eight bytes as one word, the first in the lowest byte and zeros past its
/// end, from two loads that overlap where it is not a power of two long: `None` when it is empty.
#[inline]
pub(crate) fn short(text: &[u8]) -> Option<u64> {
    let len = text.len();
    match len {
        0 => None,
        1 => text.first().map(|&only| u64::from(only)),
        2 | 3 => {
            let low = u16::from_le_bytes(text.get(..2)?.try_into().ok()?);
            let high = u16::from_le_bytes(text.get(len.wrapping_sub(2)..)?.try_into().ok()?);
            Some(u64::from(low) | u64::from(high).unbounded_shl(bits(len.wrapping_sub(2))))
        },
        _ => {
            let low = u32::from_le_bytes(text.get(..4)?.try_into().ok()?);
            let high = u32::from_le_bytes(text.get(len.wrapping_sub(4)..)?.try_into().ok()?);
            Some(u64::from(low) | u64::from(high).unbounded_shl(bits(len.wrapping_sub(4))))
        },
    }
}

/// `text`'s eight bytes from any `start`, zeros past its end, without a load past it.
trait Window {
    /// The eight bytes from `start`.
    fn at(&self, start: usize) -> u64;
}

/// A text of eight bytes or more: a window past its last eight is those, shifted down.
struct Long<'a>(&'a [u8]);

impl Window for Long<'_> {
    #[inline]
    fn at(&self, start: usize) -> u64 {
        let from = start.min(self.0.len().wrapping_sub(8));
        load(self.0, from).unbounded_shr(bits(start.wrapping_sub(from)))
    }
}

/// A text of fewer than eight bytes, already one word.
struct Short(u64);

impl Window for Short {
    #[inline]
    fn at(&self, start: usize) -> u64 {
        self.0.unbounded_shr(bits(start))
    }
}

/// The magnitude at `decimals` of `text`, `[digits][.digits]` with no sign, when it fits and its
/// digits past the scale are zeros; `None` for any other text, which the general reader then
/// reads or refuses.
///
/// Eight bytes at a time, and never past the text: each group is a window, masked to `'0'`
/// where it runs past its digits, so one test says all eight are digits and three multiplies read
/// them. The integer's groups end at the point; the fraction's are aligned to the scale's
/// places, so a group never needs scaling after it is read.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: inline, a static scale's decimals fold, and the reader runs fewer instructions"
)]
pub(crate) fn read_plain(text: &[u8], decimals: u8) -> Option<u128> {
    if text.len() >= 8 {
        read(&Long(text), text.len(), decimals)
    } else {
        read(&Short(short(text)?), text.len(), decimals)
    }
}

/// [`read_plain`] over a text of `len` bytes, as `window` gives them.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: inline, a static scale's decimals fold, and the reader runs fewer instructions"
)]
fn read(window: &impl Window, len: usize, decimals: u8) -> Option<u128> {
    let dot = point(window, len);
    let fraction_len = len.saturating_sub(dot.wrapping_add(1));
    if dot == 0 && fraction_len == 0 {
        return None;
    }
    let integer = integer(window, dot)?;
    let fraction = fraction(window, dot.wrapping_add(1), fraction_len, decimals)?;
    let power = pow10_u128(decimals)?;
    let scaled = match (u64::try_from(integer), u64::try_from(power)) {
        (Ok(integer), Ok(power)) => u128::from(integer).wrapping_mul(u128::from(power)),
        _ => integer.checked_mul(power)?,
    };
    scaled.checked_add(fraction)
}

/// Where the first `.` is, or `len` when there is none.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: inline, a static scale's decimals fold, and the reader runs fewer instructions"
)]
fn point(window: &impl Window, len: usize) -> usize {
    let mut start = 0;
    while start < len {
        let probe = window.at(start) ^ DOTS;
        let dots = probe.wrapping_sub(ONES) & !probe & HIGHS;
        if dots != 0 {
            // The first match is exact: a false one only ever follows a true one.
            let byte = usize::try_from(dots.trailing_zeros() / 8).unwrap_or(8);
            return start.wrapping_add(byte).min(len);
        }
        start = start.wrapping_add(8);
    }
    len
}

/// The value of the `dot` digits before the point, in groups of eight that end at it, the first
/// padded with leading `'0'`s: `None` past 38 digits, or for a byte that is not a digit.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: inline, a static scale's decimals fold, and the reader runs fewer instructions"
)]
fn integer(window: &impl Window, dot: usize) -> Option<u128> {
    if dot > 38 {
        return None;
    }
    let lead = dot % 8;
    let mut value = if lead > 0 {
        let pad = low_bytes(8_usize.wrapping_sub(lead));
        let shifted = window.at(0).unbounded_shl(bits(8_usize.wrapping_sub(lead)));
        let word = (shifted & !pad) | (ZEROS & pad);
        if !all_digits(word) {
            return None;
        }
        eight_digits(word)
    } else {
        0
    };
    let mut start = lead;
    // Two groups at most in a `u64`, below 10^16, so a short integer never takes a `u128` step.
    if start < dot && start <= 8 {
        let word = window.at(start);
        if !all_digits(word) {
            return None;
        }
        value = value.wrapping_mul(100_000_000).wrapping_add(eight_digits(word));
        start = start.wrapping_add(8);
    }
    let mut value = u128::from(value);
    while start < dot {
        let word = window.at(start);
        if !all_digits(word) {
            return None;
        }
        value = value.wrapping_mul(GROUP).wrapping_add(u128::from(eight_digits(word)));
        start = start.wrapping_add(8);
    }
    Some(value)
}

/// The value at `decimals` places of the `len` fraction digits from `from`: `None` for a byte
/// that is not a digit, or a digit past the scale that is not zero.
///
/// The places are read in groups aligned to their end, so the last digit read is worth one step:
/// the first group is `decimals % 8` places, padded with leading `'0'`s, and a group is padded with
/// trailing `'0'`s where the digits end before the places do.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "measured: inline, a static scale's decimals fold, and the reader runs fewer instructions"
)]
fn fraction(window: &impl Window, from: usize, len: usize, decimals: u8) -> Option<u128> {
    let places = usize::from(decimals);
    let read = len.min(places);
    // Past the scale, zeros only.
    let mut past = read;
    while past < len {
        let keep = low_bytes(len.wrapping_sub(past).min(8));
        if (window.at(from.wrapping_add(past)) & keep) | (ZEROS & !keep) != ZEROS {
            return None;
        }
        past = past.wrapping_add(8);
    }
    let group = |place: usize, width: usize| {
        let digits = read.saturating_sub(place).min(width);
        let keep = low_bytes(digits);
        let word = (window.at(from.wrapping_add(place)) & keep) | (ZEROS & !keep);
        let pad = low_bytes(8_usize.wrapping_sub(width));
        let word = (word.unbounded_shl(bits(8_usize.wrapping_sub(width))) & !pad) | (ZEROS & pad);
        all_digits(word).then(|| u128::from(eight_digits(word)))
    };
    let lead = places % 8;
    let mut value = if lead > 0 { group(0, lead)? } else { 0 };
    let mut place = lead;
    while place < places {
        value = value.wrapping_mul(GROUP).wrapping_add(group(place, 8)?);
        place = place.wrapping_add(8);
    }
    Some(value)
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference's own arithmetic, in a u128")]
mod tests {
    use alloc::format;

    use proptest::prelude::*;

    use super::{all_digits, eight_digits, read_plain, short};

    #[test]
    fn eight_digits_read_as_a_number() {
        let word = u64::from_le_bytes(*b"12345678");
        assert!(all_digits(word), "eight digits");
        assert_eq!(eight_digits(word), 12_345_678);
        assert!(!all_digits(u64::from_le_bytes(*b"1234.678")), "a point is no digit");
    }

    #[test]
    fn a_short_text_is_one_word() {
        for text in [&b"1"[..], b"12", b"123", b"1234", b"12345", b"123456", b"1234567"] {
            let mut bytes = [0; 8];
            bytes[..text.len()].copy_from_slice(text);
            assert_eq!(short(text), Some(u64::from_le_bytes(bytes)), "{text:?}");
        }
        assert_eq!(short(b""), None, "nothing");
    }

    proptest! {
        #[test]
        fn a_plain_text_reads_as_its_digits_say(integer in 0_u64..10_000_000_000_000, fraction in 0_u64..100_000_000, written in 0_usize..=8, decimals in 0_u8..=18) {
            // The fraction written with `written` places, and read at `decimals`.
            let digits = format!("{fraction:08}");
            let (shown, past) = (digits.get(..written).unwrap_or_default(), digits.get(usize::from(decimals).min(written)..written).unwrap_or_default());
            let text = format!("{integer}.{shown}");
            let kept: u128 = shown.parse().unwrap_or(0);
            let (places, written_places) = (u32::from(decimals), u32::try_from(written).expect("at most 8"));
            let whole = u128::from(integer) * 10_u128.pow(places);
            let expected = if places >= written_places {
                Some(whole + kept * 10_u128.pow(places - written_places))
            } else if past.bytes().all(|b| b == b'0') {
                Some(whole + kept / 10_u128.pow(written_places - places))
            } else {
                None
            };
            prop_assert_eq!(read_plain(text.as_bytes(), decimals), expected, "{}", text);
        }
    }
}
