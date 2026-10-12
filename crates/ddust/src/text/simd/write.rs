//! The vector writer, in `core::simd`: a value's shortest exact decimal from the 32 digits of its
//! steps, each window of its text one lookup into them, whose indices a row for the scale gives,
//! and every store within the text.

use core::simd::prelude::*;
use core::simd::{ToBytes, simd_swizzle};

use super::lanes::{JOIN, Lanes};
use super::swar::{ZEROS, digits8};
use crate::reciprocal;
use crate::word::pow10_u128;

/// `10^16`: the steps' two halves of 16 digits.
const SIXTEEN: u64 = 10_000_000_000_000_000;

/// Eight `'0'`s but a point in the first: the top half's first eight lanes, whose first, a leading
/// zero of every value the writer takes, holds the point the rows look up.
const POINTED_ZEROS: u64 = (ZEROS & !0xFF) | 0x2E;

/// For each scale `D` up to 30, the lane in the 32 digits of each place of the unsigned text, at
/// `48 + v` for the place `v` past the point: `v + 32 − D` before it, `v + 31 − D` after it, lane
/// 0, which holds the point, at it, and 255, which a lookup reads as zero, past the digits.
///
/// Lane 0 is a leading zero while the integer, `n` digits from lane `32 − D − n`, starts at lane 1
/// or later: for every value below `10^31`, up to 30 decimals.
#[expect(
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "indices below the table's bounds, and lanes below 32"
)]
static ROWS: [[u8; 112]; 31] = {
    let mut rows = [[u8::MAX; 112]; 31];
    let mut decimals = 0_usize;
    while decimals < 31 {
        let mut place = 0_usize;
        while place < 112 {
            let lane = if place < 48 {
                (place + 32).checked_sub(48 + decimals)
            } else if place == 48 {
                Some(0)
            } else {
                (place + 31).checked_sub(48 + decimals)
            };
            if let Some(lane) = lane
                && lane < 32
            {
                rows[decimals][place] = lane as u8;
            }
            place += 1;
        }
        decimals += 1;
    }
    rows
};

/// How many decimal digits `x` has, none for zero: from its bit length, and one comparison.
///
/// By hand: as `u64::checked_ilog10`, a narrow value wrote in 12.25 ns,
/// not this form's 9.8 (`bench/results/2026-10-09T04-50Z-pr13-ilog10-graviton4`).
#[inline]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a guess of at most 19"
)]
const fn digit_count(x: u64) -> u8 {
    // floor(bits × log10 2) by 1233 / 4096, below the digit count by at most one.
    let bits = 64_u32.wrapping_sub((x | 1).leading_zeros());
    let guess = bits.wrapping_mul(1233) >> 12;
    let guess = if guess > 19 { 19 } else { guess as u8 };
    match pow10_u128(guess) {
        Some(power) if u128::from(x) >= power => guess.wrapping_add(1),
        _ => guess,
    }
}

/// Writes `top` and `bottom`, the steps' 32 digits, each half with one past its last nonzero
/// digit's lane, and `count` digits long, at `decimals`, a `-` first when `negative`, into `out`:
/// the length written, or `None` past 30 decimals or where `out` is too short.
///
/// The text is stored in three windows, at its start, its middle and its end, so that none passes
/// it: of 16 bytes for a wide value or a text past 24 bytes, of 8 for any other; a text shorter
/// than its window in two of 8 or of 4 bytes, or in one.
#[inline(always)]
#[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or its lookups are calls")]
#[expect(clippy::arithmetic_side_effects, reason = "counts of at most 48 places, and 30 decimals")]
fn write<const WIDE: bool>(
    lanes: Lanes, negative: bool, top: (u8x16, u8), bottom: (u8x16, u8), count: u8, decimals: u8,
    out: &mut [u8],
) -> Option<usize> {
    let row = ROWS.get(usize::from(decimals))?;
    let integer = usize::from(count.saturating_sub(decimals).max(1));
    // The fraction's places up to its last nonzero digit, in the bottom half while it has one.
    let last = if bottom.1 != 0 {
        bottom.1 + 16
    } else if decimals > 16 {
        top.1
    } else {
        0
    };
    let fraction = usize::from(last.saturating_sub(32 - decimals));
    let unsigned = integer + if fraction > 0 { fraction + 1 } else { 0 };
    let sign = usize::from(negative);
    let len = sign + unsigned;
    let out = out.get_mut(..len)?;
    *out.first_mut()? = b'-';
    let text = out.get_mut(sign..)?;
    let digits: u8x32 = simd_swizzle!(top.0, bottom.0, JOIN);
    // The unsigned text's `width` bytes from `place`.
    let mut store = |place: usize, width: usize| -> Option<()> {
        let start = place + 48 - integer;
        let index = u8x16::from_array(row.get(start..start + 16)?.try_into().ok()?);
        let window = lanes.lookup32(digits, index.resize::<32>(u8::MAX)).extract::<0, 16>();
        text.get_mut(place..place + width)?.copy_from_slice(window.as_array().get(..width)?);
        Some(())
    };
    let width = if WIDE || unsigned > 24 { 16 } else { 8 };
    if unsigned >= width {
        for place in [0, (unsigned - width).min(width), unsigned - width] {
            store(place, width)?;
        }
    } else if unsigned >= 8 {
        store(0, 8)?;
        store(unsigned - 8, 8)?;
    } else if unsigned >= 4 {
        store(0, 4)?;
        store(unsigned - 4, 4)?;
    } else {
        store(0, unsigned)?;
    }
    Some(len)
}

/// Writes the steps `magnitude` at `decimals`, a `-` first when `negative`, into `out`: as
/// [`write`](fn@write) does, the top half, below 1845, by the SWAR writer, and
/// the bottom by the unit.
#[inline(always)]
#[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or its lookups are calls")]
pub(super) fn narrow(
    lanes: Lanes, negative: bool, magnitude: u64, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    let negative = negative && magnitude != 0;
    let (high, low) = (magnitude / SIXTEEN, magnitude % SIXTEEN);
    let digits = digits8(high);
    let top = u64x2::from_array([POINTED_ZEROS, digits | ZEROS]).to_le_bytes();
    let top_last = if digits == 0 { 0 } else { 16_u32.wrapping_sub(digits.leading_zeros() / 8) };
    let top = (top, u8::try_from(top_last).unwrap_or(0));
    write::<false>(lanes, negative, top, lanes.digits16(low), digit_count(magnitude), decimals, out)
}

/// Writes the steps `magnitude` at `decimals`, a `-` first when `negative`, into `out`: as
/// [`write`](fn@write) does, both halves by the unit; `None` too for steps of `10^31` or more,
/// whose first lane is not the point's.
#[inline(always)]
#[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or its lookups are calls")]
pub(super) fn wide(
    lanes: Lanes, negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    let negative = negative && magnitude != 0;
    let (high, low, _) = reciprocal::divide_u128(magnitude, 16)?;
    if high >= SIXTEEN / 10 {
        return None;
    }
    let (top, top_last) = lanes.digits16(high);
    // Lane 0, a leading zero below 10^31, holds the point.
    let top = simd_swizzle!(
        u8x16::splat(b'.'),
        top,
        [0, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]
    );
    let top = (top, top_last);
    let count = if high == 0 { digit_count(low) } else { digit_count(high).wrapping_add(16) };
    write::<true>(lanes, negative, top, lanes.digits16(low), count, decimals, out)
}

/// [`narrow`] in a kernel compiled for SSSE3, so that the lookups' `pshufb`s inline.
#[cfg(lanes = "checked")]
#[inline]
#[expect(unsafe_code, reason = "calls the SSSE3 kernel where `lanes` proves the CPU has SSSE3")]
pub(super) fn narrow_checked(
    lanes: Lanes, negative: bool, magnitude: u64, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    /// The kernel: [`narrow`], with SSSE3 enabled.
    #[target_feature(enable = "ssse3")]
    fn narrow_ssse3(
        lanes: Lanes, negative: bool, magnitude: u64, decimals: u8, out: &mut [u8],
    ) -> Option<usize> {
        narrow(lanes, negative, magnitude, decimals, out)
    }
    // SAFETY: the CPU has SSSE3, as `lanes` proves.
    unsafe { narrow_ssse3(lanes, negative, magnitude, decimals, out) }
}

/// [`wide`] in a kernel compiled for SSSE3, so that the lookups' `pshufb`s inline.
#[cfg(lanes = "checked")]
#[inline]
#[expect(unsafe_code, reason = "calls the SSSE3 kernel where `lanes` proves the CPU has SSSE3")]
pub(super) fn wide_checked(
    lanes: Lanes, negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    /// The kernel: [`wide`], with SSSE3 enabled.
    #[target_feature(enable = "ssse3")]
    fn wide_ssse3(
        lanes: Lanes, negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
    ) -> Option<usize> {
        wide(lanes, negative, magnitude, decimals, out)
    }
    // SAFETY: the CPU has SSSE3, as `lanes` proves.
    unsafe { wide_ssse3(lanes, negative, magnitude, decimals, out) }
}
