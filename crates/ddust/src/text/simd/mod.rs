//! The vector reader and writer, written once in `core::simd`, and the SWAR reader the vector
//! reader is held to: each reader reads a text as the other does, where both read it, as the tests
//! and the fuzz targets check; the writer writes the general writer's text, as its property checks.
//!
//! Both run on NEON, on every little-endian aarch64 target that has it, all but the soft-float
//! ones, and on SSSE3, on an `x86_64` build for a CPU that has it or one the `runtime-dispatch`
//! feature checks. Where there is no vector unit, the SWAR reader reads, and the eight-byte writer
//! writes.

use core::ops::Range;

use super::swar;

#[cfg(lanes)]
mod lanes;
#[cfg(lanes)]
mod read;
#[cfg(lanes)]
mod write;

#[cfg(lanes)]
use self::lanes::Lanes;

/// The magnitude at `decimals` of `text`, `[digits][.digits]` with no sign, as [`swar::read_plain`]
/// reads it: by the vector reader where the build has one or the CPU does, and by the SWAR reader
/// on a build with no vector unit.
///
/// The vector reader reads a text of at most 32 bytes whose steps are below `10^32`, at most 32
/// decimals; [`read_rest`] reads what it leaves.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline, a static scale's decimals fold into the reader's lanes"
)]
pub(crate) fn read_plain(text: &[u8], decimals: u8) -> Option<u128> {
    cfg_select! {
        lanes = "built" => read::read(Lanes::BUILT, text, decimals),
        lanes = "checked" => Lanes::checked().map_or_else(
            || swar::read_plain(text, decimals),
            |lanes| read::read_checked(lanes, text, decimals),
        ),
        _ => swar::read_plain(text, decimals),
    }
}

/// [`read_plain`] for the digits at `digits` of `buffer`: by the vector reader from the 16 bytes
/// from their start and the 16 before their end, the same way whatever their length, where those
/// are in the buffer; as [`read_plain`] reads the digits alone where they are not.
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline, a static scale's decimals fold into the reader's lanes"
)]
pub(crate) fn read_plain_in(buffer: &[u8], digits: Range<usize>, decimals: u8) -> Option<u128> {
    cfg_select! {
        lanes = "built" => read::read_in(Lanes::BUILT, buffer, digits, decimals),
        lanes = "checked" => match Lanes::checked() {
            Some(lanes) => read::read_in_checked(lanes, buffer, digits, decimals),
            None => swar::read_plain(buffer.get(digits)?, decimals),
        },
        _ => swar::read_plain(buffer.get(digits)?, decimals),
    }
}

/// Writes the steps `magnitude` at `decimals`, a `-` first when `negative`, into `out`, as the
/// shortest exact decimal: by the vector writer where the build has a vector unit or the CPU does.
///
/// `None` past 30 decimals, where the CPU has no vector unit, or where `out` is too short.
#[cfg(lanes)]
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline, a static scale's decimals fold into the writer's rows"
)]
pub(crate) fn write_narrow(
    negative: bool, magnitude: u64, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    cfg_select! {
        lanes = "built" => write::narrow(Lanes::BUILT, negative, magnitude, decimals, out),
        _ => Lanes::checked()
            .and_then(|lanes| write::narrow_checked(lanes, negative, magnitude, decimals, out)),
    }
}

/// [`write_narrow`] for steps past a word: `None` too for steps of `10^31` or more.
#[cfg(lanes)]
#[inline(always)]
#[expect(
    clippy::inline_always,
    reason = "inline, a static scale's decimals fold into the writer's rows"
)]
pub(crate) fn write_wide(
    negative: bool, magnitude: u128, decimals: u8, out: &mut [u8],
) -> Option<usize> {
    cfg_select! {
        lanes = "built" => write::wide(Lanes::BUILT, negative, magnitude, decimals, out),
        _ => Lanes::checked()
            .and_then(|lanes| write::wide_checked(lanes, negative, magnitude, decimals, out)),
    }
}

/// Nothing, on a build with no vector unit: the eight-byte writer writes there.
#[cfg(not(lanes))]
#[inline]
pub(crate) const fn write_narrow(
    _negative: bool, _magnitude: u64, _decimals: u8, _out: &mut [u8],
) -> Option<usize> {
    None
}

/// Nothing, on a build with no vector unit: the eight-byte writer writes there.
#[cfg(not(lanes))]
#[inline]
pub(crate) const fn write_wide(
    _negative: bool, _magnitude: u128, _decimals: u8, _out: &mut [u8],
) -> Option<usize> {
    None
}

/// The plain shape [`read_plain`] leaves to the SWAR reader where the vector reader read first:
/// past 32 bytes, steps of `10^32` or more, or past 32 decimals; `None` where the SWAR reader read
/// first.
#[cfg(lanes)]
#[inline]
pub(crate) fn read_rest(text: &[u8], decimals: u8) -> Option<u128> {
    cfg_select! {
        lanes = "built" => swar::read_plain(text, decimals),
        _ => Lanes::checked().and_then(|_lanes| swar::read_plain(text, decimals)),
    }
}

/// Nothing, where the SWAR reader has read first: [`read_plain`] leaves nothing on such a build.
#[cfg(not(lanes))]
#[inline]
pub(crate) const fn read_rest(_text: &[u8], _decimals: u8) -> Option<u128> {
    None
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::{read_plain, read_rest};
    use crate::text::swar;

    #[rstest]
    #[case::one_digit("7", 0, Some(7))]
    #[case::a_point_first(".5", 16, Some(5_000_000_000_000_000))]
    #[case::a_point_last("123456789012345.", 1, Some(1_234_567_890_123_450))]
    #[case::fifteen_bytes("1234567.1234567", 8, Some(123_456_712_345_670))]
    #[case::sixteen_bytes("1234567.12345678", 8, Some(123_456_712_345_678))]
    #[case::seventeen_bytes("12345678.12345678", 8, Some(1_234_567_812_345_678))]
    #[case::thirty_two_digits(
        "99999999999999999999999999999999",
        0,
        Some(99_999_999_999_999_999_999_999_999_999_999)
    )]
    #[case::thirty_two_bytes_a_point_first(".0000000000000000000000000000001", 32, Some(10))]
    #[case::steps_of_ten_to_the_32(
        "10000000000000000000000000000000",
        1,
        Some(100_000_000_000_000_000_000_000_000_000_000)
    )]
    #[case::thirty_three_bytes("000000000000000000000000000000001", 0, Some(1))]
    #[case::thirty_three_decimals("1", 33, Some(1_000_000_000_000_000_000_000_000_000_000_000))]
    #[case::leading_zeros_past_the_lanes("0000000000000000000000000.5", 8, Some(50_000_000))]
    #[case::zeros_past_the_scale("60000.50000000", 2, Some(6_000_050))]
    #[case::a_digit_past_the_scale("1.51", 1, None)]
    #[case::two_points("1..5", 2, None)]
    #[case::a_point_alone(".", 0, None)]
    #[case::a_letter("1.5e1", 2, None)]
    fn each_edge_of_the_vector_domain_reads_as_its_case_says(
        #[case] text: &str, #[case] decimals: u8, #[case] expected: Option<u128>,
    ) {
        // Each edge of the vector reader's domain and the first case past it, as a caller reads
        // them: NEON on aarch64, SSSE3 in CI, and the SWAR reader past the domain.
        let bytes = text.as_bytes();
        let read = read_plain(bytes, decimals).or_else(|| read_rest(bytes, decimals));
        assert_eq!(read, expected, "{text} at {decimals}");
    }

    proptest! {
        #[test]
        fn the_vector_reader_agrees_with_its_swar_twin(
            text in "[0-9.]{0,16}|[0-9]{0,10}\\.[0-9]{0,10}|[0-9.ex+-]{0,16}|[0-9]{0,30}\\.[0-9]{0,20}|0{0,20}[0-9]{0,12}\\.?[0-9]{0,8}0{0,20}",
            decimals in 0_u8..=38,
        ) {
            // Where the vector reader answers, it answers as its twin; it leaves only a text past
            // 32 bytes, steps of 33 digits or more, past 32 decimals, or what its twin refuses.
            let bytes = text.as_bytes();
            let (read, twin) = (read_plain(bytes, decimals), swar::read_plain(bytes, decimals));
            match read {
                Some(_) => prop_assert_eq!(read, twin, "{}", text),
                None => prop_assert!(
                    bytes.len() > 32 || decimals > 32 || twin.is_none_or(|steps| steps >= 10_u128.pow(32)),
                    "{} at {} left, though its twin reads {:?}", text, decimals, twin
                ),
            }
        }
    }
}
