//! Equality, order and hashing: as the steps at one scale, lined up at two run-time scales.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::decimal::Decimal;
use crate::int::Int;
use crate::round::{RoundingMode, Trunc};
use crate::scale::Scale;

/// Panics for two values whose scales never mix.
#[cold]
#[inline(never)]
#[track_caller]
#[expect(
    clippy::panic,
    reason = "two scales that never mix have no order: a mistake in every build"
)]
const fn unordered() -> ! {
    panic!("a comparison of values of two scales that never mix")
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The order of two values, or `None` for two scales that never mix.
    #[inline]
    const fn order(self, other: Self) -> Option<Ordering>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        let (left, right) = (self.scale(), other.scale());
        let (a, b) = (self.steps(), other.steps());
        if left == right {
            return Some(a.cmp(&b));
        }
        if !S::LINES_UP {
            return None;
        }
        let (from, to) = (left.decimals(), right.decimals());
        Some(if from < to {
            a.lined_up_cmp(to.wrapping_sub(from), b)
        } else {
            b.lined_up_cmp(from.wrapping_sub(to), a).reverse()
        })
    }
}

/// Values of one scale are equal when their steps are; two run-time scales line up first, so
/// `1.5 == 1.50`; values of two scales that never mix are never equal.
const impl<I: [const] Int, S: [const] Scale> PartialEq for Decimal<I, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        matches!(self.order(*other), Some(Ordering::Equal))
    }
}

const impl<I: [const] Int, S: [const] Scale> Eq for Decimal<I, S> {}

/// Values of one scale compare as their steps; two run-time scales line up first.
///
/// # Panics
/// For two scales that never mix, which have no order until one is converted.
const impl<I: [const] Int, S: [const] Scale> Ord for Decimal<I, S> {
    #[inline]
    #[track_caller]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.order(*other) {
            Some(order) => order,
            None => unordered(),
        }
    }
}

/// As [`Ord`].
///
/// # Panics
/// For two scales that never mix, as [`Ord`] does.
const impl<I: [const] Int, S: [const] Scale> PartialOrd for Decimal<I, S> {
    #[inline]
    #[track_caller]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Hashes what equality compares: the steps and the scale where two values never differ in scale
/// or never mix; for a scale that lines up, the value with its trailing zeros dropped, so `1.5` and
/// `1.50` hash alike.
impl<I: Int, S: Scale> Hash for Decimal<I, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if !S::LINES_UP {
            self.steps().hash(state);
            self.scale().hash(state);
            return;
        }
        let (mut steps, mut decimals) = (self.steps(), self.decimals());
        while decimals > 0 {
            let (tenth, exact) = steps.scale_down(1, Trunc.table());
            if !exact {
                break;
            }
            steps = tenth;
            decimals = decimals.wrapping_sub(1);
        }
        steps.hash(state);
        decimals.hash(state);
    }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference model's own arithmetic")]
pub(crate) mod tests {
    use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

    use proptest::prelude::*;

    use crate::{D64, Decimal, Dynamic, Fixed};

    /// A decimal at a run-time scale of `decimals`.
    fn dynamic(steps: i64, decimals: u8) -> Decimal<i64, Dynamic> {
        Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"))
    }

    /// FNV-1a over the bytes written: a small, deterministic hasher for the tests.
    #[derive(Default)]
    struct Fnv(u64);

    impl Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }

        fn write(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
            }
        }
    }

    /// The hash of a value.
    pub(crate) fn hash_of<T: Hash>(value: &T) -> u64 {
        BuildHasherDefault::<Fnv>::default().hash_one(value)
    }

    proptest! {
        #[test]
        fn two_run_time_scales_compare_as_their_lined_up_values(a: i32, b: i32, k in 0_u8..9) {
            let (x, y) = (dynamic(a.into(), 0), dynamic(b.into(), k));
            let lifted = i64::from(a) * 10_i64.pow(u32::from(k));
            prop_assert_eq!(x.cmp(&y), lifted.cmp(&i64::from(b)));
            prop_assert_eq!(x == y, lifted == i64::from(b));
        }

        #[test]
        fn equal_values_hash_alike_at_any_scale(a: i32, k in 0_u8..9) {
            let (x, y) = (dynamic(a.into(), 0), dynamic(i64::from(a) * 10_i64.pow(u32::from(k)), k));
            prop_assert_eq!(x, y, "the same value");
            prop_assert_eq!(hash_of(&x), hash_of(&y), "the same hash");
        }
    }

    #[test]
    fn values_of_one_scale_compare_as_their_steps() {
        let (low, high) = (D64::<2>::from_steps(-15, Fixed), D64::<2>::from_steps(14, Fixed));
        assert!(low < high, "below zero first");
        assert_eq!(low.max(high), high, "the larger");
        assert_eq!(dynamic(15, 1), dynamic(150, 2), "1.5 == 1.50");
        assert!(dynamic(60_001, 0) > dynamic(600_005, 1), "60001 > 60000.5");
    }
}
