//! The vector reader, in `core::simd`: a text of at most 32 bytes as one window of 32 lanes, its
//! digits put in their steps' places by one lookup whose indices the point and the scale give, and
//! read as a number by multiplies in the lanes.
//!
//! The lookup is the one instruction `core::simd` leaves to each vector unit, through [`Lanes`].

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::_mm_shuffle_epi8;
use core::ops::Range;
use core::simd::prelude::*;
use core::simd::{ToBytes, simd_swizzle};

use super::swar::{ZEROS, load, load_short};

/// Each lane's own index.
const LANES: u8x32 = u8x32::from_array([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
]);

/// Each lane's index, as a swizzle takes it: two halves joined in order.
const JOIN: [usize; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

/// Proof that the CPU has the lookup's instruction, NEON's `tbl` or SSSE3's `pshufb`: made where
/// the build has it, or where CPUID says the CPU does.
#[derive(Clone, Copy)]
pub(super) struct Lanes(
    // INVARIANT: a `Lanes` exists only where the CPU has the lookup's instruction. Its makers are
    // `BUILT`, under a `cfg` that requires it, and `checked`, once CPUID has found SSSE3.
    (),
);

impl Lanes {
    /// The proof on a build that has the instruction, as rustc's own `cfg` says too: a build whose
    /// flags `build.rs` did not see fails to compile here, rather than run without it.
    #[cfg(all(lanes = "built", any(target_feature = "neon", target_feature = "ssse3")))]
    pub(super) const BUILT: Self = Self(());

    /// The proof where CPUID says the CPU has SSSE3.
    #[cfg(lanes = "checked")]
    #[inline]
    pub(super) fn checked() -> Option<Self> {
        cpu::has_ssse3().then_some(Self(()))
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: one `tbl`.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "one instruction")]
    #[expect(clippy::unused_self, reason = "the proof NEON's lookup needs is the build's own")]
    fn lookup16(self, table: u8x16, index: u8x16) -> u8x16 {
        table.swizzle_dyn(index)
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: a `tbl` over both
    /// halves of the table for each half of `index`.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "two instructions")]
    #[expect(clippy::unused_self, reason = "the proof NEON's lookup needs is the build's own")]
    fn lookup32(self, table: u8x32, index: u8x32) -> u8x32 {
        table.swizzle_dyn(index)
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: one `pshufb`,
    /// which zeroes a lane whose index has its top bit set and reads any other modulo 16, so an
    /// index past 15 is pushed past 127.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or `pshufb` is a call")]
    #[expect(
        unsafe_code,
        reason = "`pshufb`, where core's `swizzle_dyn`, built without SSSE3, is a scalar loop"
    )]
    #[expect(clippy::unused_self, reason = "the proof `pshufb` needs, which its SAFETY reads")]
    fn lookup16(self, table: u8x16, index: u8x16) -> u8x16 {
        let index = index.saturating_add(u8x16::splat(0x70));
        // SAFETY: by the INVARIANT of `Lanes`, the CPU has SSSE3, all `_mm_shuffle_epi8` needs.
        unsafe { _mm_shuffle_epi8(table.into(), index.into()) }.into()
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: each half of the
    /// table looked up in for each half of `index`, and the two joined by an OR.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or `pshufb` is a call")]
    fn lookup32(self, table: u8x32, index: u8x32) -> u8x32 {
        let (low, high) = (table.extract::<0, 16>(), table.extract::<16, 16>());
        let half = |index: u8x16| {
            self.lookup16(low, index) | self.lookup16(high, index - u8x16::splat(16))
        };
        simd_swizzle!(half(index.extract::<0, 16>()), half(index.extract::<16, 16>()), JOIN)
    }
}

/// The digits of `text` right-aligned in 32 lanes, each byte as `byte ^ b'0'`, so a digit is its
/// value and the point 30, and the lanes before the text zeros: `None` for an empty text, a point
/// alone, or a text past 32 bytes.
///
/// From 16 bytes, the text's first 16 and its last 16, the first moved into place by a lookup;
/// below, its first eight bytes and its last eight, or the one word a shorter text is.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline in the SSSE3 kernel, or its lookup's `pshufb` is a call"
)]
fn window(lanes: Lanes, text: &[u8]) -> Option<u8x32> {
    let len = text.len();
    let (first, last) = if len >= 16 {
        let zeros = u8x16::splat(b'0');
        let head = u8x16::from_array(text.get(..16)?.try_into().ok()?) ^ zeros;
        let tail = u8x16::from_array(text.get(len.checked_sub(16)?..)?.try_into().ok()?) ^ zeros;
        // Lane j of the first half is byte `len - 32 + j`: one of the head's, or before the text.
        let start = u8::try_from(len).ok().filter(|&len| len <= 32)?.wrapping_sub(32);
        (lanes.lookup16(head, LANES.extract::<0, 16>() + u8x16::splat(start)), tail)
    } else {
        let (low, high) = if len >= 8 {
            (load(text, 0), load(text, len.wrapping_sub(8)))
        } else if text == b"." {
            return None;
        } else {
            (0, load_short(text)?)
        };
        // Each word moved up to end where the text does, the bytes past it shifted out.
        let bits = |bytes: usize| u32::try_from(bytes.wrapping_mul(8)).unwrap_or(u32::MAX);
        let low = (low ^ ZEROS).unbounded_shl(bits(16_usize.wrapping_sub(len)));
        let high = (high ^ ZEROS).unbounded_shl(bits(8_usize.saturating_sub(len)));
        (u8x16::splat(0), u64x2::from_array([low, high]).to_le_bytes())
    };
    Some(simd_swizzle!(first, last, JOIN))
}

/// The digits at `digits` of `buffer` as [`window`] puts them, from the 16 bytes from their start
/// and the 16 before their end, whatever their length, where the buffer has them; as [`window`]
/// does from the digits alone where it does not.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline in the SSSE3 kernel, or its lookup's `pshufb` is a call"
)]
fn window_in(lanes: Lanes, buffer: &[u8], digits: Range<usize>) -> Option<u8x32> {
    let zeros = u8x16::splat(b'0');
    let (Some(head), Some(tail)) = (
        digits.start.checked_add(16).and_then(|end| buffer.get(digits.start..end)),
        digits.end.checked_sub(16).and_then(|start| buffer.get(start..digits.end)),
    ) else {
        return window(lanes, buffer.get(digits)?);
    };
    let head = u8x16::from_array(head.try_into().ok()?) ^ zeros;
    let tail = u8x16::from_array(tail.try_into().ok()?) ^ zeros;
    let len = u8::try_from(digits.len()).ok().filter(|&len| (1..=32).contains(&len))?;
    if len == 1 && buffer.get(digits.start) == Some(&b'.') {
        return None;
    }
    // Lane j of the first half is byte `len - 32 + j`, and of the last `len - 16 + j`: one of the
    // head's or the tail's, or before the text.
    let first = lanes.lookup16(head, LANES.extract::<0, 16>() + u8x16::splat(len.wrapping_sub(32)));
    let in_text = LANES.extract::<0, 16>().simd_ge(u8x16::splat(16_u8.saturating_sub(len)));
    Some(simd_swizzle!(first, tail & in_text.to_simd().cast(), JOIN))
}

/// The magnitude at `decimals` of `text`, `[digits][.digits]` with no sign, as
/// [`swar::read_plain`](super::swar::read_plain) reads it: for a text of at most 32 bytes whose
/// steps are below `10^32`, at most 32 decimals; `None` for any other text.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline in the SSSE3 kernel, or its lookups' `pshufb`s are calls"
)]
pub(super) fn read(lanes: Lanes, text: &[u8], decimals: u8) -> Option<u128> {
    steps(lanes, window(lanes, text)?, decimals)
}

/// [`read`] for the digits at `digits` of `buffer`, from the 16 bytes from their start and the 16
/// before their end where the buffer has them.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline in the SSSE3 kernel, or its lookups' `pshufb`s are calls"
)]
pub(super) fn read_in(
    lanes: Lanes, buffer: &[u8], digits: Range<usize>, decimals: u8,
) -> Option<u128> {
    steps(lanes, window_in(lanes, buffer, digits)?, decimals)
}

/// The magnitude at `decimals` of `window`: at most 32 decimals and steps below `10^32`, or `None`.
///
/// Lane k of the steps is their digit worth `10^(31 - k)`: the window's lane `k + D - f`, `D` the
/// decimals and `f` the digits after the point, or the lane before it for an integer digit, which
/// the point follows. The steps read the window's span `D - f - 1` to `D - f + 31` but for the
/// point's lane, `31 - f`, which lies inside it while `D` is at most 32; every lane outside the
/// span must be a zero, leading or past the scale.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline in the SSSE3 kernel, or its lookups' `pshufb`s are calls"
)]
fn steps(lanes: Lanes, window: u8x32, decimals: u8) -> Option<u128> {
    if decimals > 32 {
        return None;
    }
    // `f`, or 255, which wraps as -1, for a text without a point. It stays in the lanes: a scalar
    // `D - f` is a round trip through a general register.
    let points = window.simd_eq(u8x32::splat(b'.' ^ b'0'));
    let after = points.select(u8x32::splat(31) - LANES, u8x32::splat(u8::MAX)).reduce_min();
    let after = u8x32::splat(after);
    let integer = LANES.simd_lt(u8x32::splat(32_u8.saturating_sub(decimals)));
    let decimals = u8x32::splat(decimals);
    let index = LANES + decimals - integer.select(u8x32::splat(1), u8x32::splat(0)) - after;
    let steps = lanes.lookup32(window, index);
    // The lanes outside the span the steps read.
    let unread = (LANES + u8x32::splat(1) - decimals + after).simd_gt(u8x32::splat(32));
    let stray = (window & unread.to_simd().cast()).simd_ne(u8x32::splat(0));
    if steps.simd_max(stray.to_simd().cast()).reduce_max() > 9 {
        return None;
    }
    Some(number(steps))
}

/// [`read`] in a kernel compiled for SSSE3, so that the lookups' `pshufb`s inline.
#[cfg(lanes = "checked")]
#[inline]
#[expect(unsafe_code, reason = "calls the SSSE3 kernel where `lanes` proves the CPU has SSSE3")]
pub(super) fn read_checked(lanes: Lanes, text: &[u8], decimals: u8) -> Option<u128> {
    /// The kernel: [`read`], with SSSE3 enabled.
    #[target_feature(enable = "ssse3")]
    fn read_ssse3(lanes: Lanes, text: &[u8], decimals: u8) -> Option<u128> {
        read(lanes, text, decimals)
    }
    // SAFETY: the CPU has SSSE3, as `lanes` proves.
    unsafe { read_ssse3(lanes, text, decimals) }
}

/// [`read_in`] in a kernel compiled for SSSE3, so that the lookups' `pshufb`s inline.
#[cfg(lanes = "checked")]
#[inline]
#[expect(unsafe_code, reason = "calls the SSSE3 kernel where `lanes` proves the CPU has SSSE3")]
pub(super) fn read_in_checked(
    lanes: Lanes, buffer: &[u8], digits: Range<usize>, decimals: u8,
) -> Option<u128> {
    /// The kernel: [`read_in`], with SSSE3 enabled.
    #[target_feature(enable = "ssse3")]
    fn read_in_ssse3(
        lanes: Lanes, buffer: &[u8], digits: Range<usize>, decimals: u8,
    ) -> Option<u128> {
        read_in(lanes, buffer, digits, decimals)
    }
    // SAFETY: the CPU has SSSE3, as `lanes` proves.
    unsafe { read_in_ssse3(lanes, buffer, digits, decimals) }
}

/// The 32 digits of `steps`, most significant first, as a number: pairs and fours by a multiply and
/// a shift in the lanes, as Lemire's eight digits in a word, eights by a multiply-add, then two
/// halves of 16.
#[inline]
#[expect(clippy::arithmetic_side_effects, reason = "groups of eight digits, below 10^32 in all")]
fn number(steps: u8x32) -> u128 {
    let pairs = (u16x16::from_le_bytes(steps) * u16x16::splat(10 << 8 | 1)) >> 8;
    let fours = (u32x8::from_le_bytes(pairs.to_le_bytes()) * u32x8::splat(100 << 16 | 1)) >> 16;
    let eights = simd_swizzle!(fours, [0, 2, 4, 6]) * u32x4::splat(10_000)
        + simd_swizzle!(fours, [1, 3, 5, 7]);
    let [first, second, third, fourth] = eights.to_array().map(u64::from);
    let high = first * 100_000_000 + second;
    let low = third * 100_000_000 + fourth;
    u128::from(high) * 10_000_000_000_000_000 + u128::from(low)
}

/// What the CPU has, read once by CPUID and kept: for an `x86_64` build without SSSE3, with the
/// `runtime-dispatch` feature, on a target with SSE registers; a soft-float one has none to use.
#[cfg(lanes = "checked")]
mod cpu {
    use core::arch::x86_64::__cpuid;
    use core::sync::atomic::{AtomicU8, Ordering};

    /// Not yet read, 0; without SSSE3, 1; with it, 2.
    static SSSE3: AtomicU8 = AtomicU8::new(0);

    /// Whether the CPU has SSSE3: CPUID's leaf 1, bit 9 of ECX, read on the first call alone.
    #[inline]
    pub(super) fn has_ssse3() -> bool {
        // ORDERING: Relaxed throughout. The flag publishes nothing but itself, and two threads
        // that both find it unread each run CPUID and store the same answer.
        match SSSE3.load(Ordering::Relaxed) {
            0 => {
                let has = __cpuid(1).ecx & (1 << 9) != 0;
                SSSE3.store(if has { 2 } else { 1 }, Ordering::Relaxed);
                has
            },
            seen => seen == 2,
        }
    }
}
