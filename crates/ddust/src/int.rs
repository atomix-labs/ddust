//! The integers a decimal counts its steps in, and what each of them does.

use core::cmp::Ordering;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use crate::kernel::{self, Exact};
use crate::word::{Double, Narrow, U256, Word};

/// Seals [`Int`] and [`Signed`]: the ten primitive integers are all there are.
mod sealed {
    /// Implemented for `i8` to `i128` and `u8` to `u128` alone.
    pub trait Sealed {}
}

/// What an operation came to, before overflow is settled: its result, wrapped when it is past
/// the integer's range, whether it is, and its sign. Each of the integer's families reads it its
/// own way.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct Outcome<I> {
    /// The result, modulo the integer's range.
    wrapped: I,
    /// Whether the result is past the range.
    past: bool,
    /// Whether the result is below zero.
    negative: bool,
}

impl<I: Int> Outcome<I> {
    /// A result within the range.
    #[inline]
    const fn fits(value: I) -> Self {
        Self { wrapped: value, past: false, negative: false }
    }

    /// The result of the integer's own operator: its wrapped value, whether it overflowed, and
    /// the sign of the exact result.
    #[inline]
    pub(crate) const fn from_parts(wrapped: I, past: bool, negative: bool) -> Self {
        Self { wrapped, past, negative }
    }

    /// The result, or `None` past the range.
    #[inline]
    #[must_use]
    pub const fn checked(self) -> Option<I> {
        if self.past { None } else { Some(self.wrapped) }
    }

    /// The result, wrapped around the range.
    #[inline]
    #[must_use]
    pub const fn wrapping(self) -> I {
        self.wrapped
    }

    /// The result wrapped around the range, and whether it was.
    #[inline]
    #[must_use]
    pub const fn overflowing(self) -> (I, bool) {
        (self.wrapped, self.past)
    }

    /// Whether the result is past the range.
    #[inline]
    #[must_use]
    pub const fn is_past(self) -> bool {
        self.past
    }

    /// The result as an operator gives it: past the range, a panic with overflow checks on, and
    /// the wrapped value otherwise, as the integer's own operator does.
    #[inline]
    #[track_caller]
    pub(crate) const fn operator(self, operation: Operation) -> I {
        if cfg!(overflow_checks) && self.past {
            overflowed(operation);
        }
        self.wrapped
    }

    /// The result, held at the end of the range it is past.
    #[inline]
    #[must_use]
    pub const fn saturating(self) -> I
    where
        I: [const] Int,
    {
        match (self.past, self.negative) {
            (false, _) => self.wrapped,
            (true, true) => I::MIN,
            (true, false) => I::MAX,
        }
    }
}

/// An operation whose overflow panics with the integer's own message.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
    /// `/`.
    Divide,
}

/// Panics as the integer's operator does past its range.
#[cold]
#[inline(never)]
#[track_caller]
#[expect(clippy::panic, reason = "the integer's own overflow, with its own message")]
const fn overflowed(operation: Operation) -> ! {
    match operation {
        Operation::Add => panic!("attempt to add with overflow"),
        Operation::Subtract => panic!("attempt to subtract with overflow"),
        Operation::Multiply => panic!("attempt to multiply with overflow"),
        Operation::Divide => panic!("attempt to divide with overflow"),
    }
}

/// What a decimal counts its steps in: one of the ten primitive integers, `i8` to `i128` and `u8`
/// to `u128`.
///
/// Its methods are the integers' own under one name, and the exact kernels of the decimal's
/// arithmetic, so a decimal is written once for all ten; they are hidden, since
/// a program calls the decimal's.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an integer a decimal counts its steps in",
    note = "a decimal counts its steps in `i8`, `i16`, `i32`, `i64`, `i128` or their unsigned twins"
)]
pub const trait Int:
    sealed::Sealed
    + Copy
    + Hash
    + Debug
    + Display
    + Default
    + Send
    + Sync
    + 'static
    + [const] Eq
    + [const] Ord
    + [const] Add<Output = Self>
    + [const] Sub<Output = Self>
    + [const] Mul<Output = Self>
    + [const] Div<Output = Self>
    + [const] Rem<Output = Self>
{
    /// Zero.
    const ZERO: Self;
    /// One.
    const ONE: Self;
    /// The smallest value.
    const MIN: Self;
    /// The largest value.
    const MAX: Self;
    /// How many decimal digits every value has room for: 2 for 8 bits, 4 for 16, 9 for 32, 18
    /// for `i64` and 19 for `u64`, and 38 for 128.
    const DIGITS: u8;

    /// As the integer's own.
    #[doc(hidden)]
    fn checked_add(self, other: Self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    fn checked_sub(self, other: Self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    fn checked_mul(self, other: Self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    fn checked_div(self, other: Self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    fn checked_rem(self, other: Self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_add(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_sub(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_mul(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_div(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_add(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_sub(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_mul(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_div(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_rem(self, other: Self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_add(self, other: Self) -> (Self, bool);
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_sub(self, other: Self) -> (Self, bool);
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_mul(self, other: Self) -> (Self, bool);
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_div(self, other: Self) -> (Self, bool);
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_rem(self, other: Self) -> (Self, bool);

    /// Whether the value is below zero.
    #[doc(hidden)]
    #[must_use]
    fn is_negative(self) -> bool;
    /// Whether the value is negative, and its magnitude.
    #[doc(hidden)]
    #[must_use]
    fn sign_and_magnitude(self) -> (bool, u128);
    /// The value of a sign and a magnitude, or `None` past the range.
    #[doc(hidden)]
    fn from_magnitude(negative: bool, magnitude: u128) -> Option<Self>;

    /// `self × 10^k`.
    #[doc(hidden)]
    fn scale_up(self, k: u8) -> Outcome<Self>;
    /// `self / 10^k`, rounded by `table`, and whether nothing was rounded away: never past the
    /// range, since a quotient by ten or more is smaller than its dividend.
    #[doc(hidden)]
    #[must_use]
    fn scale_down(self, k: u8, table: u16) -> (Self, bool);
    /// `self × rhs / 10^k`, rounded by `table`.
    #[doc(hidden)]
    fn mul_down(self, rhs: Self, k: u8, table: u16) -> Outcome<Self>;
    /// `self × rhs × 10^k`, exactly.
    #[doc(hidden)]
    fn mul_up(self, rhs: Self, k: u8) -> Outcome<Self>;
    /// `self × 10^k / rhs`, rounded by `table`, for an `rhs` that is not zero.
    #[doc(hidden)]
    fn div_up(self, k: u8, rhs: Self, table: u16) -> Outcome<Self>;
    /// `self / (rhs × 10^k)`, rounded by `table`, for an `rhs` that is not zero.
    #[doc(hidden)]
    fn div_down(self, rhs: Self, k: u8, table: u16) -> Outcome<Self>;
    /// `self × b / c`, rounded by `table`, for a `c` that is not zero.
    #[doc(hidden)]
    fn mul_div(self, b: Self, c: Self, table: u16) -> Outcome<Self>;
    /// The multiple of `step` that `self` rounds to by `table`, `step`'s sign ignored, for a
    /// `step` that is not zero.
    #[doc(hidden)]
    fn multiple(self, step: Self, table: u16) -> Outcome<Self>;
    /// `±self × 10^k ± rhs`, each negated when its flag says so, exactly: two values lined up at
    /// `rhs`'s scale, and added or subtracted.
    #[doc(hidden)]
    fn lined_up_add(self, negate: bool, k: u8, rhs: Self, negate_rhs: bool) -> Outcome<Self>;
    /// The order of `self × 10^k` and `rhs`.
    #[doc(hidden)]
    #[must_use]
    fn lined_up_cmp(self, k: u8, rhs: Self) -> Ordering;
    /// The remainder of `self × 10^ka` by `rhs × 10^kb`, for an `rhs` that is not zero and at
    /// most one of the powers above `10^0`: never past the range, and of `self`'s sign.
    #[doc(hidden)]
    #[must_use]
    fn lined_up_rem(self, ka: u8, rhs: Self, kb: u8) -> Self;
}

/// A signed [`Int`]: `i8` to `i128`, each with its unsigned twin.
pub const trait Signed: [const] Int + [const] Neg<Output = Self> {
    /// The unsigned integer of the same width.
    type Unsigned: Int;

    /// As the integer's own.
    #[doc(hidden)]
    fn checked_neg(self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    fn checked_abs(self) -> Option<Self>;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_neg(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn saturating_abs(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_neg(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn wrapping_abs(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_neg(self) -> (Self, bool);
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn overflowing_abs(self) -> (Self, bool);
    /// As the integer's own: with overflow checks on, the minimum panics, and wraps otherwise.
    #[doc(hidden)]
    #[must_use]
    #[track_caller]
    fn abs(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn signum(self) -> Self;
    /// As the integer's own.
    #[doc(hidden)]
    #[must_use]
    fn unsigned_abs(self) -> Self::Unsigned;
}

/// How an integer splits into a sign and a magnitude, and is made of them again: what the kernels
/// need of it.
const trait Magnitude: Copy {
    /// Its magnitude's word.
    type Unsigned: [const] Narrow;
    /// The word twice as wide, which holds a product of two magnitudes.
    type Double: [const] Double<Self::Unsigned>;

    /// Whether the value is below zero, and its magnitude.
    fn split(self) -> (bool, Self::Unsigned);
    /// The value of a sign and a magnitude, or `None` past the range.
    fn join(negative: bool, magnitude: Self::Unsigned) -> Option<Self>;
    /// The value of a sign and a magnitude, modulo the range.
    fn wrapping_join(negative: bool, magnitude: Self::Unsigned) -> Self;
}

/// An exact result, as what it came to for `I`.
#[inline]
const fn outcome<I: [const] Magnitude, D: [const] Double<I::Unsigned>>(
    exact: Exact<D>,
) -> Outcome<I> {
    let negative = exact.negative;
    let (magnitude, beyond) = match exact.magnitude.narrow() {
        Some(magnitude) => (magnitude, false),
        None => (exact.magnitude.wrapping_narrow(), true),
    };
    match (beyond, I::join(negative, magnitude)) {
        (false, Some(value)) => Outcome { wrapped: value, past: false, negative },
        _ => Outcome { wrapped: I::wrapping_join(negative, magnitude), past: true, negative },
    }
}

/// What a kernel came to for `I`: run in `I`'s double word, and again in a [`U256`] when its
/// result outgrows that. `None` only when it outgrows even the U256.
macro_rules! exact {
    ($i:ty, $kernel:ident($($argument:expr),* $(,)?)) => {
        match kernel::$kernel::<<$i as Magnitude>::Unsigned, <$i as Magnitude>::Double>($($argument),*) {
            Some(exact) => Some(outcome::<$i, <$i as Magnitude>::Double>(exact)),
            None => match kernel::$kernel::<<$i as Magnitude>::Unsigned, U256>($($argument),*) {
                Some(exact) => Some(outcome::<$i, U256>(exact)),
                None => None,
            },
        }
    };
}

/// What a kernel that never outgrows a [`U256`] came to; were it ever to, it reports the result
/// past the range rather than a wrong value.
#[inline]
const fn settled<I: [const] Magnitude + [const] Int>(
    outcome: Option<Outcome<I>>, negative: bool,
) -> Outcome<I> {
    match outcome {
        Some(outcome) => outcome,
        None => Outcome { wrapped: I::ZERO, past: true, negative },
    }
}

/// `a × 10^k`, for any integer.
#[inline]
const fn scale_up<I: [const] Magnitude + [const] Int>(a: I, k: u8) -> Outcome<I> {
    let (negative, a) = a.split();
    settled(exact!(I, scale_up(negative, a, k)), negative)
}

/// `a / 10^k`, rounded, and whether it was exact, for any integer.
#[inline]
const fn scale_down<I: [const] Magnitude + [const] Int>(a: I, k: u8, table: u16) -> (I, bool) {
    let (negative, a) = a.split();
    let (exact, whole) = match kernel::scale_down::<I::Unsigned, I::Double>(negative, a, k, table) {
        Some((exact, whole)) => (outcome::<I, I::Double>(exact), whole),
        None => match kernel::scale_down::<I::Unsigned, U256>(negative, a, k, table) {
            Some((exact, whole)) => (outcome::<I, U256>(exact), whole),
            None => (Outcome::fits(I::ZERO), a == I::Unsigned::ZERO),
        },
    };
    (exact.wrapped, whole)
}

/// `a × b / 10^k`, rounded, for any integer.
#[inline]
const fn mul_down<I: [const] Magnitude + [const] Int>(a: I, b: I, k: u8, table: u16) -> Outcome<I> {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    let negative = na != nb;
    settled(exact!(I, mul_down(negative, a, b, k, table)), negative)
}

/// The multiple of `step` that `a` rounds to, for any integer and a `step` that is not zero.
#[inline]
const fn multiple<I: [const] Magnitude>(a: I, step: I, table: u16) -> Outcome<I> {
    let ((negative, a), (_, step)) = (a.split(), step.split());
    outcome::<I, I::Double>(kernel::multiple::<I::Unsigned, I::Double>(negative, a, step, table))
}

/// `a × b × 10^k`, exactly, for any integer: past even a [`U256`], its low bits by wrapping
/// arithmetic, which keeps them exactly.
#[inline]
const fn mul_up<I: [const] Magnitude + [const] Int>(a: I, b: I, k: u8) -> Outcome<I> {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    let negative = na != nb;
    if let Some(outcome) = exact!(I, mul_up(negative, a, b, k)) {
        return outcome;
    }
    let mut power = 1_u128;
    let mut left = k;
    while left > 0 {
        power = power.wrapping_mul(10);
        left = left.wrapping_sub(1);
    }
    let low = I::Unsigned::truncate(a.to_u128().wrapping_mul(b.to_u128()).wrapping_mul(power));
    Outcome { wrapped: I::wrapping_join(negative, low), past: true, negative }
}

/// `a × 10^k / b`, rounded, for any integer and a `b` that is not zero.
#[inline]
const fn div_up<I: [const] Magnitude + [const] Int>(a: I, k: u8, b: I, table: u16) -> Outcome<I> {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    let negative = na != nb;
    settled(exact!(I, div_up(negative, a, k, b, table)), negative)
}

/// `a / (b × 10^k)`, rounded, for any integer and a `b` that is not zero.
#[inline]
const fn div_down<I: [const] Magnitude + [const] Int>(a: I, b: I, k: u8, table: u16) -> Outcome<I> {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    let negative = na != nb;
    settled(exact!(I, div_down(negative, a, b, k, table)), negative)
}

/// `a × b / c`, rounded, for any integer and a `c` that is not zero.
#[inline]
const fn mul_div<I: [const] Magnitude>(a: I, b: I, c: I, table: u16) -> Outcome<I> {
    let ((na, a), (nb, b), (nc, c)) = (a.split(), b.split(), c.split());
    let negative = (na != nb) != nc;
    outcome::<I, I::Double>(kernel::mul_div::<I::Unsigned, I::Double>(negative, a, b, c, table))
}

/// `a × 10^k` and `b` as exact results in the word `D`, or `None` when the first outgrows it.
#[inline]
const fn lined_up<U: [const] Narrow, D: [const] Double<U>>(
    na: bool, a: U, k: u8, nb: bool, b: U,
) -> Option<(Exact<D>, Exact<D>)> {
    match kernel::scale_up::<U, D>(na, a, k) {
        Some(x) => Some((x, Exact { negative: nb, magnitude: D::from_narrow(b) })),
        None => None,
    }
}

/// `a × 10^k + b`, exactly, in the word `D`.
#[inline]
const fn lined_up_sum<U: [const] Narrow, D: [const] Double<U>>(
    na: bool, a: U, k: u8, nb: bool, b: U,
) -> Option<Exact<D>> {
    match lined_up::<U, D>(na, a, k, nb, b) {
        Some((lifted, other)) => kernel::add(lifted, other),
        None => None,
    }
}

/// `±a × 10^k ± b`, exactly, for any integer.
#[inline]
const fn lined_up_add<I: [const] Magnitude + [const] Int>(
    a: I, negate: bool, k: u8, b: I, negate_b: bool,
) -> Outcome<I> {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    let (na, nb) = (na != negate, nb != negate_b);
    let sum = match lined_up_sum::<I::Unsigned, I::Double>(na, a, k, nb, b) {
        Some(exact) => Some(outcome::<I, I::Double>(exact)),
        None => match lined_up_sum::<I::Unsigned, U256>(na, a, k, nb, b) {
            Some(exact) => Some(outcome::<I, U256>(exact)),
            None => None,
        },
    };
    settled(sum, na)
}

/// The order of `a × 10^k` and `b`, for any integer: a lined-up value past even a [`U256`] is
/// past every other, and its sign decides.
#[inline]
const fn lined_up_cmp<I: [const] Magnitude>(a: I, k: u8, b: I) -> Ordering {
    let ((na, a), (nb, b)) = (a.split(), b.split());
    match lined_up::<I::Unsigned, I::Double>(na, a, k, nb, b) {
        Some((lifted, other)) => kernel::compare(lifted, other),
        None => match lined_up::<I::Unsigned, U256>(na, a, k, nb, b) {
            Some((lifted, other)) => kernel::compare(lifted, other),
            None if na => Ordering::Less,
            None => Ordering::Greater,
        },
    }
}

/// The remainder of `a × 10^ka` by `b × 10^kb`, for any integer and a `b` that is not zero.
#[inline]
const fn lined_up_rem<I: [const] Magnitude>(a: I, ka: u8, b: I, kb: u8) -> I {
    let ((negative, a), (_, b)) = (a.split(), b.split());
    // Lined up in a U256, where 10^38 times any magnitude fits; the remainder is below the
    // smaller of the two, which fits the integer.
    let (Some(dividend), Some(divisor)) = (
        kernel::scale_up::<I::Unsigned, U256>(false, a, ka),
        kernel::scale_up::<I::Unsigned, U256>(false, b, kb),
    ) else {
        return I::wrapping_join(negative, I::Unsigned::ZERO);
    };
    let (_, remainder) = dividend.magnitude.div_rem(divisor.magnitude);
    I::wrapping_join(negative, Double::<I::Unsigned>::wrapping_narrow(remainder))
}

/// The methods every integer shares, forwarded to its own, and its kernels.
macro_rules! int {
    ($($t:ty => $unsigned:ty, $double:ty, $digits:literal;)*) => {$(
        impl sealed::Sealed for $t {}

        const impl Int for $t {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;
            const DIGITS: u8 = $digits;

            #[inline]
            fn checked_add(self, other: Self) -> Option<Self> { <$t>::checked_add(self, other) }
            #[inline]
            fn checked_sub(self, other: Self) -> Option<Self> { <$t>::checked_sub(self, other) }
            #[inline]
            fn checked_mul(self, other: Self) -> Option<Self> { <$t>::checked_mul(self, other) }
            #[inline]
            fn checked_div(self, other: Self) -> Option<Self> { <$t>::checked_div(self, other) }
            #[inline]
            fn checked_rem(self, other: Self) -> Option<Self> { <$t>::checked_rem(self, other) }
            #[inline]
            fn saturating_add(self, other: Self) -> Self { <$t>::saturating_add(self, other) }
            #[inline]
            fn saturating_sub(self, other: Self) -> Self { <$t>::saturating_sub(self, other) }
            #[inline]
            fn saturating_mul(self, other: Self) -> Self { <$t>::saturating_mul(self, other) }
            #[inline]
            #[track_caller]
            fn saturating_div(self, other: Self) -> Self { <$t>::saturating_div(self, other) }
            #[inline]
            fn wrapping_add(self, other: Self) -> Self { <$t>::wrapping_add(self, other) }
            #[inline]
            fn wrapping_sub(self, other: Self) -> Self { <$t>::wrapping_sub(self, other) }
            #[inline]
            fn wrapping_mul(self, other: Self) -> Self { <$t>::wrapping_mul(self, other) }
            #[inline]
            #[track_caller]
            fn wrapping_div(self, other: Self) -> Self { <$t>::wrapping_div(self, other) }
            #[inline]
            #[track_caller]
            fn wrapping_rem(self, other: Self) -> Self { <$t>::wrapping_rem(self, other) }
            #[inline]
            fn overflowing_add(self, other: Self) -> (Self, bool) { <$t>::overflowing_add(self, other) }
            #[inline]
            fn overflowing_sub(self, other: Self) -> (Self, bool) { <$t>::overflowing_sub(self, other) }
            #[inline]
            fn overflowing_mul(self, other: Self) -> (Self, bool) { <$t>::overflowing_mul(self, other) }
            #[inline]
            #[track_caller]
            fn overflowing_div(self, other: Self) -> (Self, bool) { <$t>::overflowing_div(self, other) }
            #[inline]
            #[track_caller]
            fn overflowing_rem(self, other: Self) -> (Self, bool) { <$t>::overflowing_rem(self, other) }

            #[inline]
            fn is_negative(self) -> bool {
                self.split().0
            }

            #[inline]
            fn sign_and_magnitude(self) -> (bool, u128) {
                let (negative, magnitude) = self.split();
                (negative, magnitude.to_u128())
            }

            #[inline]
            fn from_magnitude(negative: bool, magnitude: u128) -> Option<Self> {
                match <$unsigned>::try_from(magnitude) {
                    Ok(magnitude) => Self::join(negative, magnitude),
                    Err(_) => None,
                }
            }

            #[inline]
            fn scale_up(self, k: u8) -> Outcome<Self> { scale_up(self, k) }
            #[inline]
            fn scale_down(self, k: u8, table: u16) -> (Self, bool) { scale_down(self, k, table) }
            #[inline]
            fn mul_down(self, rhs: Self, k: u8, table: u16) -> Outcome<Self> { mul_down(self, rhs, k, table) }
            #[inline]
            fn mul_up(self, rhs: Self, k: u8) -> Outcome<Self> { mul_up(self, rhs, k) }
            #[inline]
            fn div_up(self, k: u8, rhs: Self, table: u16) -> Outcome<Self> { div_up(self, k, rhs, table) }
            #[inline]
            fn div_down(self, rhs: Self, k: u8, table: u16) -> Outcome<Self> { div_down(self, rhs, k, table) }
            #[inline]
            fn mul_div(self, b: Self, c: Self, table: u16) -> Outcome<Self> { mul_div(self, b, c, table) }
            #[inline]
            fn multiple(self, step: Self, table: u16) -> Outcome<Self> { multiple(self, step, table) }
            #[inline]
            fn lined_up_add(self, negate: bool, k: u8, rhs: Self, negate_rhs: bool) -> Outcome<Self> {
                lined_up_add(self, negate, k, rhs, negate_rhs)
            }
            #[inline]
            fn lined_up_cmp(self, k: u8, rhs: Self) -> Ordering { lined_up_cmp(self, k, rhs) }
            #[inline]
            fn lined_up_rem(self, ka: u8, rhs: Self, kb: u8) -> Self { lined_up_rem(self, ka, rhs, kb) }
        }
    )*};
}

/// A signed integer's sign and magnitude.
macro_rules! signed {
    ($($t:ty => $unsigned:ty, $double:ty;)*) => {$(
        const impl Magnitude for $t {
            type Unsigned = $unsigned;
            type Double = $double;

            #[inline]
            fn split(self) -> (bool, $unsigned) {
                (self < 0, self.unsigned_abs())
            }

            #[inline]
            fn join(negative: bool, magnitude: $unsigned) -> Option<Self> {
                if negative {
                    if magnitude <= <$t>::MIN.unsigned_abs() {
                        Some(magnitude.cast_signed().wrapping_neg())
                    } else {
                        None
                    }
                } else if magnitude <= <$t>::MAX.cast_unsigned() {
                    Some(magnitude.cast_signed())
                } else {
                    None
                }
            }

            #[inline]
            fn wrapping_join(negative: bool, magnitude: $unsigned) -> Self {
                let value = magnitude.cast_signed();
                if negative { value.wrapping_neg() } else { value }
            }
        }

        const impl Signed for $t {
            type Unsigned = $unsigned;

            #[inline]
            fn checked_neg(self) -> Option<Self> { <$t>::checked_neg(self) }
            #[inline]
            fn checked_abs(self) -> Option<Self> { <$t>::checked_abs(self) }
            #[inline]
            fn saturating_neg(self) -> Self { <$t>::saturating_neg(self) }
            #[inline]
            fn saturating_abs(self) -> Self { <$t>::saturating_abs(self) }
            #[inline]
            fn wrapping_neg(self) -> Self { <$t>::wrapping_neg(self) }
            #[inline]
            fn wrapping_abs(self) -> Self { <$t>::wrapping_abs(self) }
            #[inline]
            fn overflowing_neg(self) -> (Self, bool) { <$t>::overflowing_neg(self) }
            #[inline]
            fn overflowing_abs(self) -> (Self, bool) { <$t>::overflowing_abs(self) }
            #[inline]
            #[track_caller]
            fn abs(self) -> Self { <$t>::abs(self) }
            #[inline]
            fn signum(self) -> Self { <$t>::signum(self) }
            #[inline]
            fn unsigned_abs(self) -> $unsigned { <$t>::unsigned_abs(self) }
        }
    )*};
}

/// An unsigned integer's sign, always positive, and magnitude, itself.
macro_rules! unsigned {
    ($($t:ty => $double:ty;)*) => {$(
        const impl Magnitude for $t {
            type Unsigned = $t;
            type Double = $double;

            #[inline]
            fn split(self) -> (bool, $t) {
                (false, self)
            }

            #[inline]
            fn join(negative: bool, magnitude: $t) -> Option<Self> {
                if negative && magnitude != 0 { None } else { Some(magnitude) }
            }

            #[inline]
            fn wrapping_join(negative: bool, magnitude: $t) -> Self {
                if negative { magnitude.wrapping_neg() } else { magnitude }
            }
        }
    )*};
}

int! {
    i8 => u8, u16, 2;
    i16 => u16, u32, 4;
    i32 => u32, u64, 9;
    i64 => u64, u128, 18;
    i128 => u128, U256, 38;
    u8 => u8, u16, 2;
    u16 => u16, u32, 4;
    u32 => u32, u64, 9;
    u64 => u64, u128, 19;
    u128 => u128, U256, 38;
}

signed! {
    i8 => u8, u16;
    i16 => u16, u32;
    i32 => u32, u64;
    i64 => u64, u128;
    i128 => u128, U256;
}

unsigned! {
    u8 => u16;
    u16 => u32;
    u32 => u64;
    u64 => u128;
    u128 => U256;
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the reference model's own arithmetic, in a type wide enough"
)]
mod tests {
    use core::cmp::Ordering;

    use proptest::prelude::*;

    use super::Int;

    /// Half to even's table.
    const HALF_EVEN: u16 = 0xC8C8;
    /// Truncation's table.
    const TRUNC: u16 = 0x0000;

    proptest! {
        #[test]
        fn a_kernel_on_a_narrow_integer_agrees_with_a_wide_one(a: i32, b: i32, c: i32, k in 0_u8..12) {
            // The i32 kernels against the same operation carried out in an i128 and narrowed.
            let narrow = |x: i128| i32::try_from(x).ok();
            let (a128, b128, c128, power) = (i128::from(a), i128::from(b), i128::from(c), 10_i128.pow(u32::from(k)));
            prop_assert_eq!(a.scale_up(k).checked(), narrow(a128 * power));
            prop_assert_eq!(a.mul_down(b, k, TRUNC).checked(), narrow(a128 * b128 / power));
            prop_assert_eq!(a.mul_up(b, k).checked(), narrow(a128 * b128 * power));
            if b != 0 {
                prop_assert_eq!(a.div_up(k, b, TRUNC).checked(), narrow(a128 * power / b128));
                prop_assert_eq!(a.div_down(b, k, TRUNC).checked(), narrow(a128 / (b128 * power)));
            }
            if c != 0 {
                prop_assert_eq!(a.mul_div(b, c, TRUNC).checked(), narrow(a128 * b128 / c128));
            }
            prop_assert_eq!(a.lined_up_add(false, k, b, false).checked(), narrow(a128 * power + b128));
            prop_assert_eq!(a.lined_up_add(true, k, b, false).checked(), narrow(-a128 * power + b128));
            prop_assert_eq!(a.lined_up_add(false, k, b, true).checked(), narrow(a128 * power - b128));
            prop_assert_eq!(a.lined_up_cmp(k, b), (a128 * power).cmp(&b128));
        }

        #[test]
        fn a_wrapped_result_keeps_the_low_bits(a: i32, b: i32, k in 0_u8..12) {
            let (a128, b128, power) = (i128::from(a), i128::from(b), 10_i128.pow(u32::from(k)));
            #[expect(clippy::as_conversions, clippy::cast_possible_truncation, reason = "the low bits, as wrapping keeps")]
            let low = |x: i128| x as i32;
            prop_assert_eq!(a.scale_up(k).wrapping(), low(a128 * power));
            prop_assert_eq!(a.lined_up_add(false, k, b, false).wrapping(), low(a128 * power + b128));
            prop_assert_eq!(a.mul_up(b, k).wrapping(), low(a128 * b128 * power));
        }

        #[test]
        fn a_rounded_quotient_moves_by_one_step_at_most(a: i64, b in 1_i64..) {
            let (truncated, rounded) = (a.div_down(b, 0, TRUNC).checked(), a.div_down(b, 0, HALF_EVEN).checked());
            if let (Some(t), Some(r)) = (truncated, rounded) {
                prop_assert!(t.abs_diff(r) <= 1, "{t} and {r}");
            }
        }

        #[test]
        fn a_lined_up_remainder_is_the_wide_one(a: i64, b in 1_i64.., k in 0_u8..=19) {
            let power = 10_i128.pow(u32::from(k));
            prop_assert_eq!(i128::from(a.lined_up_rem(k, b, 0)), i128::from(a) * power % i128::from(b));
            prop_assert_eq!(i128::from(a.lined_up_rem(0, b, k)), i128::from(a) % (i128::from(b) * power));
        }
    }

    #[test]
    fn a_128_bit_product_is_carried_in_256_bits() {
        let big = 10_i128.pow(25);
        assert_eq!(big.mul_down(big, 18, TRUNC).checked(), Some(10_i128.pow(32)), "10^50 / 10^18");
        assert_eq!(big.mul_down(big, 10, TRUNC).checked(), None, "10^40 is past an i128");
        assert_eq!(
            i128::MIN.mul_down(1, 0, TRUNC).checked(),
            Some(i128::MIN),
            "the minimum, unchanged"
        );
        assert_eq!(
            u128::MAX.mul_div(u128::MAX, u128::MAX, TRUNC).checked(),
            Some(u128::MAX),
            "MAX · MAX / MAX"
        );
    }

    #[test]
    fn a_lined_up_value_past_every_word_is_past_every_value() {
        assert_eq!(1_i128.lined_up_cmp(38, i128::MAX), Ordering::Less, "10^38 below 1.7 · 10^38");
        assert_eq!(2_i128.lined_up_cmp(38, i128::MAX), Ordering::Greater, "2 · 10^38 above it");
        assert_eq!((-2_i128).lined_up_cmp(38, i128::MIN), Ordering::Less, "and below the minimum");
        assert_eq!(u8::MAX.scale_down(38, TRUNC), (0, false), "a far power truncates to zero");
    }

    #[test]
    fn a_saturated_result_holds_at_the_end_it_passes() {
        assert_eq!(
            i8::MAX.lined_up_add(false, 1, 0, false).saturating(),
            i8::MAX,
            "1270 holds at 127"
        );
        assert_eq!(
            i8::MIN.lined_up_add(false, 1, 0, false).saturating(),
            i8::MIN,
            "-1280 holds at -128"
        );
        assert_eq!(0_u8.lined_up_add(false, 0, 0, false).checked(), Some(0), "zero fits");
        assert_eq!(
            i8::MIN.lined_up_add(false, 0, 0, false).checked(),
            Some(i8::MIN),
            "the minimum fits"
        );
        assert_eq!(1_u8.lined_up_add(true, 1, 3, false).checked(), None, "3 - 10 is below a u8");
        assert_eq!(1_u8.lined_up_add(true, 1, 3, false).saturating(), 0, "and holds at zero");
    }
}
