//! The operations that round: a product or quotient at a scale, a fused product over a divisor,
//! a value to a step or a whole number. Each takes its rounding mode last, and has a `checked_`
//! twin; past the range the other panics with overflow checks on, and wraps otherwise.

use crate::decimal::Decimal;
use crate::int::{Int, Operation, Outcome};
use crate::round::{Ceil, Floor, RoundingMode, Trunc};
use crate::scale::Scale;

/// The steps of a product of values at `a` and `b` decimals, at `to`, rounded by `table`.
#[inline]
const fn product<I: [const] Int>(x: I, y: I, a: u8, b: u8, to: u8, table: u16) -> Outcome<I> {
    let from = a.saturating_add(b);
    if from >= to {
        x.mul_down(y, from.wrapping_sub(to), table)
    } else {
        x.mul_up(y, to.wrapping_sub(from))
    }
}

/// The steps of the quotient of values at `a` and `b` decimals, at `to`, rounded by `table`: `x ×
/// 10^(to + b - a) / y`, for a `y` that is not zero.
#[inline]
const fn quotient<I: [const] Int>(x: I, y: I, a: u8, b: u8, to: u8, table: u16) -> Outcome<I> {
    let up = to.saturating_add(b);
    if up >= a {
        x.div_up(up.wrapping_sub(a), y, table)
    } else {
        x.div_down(y, a.wrapping_sub(up), table)
    }
}

/// Panics as the integer's `/` does for a zero divisor.
#[cold]
#[inline(never)]
#[track_caller]
#[expect(clippy::panic, reason = "the integer's own division by zero, with its own message")]
const fn divided_by_zero() -> ! {
    panic!("attempt to divide by zero")
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The product with `rhs` at this value's scale, rounded by `mode`: a rate applied to an
    /// amount, keeping the amount's decimals.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::Ceil;
    /// use ddust::{D64, dec};
    ///
    /// let (amount, fee): (D64<2>, D64<4>) = (dec!(100.10), dec!(0.0025));
    /// assert_eq!(amount.mul_round(fee, Ceil), dec!(0.26), "0.25025, up to the cent");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn mul_round<T, R>(self, rhs: Decimal<I, T>, mode: R) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        let steps = self.to_bits().mul_down(rhs.to_bits(), rhs.decimals(), mode.table());
        Self::from_bits(steps.operator(Operation::Multiply), self.scale())
    }

    /// The product with `rhs` at this value's scale, rounded by `mode`, or `None` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<1>, D8<1>) = (dec!(12), dec!(1.5));
    /// assert_eq!(a.checked_mul_round(b, HalfEven), None, "18 is past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_mul_round<T, R>(self, rhs: Decimal<I, T>, mode: R) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        match self.to_bits().mul_down(rhs.to_bits(), rhs.decimals(), mode.table()).checked() {
            Some(steps) => Some(Self::from_bits(steps, self.scale())),
            None => None,
        }
    }

    /// The product with `rhs` at `scale`, rounded by `mode` when the scale has fewer decimals than
    /// the exact product.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::Trunc;
    /// use ddust::{D64, Fixed, dec};
    ///
    /// let (price, size): (D64<2>, D64<4>) = (dec!(60000.37), dec!(0.0125));
    /// let notional: D64<3> = price.mul_round_to(size, Fixed, Trunc);
    /// assert_eq!(notional, dec!(750.004), "750.004625, truncated to three");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn mul_round_to<T, O, R>(self, rhs: Decimal<I, T>, scale: O, mode: R) -> Decimal<I, O>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        O: [const] Scale,
        R: [const] RoundingMode,
    {
        let (a, b) = (self.decimals(), rhs.decimals());
        let steps = product(self.to_bits(), rhs.to_bits(), a, b, scale.decimals(), mode.table());
        Decimal::from_bits(steps.operator(Operation::Multiply), scale)
    }

    /// The product with `rhs` at `scale`, rounded by `mode`, or `None` past the range.
    #[inline]
    #[must_use]
    pub const fn checked_mul_round_to<T, O, R>(
        self, rhs: Decimal<I, T>, scale: O, mode: R,
    ) -> Option<Decimal<I, O>>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        O: [const] Scale,
        R: [const] RoundingMode,
    {
        let (a, b) = (self.decimals(), rhs.decimals());
        match product(self.to_bits(), rhs.to_bits(), a, b, scale.decimals(), mode.table()).checked()
        {
            Some(steps) => Some(Decimal::from_bits(steps, scale)),
            None => None,
        }
    }

    /// The quotient by `rhs` at this value's scale, rounded by `mode`.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's `/` does.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::Floor;
    /// use ddust::{D64, dec};
    ///
    /// let (budget, price): (D64<4>, D64<2>) = (dec!(1000), dec!(60000.37));
    /// assert_eq!(budget.div_round(price, Floor), dec!(0.0166), "what a thousand buys, to the lot");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn div_round<T, R>(self, rhs: Decimal<I, T>, mode: R) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = rhs.to_bits();
        if divisor == I::ZERO {
            divided_by_zero();
        }
        let steps = self.to_bits().div_up(rhs.decimals(), divisor, mode.table());
        Self::from_bits(steps.operator(Operation::Divide), self.scale())
    }

    /// The quotient by `rhs` at this value's scale, rounded by `mode`, or `None` for a zero divisor
    /// or past the range.
    #[inline]
    #[must_use]
    pub const fn checked_div_round<T, R>(self, rhs: Decimal<I, T>, mode: R) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = rhs.to_bits();
        if divisor == I::ZERO {
            return None;
        }
        match self.to_bits().div_up(rhs.decimals(), divisor, mode.table()).checked() {
            Some(steps) => Some(Self::from_bits(steps, self.scale())),
            None => None,
        }
    }

    /// The quotient by `rhs` at `scale`, rounded by `mode`.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's `/` does.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, Fixed, dec};
    ///
    /// let (a, b): (D64<2>, D64<2>) = (dec!(100), dec!(3));
    /// let third: D64<6> = a.div_round_to(b, Fixed, HalfEven);
    /// assert_eq!(third, dec!(33.333333), "six decimals");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn div_round_to<T, O, R>(self, rhs: Decimal<I, T>, scale: O, mode: R) -> Decimal<I, O>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        O: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = rhs.to_bits();
        if divisor == I::ZERO {
            divided_by_zero();
        }
        let (a, b) = (self.decimals(), rhs.decimals());
        let steps = quotient(self.to_bits(), divisor, a, b, scale.decimals(), mode.table());
        Decimal::from_bits(steps.operator(Operation::Divide), scale)
    }

    /// The quotient by `rhs` at `scale`, rounded by `mode`, or `None` for a zero divisor or past
    /// the range.
    #[inline]
    #[must_use]
    pub const fn checked_div_round_to<T, O, R>(
        self, rhs: Decimal<I, T>, scale: O, mode: R,
    ) -> Option<Decimal<I, O>>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        O: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = rhs.to_bits();
        if divisor == I::ZERO {
            return None;
        }
        let (a, b) = (self.decimals(), rhs.decimals());
        match quotient(self.to_bits(), divisor, a, b, scale.decimals(), mode.table()).checked() {
            Some(steps) => Some(Decimal::from_bits(steps, scale)),
            None => None,
        }
    }

    /// One of `n` equal parts, rounded by `mode`.
    ///
    /// # Panics
    /// For zero parts, as the integer's `/` does.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, dec};
    ///
    /// let amount: D64<2> = dec!(100.10);
    /// assert_eq!(amount.div_int_round(3, HalfEven), dec!(33.37), "a third, to the cent");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn div_int_round<R>(self, n: I, mode: R) -> Self
    where
        I: [const] Int,
        R: [const] RoundingMode,
    {
        if n == I::ZERO {
            divided_by_zero();
        }
        let steps = self.to_bits().div_down(n, 0, mode.table());
        Self::from_bits(steps.operator(Operation::Divide), self.scale())
    }

    /// One of `n` equal parts, rounded by `mode`, or `None` for zero parts or past the range.
    #[inline]
    #[must_use]
    pub const fn checked_div_int_round<R>(self, n: I, mode: R) -> Option<Self>
    where
        I: [const] Int,
        R: [const] RoundingMode,
    {
        if n == I::ZERO {
            return None;
        }
        match self.to_bits().div_down(n, 0, mode.table()).checked() {
            Some(steps) => Some(Self::from_bits(steps, self.scale())),
            None => None,
        }
    }

    /// `self × numerator / denominator` at this value's scale, rounded once by `mode`: a share of
    /// an amount, or an amount converted at a rate given as two values, with no rounding between.
    ///
    /// # Panics
    /// For a zero denominator, as the integer's `/` does.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, dec};
    ///
    /// let (pot, share, shares): (D64<2>, D64<0>, D64<0>) = (dec!(1000), dec!(1), dec!(3));
    /// assert_eq!(pot.mul_div_round(share, shares, HalfEven), dec!(333.33), "a third, once rounded");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn mul_div_round<T, R>(
        self, numerator: Decimal<I, T>, denominator: Decimal<I, T>, mode: R,
    ) -> Self
    where
        I: [const] Int,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = Self::ratio_divisor(numerator, denominator);
        let steps = self.to_bits().mul_div(numerator.to_bits(), divisor, mode.table());
        Self::from_bits(steps.operator(Operation::Multiply), self.scale())
    }

    /// `self × numerator / denominator` at this value's scale, rounded once by `mode`, or `None`
    /// for a zero denominator, past the range, or for a numerator and denominator at two
    /// run-time scales.
    #[inline]
    #[must_use]
    pub const fn checked_mul_div_round<T, R>(
        self, numerator: Decimal<I, T>, denominator: Decimal<I, T>, mode: R,
    ) -> Option<Self>
    where
        I: [const] Int,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        let divisor = denominator.to_bits();
        if divisor == I::ZERO || numerator.scale() != denominator.scale() {
            return None;
        }
        match self.to_bits().mul_div(numerator.to_bits(), divisor, mode.table()).checked() {
            Some(steps) => Some(Self::from_bits(steps, self.scale())),
            None => None,
        }
    }

    /// A ratio's divisor in steps, for a numerator and denominator of one scale; panics for a zero
    /// denominator or two run-time scales.
    #[inline]
    #[track_caller]
    const fn ratio_divisor<T>(numerator: Decimal<I, T>, denominator: Decimal<I, T>) -> I
    where
        I: [const] Int,
        T: [const] Scale,
    {
        assert!(
            numerator.scale() == denominator.scale(),
            "a ratio's numerator and denominator at two scales"
        );
        let divisor = denominator.to_bits();
        if divisor == I::ZERO {
            divided_by_zero();
        }
        divisor
    }

    /// The multiple of `step` the value rounds to by `mode`: a price to its tick, a size to its
    /// lot. `step`'s sign is ignored, and a zero step leaves the value as it is; past the range it
    /// panics with overflow checks on, and wraps otherwise.
    ///
    /// # Panics
    /// For a step of a scale that never mixes with this one's; a run-time step at another scale
    /// lines up, as `+` does.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::{Ceil, Floor};
    /// use ddust::{D64, dec};
    ///
    /// let (price, tick): (D64<2>, D64<2>) = (dec!(60000.37), dec!(0.25));
    /// assert_eq!(price.round_to(tick, Floor), dec!(60000.25), "a bid, down to its tick");
    /// assert_eq!(price.round_to(tick, Ceil), dec!(60000.5), "an ask, up to its tick");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "lined up as `+` lines up: overflow checks decide"
    )]
    pub const fn round_to<R>(self, step: Self, mode: R) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        // Each at the finer of the two scales: its own for one scale, lined up for two at run time.
        let value = self + Self::from_bits(I::ZERO, step.scale());
        let step = step + Self::from_bits(I::ZERO, self.scale());
        if step.to_bits() == I::ZERO {
            return value;
        }
        let steps = value.to_bits().multiple(step.to_bits(), mode.table());
        Self::from_bits(steps.operator(Operation::Multiply), value.scale())
    }

    /// The multiple of `step` the value rounds to by `mode`, or `None` past the range or for a step
    /// of a scale that never mixes with this one's.
    #[inline]
    #[must_use]
    pub const fn checked_round_to<R>(self, step: Self, mode: R) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        let (Some(value), Some(step)) = (
            self.checked_add(Self::from_bits(I::ZERO, step.scale())),
            step.checked_add(Self::from_bits(I::ZERO, self.scale())),
        ) else {
            return None;
        };
        if step.to_bits() == I::ZERO {
            return Some(value);
        }
        match value.to_bits().multiple(step.to_bits(), mode.table()).checked() {
            Some(steps) => Some(Self::from_bits(steps, value.scale())),
            None => None,
        }
    }

    /// The value rounded to a whole number by `mode`, at its own scale.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, dec};
    ///
    /// let x: D64<1> = dec!(2.5);
    /// assert_eq!(x.round(HalfEven), dec!(2), "a tie, to the even whole");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn round<R>(self, mode: R) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        let decimals = self.decimals();
        let (whole, _) = self.to_bits().scale_down(decimals, mode.table());
        Self::from_bits(whole.scale_up(decimals).operator(Operation::Add), self.scale())
    }

    /// The value rounded to a whole number by `mode`, or `None` past the range.
    #[inline]
    #[must_use]
    pub const fn checked_round<R>(self, mode: R) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        let decimals = self.decimals();
        let (whole, _) = self.to_bits().scale_down(decimals, mode.table());
        match whole.scale_up(decimals).checked() {
            Some(steps) => Some(Self::from_bits(steps, self.scale())),
            None => None,
        }
    }

    /// The whole part, toward zero; never past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let x: D64<2> = dec!(-2.75);
    /// assert_eq!((x.trunc(), x.fract()), (dec!(-2), dec!(-0.75)), "the parts of -2.75");
    /// ```
    #[inline]
    #[must_use]
    pub const fn trunc(self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        // A truncated whole part is never larger than the value, so it fits.
        let decimals = self.decimals();
        let (whole, _) = self.to_bits().scale_down(decimals, Trunc.table());
        Self::from_bits(whole.scale_up(decimals).wrapping(), self.scale())
    }

    /// The fraction: the value less its whole part, of the value's sign.
    #[inline]
    #[must_use]
    pub const fn fract(self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        Self::from_bits(self.to_bits().wrapping_sub(self.trunc().to_bits()), self.scale())
    }

    /// The largest whole number at most the value; past the range it panics with overflow checks
    /// on, and wraps otherwise.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn floor(self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        self.round(Floor)
    }

    /// The smallest whole number at least the value; past the range it panics with overflow
    /// checks on, and wraps otherwise.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn ceil(self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        self.round(Ceil)
    }
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference model's own arithmetic")]
mod tests {
    use proptest::prelude::*;

    use crate::round::{Ceil, Expand, Floor, HalfEven, HalfExpand, Rounding, Trunc};
    use crate::{D8, D64, D128, Decimal, Dynamic, Fixed};

    /// Hundredths.
    type Cents = D64<2>;

    /// `bits` hundredths.
    const fn cents(bits: i64) -> Cents {
        Cents::from_bits(bits, Fixed)
    }

    proptest! {
        #[test]
        fn a_rounded_step_is_bracketed_by_floor_and_ceil(bits in -(1_i64 << 60)..(1_i64 << 60), step in 1_i64..1_000_000) {
            let (x, tick) = (cents(bits), cents(step));
            let (floor, ceil) = (x.round_to(tick, Floor), x.round_to(tick, Ceil));
            prop_assert!(floor <= x && x <= ceil, "brackets");
            prop_assert!(ceil.to_bits() - floor.to_bits() == 0 || ceil.to_bits() - floor.to_bits() == step);
            prop_assert_eq!(x.checked_round_to(tick, HalfEven), Some(x.round_to(tick, HalfEven)));
            prop_assert_eq!(x.round_to(cents(-step), Floor), floor, "a step's sign is ignored");
        }

        #[test]
        fn a_product_at_a_scale_is_the_exact_one_rounded(a: i64, b: i64) {
            let exact = i128::from(a) * i128::from(b);
            let truncated = cents(a).checked_mul_round_to(D64::<4>::from_bits(b, Fixed), Fixed::<3>, Trunc);
            prop_assert_eq!(truncated.map(|x| i128::from(x.to_bits())), i64::try_from(exact / 1_000).ok().map(i128::from));
        }

        #[test]
        fn a_quotient_at_a_scale_is_the_exact_one_rounded(a in -(1_i64 << 50)..(1_i64 << 50), b in 1_i64..) {
            let quotient = cents(a).checked_div_round_to(cents(b), Fixed::<6>, Floor).map(|x| i128::from(x.to_bits()));
            let numerator = i128::from(a) * 1_000_000;
            prop_assert_eq!(quotient, Some(numerator.div_euclid(i128::from(b))), "floor of a · 10^6 / b");
        }

        #[test]
        fn a_whole_number_brackets_the_value(bits in -(1_i64 << 60)..(1_i64 << 60)) {
            let x = cents(bits);
            prop_assert!(x.floor() <= x && x <= x.ceil(), "brackets");
            prop_assert_eq!(x.trunc() + x.fract(), x, "the parts sum to the value");
            prop_assert_eq!(x.floor().to_bits() % 100, 0, "a whole number");
        }
    }

    #[test]
    fn plain_decimals_multiply_and_divide_at_a_named_scale() {
        let amount = cents(10_010);
        let rate = D64::<4>::from_bits(25, Fixed);
        assert_eq!(
            amount.checked_mul_round_to(rate, Fixed::<2>, Ceil),
            Some(cents(26)),
            "up to the cent"
        );
        assert_eq!(amount.mul_round_to(rate, Fixed::<6>, Trunc).to_bits(), 250_250, "exact at six");
        let third =
            amount.checked_div_round_to(D64::<0>::from_bits(3, Fixed), Fixed::<2>, HalfEven);
        assert_eq!(third, Some(cents(3_337)), "a third, to the cent");
        assert_eq!(
            amount.checked_div_round_to(cents(0), Fixed::<2>, HalfEven),
            None,
            "never by zero"
        );
    }

    #[test]
    fn a_128_bit_product_and_quotient_cover_the_range() {
        let fourteen = D128::<18>::from_bits(14 * 10_i128.pow(18), Fixed);
        assert_eq!(fourteen.mul_round(fourteen, Trunc).to_bits(), 196 * 10_i128.pow(18), "14 × 14");
        let big = D128::<18>::from_bits(171 * 10_i128.pow(18), Fixed);
        let two = D128::<18>::from_bits(2 * 10_i128.pow(18), Fixed);
        assert_eq!(big.div_round(two, Trunc).to_bits(), 855 * 10_i128.pow(17), "171 / 2");
    }

    #[test]
    fn a_count_divides_with_a_mode() {
        assert_eq!(cents(100).div_int_round(3, Expand), cents(34), "away from zero");
        assert_eq!(cents(-100).div_int_round(3, Floor), cents(-34), "floor");
        assert_eq!(cents(100).checked_div_int_round(0, Floor), None, "never by zero");
        assert_eq!(D8::<0>::MIN.checked_div_int_round(-1, Floor), None, "past the range");
    }

    #[test]
    fn a_mode_at_run_time_rounds_as_its_type() {
        let x = cents(250);
        for (mode, value) in [(Rounding::HalfEven, cents(200)), (Rounding::HalfExpand, cents(300))]
        {
            assert_eq!(x.round(mode), value, "{mode:?}");
        }
        assert_eq!(x.round(HalfExpand), x.round(Rounding::HalfExpand), "the same as the type");
    }

    #[test]
    fn a_run_time_step_lines_up() {
        let price = Decimal::<i64, Dynamic>::from_bits(1_234, Dynamic::new(3).expect("at most 38"));
        let tick = Decimal::<i64, Dynamic>::from_bits(5, Dynamic::new(1).expect("at most 38"));
        let rounded = price.round_to(tick, Floor);
        assert_eq!((rounded.to_bits(), rounded.decimals()), (1_000, 3), "1.234 down to 0.5: 1.000");
    }
}
