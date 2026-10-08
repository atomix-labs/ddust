//! A divisor prepared once, for many divisions by it: [`Divisor`], and [`DivideBy`], what
//! [`div_round`](Decimal::div_round) takes, a decimal or a prepared divisor.

use crate::decimal::Decimal;
use crate::int::{Int, Outcome};
use crate::scale::Scale;

/// What a prepared divisor keeps, in its integer's magnitude word `U` and its double `D`: the work
/// its divisions share.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct Prepared<U, D> {
    /// Whether the divisor is below zero.
    pub(crate) negative: bool,
    /// `⌊10^k / b⌋`, modulo the magnitude's word: the divisor's whole part of each step's `10^k`.
    pub(crate) whole: U,
    /// Whether the whole part was past the word, so that every dividend but zero overflows.
    pub(crate) whole_past: bool,
    /// `⌈2^n × (10^k mod b) / b⌉`, `n` being the word's bits: the multiplier of the rest.
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
        match value.steps().prepare(value.decimals()) {
            Some(prepared) => Some(Self { value, prepared }),
            None => None,
        }
    }

    /// The value it divides by.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Divisor, dec};
    ///
    /// let price: D64<2> = dec!(99.95);
    /// let by_price = Divisor::new(price).expect("not zero");
    /// assert_eq!(by_price.get(), price);
    /// ```
    #[inline]
    #[must_use]
    pub const fn get(self) -> Decimal<I, S> {
        self.value
    }
}

/// The seal on [`DivideBy`].
mod sealed {
    use crate::decimal::Decimal;
    use crate::divisor::Divisor;
    use crate::int::Int;
    use crate::scale::Scale;

    /// Seals [`DivideBy`](super::DivideBy): a decimal and a prepared divisor are all there are.
    pub trait Sealed {}

    impl<I, S> Sealed for Decimal<I, S> {}

    impl<I: Int, S: Scale> Sealed for Divisor<I, S> {}
}

/// What a decimal of steps `I` divides by: a decimal of any scale, or a [`Divisor`] prepared from
/// one, which [`div_round`](Decimal::div_round) and
/// [`checked_div_round`](Decimal::checked_div_round) take alike.
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
pub const trait DivideBy<I: Int>: sealed::Sealed + Copy {
    /// `dividend × 10^k / self`, rounded by `table`, `k` being the divisor's decimals; `None` for a
    /// zero divisor.
    #[doc(hidden)]
    fn divide_round(self, dividend: I, table: u16) -> Option<Outcome<I>>;
}

const impl<I: [const] Int, T: [const] Scale> DivideBy<I> for Decimal<I, T> {
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        reason = "as `Int::div_up`'s: out of line, a binary that rounds by two modes passes the table at run time"
    )]
    fn divide_round(self, dividend: I, table: u16) -> Option<Outcome<I>> {
        let divisor = self.steps();
        if divisor == I::ZERO {
            return None;
        }
        Some(dividend.div_up(self.decimals(), divisor, table))
    }
}

const impl<I: [const] Int, T: Scale> DivideBy<I> for Divisor<I, T> {
    #[inline(always)]
    #[expect(
        clippy::inline_always,
        reason = "as `Int::div_up`'s: out of line, a binary that rounds by two modes passes the table at run time"
    )]
    fn divide_round(self, dividend: I, table: u16) -> Option<Outcome<I>> {
        Some(dividend.div_prepared(self.prepared, table))
    }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the tests' own arithmetic, building each edge")]
mod tests {
    use proptest::prelude::*;
    use proptest::sample::select;
    use rstest::rstest;

    use super::Divisor;
    use crate::round::{HalfEven, Rounding, RoundingMode};
    use crate::{D64, D128, Decimal, Dynamic, Int, dec};

    /// The prepared division against the decimal's, every way an outcome is read: its wrapped
    /// value, whether it overflowed, and its value held at the range's end.
    fn check<I: Int>(a: I, b: I, k: u8, mode: Rounding) -> Result<(), TestCaseError> {
        let table = mode.table();
        let Some(prepared) = b.prepare(k) else {
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

    proptest! {
        #[test]
        fn a_remainder_at_each_edge_rounds_as_the_decimal_does(
            q in 0_i64..1 << 40, b in 2_i64..1 << 20, edge in 0_usize..4, negative: bool,
            mode in select(Rounding::MODES),
        ) {
            // q × b plus no remainder, one, half of an even divisor, and one below the divisor:
            // where each class of remainder starts, at no decimals, where the dividend sets it.
            let b = b & !1;
            let remainder = [0, 1, b / 2, b - 1][edge];
            let a = q * b + remainder;
            check(if negative { -a } else { a }, b, 0, mode)?;
            check(i128::from(a) << 60, i128::from(b) << 60, 0, mode)?;
        }

        #[test]
        fn a_wide_remainder_at_each_edge_rounds_as_the_decimal_does(
            q: u64, b in 2_u128.., edge in 0_usize..4, mode in select(Rounding::MODES),
        ) {
            let b = b & !1;
            let remainder = [0, 1, b / 2, b - 1][edge];
            if let Some(a) = u128::from(q).checked_mul(b).and_then(|a| a.checked_add(remainder)) {
                check(a, b, 0, mode)?;
            }
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
    fn a_divisor_divides_as_its_value_does_and_refuses_zero() {
        let price: D64<2> = dec!(64_250.50);
        let by_price = Divisor::new(price).expect("not zero");
        let notional: D64<8> = dec!(1_000_000);
        assert_eq!(notional.div_round(by_price, HalfEven), notional.div_round(price, HalfEven));
        assert_eq!(notional.checked_div_round(by_price, HalfEven), Some(dec!(15.56408121)));
        assert_eq!(
            D64::<8>::MAX
                .checked_div_round(Divisor::new(dec!(0.5: D64<2>)).expect("not zero"), HalfEven),
            None,
            "past the range"
        );
        assert!(Divisor::new(D64::<2>::ZERO).is_none(), "never zero");
        let wide: D128<18> = dec!(-2.5);
        let by_wide = Divisor::new(wide).expect("not zero");
        let amount: D128<18> = dec!(1_000.000000000000000001);
        assert_eq!(amount.div_round(by_wide, HalfEven), amount.div_round(wide, HalfEven));
        let run_time = Decimal::from_steps(3_i64, Dynamic::new(0).expect("at most 38"));
        assert_eq!(Divisor::new(run_time).map(Divisor::get), Some(run_time), "a run-time scale");
    }
}
