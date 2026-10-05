//! [`Decimal`]: a whole number of steps, and the scale that says what a step is.

use crate::errors::{ConvertError, ConvertErrorKind};
use crate::int::Int;
use crate::round::{RoundingMode, Trunc};
use crate::scale::{Fixed, Scale, StaticScale};

/// An exact decimal: a whole number of steps of `10^-decimals`, held in an integer `I` from `i8`
/// to `u128`, at a scale `S` that is a type the compiler knows or a value carried at run time.
///
/// | Decimal | Holds 12.34 as | Size |
/// | ------- | -------------- | ---- |
/// | `Decimal<i32, Fixed<2>>`, a [`D32<2>`] | 1,234 steps of 0.01 | 4 bytes |
/// | `Decimal<i64, Fixed<2>>`, a [`D64<2>`] | 1,234 steps of 0.01 | 8 bytes |
/// | `Decimal<i128, Fixed<18>>`, a [`D128<18>`] | 12,340,000,000,000,000,000 steps of 10^-18 | 16 bytes |
/// | `Decimal<i64, Dynamic>` | 1,234 steps, and a scale of 2 | 16 bytes |
///
/// - **Every operator behaves as the integer's.** Overflow panics with overflow checks on and wraps
///   otherwise, and each operator has the integer's `checked_*`, `saturating_*`, `wrapping_*` and
///   `overflowing_*` methods.
/// - **Nothing loses a digit unless a rounding mode says so.** `a * b` is exact, its scale the sum
///   of the factors'; `a / b` truncates toward zero, as the integer's does; every other operation
///   that can lose a digit takes a [rounding mode](crate::round) as its last argument, and the
///   exact conversions refuse instead.
/// - **Scales never mix by accident.** Two static scales are two types, and adding them does not
///   compile; two run-time scales line up exactly at the finer one, as SQL's `DECIMAL` does.
///
/// # Examples
/// ```
/// use ddust::round::{Ceil, Floor};
/// use ddust::{D64, dec};
///
/// let price: D64<2> = dec!(60000.37);
/// let tick: D64<2> = dec!(0.5);
/// assert_eq!(price.round_to(tick, Floor), dec!(60000), "down to the tick");
/// assert_eq!(price.round_to(tick, Ceil), dec!(60000.5), "up to the tick");
/// assert_eq!((price + tick).to_string(), "60000.87", "the sum, exact");
/// ```
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Decimal<I, S> {
    /// How many steps.
    steps: I,
    /// What a step is.
    scale: S,
}

/// A decimal of `i8` steps at `D` decimals: one byte.
pub type D8<const D: u8> = Decimal<i8, Fixed<D>>;
/// A decimal of `i16` steps at `D` decimals: two bytes.
pub type D16<const D: u8> = Decimal<i16, Fixed<D>>;
/// A decimal of `i32` steps at `D` decimals: four bytes.
pub type D32<const D: u8> = Decimal<i32, Fixed<D>>;
/// A decimal of `i64` steps at `D` decimals: eight bytes.
pub type D64<const D: u8> = Decimal<i64, Fixed<D>>;
/// A decimal of `i128` steps at `D` decimals: sixteen bytes.
pub type D128<const D: u8> = Decimal<i128, Fixed<D>>;
/// An unsigned decimal of `u8` steps at `D` decimals: one byte.
pub type UD8<const D: u8> = Decimal<u8, Fixed<D>>;
/// An unsigned decimal of `u16` steps at `D` decimals: two bytes.
pub type UD16<const D: u8> = Decimal<u16, Fixed<D>>;
/// An unsigned decimal of `u32` steps at `D` decimals: four bytes.
pub type UD32<const D: u8> = Decimal<u32, Fixed<D>>;
/// An unsigned decimal of `u64` steps at `D` decimals: eight bytes.
pub type UD64<const D: u8> = Decimal<u64, Fixed<D>>;
/// An unsigned decimal of `u128` steps at `D` decimals: sixteen bytes.
pub type UD128<const D: u8> = Decimal<u128, Fixed<D>>;

/// `value`, given at `from` decimals, at `to`, exactly: `Inexact` for a non-zero digit past `to`.
#[inline]
const fn shift<I: [const] Int>(value: I, from: u8, to: u8) -> Result<I, ConvertError> {
    if to >= from {
        return match value.scale_up(to.wrapping_sub(from)).checked() {
            Some(steps) => Ok(steps),
            None => Err(ConvertError::overflow(value.is_negative())),
        };
    }
    match value.scale_down(from.wrapping_sub(to), Trunc.table()) {
        (steps, true) => Ok(steps),
        (_, false) => Err(ConvertError::new(ConvertErrorKind::Inexact)),
    }
}

/// `value`, given at `from` decimals, at `to`, rounded by `mode` when it has more.
#[inline]
const fn shift_round<I: [const] Int, R: [const] RoundingMode>(
    value: I, from: u8, to: u8, mode: R,
) -> Result<I, ConvertError> {
    if to >= from {
        return shift(value, from, to);
    }
    Ok(value.scale_down(from.wrapping_sub(to), mode.table()).0)
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The decimal of `bits` steps at `scale`: its raw representation, for storage and for types
    /// built on it; [`new`](Self::new), [`dec!`](crate::dec!) and parsing are what a program reads
    /// values with.
    #[inline]
    #[must_use]
    pub const fn from_bits(bits: I, scale: S) -> Self {
        Self { steps: bits, scale }
    }

    /// The steps: the raw representation [`from_bits`](Self::from_bits) takes.
    #[inline]
    #[must_use]
    pub const fn to_bits(self) -> I {
        self.steps
    }

    /// The scale.
    #[inline]
    #[must_use]
    pub const fn scale(self) -> S {
        self.scale
    }

    /// How many decimals a step is.
    #[inline]
    #[must_use]
    pub const fn decimals(self) -> u8
    where
        S: [const] Scale,
    {
        self.scale.decimals()
    }

    /// `digits × 10^-decimals` at `scale`, exactly: `new(600_005, 1, Fixed)` is 60000.5.
    ///
    /// # Errors
    /// [`ConvertError`]: `Inexact` for a non-zero digit past the scale, `PosOverflow` or
    /// `NegOverflow` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Fixed};
    ///
    /// assert_eq!(D64::<2>::new(600_005, 1, Fixed)?.to_string(), "60000.5", "one decimal, in two");
    /// assert!(D64::<2>::new(1_005, 3, Fixed).is_err(), "three decimals do not fit two");
    /// # Ok::<(), ddust::ConvertError>(())
    /// ```
    #[inline]
    pub const fn new(digits: I, decimals: u8, scale: S) -> Result<Self, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        match shift(digits, decimals, scale.decimals()) {
            Ok(steps) => Ok(Self { steps, scale }),
            Err(error) => Err(error),
        }
    }

    /// `digits × 10^-decimals` at `scale`, rounded by `mode` when it has more decimals.
    ///
    /// # Errors
    /// [`ConvertError`]: `PosOverflow` or `NegOverflow` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, Fixed};
    ///
    /// let rounded = D64::<2>::new_round(1_005, 3, Fixed, HalfEven)?;
    /// assert_eq!(rounded.to_string(), "1", "1.005, half to the even cent");
    /// # Ok::<(), ddust::ConvertError>(())
    /// ```
    #[inline]
    pub const fn new_round<R>(
        digits: I, decimals: u8, scale: S, mode: R,
    ) -> Result<Self, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        match shift_round(digits, decimals, scale.decimals(), mode) {
            Ok(steps) => Ok(Self { steps, scale }),
            Err(error) => Err(error),
        }
    }

    /// The value as digits at `decimals`, exactly: 60000.5 at 4 decimals is 600,005,000.
    ///
    /// # Errors
    /// [`ConvertError`]: `Inexact` when the value has more decimals, `PosOverflow` or
    /// `NegOverflow` past the integer's range; [`widen`](Decimal::widen) first for more room.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, dec};
    ///
    /// let price: D64<2> = dec!(60000.5);
    /// assert_eq!(price.to_digits(4), Ok(600_005_000), "more decimals");
    /// assert!(price.to_digits(0).is_err(), "fewer: the half would be lost");
    /// ```
    #[inline]
    pub const fn to_digits(self, decimals: u8) -> Result<I, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        shift(self.steps, self.scale.decimals(), decimals)
    }

    /// The value as digits at `decimals`, rounded by `mode` when it has more.
    ///
    /// # Errors
    /// [`ConvertError`]: `PosOverflow` or `NegOverflow` past the integer's range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfExpand;
    /// use ddust::{D64, dec};
    ///
    /// let price: D64<2> = dec!(60000.5);
    /// assert_eq!(price.to_digits_round(0, HalfExpand), Ok(60_001), "a half, away from zero");
    /// ```
    #[inline]
    pub const fn to_digits_round<R>(self, decimals: u8, mode: R) -> Result<I, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        shift_round(self.steps, self.scale.decimals(), decimals, mode)
    }

    /// The value at another scale, exactly.
    ///
    /// # Errors
    /// [`ConvertError`]: `Inexact` when the value has more decimals than `scale`, `PosOverflow`
    /// or `NegOverflow` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D64, Dynamic, Fixed, dec};
    ///
    /// let price: D64<2> = dec!(60000.5);
    /// let finer: D64<4> = price.rescale(Fixed)?;
    /// assert_eq!(finer.to_bits(), 600_005_000, "four decimals");
    /// assert_eq!(price.rescale(Dynamic::new(1).expect("at most 38"))?.to_bits(), 600_005);
    /// # Ok::<(), ddust::ConvertError>(())
    /// ```
    #[inline]
    pub const fn rescale<T>(self, scale: T) -> Result<Decimal<I, T>, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
    {
        match shift(self.steps, self.scale.decimals(), scale.decimals()) {
            Ok(steps) => Ok(Decimal { steps, scale }),
            Err(error) => Err(error),
        }
    }

    /// The value at another scale, rounded by `mode` when it has more decimals.
    ///
    /// # Errors
    /// [`ConvertError`]: `PosOverflow` or `NegOverflow` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::HalfEven;
    /// use ddust::{D64, Fixed, dec};
    ///
    /// let rate: D64<6> = dec!(0.002_545);
    /// let shown: D64<4> = rate.rescale_round(Fixed, HalfEven)?;
    /// assert_eq!(shown, dec!(0.0025), "a tie, to the even digit");
    /// # Ok::<(), ddust::ConvertError>(())
    /// ```
    #[inline]
    pub const fn rescale_round<T, R>(self, scale: T, mode: R) -> Result<Decimal<I, T>, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
        T: [const] Scale,
        R: [const] RoundingMode,
    {
        match shift_round(self.steps, self.scale.decimals(), scale.decimals(), mode) {
            Ok(steps) => Ok(Decimal { steps, scale }),
            Err(error) => Err(error),
        }
    }

    /// Whether the value is zero.
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool
    where
        I: [const] Int,
    {
        self.steps == I::ZERO
    }

    /// Whether the value is above zero.
    #[inline]
    #[must_use]
    pub const fn is_positive(self) -> bool
    where
        I: [const] Int,
    {
        self.steps > I::ZERO
    }
}

impl<I: Int, S: StaticScale> Decimal<I, S> {
    /// Zero.
    pub const ZERO: Self = Self { steps: I::ZERO, scale: S::INSTANCE };
    /// The smallest value.
    pub const MIN: Self = Self { steps: I::MIN, scale: S::INSTANCE };
    /// The largest value.
    pub const MAX: Self = Self { steps: I::MAX, scale: S::INSTANCE };
}

impl<I: const Int, S: StaticScale + const Scale> Decimal<I, S> {
    /// One: `10^decimals` steps.
    ///
    /// # Examples
    /// ```
    /// use ddust::D64;
    ///
    /// assert_eq!(D64::<2>::ONE.to_bits(), 100, "a hundred hundredths");
    /// ```
    ///
    /// Naming it where one does not fit fails the build:
    ///
    /// ```compile_fail,E0080
    /// let _one = ddust::D8::<3>::ONE;
    /// ```
    pub const ONE: Self = {
        let one = Self::new(I::ONE, 0, S::INSTANCE);
        assert!(one.is_ok(), "one is past the decimal's range");
        match one {
            Ok(one) => one,
            Err(_) => Self::ZERO,
        }
    };
}

/// Zero, at the scale's default: a static scale's own, or a run-time scale of no decimals.
impl<I: Int, S: Scale + Default> Default for Decimal<I, S> {
    #[inline]
    fn default() -> Self {
        Self { steps: I::ZERO, scale: S::default() }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::round::{Ceil, Floor, HalfEven, HalfExpand, Rounding, Trunc};
    use crate::{ConvertError, ConvertErrorKind, D8, D64, D128, Decimal, Dynamic, Fixed, UD64};

    /// Hundredths.
    type Cents = D64<2>;

    #[rstest]
    #[case::exact(1_005, 1, Ok(10_050))]
    #[case::more_decimals(1_005, 3, Err(ConvertErrorKind::Inexact))]
    #[case::whole(25, 0, Ok(2_500))]
    #[case::above(i64::MAX, 0, Err(ConvertErrorKind::PosOverflow))]
    #[case::below(i64::MIN, 0, Err(ConvertErrorKind::NegOverflow))]
    #[case::far_below(1, 60, Err(ConvertErrorKind::Inexact))]
    #[case::zero_far_below(0, 60, Ok(0))]
    fn new_takes_digits_at_any_decimals(
        #[case] digits: i64, #[case] decimals: u8, #[case] expected: Result<i64, ConvertErrorKind>,
    ) {
        let made =
            Cents::new(digits, decimals, Fixed).map(Cents::to_bits).map_err(ConvertError::kind);
        assert_eq!(made, expected, "{digits} at {decimals}");
    }

    #[rstest]
    #[case::floor(Rounding::Floor, 100)]
    #[case::ceil(Rounding::Ceil, 101)]
    #[case::half_even(Rounding::HalfEven, 100)]
    #[case::half_expand(Rounding::HalfExpand, 101)]
    fn new_round_rounds_extra_decimals(#[case] mode: Rounding, #[case] bits: i64) {
        let made = Cents::new_round(10_050, 4, Fixed, mode).map(Cents::to_bits);
        assert_eq!(made, Ok(bits), "1.005 at two decimals, {mode:?}");
    }

    #[test]
    fn digits_come_out_at_any_decimals() {
        let price = Cents::from_bits(6_000_045, Fixed);
        assert_eq!(price.to_digits(4), Ok(600_004_500), "more decimals");
        assert_eq!(price.to_digits(1).map_err(ConvertError::kind), Err(ConvertErrorKind::Inexact));
        assert_eq!(price.to_digits_round(1, HalfEven), Ok(600_004), "a tie to the even digit");
        assert_eq!(price.to_digits_round(1, HalfExpand), Ok(600_005), "or away from zero");
        assert_eq!(Cents::from_bits(-1, Fixed).to_digits_round(0, Floor), Ok(-1), "floor of -0.01");
        assert_eq!(
            Cents::from_bits(1, Fixed).to_digits_round(60, Trunc).map_err(ConvertError::kind),
            Err(ConvertErrorKind::PosOverflow),
            "past an i64"
        );
    }

    #[test]
    fn a_rescale_is_exact_or_rounded() {
        let price = Cents::from_bits(6_000_045, Fixed);
        let tenths = Dynamic::new(1).expect("at most 38");
        assert_eq!(
            price.rescale(tenths).map_err(ConvertError::kind),
            Err(ConvertErrorKind::Inexact)
        );
        assert_eq!(price.rescale_round(tenths, Ceil).map(Decimal::to_bits), Ok(600_005), "up");
        let finer = price.rescale(Fixed::<18>).map_err(ConvertError::kind);
        assert_eq!(
            finer,
            Err(ConvertErrorKind::PosOverflow),
            "60000.45 has no room at 18 in an i64"
        );
        let wide =
            D128::<2>::from_bits(6_000_045, Fixed).rescale(Fixed::<18>).map(Decimal::to_bits);
        assert_eq!(wide, Ok(60_000_450_000_000_000_000_000), "but has in an i128");
    }

    #[test]
    fn the_constants_are_the_integers() {
        assert_eq!(
            (Cents::ZERO.to_bits(), Cents::MIN.to_bits(), Cents::MAX.to_bits()),
            (0, i64::MIN, i64::MAX)
        );
        assert_eq!((Cents::ONE.to_bits(), D8::<2>::ONE.to_bits()), (100, 100), "one: 10^2 steps");
        assert_eq!(UD64::<0>::ONE.to_bits(), 1, "at no decimals, one step");
        assert_eq!(Decimal::<i64, Dynamic>::default().decimals(), 0, "a run-time zero has none");
        assert!(Cents::ZERO.is_zero() && !Cents::ZERO.is_positive() && Cents::ONE.is_positive());
    }
}
