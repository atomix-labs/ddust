//! The kernels in NEON, which every aarch64 target has.
//!
//! Each is a `#[target_feature(enable = "neon")]` function, in which the intrinsics are safe to
//! call, and its one caller is the only `unsafe`: calling it is sound wherever NEON is enabled,
//! which the module's `cfg` checks.

#![expect(
    unsafe_code,
    reason = "each kernel is called from code without its `#[target_feature]`: sound, since \
              `cfg(target_feature = \"neon\")` holds wherever the module is compiled"
)]

use core::arch::aarch64::{
    uint8x16_t, vaddq_u8, vandq_u8, vandq_u16, vandq_u32, vbslq_u8, vceqq_u8, vcgeq_u8, vcgtq_u8,
    vcltq_u8, vcombine_u8, vcreate_u8, vdupq_n_u8, vdupq_n_u16, vdupq_n_u32, veorq_u8,
    vget_lane_u64, vgetq_lane_u64, vmaxvq_u8, vmlaq_n_u16, vmlaq_n_u32, vqtbl1q_u8,
    vreinterpret_u64_u8, vreinterpretq_u16_u8, vreinterpretq_u32_u16, vreinterpretq_u64_u32,
    vshrn_n_u16, vshrq_n_u16, vshrq_n_u32, vsubq_u8,
};

use crate::text::swar::{load, short};

/// The lanes' own indices, 0 to 15.
const IOTA: [u64; 2] = [0x0706_0504_0302_0100, 0x0F0E_0D0C_0B0A_0908];

/// The short texts [`super::read_plain`] reads, on NEON: the text in one vector from two loads, the
/// point found by one compare, and the steps' digits put in place by one table lookup whose indices
/// the point and the scale give, so no load waits on where the point is.
#[inline]
pub(super) fn read_short(text: &[u8], decimals: u8) -> Option<u64> {
    let len = u8::try_from(text.len()).ok().filter(|&len| len <= 16)?;
    if decimals > 16 {
        return None;
    }
    let (low, high) = if len >= 8 {
        (load(text, 0), load(text, usize::from(len).wrapping_sub(8)))
    } else {
        (short(text)?, 0)
    };
    // SAFETY: NEON is enabled, as this module's `cfg` requires, so its kernel may be called.
    unsafe { read16(low, high, len, decimals) }
}

/// The 16 lanes of `low` and `high`, the first in lane 0.
#[target_feature(enable = "neon")]
fn vector(low: u64, high: u64) -> uint8x16_t {
    vcombine_u8(vcreate_u8(low), vcreate_u8(high))
}

/// Reads the `len` bytes that `low` holds first and `high` holds last, at `decimals`.
#[target_feature(enable = "neon")]
#[expect(clippy::arithmetic_side_effects, reason = "counts of at most 16 lanes, and 16 decimals")]
fn read16(low: u64, high: u64, len: u8, decimals: u8) -> Option<u64> {
    let iota = vector(IOTA[0], IOTA[1]);
    let in_text = vcltq_u8(iota, vdupq_n_u8(len));
    // The text in order: past lane 7, the high word's bytes, which overlap the low word's by
    // 16 - len.
    let upper = vandq_u8(vcgeq_u8(iota, vdupq_n_u8(8)), vdupq_n_u8(16_u8.wrapping_sub(len)));
    let order = vbslq_u8(in_text, vaddq_u8(iota, upper), vdupq_n_u8(0xFF));
    let text = vqtbl1q_u8(vector(low, high), order);
    // The point: the first `.`, by a nibble a lane.
    let points = vceqq_u8(text, vdupq_n_u8(b'.'));
    let nibbles =
        vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(vreinterpretq_u16_u8(points))));
    let point = if nibbles == 0 { len } else { u8::try_from(nibbles.trailing_zeros() / 4).ok()? };
    let fraction_len = len.saturating_sub(point + 1);
    if point == 0 && fraction_len == 0 {
        return None;
    }
    // Every byte a digit but the point, and every digit past the scale a zero.
    let values = vsubq_u8(text, vdupq_n_u8(b'0'));
    let not_digit = vandq_u8(vcgtq_u8(values, vdupq_n_u8(9)), in_text);
    let at_point = vandq_u8(vceqq_u8(iota, vdupq_n_u8(point)), in_text);
    let digits = vandq_u8(values, in_text);
    let past = vandq_u8(vcgeq_u8(iota, vdupq_n_u8((point + 1).saturating_add(decimals))), digits);
    if vmaxvq_u8(veorq_u8(not_digit, at_point)) != 0 || vmaxvq_u8(past) != 0 {
        return None;
    }
    // The steps' digits right-aligned: lane k takes digit k + width - 16 of them, an integer
    // digit below the point and a place from it, one byte on, past the point. A lane before
    // the first wraps past 15, which the lookup reads as zero, and a place past the text reads
    // the zero there.
    let width = point + decimals;
    if width > 16 {
        return None;
    }
    let past_point = vandq_u8(vcgeq_u8(iota, vdupq_n_u8(16 - decimals)), vdupq_n_u8(1));
    let index = vaddq_u8(vaddq_u8(iota, vdupq_n_u8(width.wrapping_sub(16))), past_point);
    let steps = vqtbl1q_u8(digits, index);
    // Sixteen digits to a number: pairs, fours, then two halves of eight.
    let pairs = vreinterpretq_u16_u8(steps);
    let pairs = vmlaq_n_u16(vshrq_n_u16::<8>(pairs), vandq_u16(pairs, vdupq_n_u16(0xFF)), 10);
    let fours = vreinterpretq_u32_u16(pairs);
    let fours = vmlaq_n_u32(vshrq_n_u32::<16>(fours), vandq_u32(fours, vdupq_n_u32(0xFFFF)), 100);
    let halves = vreinterpretq_u64_u32(fours);
    let half = |lane: u64| (lane & 0xFFFF_FFFF) * 10_000 + (lane >> 32);
    Some(half(vgetq_lane_u64::<0>(halves)) * 100_000_000 + half(vgetq_lane_u64::<1>(halves)))
}
