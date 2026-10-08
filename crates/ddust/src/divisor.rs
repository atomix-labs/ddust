//! A divisor prepared once, for many divisions by it: [`Divisor`], and [`DivisorOf`], what
//! [`div_round`](Decimal::div_round) takes, a decimal or a prepared divisor.

use core::hash::{Hash, Hasher};

use crate::decimal::Decimal;
use crate::int::{Int, Outcome};
use crate::scale::Scale;

/// The work a prepared divisor's divisions share, in the magnitude's word `U` and its double `D`.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct Prepared<U, D> {
    /// Whether the divisor is below zero.
    pub(crate) negative: bool,
    /// `⌊10^k / b⌋`, modulo the magnitude's word: the divisor's whole part of each step's `10^k`.
    pub(crate) whole: U,
    /// Whether the whole part was past the word, so that every dividend but zero overflows.
    pub(crate) whole_past: bool,
    /// `⌈2^n × (10^k mod b) / b⌉`, `n` being the double's bits: the multiplier of the rest.
    pub(crate) multiplier: D,
    /// The fractions a remainder of one leaves, of half rounded up, and of half rounded down and
    /// one: where each class of remainder starts.
    pub(crate) thresholds: [D; 3],
}

/// A divisor prepared once, for dividing many values by it.
///
/// A book's notionals by one price, or amounts by one rate: [`div_round`](Decimal::div_round) and
/// [`checked_div_round`](Decimal::checked_div_round) take it where they take a decimal, and divide
/// by two multiplications and a comparison where by a decimal they divide. Preparing it takes a
/// division, so it pays where one divisor divides many values.
///
/// # Examples
/// ```
/// use ddust::round::HalfEven;
/// use ddust::{D64, Divisor, dec};
///
/// let price: D64<2> = dec!(64_250.50);
/// let by_price = Divisor::new(price).expect("not zero");
/// let notional: D64<8> = dec!(1_000_000);
/// assert_eq!(notional.div_round(by_price, HalfEven), dec!(15.56408121), "units of it");
/// assert_eq!(notional.div_round(by_price, HalfEven), notional.div_round(price, HalfEven));
/// assert!(Divisor::new(D64::<2>::ZERO).is_none(), "never zero");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Divisor<I: Int, S: Scale> {
    /// The value it divides by.
    value: Decimal<I, S>,
    /// What its divisions share.
    prepared: I::Prepared,
}

// By its value alone: what is prepared follows from it, and two values equal as decimals, `1.5`
// and `1.50` at run-time scales, divide alike.
const impl<I: [const] Int, S: [const] Scale> PartialEq for Divisor<I, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

const impl<I: [const] Int, S: [const] Scale> Eq for Divisor<I, S> {}

impl<I: Int, S: Scale> Hash for Divisor<I, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<I: Int, S: Scale> Divisor<I, S> {
    /// `value` prepared as a divisor, or `None` for zero.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Divisor, dec};
    ///
    /// let rate: D64<4> = dec!(1.0825);
    /// assert_eq!(Divisor::new(rate).map(Divisor::get), Some(rate), "prepared");
    /// assert!(Divisor::new(D64::<4>::ZERO).is_none(), "never zero");
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(value: Decimal<I, S>) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match value.steps().prepare_divisor(value.decimals()) {
            Some(prepared) => Some(Self { value, prepared }),
            None => None,
        }
    }

    /// The value it divides by.
    #[inline]
    #[must_use]
    pub const fn get(self) -> Decimal<I, S> {
        self.value
    }
}

/// The seal on [`DivisorOf`].
mod sealed {
    use crate::decimal::Decimal;
    use crate::divisor::Divisor;
    use crate::int::Int;
    use crate::scale::Scale;

    /// Seals [`DivisorOf`](super::DivisorOf): a decimal and a prepared divisor are all there are.
    pub trait Sealed {}

    impl<I, S> Sealed for Decimal<I, S> {}

    impl<I: Int, S: Scale> Sealed for Divisor<I, S> {}
}

/// What [`div_round`](Decimal::div_round) and [`checked_div_round`](Decimal::checked_div_round)
/// divide a decimal of steps `I` by: a decimal of any scale, or a [`Divisor`] prepared from one.
///
/// # Examples
/// ```
/// use ddust::round::Trunc;
/// use ddust::{D64, Divisor, dec};
///
/// let (total, count): (D64<2>, D64<0>) = (dec!(100), dec!(3));
/// let by_count = Divisor::new(count).expect("not zero");
/// assert_eq!(total.div_round(count, Trunc), dec!(33.33), "by a decimal");
/// assert_eq!(total.div_round(by_count, Trunc), dec!(33.33), "by a prepared divisor");
/// ```
pub const trait DivisorOf<I: Int>: sealed::Sealed + Copy {
    /// `dividend × 10^k / divisor`, rounded by `table`, `k` being the divisor's decimals; `None`
    /// for a zero divisor.
    #[doc(hidden)]
    fn divide_round(dividend: I, divisor: Self, table: u16) -> Option<Outcome<I>>;
}

const impl<I: [const] Int, T: [const] Scale> DivisorOf<I> for Decimal<I, T> {
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        reason = "as `Int::div_up`'s: out of line, a binary that rounds by two modes passes the table at run time"
    )]
    fn divide_round(dividend: I, divisor: Self, table: u16) -> Option<Outcome<I>> {
        let steps = divisor.steps();
        if steps == I::ZERO {
            return None;
        }
        Some(dividend.div_up(divisor.decimals(), steps, table))
    }
}

const impl<I: [const] Int, T: Scale> DivisorOf<I> for Divisor<I, T> {
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        reason = "as `Int::div_up`'s: out of line, a binary that rounds by two modes passes the table at run time"
    )]
    fn divide_round(dividend: I, divisor: Self, table: u16) -> Option<Outcome<I>> {
        Some(dividend.div_prepared(divisor.prepared, table))
    }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the tests' own arithmetic, building each edge")]
mod tests {
    use proptest::prelude::*;
    use proptest::sample::select;
    use rstest::rstest;

    use super::Divisor;
    use crate::cmp::tests::hash_of;
    use crate::round::{HalfEven, Rounding, RoundingMode};
    use crate::{D64, D128, Decimal, Dynamic, Int, dec};

    /// The prepared division against the decimal's, every way an outcome is read: its wrapped
    /// value, whether it overflowed, and its value held at the range's end.
    fn check<I: Int>(a: I, b: I, k: u8, mode: Rounding) -> Result<(), TestCaseError> {
        let table = mode.table();
        let Some(prepared) = b.prepare_divisor(k) else {
            prop_assert_eq!(b, I::ZERO, "only zero is refused");
            return Ok(());
        };
        let (expected, got) = (a.div_up(k, b, table), a.div_prepared(prepared, table));
        prop_assert_eq!(
            got.overflowing(),
            expected.overflowing(),
            "{} × 10^{} / {}, {:?}",
            a,
            k,
            b,
            mode
        );
        prop_assert_eq!(
            got.saturating(),
            expected.saturating(),
            "{} × 10^{} / {}, {:?}",
            a,
            k,
            b,
            mode
        );
        Ok(())
    }

    /// Every integer type, by any dividend and divisor, at every scale and in every mode.
    macro_rules! every_type {
        ($($name:ident: $t:ty),*) => {
            proptest! {$(
                #[test]
                fn $name(a: $t, b: $t, k in 0_u8..=38, mode in select(Rounding::MODES)) {
                    check(a, b, k, mode)?;
                }
            )*}
        };
    }

    every_type!(
        an_i8_divides_as_the_decimal_does: i8,
        an_i16_divides_as_the_decimal_does: i16,
        an_i32_divides_as_the_decimal_does: i32,
        an_i64_divides_as_the_decimal_does: i64,
        an_i128_divides_as_the_decimal_does: i128,
        a_u8_divides_as_the_decimal_does: u8,
        a_u16_divides_as_the_decimal_does: u16,
        a_u32_divides_as_the_decimal_does: u32,
        a_u64_divides_as_the_decimal_does: u64,
        a_u128_divides_as_the_decimal_does: u128
    );

    /// No remainder, then either side of where each class of remainder starts: one, half rounded
    /// up, and half rounded down and one, which meet for an odd divisor.
    fn edge_remainder<T: Int>(b: T, edge: usize) -> T {
        let two = T::ONE + T::ONE;
        let (half, half_up) = (b / two, b - b / two);
        [T::ZERO, T::ONE, half_up - T::ONE, half_up, half, half + T::ONE, b - T::ONE][edge]
    }

    proptest! {
        #[test]
        fn a_remainder_at_each_edge_rounds_as_the_decimal_does(
            q in 0_i64..1 << 40, b in 3_i64..1 << 20, edge in 0_usize..7, negative: bool,
            mode in select(Rounding::MODES),
        ) {
            // At no decimals, where the dividend alone sets the remainder.
            let a = q * b + edge_remainder(b, edge);
            check(if negative { -a } else { a }, b, 0, mode)?;
            check(i128::from(a) << 60, i128::from(b) << 60, 0, mode)?;
        }

        #[test]
        fn a_wide_remainder_at_each_edge_rounds_as_the_decimal_does(
            q: u128, b in 3_u128.., edge in 0_usize..7, mode in select(Rounding::MODES),
        ) {
            // Fewer whole divisors than `u128::MAX / b`, so the dividend fits.
            let a = q % (u128::MAX / b) * b + edge_remainder(b, edge);
            check(a, b, 0, mode)?;
        }
    }

    #[rstest]
    fn each_edge_divides_as_the_decimal_does(
        #[values(i64::MIN, -3, -1, 0, 1, 2, 7, i64::MAX)] a: i64,
        #[values(i64::MIN, -2, -1, 1, 2, 3, 100_000_000, 100_000_001, i64::MAX)] b: i64,
        #[values(0, 1, 8, 18, 19, 38)] k: u8,
    ) {
        for mode in Rounding::MODES {
            check(a, b, k, *mode).expect("agrees");
            check(i128::from(a) * i128::from(a), i128::from(b) << 64, k, *mode).expect("agrees");
            check(a.unsigned_abs(), b.unsigned_abs(), k, *mode).expect("agrees");
        }
    }

    #[test]
    fn a_divisor_divides_as_its_value_does() {
        let price: D64<2> = dec!(64_250.50);
        let by_price = Divisor::new(price).expect("not zero");
        let notional: D64<8> = dec!(1_000_000);
        assert_eq!(
            notional.div_round(by_price, HalfEven),
            notional.div_round(price, HalfEven),
            "narrow"
        );
        assert_eq!(
            notional.checked_div_round(by_price, HalfEven),
            Some(dec!(15.56408121)),
            "units of it"
        );
        let wide: D128<18> = dec!(-2.5);
        let by_wide = Divisor::new(wide).expect("not zero");
        let amount: D128<18> = dec!(1_000.000000000000000001);
        assert_eq!(amount.div_round(by_wide, HalfEven), amount.div_round(wide, HalfEven), "wide");
        let run_time = Decimal::from_steps(3_i64, Dynamic::new(0).expect("at most 38"));
        assert_eq!(Divisor::new(run_time).map(Divisor::get), Some(run_time), "a run-time scale");
    }

    #[test]
    fn a_quotient_past_the_range_is_none() {
        let by_half = Divisor::new(dec!(0.5: D64<2>)).expect("0.5 is not zero");
        assert_eq!(D64::<8>::MAX.checked_div_round(by_half, HalfEven), None, "twice the largest");
    }

    #[test]
    fn a_zero_divisor_is_refused() {
        assert_eq!(Divisor::new(D64::<2>::ZERO), None, "never zero");
    }

    #[test]
    fn two_divisors_equal_as_decimals_are_equal_and_hash_alike() {
        let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
        let (short, long) =
            (Decimal::from_steps(15_i64, scale(1)), Decimal::from_steps(150_i64, scale(2)));
        let (a, b) = (Divisor::new(short), Divisor::new(long));
        assert_eq!(a, b, "1.5 and 1.50");
        assert_eq!(hash_of(&a), hash_of(&b), "and hash alike");
    }
}
