//! The vector readers: each reads what its SWAR twin in [`swar`] reads, in vector registers, and is
//! held to it by the tests and the fuzz targets.
//!
//! NEON reads on every aarch64 target that has it, all but the soft-float ones, and SSSE3 on an
//! `x86_64` build for a CPU that has it, or on one the `runtime-dispatch` feature checks. Where
//! there is no vector unit, the SWAR twin reads.

use super::swar;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
mod neon;
#[cfg(all(
    target_arch = "x86_64",
    any(target_feature = "ssse3", all(target_feature = "sse2", feature = "runtime-dispatch"))
))]
mod sse;

/// A text of at most 16 bytes as its length and two words, its first eight bytes and its last
/// eight, or the one word a shorter text is: `None` past 16 bytes, or for an empty text. For the
/// vector readers, so for a build that has one.
#[cfg(any(
    all(target_arch = "aarch64", target_feature = "neon"),
    all(
        target_arch = "x86_64",
        any(target_feature = "ssse3", all(target_feature = "sse2", feature = "runtime-dispatch"))
    )
))]
#[inline]
fn words(text: &[u8]) -> Option<(u8, u64, u64)> {
    let len = u8::try_from(text.len()).ok().filter(|&len| len <= 16)?;
    if len >= 8 {
        Some((len, swar::load(text, 0), swar::load(text, usize::from(len).wrapping_sub(8))))
    } else {
        Some((len, swar::load_short(text)?, 0))
    }
}

/// The magnitude at `decimals` of `text`, `[digits][.digits]` with no sign, as
/// [`swar::read_plain`] reads it: a text of at most 16 bytes, at most 16 digits once scaled,
/// by the vector reader where the build has one, and any other text, or one the vector reader
/// leaves, by the SWAR twin.
#[inline]
pub(crate) fn read_plain(text: &[u8], decimals: u8) -> Option<u128> {
    cfg_select! {
        all(target_arch = "aarch64", target_feature = "neon") => {
            if text.len() <= 16
                && let Some(steps) = neon::read(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        all(target_arch = "x86_64", target_feature = "ssse3") => {
            if text.len() <= 16
                && let Some(steps) = sse::read(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        all(target_arch = "x86_64", target_feature = "sse2", feature = "runtime-dispatch") => {
            if text.len() <= 16
                && let Some(steps) = sse::read_checked(text, decimals)
            {
                return Some(u128::from(steps));
            }
        },
        _ => {},
    }
    swar::read_plain(text, decimals)
}

/// What the CPU has, read once by CPUID and kept: for an `x86_64` build without SSSE3, with the
/// `runtime-dispatch` feature, on a target with SSE registers; a soft-float one has none to use.
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "sse2",
    not(target_feature = "ssse3"),
    feature = "runtime-dispatch"
))]
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::read_plain;
    use crate::text::swar;

    #[rstest]
    #[case::point_first_at_16_decimals(".5", 16, Some(5_000_000_000_000_000))]
    #[case::point_last("123456789012345.", 1, Some(1_234_567_890_123_450))]
    #[case::sixteen_digits_no_point("1234567890123456", 0, Some(1_234_567_890_123_456))]
    #[case::sixteen_bytes("1234567.12345678", 8, Some(123_456_712_345_678))]
    #[case::seventeen_bytes("12345678.12345678", 8, Some(1_234_567_812_345_678))]
    #[case::seventeen_digits_once_scaled("1.5", 16, Some(15_000_000_000_000_000))]
    #[case::the_last_place_past_the_text("12345678901234.5", 2, Some(1_234_567_890_123_450))]
    #[case::zeros_past_the_scale("1.5000", 1, Some(15))]
    #[case::a_digit_past_the_scale("1.51", 1, None)]
    #[case::two_points("1..5", 2, None)]
    #[case::a_point_alone(".", 0, None)]
    #[case::a_letter("1.5e1", 2, None)]
    fn each_edge_of_the_vector_domain_reads_as_its_case_says(
        #[case] text: &str, #[case] decimals: u8, #[case] expected: Option<u128>,
    ) {
        // Each edge of the vector reader's domain, on every run: NEON on aarch64, SSSE3 in CI.
        assert_eq!(read_plain(text.as_bytes(), decimals), expected, "{text} at {decimals}");
    }

    proptest! {
        #[test]
        fn the_vector_reader_agrees_with_its_swar_twin(
            text in "[0-9.]{0,16}|[0-9]{0,10}\\.[0-9]{0,10}|[0-9.ex+-]{0,16}|[0-9]{0,30}\\.[0-9]{0,20}",
            decimals in 0_u8..=20,
        ) {
            let bytes = text.as_bytes();
            prop_assert_eq!(read_plain(bytes, decimals), swar::read_plain(bytes, decimals), "{}", text);
        }
    }
}
