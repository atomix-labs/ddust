//! What the vector reader and writer leave to each vector unit, behind [`Lanes`]: a table lookup,
//! and the digits of a number below `10^16`.

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{
    vaddq_u8, vcgtzq_s8, vcreate_u64, vdupq_n_u8, vget_lane_u64, vmla_n_s32, vmlaq_n_s16,
    vmlaq_n_s32, vqdmulh_n_s32, vqdmulhq_n_s16, vqdmulhq_n_s32, vreinterpret_s32_u32,
    vreinterpret_s32_u64, vreinterpret_u16_s32, vreinterpret_u32_s32, vreinterpret_u64_u8,
    vreinterpretq_s8_u8, vreinterpretq_s16_s32, vreinterpretq_s32_u32, vreinterpretq_u8_s16,
    vreinterpretq_u16_u8, vrev64q_u8, vshll_n_u16, vshr_n_u32, vshrn_n_u16,
};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::_mm_shuffle_epi8;
use core::simd::prelude::*;
#[cfg(target_arch = "x86_64")]
use core::simd::{ToBytes, simd_swizzle};

#[cfg(target_arch = "x86_64")]
use super::swar::GROUP;

/// Each lane's own index.
pub(super) const LANES: u8x32 = u8x32::from_array([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
]);

/// Each lane's index, as a swizzle takes it: two halves joined in order.
pub(super) const JOIN: [usize; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

/// Proof that the CPU has the vector unit's instructions, NEON or SSSE3: made where the build has
/// them, or where CPUID says the CPU does.
#[derive(Clone, Copy)]
pub(super) struct Lanes(
    // INVARIANT: a `Lanes` exists only where the CPU has the unit's instructions. Its makers are
    // `BUILT`, under a `cfg` that requires them, and `checked`, once CPUID has found SSSE3.
    (),
);

impl Lanes {
    /// The proof on a build that has the instructions, as rustc's own `cfg` says too: a build whose
    /// flags `build.rs` did not see fails to compile here, rather than run without them.
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
    pub(super) fn lookup16(self, table: u8x16, index: u8x16) -> u8x16 {
        table.swizzle_dyn(index)
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: a `tbl` over both
    /// halves of the table for each half of `index`.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "two instructions")]
    #[expect(clippy::unused_self, reason = "the proof NEON's lookup needs is the build's own")]
    pub(super) fn lookup32(self, table: u8x32, index: u8x32) -> u8x32 {
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
    pub(super) fn lookup16(self, table: u8x16, index: u8x16) -> u8x16 {
        let index = index.saturating_add(u8x16::splat(0x70));
        // SAFETY: by the INVARIANT of `Lanes`, the CPU has SSSE3, all `_mm_shuffle_epi8` needs.
        unsafe { _mm_shuffle_epi8(table.into(), index.into()) }.into()
    }

    /// `table`'s lane at each lane of `index`, zero for an index past the table: each half of the
    /// table looked up in for each half of `index`, and the two joined by an OR.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, or `pshufb` is a call")]
    pub(super) fn lookup32(self, table: u8x32, index: u8x32) -> u8x32 {
        let (low, high) = (table.extract::<0, 16>(), table.extract::<16, 16>());
        let half = |index: u8x16| {
            self.lookup16(low, index) | self.lookup16(high, index - u8x16::splat(16))
        };
        simd_swizzle!(half(index.extract::<0, 16>()), half(index.extract::<16, 16>()), JOIN)
    }

    /// The 16 digits of `value`, below `10^16`, as ASCII, the most significant first, and one past
    /// the last lane that holds a nonzero digit, zero for none: Dougall Johnson's NEON kernel, in
    /// which a doubling multiply-high takes each halving's quotient in one instruction.
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "inline in the writer, where its value is ready")]
    #[expect(
        unsafe_code,
        reason = "calls the NEON kernel from code compiled without its `#[target_feature]`"
    )]
    #[expect(clippy::unused_self, reason = "the proof NEON's kernel needs, which its SAFETY reads")]
    pub(super) fn digits16(self, value: u64) -> (u8x16, u8) {
        // SAFETY: by the INVARIANT of `Lanes` the CPU has NEON, all `neon_digits16` needs: on
        // aarch64 `BUILT` is its one maker, and its `cfg` requires `target_feature = "neon"`.
        let (digits, nibbles) = unsafe { neon_digits16(value) };
        let last = 16_u32.wrapping_sub(nibbles.leading_zeros() / 4);
        (digits, u8::try_from(last).unwrap_or(0))
    }

    /// The 16 digits of `value`, below `10^16`, as ASCII, the most significant first, and one past
    /// the last lane that holds a nonzero digit, zero for none: each group of eight halved by a
    /// multiply and a subtraction, in a word and then in the lanes, as Lemire's eight digits.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    #[expect(clippy::inline_always, reason = "inline in the SSSE3 kernel, where its lanes are")]
    #[expect(
        clippy::unused_self,
        reason = "the proof the SSSE3 kernel needs, which its caller holds"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "groups below 10^8: every step stays in range"
    )]
    pub(super) fn digits16(self, value: u64) -> (u8x16, u8) {
        // Each group of eight as [first four | last four << 32], by
        // `(n << 32) − q × (10^4 × 2^32 − 1)`.
        let halves = |n: u64| {
            let q = (n * 109_951_163) >> 40;
            (n << 32) - q * ((10_000 << 32) - 1)
        };
        let groups = u64x2::from_array([value / GROUP, value % GROUP].map(halves));
        let fours = u32x4::from_le_bytes(groups.to_le_bytes());
        let q = (fours * u32x4::splat(5243)) >> 19;
        let pairs =
            u16x8::from_le_bytes(((fours << 16) - q * u32x4::splat((100 << 16) - 1)).to_le_bytes());
        let q = (pairs * u16x8::splat(103)) >> 10;
        let digits =
            u8x16::from_le_bytes(((pairs << 8) - q * u16x8::splat((10 << 8) - 1)).to_le_bytes());
        let nonzero = u16::try_from(digits.simd_ne(u8x16::splat(0)).to_bitmask()).unwrap_or(0);
        let last = 16_u32.wrapping_sub(nonzero.leading_zeros());
        (digits | u8x16::splat(b'0'), u8::try_from(last).unwrap_or(0))
    }
}

/// [`Lanes::digits16`] in NEON: the digits, and a nibble set for each lane that holds a nonzero
/// one (zmij's `to_unshuffled_digits` and a `rev64`).
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a value below 10^16: every step stays in range"
)]
fn neon_digits16(value: u64) -> (u8x16, u64) {
    let high = value / 100_000_000;
    let low = value - high * 100_000_000;
    // Both halves of eight in one register; each step below halves every group in its lanes, a
    // doubling multiply-high by a reciprocal and a multiply-add of the negated divisor putting the
    // quotient and the remainder side by side.
    let eights = vreinterpret_s32_u64(vcreate_u64((low << 32) | high));
    let fours = vreinterpret_s32_u32(vshr_n_u32::<9>(vreinterpret_u32_s32(vqdmulh_n_s32(
        eights,
        109_951_163,
    ))));
    let fours = vmla_n_s32(eights, fours, 0x1_0000 - 10_000);
    let fours = vreinterpretq_s32_u32(vshll_n_u16::<0>(vreinterpret_u16_s32(fours)));
    let pairs = vqdmulhq_n_s32(fours, 5243 << 12);
    let pairs = vreinterpretq_s16_s32(vmlaq_n_s32(fours, pairs, 0x1_0000 - 100));
    let tens = vqdmulhq_n_s16(pairs, 0xCE0);
    let digits = vrev64q_u8(vreinterpretq_u8_s16(vmlaq_n_s16(pairs, tens, 0x100 - 10)));
    let nonzero = vreinterpretq_u16_u8(vcgtzq_s8(vreinterpretq_s8_u8(digits)));
    let nibbles = vget_lane_u64::<0>(vreinterpret_u64_u8(vshrn_n_u16::<4>(nonzero)));
    (vaddq_u8(digits, vdupq_n_u8(b'0')).into(), nibbles)
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
