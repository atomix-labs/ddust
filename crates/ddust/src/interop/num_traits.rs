//! num-traits' traits whose contracts a decimal meets. `One`, `Num` and the traits built on them
//! need a `*` whose product keeps its factors' scale, which a decimal's does not: its scale is the
//! sum of theirs.

use num_traits::{
    Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedNeg, CheckedRem, CheckedSub, ConstZero,
    FromPrimitive, SaturatingAdd, SaturatingMul, SaturatingSub, ToPrimitive, WrappingAdd,
    WrappingMul, WrappingNeg, WrappingSub, Zero,
};

use crate::round::{HalfEven, Trunc};
use crate::{Decimal, Dynamic, Int, Scale, Signed, StaticScale};

/// Zero steps.
///
/// # Examples
/// ```
/// use ddust::D64;
/// use num_traits::Zero;
///
/// assert!(D64::<2>::zero().is_zero(), "zero steps of 0.01");
/// ```
impl<I: Int, S: StaticScale> Zero for Decimal<I, S> {
    #[inline]
    fn zero() -> Self {
        Self::ZERO
    }

    #[inline]
    fn is_zero(&self) -> bool {
        Self::is_zero(*self)
    }
}

/// Zero steps, as a constant.
impl<I: Int, S: StaticScale> ConstZero for Decimal<I, S> {
    const ZERO: Self = Self::ZERO;
}

/// The integer's bounds, in steps.
///
/// # Examples
/// ```
/// use ddust::{D8, dec};
/// use num_traits::Bounded;
///
/// assert_eq!(D8::<1>::max_value(), dec!(12.7), "127 steps of 0.1");
/// ```
impl<I: Int, S: StaticScale> Bounded for Decimal<I, S> {
    #[inline]
    fn min_value() -> Self {
        Self::MIN
    }

    #[inline]
    fn max_value() -> Self {
        Self::MAX
    }
}

/// A num-traits trait whose method takes `&self` and `&Self`, forwarded to the decimal's own.
macro_rules! forward {
    ($generics:tt $decimal:ty: $($trait:ident :: $method:ident -> $output:ty),* $(,)?) => {$(
        forward!(@one $generics $decimal, $trait, $method, $output);
    )*};
    (@one [$($generics:tt)*] $decimal:ty, $trait:ident, $method:ident, $output:ty) => {
        #[doc = concat!("As [`Decimal::", stringify!($method), "`].")]
        impl<$($generics)*> $trait for $decimal {
            #[inline]
            fn $method(&self, rhs: &Self) -> $output {
                Decimal::$method(*self, *rhs)
            }
        }
    };
}

forward!([I: Int, S: Scale] Decimal<I, S>:
    CheckedAdd::checked_add -> Option<Self>,
    CheckedSub::checked_sub -> Option<Self>,
    CheckedDiv::checked_div -> Option<Self>,
    CheckedRem::checked_rem -> Option<Self>,
    SaturatingAdd::saturating_add -> Self,
    SaturatingSub::saturating_sub -> Self,
    WrappingAdd::wrapping_add -> Self,
    WrappingSub::wrapping_sub -> Self,
);

// Only a run-time scale's product is of its factors' type, as these traits' `Mul` bound needs.
forward!([I: Int] Decimal<I, Dynamic>:
    CheckedMul::checked_mul -> Option<Self>,
    SaturatingMul::saturating_mul -> Self,
    WrappingMul::wrapping_mul -> Self,
);

/// As [`Decimal::checked_neg`].
impl<I: Signed, S: Scale> CheckedNeg for Decimal<I, S> {
    #[inline]
    fn checked_neg(&self) -> Option<Self> {
        Self::checked_neg(*self)
    }
}

/// As [`Decimal::wrapping_neg`].
impl<I: Signed, S: Scale> WrappingNeg for Decimal<I, S> {
    #[inline]
    fn wrapping_neg(&self) -> Self {
        Self::wrapping_neg(*self)
    }
}

/// `n` in another integer, or `None` past its range.
#[inline]
fn convert<J: Int, I: Int>(n: J) -> Option<I> {
    let (negative, magnitude) = n.sign_and_magnitude();
    I::from_magnitude(negative, magnitude)
}

/// A whole number exactly, and a float at the nearest step, a tie to the even one, where an
/// integer truncates: a float is rarely exact at a decimal scale, and its nearest step is the
/// decimal it was written from while the steps stay below 2^52. `None` past the range, and for a
/// float that is not finite.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
/// use num_traits::FromPrimitive;
///
/// assert_eq!(D64::<2>::from_i64(-3), Some(dec!(-3)), "a whole number, exactly");
/// assert_eq!(<D64<2> as FromPrimitive>::from_f64(0.1), Some(dec!(0.1)), "the nearest cent");
/// assert_eq!(D64::<2>::from_u64(u64::MAX), None, "past an i64 of cents");
/// ```
impl<I: Int, S: StaticScale> FromPrimitive for Decimal<I, S> {
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        Self::from_int(convert(n)?, S::INSTANCE).ok()
    }

    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        Self::from_int(convert(n)?, S::INSTANCE).ok()
    }

    #[inline]
    fn from_i128(n: i128) -> Option<Self> {
        Self::from_int(convert(n)?, S::INSTANCE).ok()
    }

    #[inline]
    fn from_u128(n: u128) -> Option<Self> {
        Self::from_int(convert(n)?, S::INSTANCE).ok()
    }

    #[inline]
    fn from_f32(x: f32) -> Option<Self> {
        Self::from_f64(f64::from(x), S::INSTANCE, HalfEven)
    }

    #[inline]
    fn from_f64(x: f64) -> Option<Self> {
        Self::from_f64(x, S::INSTANCE, HalfEven)
    }
}

/// The whole part, truncated toward zero as a float's is, or `None` past the integer's range; and
/// the nearest `f64`, which an `f32` is then rounded from.
///
/// # Examples
/// ```
/// use ddust::{D64, dec};
/// use num_traits::ToPrimitive;
///
/// let x: D64<2> = dec!(-2.75);
/// assert_eq!(x.to_i64(), Some(-2), "truncated toward zero");
/// assert_eq!(x.to_u64(), None, "below zero");
/// assert_eq!(ToPrimitive::to_f64(&x), Some(-2.75), "the nearest double");
/// ```
impl<I: Int, S: Scale> ToPrimitive for Decimal<I, S> {
    #[inline]
    fn to_i64(&self) -> Option<i64> {
        convert(self.to_int(Trunc))
    }

    #[inline]
    fn to_u64(&self) -> Option<u64> {
        convert(self.to_int(Trunc))
    }

    #[inline]
    fn to_i128(&self) -> Option<i128> {
        convert(self.to_int(Trunc))
    }

    #[inline]
    fn to_u128(&self) -> Option<u128> {
        convert(self.to_int(Trunc))
    }

    #[inline]
    fn to_f64(&self) -> Option<f64> {
        Some(Self::to_f64(*self))
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;

    use num_traits::{
        Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedNeg, FromPrimitive, SaturatingAdd,
        ToPrimitive, WrappingSub, Zero,
    };

    use crate::{D8, D64, D128, Decimal, Dynamic, Fixed, UD64, dec};

    /// The sum of `values`, or `None` past the range: code generic over num-traits alone.
    fn checked_sum<T: Zero + CheckedAdd>(values: &[T]) -> Option<T> {
        values.iter().try_fold(T::zero(), |sum, value| sum.checked_add(value))
    }

    #[test]
    fn generic_code_computes_with_decimals() {
        let prices: [D64<2>; 3] = [dec!(19.99), dec!(0.01), dec!(80)];
        assert_eq!(checked_sum(&prices), Some(dec!(100)));
        assert_eq!(checked_sum(&[D8::<1>::max_value(), dec!(0.1)]), None, "past 12.7");
    }

    #[test]
    fn each_trait_forwards_to_the_decimal() {
        let x: D64<2> = dec!(7.5);
        assert_eq!(CheckedDiv::checked_div(&x, &D64::<2>::zero()), None, "never by zero");
        assert_eq!(CheckedNeg::checked_neg(&D64::<2>::min_value()), None, "past the range");
        assert_eq!(
            SaturatingAdd::saturating_add(&D64::<2>::max_value(), &x),
            D64::<2>::max_value()
        );
        assert_eq!(
            WrappingSub::wrapping_sub(&D64::<2>::min_value(), &D64::<2>::from_steps(1, Fixed)),
            D64::<2>::max_value()
        );
        let (a, b): (Decimal<i64, Dynamic>, Decimal<i64, Dynamic>) =
            ("1.5".parse().expect("a decimal"), "0.25".parse().expect("a decimal"));
        let product = CheckedMul::checked_mul(&a, &b).expect("in range");
        assert_eq!(product.to_string(), "0.375", "exact, at three decimals");
    }

    #[test]
    fn primitives_convert_by_the_conversion_rules() {
        assert_eq!(D128::<18>::from_u128(u128::MAX), None, "past an i128 at 18 decimals");
        assert_eq!(D128::<0>::from_i128(i128::MIN), Some(D128::<0>::MIN), "the widest, exactly");
        assert_eq!(UD64::<2>::from_i64(-1), None, "below an unsigned decimal");
        assert_eq!(<D64<2> as FromPrimitive>::from_f64(f64::NAN), None, "not a number");
        assert_eq!(D64::<2>::from_f32(0.1), Some(dec!(0.1)), "the nearest cent to the single");
        assert_eq!(D64::<2>::max_value().to_i128(), Some(i128::from(i64::MAX / 100)), "whole part");
        assert_eq!(D64::<2>::from_steps(-50, Fixed).to_u64(), Some(0), "-0.5, truncated to zero");
    }
}
