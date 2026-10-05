//! Conversions: between integer widths as std's integers convert, between scales, and to and from
//! integers.

use crate::decimal::Decimal;
use crate::errors::ConvertError;
use crate::int::Int;
use crate::round::RoundingMode;
use crate::scale::{Dynamic, Fixed, Scale, StaticScale, Sum};

/// `From` for each pair of integers where every value of the first fits the second.
macro_rules! lossless {
    ($($from:ty => $($to:ty),*;)*) => {$($(
        /// Every value fits: the same steps, at the same scale.
        const impl<S: [const] Scale> From<Decimal<$from, S>> for Decimal<$to, S> {
            #[inline]
            fn from(value: Decimal<$from, S>) -> Self {
                Self::from_steps(<$to>::from(value.steps()), value.scale())
            }
        }
    )*)*};
}

lossless! {
    i8 => i16, i32, i64, i128;
    i16 => i32, i64, i128;
    i32 => i64, i128;
    i64 => i128;
    u8 => u16, u32, u64, u128, i16, i32, i64, i128;
    u16 => u32, u64, u128, i32, i64, i128;
    u32 => u64, u128, i64, i128;
    u64 => u128, i128;
}

/// `TryFrom` for every other pair of integers.
macro_rules! lossy {
    ($($from:ty => $($to:ty),*;)*) => {$($(
        /// The same steps at the same scale, or `PosOverflow` or `NegOverflow` when they do not
        /// fit.
        const impl<S: [const] Scale> TryFrom<Decimal<$from, S>> for Decimal<$to, S> {
            type Error = ConvertError;

            #[inline]
            fn try_from(value: Decimal<$from, S>) -> Result<Self, ConvertError> {
                let steps = value.steps();
                match <$to>::try_from(steps) {
                    Ok(narrowed) => Ok(Self::from_steps(narrowed, value.scale())),
                    Err(_out_of_range) => Err(ConvertError::overflow(steps.is_negative())),
                }
            }
        }
    )*)*};
}

lossy! {
    i8 => u8, u16, u32, u64, u128;
    i16 => i8, u8, u16, u32, u64, u128;
    i32 => i8, i16, u8, u16, u32, u64, u128;
    i64 => i8, i16, i32, u8, u16, u32, u64, u128;
    i128 => i8, i16, i32, i64, u8, u16, u32, u64, u128;
    u8 => i8;
    u16 => i8, i16, u8;
    u32 => i8, i16, i32, u8, u16;
    u64 => i8, i16, i32, i64, u8, u16, u32;
    u128 => i8, i16, i32, i64, i128, u8, u16, u32, u64;
}

/// `widen` for each integer with a wider twin.
macro_rules! widen {
    ($($t:ty => $wide:ty),*) => {$(
        impl<S: Scale> Decimal<$t, S> {
            #[doc = concat!(
                "The same value in a `", stringify!($wide), "`, twice as wide: where a product, a ",
                "quotient or more digits have room."
            )]
            ///
            /// # Examples
            /// ```
            /// use ddust::{D64, dec};
            ///
            /// let (a, b): (D64<8>, D64<8>) = (dec!(60000.37), dec!(125.5));
            /// let product = a.widen() * b.widen();
            /// assert_eq!(product.decimals(), 16, "an i128 at sixteen decimals, exact");
            /// ```
            #[inline]
            #[must_use]
            pub const fn widen(self) -> Decimal<$wide, S> {
                Decimal::from_steps(<$wide>::from(self.steps()), self.scale())
            }
        }
    )*};
}

widen!(i8 => i16, i16 => i32, i32 => i64, i64 => i128, u8 => u16, u16 => u32, u32 => u64, u64 => u128);

/// A static scale's value as a run-time one: the same steps, at the same decimals.
const impl<I: [const] Int, S: StaticScale> From<Decimal<I, S>> for Decimal<I, Dynamic> {
    #[inline]
    fn from(value: Decimal<I, S>) -> Self {
        Self::from_steps(value.steps(), Dynamic::from(value.scale()))
    }
}

/// A run-time scale's value at a static one, exactly.
const impl<I: [const] Int, const D: u8> TryFrom<Decimal<I, Dynamic>> for Decimal<I, Fixed<D>> {
    type Error = ConvertError;

    #[inline]
    fn try_from(value: Decimal<I, Dynamic>) -> Result<Self, ConvertError> {
        value.rescale(Fixed)
    }
}

/// A product's scale as the `Fixed` of its decimals, with no instruction; a `Fixed` of other
/// decimals fails the build.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
///
/// let (unit_price, quantity): (D64<2>, D64<3>) = (dec!(19.99), dec!(2.375));
/// let total: D64<5> = (unit_price * quantity).into();
/// assert_eq!(total, dec!(47.47625), "two decimals and three: five");
/// ```
///
/// Any other scale fails the build:
///
/// ```compile_fail,E0080
/// use ddust::{D64, dec};
///
/// let (unit_price, quantity): (D64<2>, D64<3>) = (dec!(19.99), dec!(2.375));
/// let total: D64<4> = (unit_price * quantity).into();
/// ```
const impl<I: [const] Int, A: StaticScale, B: StaticScale, const C: u8> From<Decimal<I, Sum<A, B>>>
    for Decimal<I, Fixed<C>>
{
    #[inline]
    fn from(value: Decimal<I, Sum<A, B>>) -> Self {
        const { assert!(<Sum<A, B>>::DECIMALS == C, "a product's decimals are the sum of its factors'") };
        Self::from_steps(value.steps(), Fixed)
    }
}

impl<I: Int, S: Scale> Decimal<I, S> {
    /// The whole number `n` at `scale`: `n × 10^decimals` steps.
    ///
    /// # Errors
    /// [`ConvertError`]: `PosOverflow` or `NegOverflow` past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::{D8, D64, Fixed};
    ///
    /// assert_eq!(D64::<2>::from_int(5, Fixed)?.steps(), 500, "five, in hundredths");
    /// assert!(D8::<2>::from_int(2, Fixed).is_err(), "two is past 1.27");
    /// # Ok::<(), ddust::ConvertError>(())
    /// ```
    #[inline]
    pub const fn from_int(n: I, scale: S) -> Result<Self, ConvertError>
    where
        I: [const] Int,
        S: [const] Scale,
    {
        Self::new(n, 0, scale)
    }

    /// The value rounded to a whole number by `mode`, as an integer: never past the range.
    ///
    /// # Examples
    /// ```
    /// use ddust::round::{Floor, HalfEven};
    /// use ddust::{D64, dec};
    ///
    /// let x: D64<2> = dec!(-2.5);
    /// assert_eq!((x.to_int(Floor), x.to_int(HalfEven)), (-3, -2));
    /// ```
    #[inline]
    #[must_use]
    pub const fn to_int<R>(self, mode: R) -> I
    where
        I: [const] Int,
        S: [const] Scale,
        R: [const] RoundingMode,
    {
        self.steps().scale_down(self.decimals(), mode.table()).0
    }
}

/// `TryFrom` an integer, for each integer: a generic one would overlap core's blanket.
macro_rules! from_integer {
    ($($t:ty),*) => {$(
        /// The whole number at the scale, or `PosOverflow` or `NegOverflow` past the range.
        const impl<S: StaticScale + [const] Scale> TryFrom<$t> for Decimal<$t, S> {
            type Error = ConvertError;

            #[inline]
            fn try_from(n: $t) -> Result<Self, ConvertError> {
                Self::from_int(n, S::INSTANCE)
            }
        }
    )*};
}

from_integer!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

#[cfg(test)]
mod tests {
    use crate::round::{Ceil, Floor};
    use crate::{
        ConvertError, ConvertErrorKind, D8, D16, D32, D64, D128, Decimal, Dynamic, Fixed, UD8, UD64,
    };

    #[test]
    fn a_width_converts_as_the_integers_do() {
        let small = D32::<2>::from_steps(-1_234, Fixed);
        assert_eq!(D64::<2>::from(small).steps(), -1_234, "every i32 fits an i64");
        assert_eq!(small.widen(), D64::<2>::from(small), "widen, the same");
        let refused = UD64::<2>::try_from(D64::<2>::from(small)).map_err(ConvertError::kind);
        assert_eq!(refused, Err(ConvertErrorKind::NegOverflow), "no negative is unsigned");
        let narrowed =
            D8::<2>::try_from(D16::<2>::from_steps(300, Fixed)).map_err(ConvertError::kind);
        assert_eq!(narrowed, Err(ConvertErrorKind::PosOverflow), "3.00 is past 1.27");
        assert_eq!(
            D128::<2>::from(UD8::<2>::from_steps(255, Fixed)).steps(),
            255,
            "unsigned to signed"
        );
    }

    #[test]
    fn a_scale_converts_to_run_time_and_back() {
        let value = D64::<2>::from_steps(6_000_050, Fixed);
        let run_time = Decimal::<i64, Dynamic>::from(value);
        assert_eq!((run_time.steps(), run_time.decimals()), (6_000_050, 2), "the same");
        assert_eq!(D64::<4>::try_from(run_time).map(D64::steps), Ok(600_005_000), "finer, exact");
        let coarser = D64::<1>::try_from(run_time).map(D64::steps);
        assert_eq!(coarser, Ok(600_005), "60000.50 has a digit to spare at one decimal");
        let refused = D64::<0>::try_from(run_time).map_err(ConvertError::kind);
        assert_eq!(refused, Err(ConvertErrorKind::TooManyDecimals), "but not at none");
    }

    #[test]
    fn a_whole_number_converts_both_ways() {
        assert_eq!(D64::<2>::try_from(7).map(D64::steps), Ok(700), "seven, in hundredths");
        let x = D64::<2>::from_steps(-250, Fixed);
        assert_eq!((x.to_int(Floor), x.to_int(Ceil)), (-3, -2), "-2.5, both ways");
    }
}
