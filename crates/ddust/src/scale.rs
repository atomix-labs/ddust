//! What gives a decimal its scale: a type the compiler knows, or a value carried at run time; and
//! the scale of a product, whose decimals are the sum of its factors'.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

#[cfg(feature = "bytemuck")]
use bytemuck::{AnyBitPattern, NoUninit, Zeroable};
#[cfg(feature = "defmt")]
use defmt::Format;
#[cfg(feature = "zerocopy-08")]
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// The most decimals a scale has: 38, since `10^38` is the largest power of ten a 128-bit integer
/// holds.
pub const MAX_DECIMALS: u8 = 38;

/// What gives a decimal its scale: how many decimals a step is.
///
/// A scale the compiler knows is zero-sized, so a decimal of it is its integer alone, and two
/// values of it never differ in scale. One known at run time is carried beside the integer, and
/// [`LINES_UP`](Self::LINES_UP) says what happens when two of them differ.
///
/// # Examples
/// ```
/// use ddust::{Dynamic, Fixed, Scale};
///
/// assert_eq!(Fixed::<2>.decimals(), 2, "known at compile time");
/// assert_eq!(Dynamic::new(4).map(Scale::decimals), Some(4), "carried at run time");
/// ```
pub const trait Scale:
    Copy + [const] Eq + [const] Ord + Hash + fmt::Debug + Send + Sync + 'static
{
    /// Whether two values of this scale at different decimals line up at the finer one, as two
    /// [`Dynamic`] values do, rather than never mixing: an operator then panics, in every build,
    /// and a `checked_*` method returns `None`. Two values of a static scale never differ.
    const LINES_UP: bool = false;

    /// How many decimals a step is: a step is `10^-decimals`, and there are at most
    /// [`MAX_DECIMALS`].
    fn decimals(self) -> u8;
}

/// A [`Scale`] the compiler knows, so every value of it shares one scale, and parses from text
/// alone.
pub trait StaticScale: Scale {
    /// The scale itself, for a `const` context.
    const INSTANCE: Self;
    /// Its decimals, for a `const` context.
    const DECIMALS: u8;
}

/// A scale fixed at compile time: `Decimal<i64, Fixed<2>>` holds 12.34 as 1,234 steps of 0.01 in
/// eight bytes. More than 38 decimals fails the build.
///
/// # Examples
/// ```
/// use ddust::{Fixed, Scale};
///
/// assert_eq!(Fixed::<4>.decimals(), 4, "four decimals");
/// assert_eq!(size_of::<Fixed<4>>(), 0, "known at compile time, so it takes no room");
/// ```
///
/// More than 38 decimals fails the build:
///
/// ```compile_fail,E0080
/// use ddust::{Fixed, Scale};
///
/// let _decimals = Fixed::<39>.decimals();
/// ```
#[derive(Clone, Copy, Default, Hash)]
#[derive_const(PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "bytemuck", derive(AnyBitPattern))]
#[cfg_attr(
    feature = "zerocopy-08",
    derive(FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned)
)]
#[repr(C)]
pub struct Fixed<const D: u8>;

/// `Fixed<D>`, with its decimals.
impl<const D: u8> fmt::Debug for Fixed<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed<{D}>")
    }
}

const impl<const D: u8> Scale for Fixed<D> {
    #[inline]
    fn decimals(self) -> u8 {
        Self::DECIMALS
    }
}

impl<const D: u8> StaticScale for Fixed<D> {
    const INSTANCE: Self = Self;
    const DECIMALS: u8 = {
        assert!(D <= MAX_DECIMALS, "a scale has at most 38 decimals");
        D
    };
}

/// A scale chosen at run time and carried beside the integer: a precision read from a
/// configuration or a database column, or the scale a number's text spells.
///
/// Two values of different run-time scales line up exactly at the finer one, as SQL's `DECIMAL`
/// does: `1.5 + 1.25` is `2.75`, and `1.5 == 1.50`.
///
/// # Examples
/// ```
/// use ddust::{Dynamic, Scale};
///
/// let precision = Dynamic::new(3).expect("at most 38");
/// assert_eq!(precision.decimals(), 3, "chosen at run time");
/// assert_eq!(Dynamic::new(39), None, "past the finest step any integer holds");
/// ```
#[derive(Debug, Clone, Copy, Default, Hash)]
#[derive_const(PartialEq, Eq, PartialOrd, Ord)]
// Its byte is written, and never read back, since a byte may be past 38 decimals.
#[cfg_attr(feature = "bytemuck", derive(NoUninit, Zeroable))]
#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "zerocopy-08", derive(Immutable, IntoBytes, KnownLayout, Unaligned))]
#[repr(C)]
pub struct Dynamic {
    /// How many decimals a step is.
    decimals: u8,
}

impl Dynamic {
    /// A scale of `decimals`, or `None` past [`MAX_DECIMALS`].
    #[inline]
    #[must_use]
    pub const fn new(decimals: u8) -> Option<Self> {
        if decimals <= MAX_DECIMALS { Some(Self { decimals }) } else { None }
    }

    /// The scale of a product: the sum of its factors' decimals.
    ///
    /// # Panics
    /// Past [`MAX_DECIMALS`], a scale no integer holds a step of.
    #[inline]
    #[track_caller]
    const fn product(a: u8, b: u8) -> Self {
        let decimals = a.saturating_add(b);
        assert!(decimals <= MAX_DECIMALS, "a product's decimals are past 38");
        Self { decimals }
    }

    /// The scale of a product, or `None` past [`MAX_DECIMALS`].
    #[inline]
    const fn checked_product(a: u8, b: u8) -> Option<Self> {
        Self::new(a.saturating_add(b))
    }
}

const impl Scale for Dynamic {
    const LINES_UP: bool = true;

    #[inline]
    fn decimals(self) -> u8 {
        self.decimals
    }
}

/// A static scale as a run-time one, at the same decimals.
const impl<S: StaticScale> From<S> for Dynamic {
    #[inline]
    fn from(_scale: S) -> Self {
        Self { decimals: S::DECIMALS }
    }
}

/// The scale of a product of two values of static scales: `A`'s decimals and `B`'s, summed.
///
/// Stable Rust cannot write `Fixed<{A + B}>`, so a product's scale is this type, and converts into
/// the `Fixed` of its decimals, its steps unchanged; a `Fixed` of other decimals fails the build.
///
/// # Examples
/// ```
/// use ddust::scale::Sum;
/// use ddust::{Fixed, StaticScale};
///
/// assert_eq!(<Sum<Fixed<2>, Fixed<4>>>::DECIMALS, 6, "two decimals and four");
/// ```
pub struct Sum<A, B>(PhantomData<fn() -> (A, B)>);

impl<A, B> Clone for Sum<A, B> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Copy for Sum<A, B> {}

impl<A, B> Default for Sum<A, B> {
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Every value of the type is the one scale.
const impl<A, B> PartialEq for Sum<A, B> {
    #[inline]
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

const impl<A, B> Eq for Sum<A, B> {}

/// Every value of the type is the one scale.
const impl<A, B> PartialOrd for Sum<A, B> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Every value of the type is the one scale.
const impl<A, B> Ord for Sum<A, B> {
    #[inline]
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

/// Hashes nothing, as a unit struct's derive does.
impl<A, B> Hash for Sum<A, B> {
    #[inline]
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

/// `Sum<A, B>`, with its factors.
impl<A: StaticScale, B: StaticScale> fmt::Debug for Sum<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sum<{:?}, {:?}>", A::INSTANCE, B::INSTANCE)
    }
}

const impl<A: StaticScale, B: StaticScale> Scale for Sum<A, B> {
    #[inline]
    fn decimals(self) -> u8 {
        Self::DECIMALS
    }
}

impl<A: StaticScale, B: StaticScale> StaticScale for Sum<A, B> {
    const INSTANCE: Self = Self(PhantomData);
    const DECIMALS: u8 = {
        let decimals = A::DECIMALS.saturating_add(B::DECIMALS);
        assert!(decimals <= MAX_DECIMALS, "a product's decimals are past 38");
        decimals
    };
}

/// The scale of a product of a value at `Self` and one at `Rhs`, whose decimals are the sum of
/// theirs: a [`Sum`] of two static scales, and a [`Dynamic`] when either is one.
///
/// # Examples
/// ```
/// use ddust::scale::Times;
/// use ddust::{Dynamic, Fixed, Scale};
///
/// let product = Fixed::<2>.times(Dynamic::new(3).expect("at most 38"));
/// assert_eq!(product.decimals(), 5, "two decimals and three, at run time");
/// ```
pub const trait Times<Rhs: Scale>: Scale {
    /// The product's scale.
    type Output: Scale;

    /// The product's scale.
    ///
    /// # Panics
    /// For a run-time product past 38 decimals; a static one fails the build instead.
    #[track_caller]
    fn times(self, rhs: Rhs) -> Self::Output;

    /// The product's scale, or `None` for a run-time product past 38 decimals.
    fn checked_times(self, rhs: Rhs) -> Option<Self::Output>;
}

const impl<A: StaticScale, B: StaticScale> Times<B> for A {
    type Output = Sum<A, B>;

    #[inline]
    fn times(self, _rhs: B) -> Sum<A, B> {
        Sum::<A, B>::INSTANCE
    }

    #[inline]
    fn checked_times(self, _rhs: B) -> Option<Sum<A, B>> {
        Some(Sum::<A, B>::INSTANCE)
    }
}

const impl<A: StaticScale> Times<Dynamic> for A {
    type Output = Dynamic;

    #[inline]
    #[track_caller]
    fn times(self, rhs: Dynamic) -> Dynamic {
        Dynamic::product(A::DECIMALS, rhs.decimals)
    }

    #[inline]
    fn checked_times(self, rhs: Dynamic) -> Option<Dynamic> {
        Dynamic::checked_product(A::DECIMALS, rhs.decimals)
    }
}

const impl<B: [const] Scale> Times<B> for Dynamic {
    type Output = Dynamic;

    #[inline]
    #[track_caller]
    fn times(self, rhs: B) -> Dynamic {
        Dynamic::product(self.decimals, rhs.decimals())
    }

    #[inline]
    fn checked_times(self, rhs: B) -> Option<Dynamic> {
        Dynamic::checked_product(self.decimals, rhs.decimals())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::{Dynamic, Fixed, Scale, StaticScale, Sum, Times};

    #[test]
    fn a_static_scale_takes_no_room_and_says_its_decimals() {
        assert_eq!((size_of::<Fixed<18>>(), Fixed::<18>.decimals()), (0, 18), "zero-sized");
        assert_eq!(<Sum<Fixed<2>, Fixed<7>>>::DECIMALS, 9, "a product's: the sum");
        assert_eq!(format!("{:?}", <Sum<Fixed<2>, Fixed<7>>>::INSTANCE), "Sum<Fixed<2>, Fixed<7>>");
    }

    #[test]
    fn a_run_time_scale_is_checked_once_and_lines_up() {
        assert_eq!(Dynamic::new(38).map(Scale::decimals), Some(38), "the finest");
        assert_eq!(Dynamic::new(39), None, "no integer holds a finer step");
        const { assert!(Dynamic::LINES_UP && !Fixed::<2>::LINES_UP, "run-time scales line up") };
        assert_eq!(Dynamic::from(Fixed::<6>).decimals(), 6, "a static scale as a run-time one");
    }

    #[test]
    fn a_product_with_a_run_time_scale_is_one() {
        let three = Dynamic::new(3).expect("at most 38");
        assert_eq!(Fixed::<2>.times(three).decimals(), 5, "static times run time");
        assert_eq!(three.times(Fixed::<2>).decimals(), 5, "and the other way");
        assert_eq!(three.times(three).decimals(), 6, "run time times run time");
    }

    #[test]
    fn a_run_time_product_past_38_decimals_has_no_scale() {
        let fine = Dynamic::new(20).expect("at most 38");
        assert_eq!(fine.checked_times(fine), None, "40 decimals");
        assert_eq!(
            fine.checked_times(Fixed::<18>).map(Scale::decimals),
            Some(38),
            "38, the finest"
        );
    }

    #[test]
    #[should_panic(expected = "a product's decimals are past 38")]
    fn a_run_time_product_past_38_decimals_panics() {
        let fine = Dynamic::new(20).expect("at most 38");
        let _product_scale = fine.times(fine);
    }
}
