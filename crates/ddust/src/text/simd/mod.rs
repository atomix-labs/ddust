//! The SIMD kernels: each computes what its SWAR twin in [`swar`] does, in vector registers, and
//! is held to it by the tests and the fuzz targets.
//!
//! NEON is on every aarch64 target, and SSSE3 on an `x86_64` build for a CPU that has it, or one
//! the `runtime-dispatch` feature checks. Where there is no vector unit, the SWAR twin answers.

use super::swar;

#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(all(
    target_arch = "x86_64",
    any(target_feature = "ssse3", all(target_feature = "sse2", feature = "runtime-dispatch"))
))]
mod sse;

/// The magnitude at `decimals` of `text`, `[digits][.digits]` with no sign, as
/// [`swar::read_plain`] reads it: a text of at most 16 bytes, at most 16 digits once scaled, by the
/// vector kernel where the build has one, and any other text, or one the kernel leaves, by the SWAR
/// twin.
#[inline]
pub(crate) fn read_plain(text: &[u8], decimals: u8) -> Option<u128> {
    cfg_select! {
        all(target_arch = "aarch64", target_feature = "neon") => {
            if text.len() <= 16
                && let Some(steps) = neon::read_short(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        all(target_arch = "x86_64", target_feature = "ssse3") => {
            if text.len() <= 16
                && let Some(steps) = sse::read_short(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        all(target_arch = "x86_64", target_feature = "sse2", feature = "runtime-dispatch") => {
            if text.len() <= 16
                && dispatch::ssse3()
                && let Some(steps) = sse::read_short_checked(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        _ => {},
    }
    swar::read_plain(text, decimals)
}

/// What the CPU has, checked once by CPUID and kept: for an `x86_64` build without SSSE3, with the
/// `runtime-dispatch` feature, on a target with SSE registers; a soft-float one has none to use.
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "sse2",
    not(target_feature = "ssse3"),
    feature = "runtime-dispatch"
))]
mod dispatch {
    use core::arch::x86_64::__cpuid;
    use core::sync::atomic::{AtomicU8, Ordering};

    /// Not yet checked, 0; without SSSE3, 1; with it, 2.
    static SSSE3: AtomicU8 = AtomicU8::new(0);

    /// Whether the CPU has SSSE3: CPUID's leaf 1, bit 9 of ECX, read on the first call alone.
    #[inline]
    pub(super) fn ssse3() -> bool {
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::read_plain;
    use crate::text::swar;

    proptest! {
        #[test]
        fn the_vector_reader_agrees_with_its_swar_twin(
            text in "[0-9.]{0,16}|[0-9]{0,10}\\.[0-9]{0,10}|[0-9.ex+-]{0,16}|[0-9]{0,30}\\.[0-9]{0,20}",
            decimals in 0_u8..=20,
        ) {
            // Wherever a kernel reads a text, it reads what the twin reads.
            let bytes = text.as_bytes();
            prop_assert_eq!(read_plain(bytes, decimals), swar::read_plain(bytes, decimals), "{}", text);
        }
    }
}
