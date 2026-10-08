//! The values every contender reads: drawn once from a fixed seed, as steps at the decimals a width
//! carries, predictable or not, each operation timed on both.
//!
//! - **Predictable**: one sign and one count of digits, 1,024 values, which stay in the level 1
//!   cache: the best case of a branch on the value.
//! - **Unpredictable**: random signs and counts of digits, 65,536 values, more than a branch
//!   predictor learns, so a branch on the value mispredicts as it would on a program's own.
//!
//! The values are in a range every contender holds, and so are their results: no row is timed on an
//! overflow.

use crate::oracle;

/// The values in a predictable set.
pub const PREDICTABLE_COUNT: usize = 1024;

/// The values in an unpredictable set.
pub const UNPREDICTABLE_COUNT: usize = 65_536;

/// A width the suite compares at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    /// 64-bit values at 8 decimals: a price or a quantity, as most venues send it.
    Narrow,
    /// 128-bit values at 18 decimals: a token's amount, as a chain holds it.
    Wide,
}

impl Width {
    /// Both widths.
    pub const ALL: [Self; 2] = [Self::Narrow, Self::Wide];

    /// The decimals the width's values carry.
    #[must_use]
    pub const fn decimals(self) -> u8 {
        match self {
            Self::Narrow => 8,
            Self::Wide => 18,
        }
    }

    /// The width's name in a measurement's.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Narrow => "narrow",
            Self::Wide => "wide",
        }
    }
}

/// How predictable a branch on the values is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Predictability {
    /// One sign and one count of digits, in the level 1 cache.
    Predictable,
    /// Random signs and counts of digits, past what a predictor learns.
    Unpredictable,
}

impl Predictability {
    /// Both.
    pub const ALL: [Self; 2] = [Self::Predictable, Self::Unpredictable];

    /// Its name in a measurement's.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Predictable => "predictable",
            Self::Unpredictable => "unpredictable",
        }
    }

    /// How many values it holds.
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::Predictable => PREDICTABLE_COUNT,
            Self::Unpredictable => UNPREDICTABLE_COUNT,
        }
    }

    /// The seed its values are drawn from, the same on every run.
    const fn seed(self, salt: u64) -> u64 {
        match self {
            Self::Predictable => 0x5EED_0001 ^ salt,
            Self::Unpredictable => 0x5EED_0002 ^ salt,
        }
    }
}

/// Two operands for each operation, side by side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairs {
    /// The left operands.
    pub left: Vec<i128>,
    /// The right operands.
    pub right: Vec<i128>,
}

/// Operands of a sum or a comparison: values below `10^9` at 8 decimals, and `10^10` at 18, so a
/// sum fits the narrower contenders.
#[must_use]
pub fn addends(width: Width, predictability: Predictability) -> Pairs {
    let digits = sum_digits(width);
    pairs(
        predictability,
        1,
        |random| random.value(digits, predictability),
        |random| random.value(digits, predictability),
    )
}

/// Operands of a product at one scale: values below `10^5`, so the product is below `10^10`.
#[must_use]
pub fn factors(width: Width, predictability: Predictability) -> Pairs {
    let digits = u32::from(width.decimals()).saturating_add(5);
    pairs(
        predictability,
        2,
        |random| random.value(digits, predictability),
        |random| random.value(digits, predictability),
    )
}

/// Operands of a quotient at one scale: dividends below `10^6` and divisors from `10^-3` to `10^5`,
/// so the quotient is below `10^9`.
#[must_use]
pub fn dividends_and_divisors(width: Width, predictability: Predictability) -> Pairs {
    let decimals = u32::from(width.decimals());
    pairs(
        predictability,
        3,
        |random| random.value(decimals.saturating_add(6), predictability),
        |random| {
            random.at_least(decimals.saturating_sub(3), decimals.saturating_add(5), predictability)
        },
    )
}

/// Operands of a quotient by one divisor: the dividends of [`dividends_and_divisors`], each by the
/// first of its divisors, as a book is divided by one price.
#[must_use]
pub fn dividends_and_one_divisor(width: Width, predictability: Predictability) -> Pairs {
    let mut set = dividends_and_divisors(width, predictability);
    let first = set.right.first().copied().unwrap_or(1);
    set.right.fill(first);
    set
}

/// A price's steps at 2 decimals, below 100,000, and a quantity's at 5, below 100: their exact
/// product, at 7, fits every width.
#[must_use]
pub fn prices_and_quantities(predictability: Predictability) -> Pairs {
    pairs(
        predictability,
        4,
        |random| random.value(7, predictability),
        |random| random.value(7, predictability),
    )
}

/// Values to round, write, or convert to a double: the operands of a sum.
#[must_use]
pub fn steps(width: Width, predictability: Predictability) -> Vec<i128> {
    addends(width, predictability).left
}

/// Each value's shortest exact text, to parse.
#[must_use]
pub fn texts(width: Width, predictability: Predictability) -> Vec<String> {
    steps(width, predictability)
        .into_iter()
        .map(|value| oracle::text(value, width.decimals()))
        .collect()
}

/// The double nearest each value, to convert back.
#[must_use]
pub fn doubles(width: Width, predictability: Predictability) -> Vec<f64> {
    steps(width, predictability)
        .into_iter()
        .map(|value| oracle::to_f64(value, width.decimals()))
        .collect()
}

/// The digits of steps a sum's operands have at most, at `width`.
const fn sum_digits(width: Width) -> u32 {
    match width {
        Width::Narrow => 17,
        Width::Wide => 28,
    }
}

/// `predictability.count()` pairs, the left from `left` and the right from `right`, drawn from the
/// predictability's seed salted by `salt`, so each operation has its own values.
fn pairs(
    predictability: Predictability, salt: u64, mut left: impl FnMut(&mut Random) -> i128,
    mut right: impl FnMut(&mut Random) -> i128,
) -> Pairs {
    let mut random = Random(predictability.seed(salt));
    let (mut lefts, mut rights) =
        (Vec::with_capacity(predictability.count()), Vec::with_capacity(predictability.count()));
    for _ in 0..predictability.count() {
        lefts.push(left(&mut random));
        rights.push(right(&mut random));
    }
    Pairs { left: lefts, right: rights }
}

/// `SplitMix64`: a small generator whose stream is fixed by its seed.
#[derive(Debug, Clone)]
struct Random(u64);

impl Random {
    /// The next 64 random bits.
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A random number below `bound`, which is not zero; the bias of a modulus this far below 2^128
    /// is no concern of a benchmark's.
    fn below(&mut self, bound: u128) -> u128 {
        let bits = (u128::from(self.next()) << 64) | u128::from(self.next());
        bits.checked_rem(bound).unwrap_or(0)
    }

    /// A value of up to `digits` digits of steps: in a predictable set, positive with all
    /// `digits`; in an unpredictable one, of a random sign and a random count of digits.
    fn value(&mut self, digits: u32, predictability: Predictability) -> i128 {
        self.at_least(0, digits, predictability)
    }

    /// A value of more than `least` and at most `most` digits of steps, so at least `10^least`:
    /// in a predictable set, positive with all `most`; in an unpredictable one, of a random sign
    /// and a random count of digits.
    fn at_least(&mut self, least: u32, most: u32, predictability: Predictability) -> i128 {
        let digits = match predictability {
            Predictability::Predictable => most,
            Predictability::Unpredictable => {
                let span = u128::from(most.saturating_sub(least).max(1));
                least.saturating_add(1).saturating_add(u32::try_from(self.below(span)).unwrap_or(0))
            },
        };
        let floor = 10_u128.pow(digits.saturating_sub(1)).max(10_u128.pow(least));
        let ceiling = 10_u128.pow(digits);
        let magnitude = floor.saturating_add(self.below(ceiling.saturating_sub(floor).max(1)));
        let magnitude = i128::try_from(magnitude).unwrap_or(i128::MAX);
        match predictability {
            Predictability::Unpredictable if self.next() & 1 == 1 => magnitude.wrapping_neg(),
            Predictability::Predictable | Predictability::Unpredictable => magnitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Predictability, Width, addends, dividends_and_divisors, doubles, factors,
        prices_and_quantities, texts,
    };
    use crate::oracle::{self, Mode};

    #[test]
    fn a_set_is_the_same_on_every_run() {
        assert_eq!(
            addends(Width::Narrow, Predictability::Unpredictable),
            addends(Width::Narrow, Predictability::Unpredictable),
            "one seed"
        );
        assert_ne!(
            addends(Width::Narrow, Predictability::Unpredictable).left,
            factors(Width::Narrow, Predictability::Unpredictable).left,
            "a salt each"
        );
    }

    #[test]
    fn a_predictable_set_has_one_sign_and_one_count_of_digits() {
        let set = factors(Width::Narrow, Predictability::Predictable);
        assert_eq!(set.left.len(), 1024, "L1-sized");
        assert!(
            set.left.iter().all(|&value| (10_i128.pow(12)..10_i128.pow(13)).contains(&value)),
            "13 digits, positive"
        );
    }

    #[test]
    fn an_unpredictable_set_mixes_signs_and_digits() {
        let set = addends(Width::Wide, Predictability::Unpredictable);
        assert_eq!(set.left.len(), 65_536, "past the predictor");
        let negative = set.left.iter().filter(|&&value| value < 0).count();
        assert!((30_000..35_000).contains(&negative), "about half below zero: {negative}");
        let short =
            set.left.iter().filter(|&&value| value.unsigned_abs() < 10_u128.pow(10)).count();
        assert!(short > 10_000, "many with few digits: {short}");
    }

    #[test]
    fn every_result_fits_its_width() {
        for width in Width::ALL {
            let decimals = width.decimals();
            let limit = match width {
                Width::Narrow => i128::from(i64::MAX),
                Width::Wide => 79_228_162_514_264_337_593_543_950_335, // rust_decimal's largest
            };
            for predictability in Predictability::ALL {
                let set = addends(width, predictability);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    assert!(oracle::add(a, b).is_some_and(|sum| sum.abs() <= limit), "{a} + {b}");
                }
                let set = factors(width, predictability);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    let product = oracle::mul_round(a, b, decimals, Mode::HalfEven)
                        .expect("factors below 10^5 make a product an i128 holds");
                    assert!(product.abs() <= limit, "{a} × {b}");
                }
                let set = dividends_and_divisors(width, predictability);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    assert!(
                        b.unsigned_abs() >= 10_u128.pow(u32::from(decimals) - 3),
                        "a divisor of 0.001 or more: {b}"
                    );
                    let quotient = oracle::div_round(a, b, decimals, Mode::HalfEven)
                        .expect("a divisor of 0.001 or more keeps the quotient below 10^9");
                    assert!(quotient.abs() <= limit, "{a} / {b}");
                }
            }
        }
        let set = prices_and_quantities(Predictability::Unpredictable);
        assert!(
            set.left
                .iter()
                .zip(&set.right)
                .all(|(&price, &quantity)| oracle::mul_exact(price, quantity).is_some()),
            "exact"
        );
    }

    #[test]
    fn a_text_and_a_double_are_of_the_set_values() {
        let texts = texts(Width::Narrow, Predictability::Predictable);
        let first = texts.first().expect("a predictable set holds 1,024 values");
        assert_eq!(
            oracle::parse(first, 8),
            addends(Width::Narrow, Predictability::Predictable).left.first().copied(),
            "{first}"
        );
        assert!(
            doubles(Width::Wide, Predictability::Unpredictable)
                .iter()
                .all(|double| double.is_finite()),
            "finite"
        );
    }
}
