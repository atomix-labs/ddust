//! The kernels in SSSE3, on `x86_64`: compiled in where the build's CPU has it, or chosen at run
//! time with the `runtime-dispatch` feature.
//!
//! Each is a `#[target_feature(enable = "ssse3")]` function, in which the intrinsics are safe to
//! call, and its callers are the only `unsafe`: calling it is sound where SSSE3 is enabled, or
//! where the CPU was checked for it.

#![expect(
    unsafe_code,
    reason = "each kernel is called from code without its `#[target_feature]`: sound where the \
              build enables SSSE3, or after the CPU is checked for it"
)]

use core::arch::x86_64::{
    __m128i, _mm_add_epi8, _mm_and_si128, _mm_andnot_si128, _mm_cmpeq_epi8, _mm_cmpgt_epi8,
    _mm_cmplt_epi8, _mm_cvtsi128_si64, _mm_madd_epi16, _mm_maddubs_epi16, _mm_min_epu8,
    _mm_movemask_epi8, _mm_or_si128, _mm_set_epi64x, _mm_set1_epi8, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_srli_si128, _mm_sub_epi8, _mm_xor_si128,
};

use crate::text::swar::{load, short};

/// The short texts [`super::read_plain`] reads, in SSSE3: as the NEON kernel reads them, the point
/// found by one compare and the steps' digits put in place by one shuffle whose indices the point
/// and the scale give.
#[cfg(target_feature = "ssse3")]
#[inline]
pub(super) fn read_short(text: &[u8], decimals: u8) -> Option<u64> {
    let (len, low, high) = words(text, decimals)?;
    // SAFETY: SSSE3 is enabled in this build, which `super::read_plain`'s `cfg` checks before it
    // calls.
    unsafe { read16(low, high, len, decimals) }
}

/// The short texts [`super::read_plain`] reads, in SSSE3, for a CPU checked for it.
#[cfg(all(target_feature = "sse2", not(target_feature = "ssse3"), feature = "runtime-dispatch"))]
#[inline]
pub(super) fn read_short_checked(text: &[u8], decimals: u8) -> Option<u64> {
    let (len, low, high) = words(text, decimals)?;
    // SAFETY: the CPU has SSSE3, which `super::read_plain` checked by CPUID before it calls.
    unsafe { read16(low, high, len, decimals) }
}

/// The text's length and its two words, the first eight bytes and the last eight, or the one word
/// a shorter text is: `None` past 16 bytes or 16 decimals, or for an empty text.
#[inline]
fn words(text: &[u8], decimals: u8) -> Option<(u8, u64, u64)> {
    let len = u8::try_from(text.len()).ok().filter(|&len| len <= 16)?;
    if decimals > 16 {
        return None;
    }
    if len >= 8 {
        Some((len, load(text, 0), load(text, usize::from(len).wrapping_sub(8))))
    } else {
        Some((len, short(text)?, 0))
    }
}

/// A byte's every lane set to `byte`.
#[target_feature(enable = "ssse3")]
fn splat(byte: u8) -> __m128i {
    _mm_set1_epi8(i8::from_ne_bytes([byte]))
}

/// Reads the `len` bytes that `low` holds first and `high` holds last, at `decimals`, as the NEON
/// kernel does. `pshufb` zeroes a lane only when its index has the top bit set, and reads any other
/// index modulo 16, so an index past the text is given the top bit.
#[target_feature(enable = "ssse3")]
#[expect(clippy::arithmetic_side_effects, reason = "counts of at most 16 lanes, and 16 decimals")]
fn read16(low: u64, high: u64, len: u8, decimals: u8) -> Option<u64> {
    let iota = _mm_setr_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
    let in_text = _mm_cmplt_epi8(iota, splat(len));
    let upper = _mm_and_si128(_mm_cmpgt_epi8(iota, splat(7)), splat(16_u8.wrapping_sub(len)));
    let order = _mm_add_epi8(iota, upper);
    let order = _mm_or_si128(_mm_and_si128(in_text, order), _mm_andnot_si128(in_text, splat(0x80)));
    let words = _mm_set_epi64x(
        i64::from_ne_bytes(high.to_ne_bytes()),
        i64::from_ne_bytes(low.to_ne_bytes()),
    );
    let text = _mm_shuffle_epi8(words, order);
    // The point: the first `.`, by a bit a lane.
    let points =
        u32::from(u16::try_from(_mm_movemask_epi8(_mm_cmpeq_epi8(text, splat(b'.')))).ok()?);
    let point = if points == 0 { len } else { u8::try_from(points.trailing_zeros()).ok()? };
    let fraction_len = len.saturating_sub(point + 1);
    if point == 0 && fraction_len == 0 {
        return None;
    }
    // Every byte a digit but the point, and every digit past the scale a zero.
    let values = _mm_sub_epi8(text, splat(b'0'));
    let digit = _mm_cmpeq_epi8(_mm_min_epu8(values, splat(9)), values);
    let not_digit = _mm_andnot_si128(digit, in_text);
    let at_point = _mm_and_si128(_mm_cmpeq_epi8(iota, splat(point)), in_text);
    let digits = _mm_and_si128(values, in_text);
    let first_past = (point + 1).saturating_add(decimals).min(17);
    let past = _mm_and_si128(_mm_cmpgt_epi8(iota, splat(first_past.wrapping_sub(1))), digits);
    if _mm_movemask_epi8(_mm_xor_si128(not_digit, at_point)) != 0
        || _mm_movemask_epi8(_mm_cmpeq_epi8(past, splat(0))) != 0xFFFF
    {
        return None;
    }
    // The steps' digits right-aligned, as the NEON kernel puts them; the one index that can reach
    // 16, the last place's, past the text, gets the top bit, which a lane before the first has.
    let width = point + decimals;
    if width > 16 {
        return None;
    }
    let past_point = _mm_and_si128(_mm_cmpgt_epi8(iota, splat(15 - decimals)), splat(1));
    let index = _mm_add_epi8(_mm_add_epi8(iota, splat(width.wrapping_sub(16))), past_point);
    let index = _mm_or_si128(index, _mm_and_si128(_mm_cmpgt_epi8(index, splat(15)), splat(0x80)));
    let steps = _mm_shuffle_epi8(digits, index);
    // Sixteen digits to a number: pairs, fours, then two halves of eight.
    let pairs = _mm_maddubs_epi16(
        steps,
        _mm_setr_epi8(10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1),
    );
    let fours = _mm_madd_epi16(
        pairs,
        _mm_setr_epi8(100, 0, 1, 0, 100, 0, 1, 0, 100, 0, 1, 0, 100, 0, 1, 0),
    );
    let lane = |halves: __m128i| u64::from_ne_bytes(_mm_cvtsi128_si64(halves).to_ne_bytes());
    let half = |lane: u64| (lane & 0xFFFF_FFFF) * 10_000 + (lane >> 32);
    Some(half(lane(fours)) * 100_000_000 + half(lane(_mm_srli_si128::<8>(fours))))
}
