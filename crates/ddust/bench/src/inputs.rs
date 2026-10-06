//! The values every contender reads: drawn once from a fixed seed, as steps at the decimals a width
//! carries, in two sets each operation is timed on.
//!
//! - **Predictable**: one sign and one count of digits, 1,024 values, which stay in the level 1
//!   cache: the best case of a branch on the value.
//! - **Unpredictable**: random signs and counts of digits, 65,536 values, more than a branch
//!   predictor learns; a Neoverse V2 learns 2,048 random branches perfectly, and misses half of
//!   65,536.
//!
//! Each set's values are in a range every contender holds, and its results too: no row is timed on
//! an overflow.

use crate::oracle;

/// The values in a predictable set.
pub const PREDICTABLE: usize = 1024;

/// The values in an unpredictable set.
pub const UNPREDICTABLE: usize = 65_536;

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
            Self::Narrow => "64",
            Self::Wide => "128",
        }
    }
}

/// Which of the two sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Inputs {
    /// One sign and one count of digits, in the level 1 cache.
    Predictable,
    /// Random signs and counts of digits, past what a predictor learns.
    Unpredictable,
}

impl Inputs {
    /// Both sets.
    pub const ALL: [Self; 2] = [Self::Predictable, Self::Unpredictable];

    /// The set's name in a measurement's.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Predictable => "predictable",
            Self::Unpredictable => "unpredictable",
        }
    }

    /// How many values the set holds.
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::Predictable => PREDICTABLE,
            Self::Unpredictable => UNPREDICTABLE,
        }
    }

    /// The seed the set's values are drawn from, the same on every run.
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
pub fn sums(width: Width, inputs: Inputs) -> Pairs {
    let digits = sum_digits(width);
    pairs(inputs, 1, |random| random.value(digits, inputs), |random| random.value(digits, inputs))
}

/// Operands of a product at one scale: values below `10^5`, so the product is below `10^10`.
#[must_use]
pub fn products(width: Width, inputs: Inputs) -> Pairs {
    let digits = u32::from(width.decimals()).saturating_add(5);
    pairs(inputs, 2, |random| random.value(digits, inputs), |random| random.value(digits, inputs))
}

/// Operands of a quotient at one scale: dividends below `10^6` and divisors from `10^-3` to `10^5`,
/// so the quotient is below `10^9`.
#[must_use]
pub fn quotients(width: Width, inputs: Inputs) -> Pairs {
    let decimals = u32::from(width.decimals());
    pairs(
        inputs,
        3,
        |random| random.value(decimals.saturating_add(6), inputs),
        |random| random.at_least(decimals.saturating_sub(3), decimals.saturating_add(5), inputs),
    )
}

/// A price's steps at 2 decimals, below 100,000, and a quantity's at 5, below 100: their exact
/// product, at 7, fits every width.
#[must_use]
pub fn prices_and_quantities(inputs: Inputs) -> Pairs {
    pairs(inputs, 4, |random| random.value(7, inputs), |random| random.value(7, inputs))
}

/// Values to round, write, or convert to a double: the operands of a sum.
#[must_use]
pub fn values(width: Width, inputs: Inputs) -> Vec<i128> {
    sums(width, inputs).left
}

/// Each value's shortest exact text, to parse.
#[must_use]
pub fn texts(width: Width, inputs: Inputs) -> Vec<String> {
    values(width, inputs).into_iter().map(|steps| oracle::text(steps, width.decimals())).collect()
}

/// The double nearest each value, to convert back.
#[must_use]
pub fn doubles(width: Width, inputs: Inputs) -> Vec<f64> {
    values(width, inputs).into_iter().map(|steps| oracle::to_f64(steps, width.decimals())).collect()
}

/// The digits of steps a sum's operands have at most, at `width`.
const fn sum_digits(width: Width) -> u32 {
    match width {
        Width::Narrow => 17,
        Width::Wide => 28,
    }
}

/// `inputs.count()` pairs, the left from `left` and the right from `right`, drawn from the set's
/// seed salted by `salt`, so each operation has its own values.
fn pairs(
    inputs: Inputs, salt: u64, mut left: impl FnMut(&mut Random) -> i128,
    mut right: impl FnMut(&mut Random) -> i128,
) -> Pairs {
    let mut random = Random(inputs.seed(salt));
    let (mut lefts, mut rights) =
        (Vec::with_capacity(inputs.count()), Vec::with_capacity(inputs.count()));
    for _ in 0..inputs.count() {
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
    fn value(&mut self, digits: u32, inputs: Inputs) -> i128 {
        self.at_least(0, digits, inputs)
    }

    /// A value of more than `least` and at most `most` digits of steps, so at least `10^least`:
    /// in a predictable set, positive with all `most`; in an unpredictable one, of a random sign
    /// and a random count of digits.
    fn at_least(&mut self, least: u32, most: u32, inputs: Inputs) -> i128 {
        let digits = match inputs {
            Inputs::Predictable => most,
            Inputs::Unpredictable => {
                let span = u128::from(most.saturating_sub(least).max(1));
                least.saturating_add(1).saturating_add(u32::try_from(self.below(span)).unwrap_or(0))
            },
        };
        let floor = 10_u128.pow(digits.saturating_sub(1)).max(10_u128.pow(least));
        let ceiling = 10_u128.pow(digits);
        let magnitude = floor.saturating_add(self.below(ceiling.saturating_sub(floor).max(1)));
        let magnitude = i128::try_from(magnitude).unwrap_or(i128::MAX);
        match inputs {
            Inputs::Unpredictable if self.next() & 1 == 1 => magnitude.wrapping_neg(),
            Inputs::Predictable | Inputs::Unpredictable => magnitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Inputs, Width, doubles, prices_and_quantities, products, quotients, sums, texts};
    use crate::oracle::{self, Mode};

    #[test]
    fn a_set_is_the_same_on_every_run() {
        assert_eq!(
            sums(Width::Narrow, Inputs::Unpredictable),
            sums(Width::Narrow, Inputs::Unpredictable),
            "one seed"
        );
        assert_ne!(
            sums(Width::Narrow, Inputs::Unpredictable).left,
            products(Width::Narrow, Inputs::Unpredictable).left,
            "a salt each"
        );
    }

    #[test]
    fn a_predictable_set_has_one_sign_and_one_count_of_digits() {
        let set = products(Width::Narrow, Inputs::Predictable);
        assert_eq!(set.left.len(), 1024, "L1-sized");
        assert!(
            set.left.iter().all(|&value| (10_i128.pow(12)..10_i128.pow(13)).contains(&value)),
            "13 digits, positive"
        );
    }

    #[test]
    fn an_unpredictable_set_mixes_signs_and_digits() {
        let set = sums(Width::Wide, Inputs::Unpredictable);
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
            for inputs in Inputs::ALL {
                let set = sums(width, inputs);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    assert!(oracle::add(a, b).is_some_and(|sum| sum.abs() <= limit), "{a} + {b}");
                }
                let set = products(width, inputs);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    let product =
                        oracle::mul_round(a, b, decimals, Mode::HalfEven).expect("an i128");
                    assert!(product.abs() <= limit, "{a} × {b}");
                }
                let set = quotients(width, inputs);
                for (&a, &b) in set.left.iter().zip(&set.right) {
                    assert!(
                        b.unsigned_abs() >= 10_u128.pow(u32::from(decimals) - 3),
                        "a divisor of 0.001 or more: {b}"
                    );
                    let quotient =
                        oracle::div_round(a, b, decimals, Mode::HalfEven).expect("an i128");
                    assert!(quotient.abs() <= limit, "{a} / {b}");
                }
            }
        }
        let set = prices_and_quantities(Inputs::Unpredictable);
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
        let texts = texts(Width::Narrow, Inputs::Predictable);
        let first = texts.first().expect("a value");
        assert_eq!(
            oracle::parse(first, 8),
            sums(Width::Narrow, Inputs::Predictable).left.first().copied(),
            "{first}"
        );
        assert!(
            doubles(Width::Wide, Inputs::Unpredictable).iter().all(|double| double.is_finite()),
            "finite"
        );
    }
}
