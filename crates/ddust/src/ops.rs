//! The operators, and each one's four families as the integer has them: checked, saturating,
//! wrapping and overflowing.

use core::iter::Sum;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use crate::decimal::Decimal;
use crate::int::{Int, Operation, Outcome, Signed};
use crate::round::{RoundingMode, Trunc};
use crate::scale::{Scale, Times};

/// Panics for two values whose scales never mix.
#[cold]
#[inline(never)]
#[track_caller]
#[expect(clippy::panic, reason = "two scales that never mix: a mistake in every build")]
const fn unmixed() -> ! {
    panic!("an operation on values of two scales that never mix")
}

/// How two values meet.
#[derive(Clone, Copy)]
enum Meet<S> {
    /// At one scale: each value's steps as they are.
    Shared,
    /// At two scales that line up: the coarser value lifted by `k` decimals to `scale`, the
    /// finer one's.
    Lined {
        /// Whether the left value is the coarser.
        left: bool,
        /// How many decimals the coarser is lifted.
        k: u8,
        /// The finer scale, where they meet.
        scale: S,
    },
    /// At two scales that never mix.
    Unmixed,
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// How this value and `rhs` meet.
    #[inline]
    const fn meet(self, rhs: Self) -> Meet<S>
    where
        S: [const] Scale,
    {
        let (left, right) = (self.scale(), rhs.scale());
        if left == right {
            return Meet::Shared;
        }
        if !S::LINES_UP {
            return Meet::Unmixed;
        }
        let (a, b) = (left.decimals(), right.decimals());
        if a < b {
            Meet::Lined { left: true, k: b.wrapping_sub(a), scale: right }
        } else {
            Meet::Lined { left: false, k: a.wrapping_sub(b), scale: left }
        }
    }

    /// The exact sum, or difference when `subtract`, of two values lined up at the finer scale.
    #[inline(never)]
    const fn lined_sum(self, rhs: Self, left: bool, k: u8, subtract: bool) -> Outcome<I>
    where
        I: [const] Int,
    {
        let (a, b) = (self.steps(), rhs.steps());
        if left {
            a.lined_up_add(false, k, b, subtract)
        } else {
            b.lined_up_add(subtract, k, a, false)
        }
    }

    /// The sum, or difference when `subtract`, as an outcome at the scale both meet at; `None`
    /// for two scales that never mix.
    #[inline]
    const fn sum_outcome(self, rhs: Self, subtract: bool) -> Option<(Outcome<I>, S)>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.meet(rhs) {
            Meet::Shared => {
                let (a, b) = (self.steps(), rhs.steps());
                let (steps, past) =
                    if subtract { a.overflowing_sub(b) } else { a.overflowing_add(b) };
                let negative = if subtract { a < b } else { a.is_negative() };
                Some((Outcome::from_parts(steps, past, negative), self.scale()))
            },
            Meet::Lined { left, k, scale } => Some((self.lined_sum(rhs, left, k, subtract), scale)),
            Meet::Unmixed => None,
        }
    }

    /// The sum, or `None` past the range or for two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<1>, D8<1>) = (dec!(10.5), dec!(1.5));
    /// assert_eq!(a.checked_add(b), Some(dec!(12)), "in range");
    /// assert_eq!(D8::<1>::MAX.checked_add(b), None, "past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, false) {
            Some((outcome, scale)) => match outcome.checked() {
                Some(steps) => Some(Self::from_steps(steps, scale)),
                None => None,
            },
            None => None,
        }
    }

    /// The difference, or `None` past the range or for two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{UD64, dec};
    ///
    /// let (a, b): (UD64<2>, UD64<2>) = (dec!(1.5), dec!(2.25));
    /// assert_eq!(b.checked_sub(a), Some(dec!(0.75)), "in range");
    /// assert_eq!(a.checked_sub(b), None, "below an unsigned zero");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, true) {
            Some((outcome, scale)) => match outcome.checked() {
                Some(steps) => Some(Self::from_steps(steps, scale)),
                None => None,
            },
            None => None,
        }
    }

    /// The sum, held at the end of the range it passes.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(12.5);
    /// assert_eq!(a.saturating_add(dec!(1.5)), D8::<1>::MAX, "held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn saturating_add(self, rhs: Self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, false) {
            Some((outcome, scale)) => Self::from_steps(outcome.saturating(), scale),
            None => unmixed(),
        }
    }

    /// The difference, held at the end of the range it passes.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{UD64, dec};
    ///
    /// let a: UD64<2> = dec!(1.5);
    /// assert_eq!(a.saturating_sub(dec!(2.25)), UD64::<2>::ZERO, "held at zero");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn saturating_sub(self, rhs: Self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, true) {
            Some((outcome, scale)) => Self::from_steps(outcome.saturating(), scale),
            None => unmixed(),
        }
    }

    /// The sum, wrapped around the range.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(12.7);
    /// assert_eq!(a.wrapping_add(dec!(0.1)), D8::<1>::MIN, "127 steps and one wrap to -128");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn wrapping_add(self, rhs: Self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, false) {
            Some((outcome, scale)) => Self::from_steps(outcome.wrapping(), scale),
            None => unmixed(),
        }
    }

    /// The difference, wrapped around the range.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(-12.8);
    /// assert_eq!(a.wrapping_sub(dec!(0.1)), D8::<1>::MAX, "-128 steps less one wrap to 127");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn wrapping_sub(self, rhs: Self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, true) {
            Some((outcome, scale)) => Self::from_steps(outcome.wrapping(), scale),
            None => unmixed(),
        }
    }

    /// The sum wrapped around the range, and whether it wrapped.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(12.7);
    /// assert_eq!(a.overflowing_add(dec!(0.1)), (D8::<1>::MIN, true), "wrapped");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn overflowing_add(self, rhs: Self) -> (Self, bool)
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, false) {
            Some((outcome, scale)) => {
                let (steps, wrapped) = outcome.overflowing();
                (Self::from_steps(steps, scale), wrapped)
            },
            None => unmixed(),
        }
    }

    /// The difference wrapped around the range, and whether it wrapped.
    ///
    /// # Panics
    /// For two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(-12.8);
    /// assert_eq!(a.overflowing_sub(dec!(0.1)), (D8::<1>::MAX, true), "wrapped");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool)
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.sum_outcome(rhs, true) {
            Some((outcome, scale)) => {
                let (steps, wrapped) = outcome.overflowing();
                (Self::from_steps(steps, scale), wrapped)
            },
            None => unmixed(),
        }
    }

    /// The value `n` times over, or `None` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(4.2);
    /// assert_eq!(a.checked_mul_int(3), Some(dec!(12.6)), "in range");
    /// assert_eq!(a.checked_mul_int(4), None, "past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_mul_int(self, n: I) -> Option<Self>
    where
        I: [const] Int,
    {
        match self.steps().checked_mul(n) {
            Some(steps) => Some(Self::from_steps(steps, self.scale())),
            None => None,
        }
    }

    /// The value `n` times over, held at the end of the range it passes.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(4.2);
    /// assert_eq!(a.saturating_mul_int(4), D8::<1>::MAX, "held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_mul_int(self, n: I) -> Self
    where
        I: [const] Int,
    {
        Self::from_steps(self.steps().saturating_mul(n), self.scale())
    }

    /// The value `n` times over, wrapped around the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(6.4);
    /// assert_eq!(a.wrapping_mul_int(2), D8::<1>::MIN, "128 steps wrap to -128");
    /// ```
    #[inline]
    #[must_use]
    pub const fn wrapping_mul_int(self, n: I) -> Self
    where
        I: [const] Int,
    {
        Self::from_steps(self.steps().wrapping_mul(n), self.scale())
    }

    /// The value `n` times over, wrapped around the range, and whether it wrapped.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let a: D8<1> = dec!(6.4);
    /// assert_eq!(a.overflowing_mul_int(2), (D8::<1>::MIN, true), "wrapped");
    /// ```
    #[inline]
    #[must_use]
    pub const fn overflowing_mul_int(self, n: I) -> (Self, bool)
    where
        I: [const] Int,
    {
        let (steps, wrapped) = self.steps().overflowing_mul(n);
        (Self::from_steps(steps, self.scale()), wrapped)
    }

    /// One of `n` equal parts, truncated toward zero, or `None` for zero parts and for the
    /// minimum in -1 parts.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let a: D64<2> = dec!(10);
    /// assert_eq!(a.checked_div_int(3), Some(dec!(3.33)), "truncated");
    /// assert_eq!(a.checked_div_int(0), None, "never by zero");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_div_int(self, n: I) -> Option<Self>
    where
        I: [const] Int,
    {
        match self.steps().checked_div(n) {
            Some(steps) => Some(Self::from_steps(steps, self.scale())),
            None => None,
        }
    }

    /// One of `n` equal parts, truncated toward zero; the minimum in -1 parts is held at the
    /// maximum.
    ///
    /// # Panics
    /// For zero parts, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.saturating_div_int(-1), D8::<1>::MAX, "held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn saturating_div_int(self, n: I) -> Self
    where
        I: [const] Int,
    {
        Self::from_steps(self.steps().saturating_div(n), self.scale())
    }

    /// One of `n` equal parts, truncated toward zero; the minimum in -1 parts wraps to itself.
    ///
    /// # Panics
    /// For zero parts, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.wrapping_div_int(-1), D8::<1>::MIN, "wrapped");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn wrapping_div_int(self, n: I) -> Self
    where
        I: [const] Int,
    {
        Self::from_steps(self.steps().wrapping_div(n), self.scale())
    }

    /// One of `n` equal parts, truncated toward zero, and whether it wrapped.
    ///
    /// # Panics
    /// For zero parts, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.overflowing_div_int(-1), (D8::<1>::MIN, true), "wrapped");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn overflowing_div_int(self, n: I) -> (Self, bool)
    where
        I: [const] Int,
    {
        let (steps, wrapped) = self.steps().overflowing_div(n);
        (Self::from_steps(steps, self.scale()), wrapped)
    }

    /// The quotient by `rhs` at this value's scale, truncated toward zero as `/` is, or `None` for
    /// a zero divisor or past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let (total, quantity): (D64<2>, D64<4>) = (dec!(100), dec!(3.0001));
    /// assert_eq!(total.checked_div(quantity), Some(dec!(33.33)), "truncated, at two decimals");
    /// assert_eq!(total.checked_div(D64::<4>::ZERO), None, "never by zero");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_div<T>(self, rhs: Decimal<I, T>) -> Option<Self>
    where
        I: [const] Int,
        T: [const] Scale,
    {
        match self.quotient(rhs) {
            Some(outcome) => match outcome.checked() {
                Some(steps) => Some(Self::from_steps(steps, self.scale())),
                None => None,
            },
            None => None,
        }
    }

    /// The quotient by `rhs` at this value's scale, truncated toward zero, held at the end of
    /// the range it passes.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<1>, D8<1>) = (dec!(10), dec!(0.5));
    /// assert_eq!(a.saturating_div(b), D8::<1>::MAX, "20 is held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn saturating_div<T>(self, rhs: Decimal<I, T>) -> Self
    where
        I: [const] Int,
        T: [const] Scale,
    {
        Self::from_steps(self.nonzero_quotient(rhs).saturating(), self.scale())
    }

    /// The quotient by `rhs` at this value's scale, truncated toward zero, wrapped around the
    /// range.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<1>, D8<1>) = (dec!(6.4), dec!(0.5));
    /// assert_eq!(a.wrapping_div(b), D8::<1>::MIN, "12.8: 128 steps wrap to -128");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn wrapping_div<T>(self, rhs: Decimal<I, T>) -> Self
    where
        I: [const] Int,
        T: [const] Scale,
    {
        Self::from_steps(self.nonzero_quotient(rhs).wrapping(), self.scale())
    }

    /// The quotient by `rhs` at this value's scale, truncated toward zero, wrapped around the
    /// range, and whether it wrapped.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's does.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<1>, D8<1>) = (dec!(6.4), dec!(0.5));
    /// assert_eq!(a.overflowing_div(b), (D8::<1>::MIN, true), "wrapped");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn overflowing_div<T>(self, rhs: Decimal<I, T>) -> (Self, bool)
    where
        I: [const] Int,
        T: [const] Scale,
    {
        let (steps, wrapped) = self.nonzero_quotient(rhs).overflowing();
        (Self::from_steps(steps, self.scale()), wrapped)
    }

    /// The truncated quotient by `rhs` at this value's scale, or `None` for a zero divisor:
    /// `self × 10^decimals(rhs) / rhs`, in steps.
    #[inline]
    const fn quotient<T>(self, rhs: Decimal<I, T>) -> Option<Outcome<I>>
    where
        I: [const] Int,
        T: [const] Scale,
    {
        let divisor = rhs.steps();
        if divisor == I::ZERO {
            return None;
        }
        Some(self.steps().div_up(rhs.decimals(), divisor, Trunc.table()))
    }

    /// The truncated quotient by `rhs` at this value's scale; panics for a zero divisor.
    #[inline]
    #[track_caller]
    const fn nonzero_quotient<T>(self, rhs: Decimal<I, T>) -> Outcome<I>
    where
        I: [const] Int,
        T: [const] Scale,
    {
        match self.quotient(rhs) {
            Some(outcome) => outcome,
            None => divided_by_zero(),
        }
    }

    /// The remainder by `rhs`, or `None` for a zero divisor, for the minimum by -1, or for two
    /// scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let (a, b): (D64<1>, D64<1>) = (dec!(7.5), dec!(2));
    /// assert_eq!(a.checked_rem(b), Some(dec!(1.5)), "7.5 is 3 × 2 and 1.5");
    /// assert_eq!(a.checked_rem(D64::<1>::ZERO), None, "never by zero");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_rem(self, rhs: Self) -> Option<Self>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.meet(rhs) {
            Meet::Shared => match self.steps().checked_rem(rhs.steps()) {
                Some(steps) => Some(Self::from_steps(steps, self.scale())),
                None => None,
            },
            Meet::Lined { left, k, scale } => {
                if rhs.steps() == I::ZERO {
                    return None;
                }
                Some(Self::from_steps(self.lined_rem(rhs, left, k), scale))
            },
            Meet::Unmixed => None,
        }
    }

    /// The remainder by `rhs`; the minimum by -1 wraps to zero.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's does, and for two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// let minus_one = D8::<0>::from_steps(-1, ddust::Fixed);
    /// assert_eq!(D8::<0>::MIN.wrapping_rem(minus_one), D8::<0>::ZERO, "no remainder");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn wrapping_rem(self, rhs: Self) -> Self
    where
        I: [const] Int,
        S: [const] Scale,
    {
        self.overflowing_rem(rhs).0
    }

    /// The remainder by `rhs`, and whether its quotient overflowed: only the minimum by -1, whose
    /// remainder is zero.
    ///
    /// # Panics
    /// For a zero divisor, as the integer's does, and for two scales that never mix.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// let minus_one = D8::<0>::from_steps(-1, ddust::Fixed);
    /// assert_eq!(D8::<0>::MIN.overflowing_rem(minus_one), (D8::<0>::ZERO, true), "flagged");
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn overflowing_rem(self, rhs: Self) -> (Self, bool)
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match self.meet(rhs) {
            Meet::Shared => {
                let (steps, wrapped) = self.steps().overflowing_rem(rhs.steps());
                (Self::from_steps(steps, self.scale()), wrapped)
            },
            Meet::Lined { left, k, scale } => {
                if rhs.steps() == I::ZERO {
                    remainder_by_zero();
                }
                (Self::from_steps(self.lined_rem(rhs, left, k), scale), false)
            },
            Meet::Unmixed => unmixed(),
        }
    }

    /// The remainder of two values lined up at the finer scale, by an `rhs` that is not zero.
    #[inline(never)]
    const fn lined_rem(self, rhs: Self, left: bool, k: u8) -> I
    where
        I: [const] Int,
    {
        let (a, b) = (self.steps(), rhs.steps());
        if left { a.lined_up_rem(k, b, 0) } else { a.lined_up_rem(0, b, k) }
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

/// Panics as the integer's `%` does for a zero divisor.
#[cold]
#[inline(never)]
#[track_caller]
#[expect(clippy::panic, reason = "the integer's own remainder by zero, with its own message")]
const fn remainder_by_zero() -> ! {
    panic!("attempt to calculate the remainder with a divisor of zero")
}

/// The exact product of two decimals: the steps multiplied as the integer's `*` does, the scales
/// summed. A product at the sum of two static scales is a [`Sum`](crate::scale::Sum) scale, which
/// converts into the `Fixed` of those decimals for free.
impl<I: Int, S: Scale> Decimal<I, S> {
    /// The exact product, or `None` past the range or for a run-time product past 38 decimals.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D32, D64, dec};
    ///
    /// let (unit_price, quantity): (D64<2>, D64<3>) = (dec!(19.99), dec!(2.375));
    /// let total: Option<D64<5>> = unit_price.checked_mul(quantity).map(Into::into);
    /// assert_eq!(total, Some(dec!(47.47625)), "exact, at five decimals");
    /// let (big, many): (D32<2>, D32<2>) = (dec!(1000), dec!(30000));
    /// assert_eq!(big.checked_mul(many), None, "3 · 10^11 steps are past an i32");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_mul<T: Scale>(
        self, rhs: Decimal<I, T>,
    ) -> Option<Decimal<I, <S as Times<T>>::Output>>
    where
        I: [const] Int,
        S: [const] Times<T>,
    {
        match (self.steps().checked_mul(rhs.steps()), self.scale().checked_times(rhs.scale())) {
            (Some(steps), Some(scale)) => Some(Decimal::from_steps(steps, scale)),
            _ => None,
        }
    }

    /// The exact product, held at the end of the range it passes.
    ///
    /// # Panics
    /// For a run-time product past 38 decimals, a scale no integer holds a step of.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D32, Fixed, dec};
    ///
    /// let (big, many): (D32<2>, D32<2>) = (dec!(1000), dec!(30000));
    /// let held: D32<4> = big.saturating_mul(many).into();
    /// assert_eq!(held, D32::<4>::MAX, "held at the largest");
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_mul<T: Scale>(
        self, rhs: Decimal<I, T>,
    ) -> Decimal<I, <S as Times<T>>::Output>
    where
        I: [const] Int,
        S: [const] Times<T>,
    {
        Decimal::from_steps(
            self.steps().saturating_mul(rhs.steps()),
            self.scale().times(rhs.scale()),
        )
    }

    /// The exact product, wrapped around the range.
    ///
    /// # Panics
    /// For a run-time product past 38 decimals, a scale no integer holds a step of.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<0>, D8<0>) = (dec!(16), dec!(8));
    /// let wrapped: D8<0> = a.wrapping_mul(b).into();
    /// assert_eq!(wrapped, D8::<0>::MIN, "128 wraps to -128");
    /// ```
    #[inline]
    #[must_use]
    pub const fn wrapping_mul<T: Scale>(
        self, rhs: Decimal<I, T>,
    ) -> Decimal<I, <S as Times<T>>::Output>
    where
        I: [const] Int,
        S: [const] Times<T>,
    {
        Decimal::from_steps(self.steps().wrapping_mul(rhs.steps()), self.scale().times(rhs.scale()))
    }

    /// The exact product, wrapped around the range, and whether it wrapped.
    ///
    /// # Panics
    /// For a run-time product past 38 decimals, a scale no integer holds a step of.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let (a, b): (D8<0>, D8<0>) = (dec!(16), dec!(8));
    /// assert!(a.overflowing_mul(b).1, "128 is past an i8");
    /// ```
    #[inline]
    #[must_use]
    pub const fn overflowing_mul<T: Scale>(
        self, rhs: Decimal<I, T>,
    ) -> (Decimal<I, <S as Times<T>>::Output>, bool)
    where
        I: [const] Int,
        S: [const] Times<T>,
    {
        let (steps, wrapped) = self.steps().overflowing_mul(rhs.steps());
        (Decimal::from_steps(steps, self.scale().times(rhs.scale())), wrapped)
    }
}

impl<I: Signed, S: Scale> Decimal<I, S> {
    /// Whether the value is below zero.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let (loss, gain): (D64<2>, D64<2>) = (dec!(-12.5), dec!(12.5));
    /// assert!(loss.is_negative() && !gain.is_negative());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_negative(self) -> bool
    where
        I: [const] Signed,
    {
        self.steps() < I::ZERO
    }

    /// `-1`, `0` or `1`, as the value's sign.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let loss: D64<2> = dec!(-12.5);
    /// assert_eq!((loss.signum(), D64::<2>::ZERO.signum()), (-1, 0));
    /// ```
    #[inline]
    #[must_use]
    pub const fn signum(self) -> I
    where
        I: [const] Signed,
    {
        self.steps().signum()
    }

    /// The magnitude; for the minimum it panics with overflow checks on, and wraps otherwise.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let loss: D64<2> = dec!(-12.5);
    /// assert_eq!(loss.abs(), dec!(12.5));
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn abs(self) -> Self
    where
        I: [const] Signed,
    {
        Self::from_steps(self.steps().abs(), self.scale())
    }

    /// The magnitude as an unsigned decimal, exact for every value.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.unsigned_abs().steps(), 128, "-12.8's magnitude, in a u8");
    /// ```
    #[inline]
    #[must_use]
    pub const fn unsigned_abs(self) -> Decimal<I::Unsigned, S>
    where
        I: [const] Signed,
    {
        Decimal::from_steps(self.steps().unsigned_abs(), self.scale())
    }

    /// The negation, or `None` for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let debt: D8<1> = dec!(-2.5);
    /// assert_eq!(debt.checked_neg(), Some(dec!(2.5)), "repaid");
    /// assert_eq!(D8::<1>::MIN.checked_neg(), None, "12.8 is past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_neg(self) -> Option<Self>
    where
        I: [const] Signed,
    {
        match self.steps().checked_neg() {
            Some(steps) => Some(Self::from_steps(steps, self.scale())),
            None => None,
        }
    }

    /// The magnitude, or `None` for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, dec};
    ///
    /// let debt: D8<1> = dec!(-2.5);
    /// assert_eq!(debt.checked_abs(), Some(dec!(2.5)));
    /// assert_eq!(D8::<1>::MIN.checked_abs(), None, "12.8 is past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_abs(self) -> Option<Self>
    where
        I: [const] Signed,
    {
        match self.steps().checked_abs() {
            Some(steps) => Some(Self::from_steps(steps, self.scale())),
            None => None,
        }
    }

    /// The negation, the maximum for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.saturating_neg(), D8::<1>::MAX, "held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_neg(self) -> Self
    where
        I: [const] Signed,
    {
        Self::from_steps(self.steps().saturating_neg(), self.scale())
    }

    /// The magnitude, the maximum for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.saturating_abs(), D8::<1>::MAX, "held at 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_abs(self) -> Self
    where
        I: [const] Signed,
    {
        Self::from_steps(self.steps().saturating_abs(), self.scale())
    }

    /// The negation, the minimum for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.wrapping_neg(), D8::<1>::MIN, "wrapped to itself");
    /// ```
    #[inline]
    #[must_use]
    pub const fn wrapping_neg(self) -> Self
    where
        I: [const] Signed,
    {
        Self::from_steps(self.steps().wrapping_neg(), self.scale())
    }

    /// The magnitude, the minimum for the minimum.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.wrapping_abs(), D8::<1>::MIN, "wrapped to itself");
    /// ```
    #[inline]
    #[must_use]
    pub const fn wrapping_abs(self) -> Self
    where
        I: [const] Signed,
    {
        Self::from_steps(self.steps().wrapping_abs(), self.scale())
    }

    /// The negation, and whether it wrapped.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.overflowing_neg(), (D8::<1>::MIN, true), "12.8 is past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn overflowing_neg(self) -> (Self, bool)
    where
        I: [const] Signed,
    {
        let (steps, wrapped) = self.steps().overflowing_neg();
        (Self::from_steps(steps, self.scale()), wrapped)
    }

    /// The magnitude, and whether it wrapped.
    ///
    /// # Examples
    /// ```
    /// use ddust::D8;
    ///
    /// assert_eq!(D8::<1>::MIN.overflowing_abs(), (D8::<1>::MIN, true), "12.8 is past 12.7");
    /// ```
    #[inline]
    #[must_use]
    pub const fn overflowing_abs(self) -> (Self, bool)
    where
        I: [const] Signed,
    {
        let (steps, wrapped) = self.steps().overflowing_abs();
        (Self::from_steps(steps, self.scale()), wrapped)
    }
}

/// Values of one scale add as their steps do; two run-time scales line up first. Overflow panics
/// with overflow checks on, and wraps otherwise; two scales that never mix panic.
const impl<I: [const] Int, S: [const] Scale> Add for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: overflow checks decide")]
    fn add(self, rhs: Self) -> Self {
        match self.meet(rhs) {
            Meet::Shared => Self::from_steps(self.steps() + rhs.steps(), self.scale()),
            Meet::Lined { left, k, scale } => Self::from_steps(
                self.lined_sum(rhs, left, k, false).operator(Operation::Add),
                scale,
            ),
            Meet::Unmixed => unmixed(),
        }
    }
}

/// Values of one scale subtract as their steps do; two run-time scales line up first. Overflow
/// panics with overflow checks on, and wraps otherwise; two scales that never mix panic.
const impl<I: [const] Int, S: [const] Scale> Sub for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: overflow checks decide")]
    fn sub(self, rhs: Self) -> Self {
        match self.meet(rhs) {
            Meet::Shared => Self::from_steps(self.steps() - rhs.steps(), self.scale()),
            Meet::Lined { left, k, scale } => Self::from_steps(
                self.lined_sum(rhs, left, k, true).operator(Operation::Subtract),
                scale,
            ),
            Meet::Unmixed => unmixed(),
        }
    }
}

/// The minimum's negation overflows: it panics with overflow checks on, and wraps otherwise.
const impl<I: [const] Signed, S: [const] Scale> Neg for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: overflow checks decide")]
    fn neg(self) -> Self {
        Self::from_steps(-self.steps(), self.scale())
    }
}

/// The value `n` times over; overflow panics with overflow checks on, and wraps otherwise.
const impl<I: [const] Int, S: [const] Scale> Mul<I> for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: overflow checks decide")]
    fn mul(self, n: I) -> Self {
        Self::from_steps(self.steps() * n, self.scale())
    }
}

/// One of `n` equal parts, truncated toward zero; panics for zero parts and for the minimum in -1
/// parts, as the integer's `/` does.
const impl<I: [const] Int, S: [const] Scale> Div<I> for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: zero panics")]
    fn div(self, n: I) -> Self {
        Self::from_steps(self.steps() / n, self.scale())
    }
}

/// The exact product: the steps multiplied as the integer's `*` does, the scales summed; overflow
/// panics with overflow checks on, and wraps otherwise. A run-time product past 38 decimals panics,
/// a scale no integer holds a step of.
const impl<I: [const] Int, S: [const] Times<T>, T: [const] Scale> Mul<Decimal<I, T>>
    for Decimal<I, S>
{
    type Output = Decimal<I, <S as Times<T>>::Output>;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: overflow checks decide")]
    fn mul(self, rhs: Decimal<I, T>) -> Self::Output {
        Decimal::from_steps(self.steps() * rhs.steps(), self.scale().times(rhs.scale()))
    }
}

/// The quotient at the dividend's scale, truncated toward zero as the integer's `/` is; panics for
/// a zero divisor, and past the range with overflow checks on, and wraps otherwise.
const impl<I: [const] Int, S: [const] Scale, T: [const] Scale> Div<Decimal<I, T>>
    for Decimal<I, S>
{
    type Output = Self;

    #[inline]
    #[track_caller]
    fn div(self, rhs: Decimal<I, T>) -> Self {
        Self::from_steps(self.nonzero_quotient(rhs).operator(Operation::Divide), self.scale())
    }
}

/// The remainder after a whole quotient, exact, as `f64`'s `%` is; two run-time scales line up
/// first. Panics for a zero divisor, and for the minimum by -1 as the integer's `%` does.
const impl<I: [const] Int, S: [const] Scale> Rem for Decimal<I, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    #[expect(clippy::arithmetic_side_effects, reason = "the integer's own: zero panics")]
    fn rem(self, rhs: Self) -> Self {
        match self.meet(rhs) {
            Meet::Shared => Self::from_steps(self.steps() % rhs.steps(), self.scale()),
            Meet::Lined { left, k, scale } => {
                if rhs.steps() == I::ZERO {
                    remainder_by_zero();
                }
                Self::from_steps(self.lined_rem(rhs, left, k), scale)
            },
            Meet::Unmixed => unmixed(),
        }
    }
}

/// As `+`.
const impl<I: [const] Int, S: [const] Scale> AddAssign for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// As `-`.
const impl<I: [const] Int, S: [const] Scale> SubAssign for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// As `*` by a count.
const impl<I: [const] Int, S: [const] Scale> MulAssign<I> for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn mul_assign(&mut self, n: I) {
        *self = *self * n;
    }
}

/// As `*`, for a product whose scale is this value's: a run-time one.
const impl<I: [const] Int, S: [const] Times<T, Output = S>, T: [const] Scale>
    MulAssign<Decimal<I, T>> for Decimal<I, S>
{
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn mul_assign(&mut self, rhs: Decimal<I, T>) {
        *self = *self * rhs;
    }
}

/// As `/` by a count.
const impl<I: [const] Int, S: [const] Scale> DivAssign<I> for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn div_assign(&mut self, n: I) {
        *self = *self / n;
    }
}

/// As `/`.
const impl<I: [const] Int, S: [const] Scale, T: [const] Scale> DivAssign<Decimal<I, T>>
    for Decimal<I, S>
{
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn div_assign(&mut self, rhs: Decimal<I, T>) {
        *self = *self / rhs;
    }
}

/// As `%`.
const impl<I: [const] Int, S: [const] Scale> RemAssign for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

/// From zero at the scale's default: a static scale's, or a run-time scale of no decimals, which
/// the first value lines up.
impl<I: Int, S: Scale + Default> Sum for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn sum<V: Iterator<Item = Self>>(values: V) -> Self {
        values.fold(Self::default(), |sum, value| sum + value)
    }
}

/// From zero at the scale's default, as the sum of values.
impl<'a, I: Int, S: Scale + Default> Sum<&'a Self> for Decimal<I, S> {
    #[inline]
    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the operator's own: overflow checks decide"
    )]
    fn sum<V: Iterator<Item = &'a Self>>(values: V) -> Self {
        values.fold(Self::default(), |sum, &value| sum + value)
    }
}

/// `n * value`, for each integer, as `value * n`.
macro_rules! scaled_by {
    ($($t:ty),*) => {$(
        /// As `value * n`.
        const impl<S: [const] Scale> Mul<Decimal<$t, S>> for $t {
            type Output = Decimal<$t, S>;

            #[inline]
            #[track_caller]
            #[expect(clippy::arithmetic_side_effects, reason = "the operator's own: overflow checks decide")]
            fn mul(self, value: Decimal<$t, S>) -> Decimal<$t, S> {
                value * self
            }
        }
    )*};
}

scaled_by!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the operators under test")]
mod tests {
    use proptest::prelude::*;

    use crate::scale::Sum;
    use crate::{D8, D64, Decimal, Dynamic, Fixed, UD8};

    /// Hundredths.
    type Cents = D64<2>;

    /// `steps` hundredths.
    const fn cents(steps: i64) -> Cents {
        Cents::from_steps(steps, Fixed)
    }

    /// A decimal at a run-time scale of `decimals`.
    fn dynamic(steps: i64, decimals: u8) -> Decimal<i64, Dynamic> {
        Decimal::from_steps(steps, Dynamic::new(decimals).expect("at most 38"))
    }

    proptest! {
        #[test]
        fn every_binary_method_is_the_integers(a: i64, b: i64) {
            let (x, y) = (cents(a), cents(b));
            prop_assert_eq!(x.checked_add(y).map(Cents::steps), a.checked_add(b));
            prop_assert_eq!(x.checked_sub(y).map(Cents::steps), a.checked_sub(b));
            prop_assert_eq!(x.checked_mul_int(b).map(Cents::steps), a.checked_mul(b));
            prop_assert_eq!(x.checked_div_int(b).map(Cents::steps), a.checked_div(b));
            prop_assert_eq!(x.checked_rem(y).map(Cents::steps), a.checked_rem(b));
            prop_assert_eq!(x.saturating_add(y).steps(), a.saturating_add(b));
            prop_assert_eq!(x.saturating_sub(y).steps(), a.saturating_sub(b));
            prop_assert_eq!(x.saturating_mul_int(b).steps(), a.saturating_mul(b));
            prop_assert_eq!(x.wrapping_add(y).steps(), a.wrapping_add(b));
            prop_assert_eq!(x.wrapping_sub(y).steps(), a.wrapping_sub(b));
            prop_assert_eq!(x.wrapping_mul_int(b).steps(), a.wrapping_mul(b));
            prop_assert_eq!(x.overflowing_add(y).1, a.overflowing_add(b).1);
            prop_assert_eq!(x.overflowing_sub(y).1, a.overflowing_sub(b).1);
            prop_assert_eq!(x.checked_mul(y).map(Decimal::steps), a.checked_mul(b), "the exact product's steps");
            if b != 0 {
                prop_assert_eq!(x.wrapping_rem(y).steps(), a.wrapping_rem(b));
                prop_assert_eq!(x.wrapping_div_int(b).steps(), a.wrapping_div(b));
                prop_assert_eq!(x.saturating_div_int(b).steps(), a.saturating_div(b));
            }
        }

        #[test]
        fn every_unary_method_is_the_integers(a: i64) {
            let x = cents(a);
            prop_assert_eq!(x.checked_neg().map(Cents::steps), a.checked_neg());
            prop_assert_eq!(x.checked_abs().map(Cents::steps), a.checked_abs());
            prop_assert_eq!(x.saturating_neg().steps(), a.saturating_neg());
            prop_assert_eq!(x.saturating_abs().steps(), a.saturating_abs());
            prop_assert_eq!(x.wrapping_neg().steps(), a.wrapping_neg());
            prop_assert_eq!(x.wrapping_abs().steps(), a.wrapping_abs());
            prop_assert_eq!(x.overflowing_neg().1, a.overflowing_neg().1);
            prop_assert_eq!(x.overflowing_abs().1, a.overflowing_abs().1);
            prop_assert_eq!(x.unsigned_abs().steps(), a.unsigned_abs());
            prop_assert_eq!((x.signum(), x.is_negative(), x.is_positive(), x.is_zero()), (a.signum(), a < 0, a > 0, a == 0));
        }

        #[test]
        fn a_quotient_is_the_truncated_exact_one(a in -(1_i64 << 40)..(1_i64 << 40), b in 1_i64..1_000_000) {
            // a / b at two decimals: a · 10^2 / b steps, as an i128 truncates it.
            let quotient = cents(a).checked_div(cents(b)).map(Cents::steps);
            prop_assert_eq!(quotient.map(i128::from), Some(i128::from(a) * 100 / i128::from(b)));
        }

        #[test]
        fn two_run_time_scales_add_as_their_lined_up_steps(a: i32, b: i32, k in 0_u8..9) {
            let (x, y) = (dynamic(a.into(), 0), dynamic(b.into(), k));
            let lifted = i64::from(a) * 10_i64.pow(u32::from(k));
            prop_assert_eq!((x + y).steps(), lifted + i64::from(b), "the sum");
            prop_assert_eq!((x - y).steps(), lifted - i64::from(b), "the difference");
            prop_assert_eq!((y - x).steps(), i64::from(b) - lifted, "the other difference");
            prop_assert_eq!((x + y).decimals(), k, "at the finer scale");
        }
    }

    #[test]
    fn the_operators_are_the_integers() {
        let (x, y) = (cents(700), cents(-300));
        assert_eq!((x + y, x - y, -x), (cents(400), cents(1_000), cents(-700)), "+, -, negation");
        assert_eq!(
            (x * 2, 2 * x, x / 2, -x / 2),
            (cents(1_400), cents(1_400), cents(350), cents(-350))
        );
        let mut z = x;
        z += y;
        z -= cents(100);
        z *= 3;
        z /= 2;
        z %= cents(400);
        assert_eq!(z, cents(50), "in place");
        assert_eq!([x, y, cents(100)].iter().sum::<Cents>(), cents(500), "a sum, from zero");
    }

    #[test]
    fn a_product_is_exact_and_its_scale_the_sum() {
        let (unit_price, quantity) = (cents(6_000_037), D64::<4>::from_steps(125, Fixed));
        let total: Decimal<i64, Sum<Fixed<2>, Fixed<4>>> = unit_price * quantity;
        let total: D64<6> = total.into();
        assert_eq!(total.steps(), 750_004_625, "one multiply, exact");
        let run_time = unit_price * dynamic(125, 4);
        assert_eq!((run_time.steps(), run_time.decimals()), (750_004_625, 6), "at run time too");
    }

    #[test]
    fn a_run_time_product_takes_its_scale_in_place() {
        let mut x = dynamic(15, 1);
        x *= dynamic(25, 2);
        assert_eq!((x.steps(), x.decimals()), (375, 3), "1.5 × 0.25 = 0.375");
    }

    #[test]
    fn two_run_time_scales_line_up_in_every_family() {
        let (tenths, hundredths) = (dynamic(15, 1), dynamic(125, 2));
        assert_eq!(tenths.checked_add(hundredths).map(Decimal::steps), Some(275), "1.5 + 1.25");
        assert_eq!((tenths % hundredths).steps(), 25, "1.5 % 1.25 = 0.25");
        let near = Decimal::<i8, Dynamic>::from_steps(13, Dynamic::new(0).expect("at most 38"));
        let back = Decimal::<i8, Dynamic>::from_steps(-100, Dynamic::new(1).expect("at most 38"));
        assert_eq!((near + back).steps(), 30, "13 lined up is 130, past an i8, yet 13 - 10 fits");
    }

    #[test]
    fn unsigned_values_never_go_below_zero() {
        let (a, b) = (UD8::<1>::from_steps(15, Fixed), UD8::<1>::from_steps(25, Fixed));
        assert_eq!(a.checked_sub(b), None, "1.5 - 2.5 is below zero");
        assert_eq!(a.saturating_sub(b).steps(), 0, "held at zero");
        assert_eq!(a.wrapping_sub(b).steps(), 246, "wrapped");
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn a_quotient_by_zero_panics() {
        let _quotient = cents(100) / cents(0);
    }

    #[test]
    #[cfg(overflow_checks)]
    #[should_panic(expected = "attempt to add with overflow")]
    fn a_lined_up_sum_past_the_range_panics_with_overflow_checks() {
        let big = Decimal::<i8, Dynamic>::from_steps(100, Dynamic::new(0).expect("at most 38"));
        let _sum = big + Decimal::from_steps(1, Dynamic::new(1).expect("at most 38"));
    }

    #[test]
    #[cfg(not(overflow_checks))]
    fn a_lined_up_sum_past_the_range_wraps_without_overflow_checks() {
        let big = Decimal::<i8, Dynamic>::from_steps(100, Dynamic::new(0).expect("at most 38"));
        let sum = big + Decimal::from_steps(1, Dynamic::new(1).expect("at most 38"));
        assert_eq!(
            sum.steps(),
            1_001_i16.to_le_bytes()[0].cast_signed(),
            "1001's low byte, as i8 wraps"
        );
    }

    #[test]
    fn eight_bits_divide_at_their_scale() {
        let (a, b) = (D8::<1>::from_steps(64, Fixed), D8::<1>::from_steps(5, Fixed));
        assert_eq!(a.checked_div(b), None, "12.8 is past 12.7");
        assert_eq!(a.wrapping_div(b).steps(), -128, "and wraps");
    }
}
