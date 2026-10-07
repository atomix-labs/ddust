//! proptest's `Arbitrary`, for property tests: any steps at any scale, and any mode.

use core::ops::RangeInclusive;

use proptest::arbitrary::{Arbitrary, any_with};
use proptest::sample::{Select, select};
use proptest::strategy::{Just, Map, Strategy};

use crate::round::Rounding;
use crate::scale::MAX_DECIMALS;
use crate::{Decimal, Dynamic, Fixed, Int, Scale};

/// Any steps, at any scale its scale type takes.
impl<I: Int + Arbitrary, S: Scale + Arbitrary> Arbitrary for Decimal<I, S> {
    type Parameters = (I::Parameters, S::Parameters);
    type Strategy = Map<(I::Strategy, S::Strategy), fn((I, S)) -> Self>;

    fn arbitrary_with((steps, scale): Self::Parameters) -> Self::Strategy {
        (any_with::<I>(steps), any_with::<S>(scale))
            .prop_map(|(steps, scale)| Self::from_steps(steps, scale))
    }
}

/// The one scale.
impl<const D: u8> Arbitrary for Fixed<D> {
    type Parameters = ();
    type Strategy = Just<Self>;

    fn arbitrary_with((): ()) -> Just<Self> {
        Just(Self)
    }
}

/// Any scale, from 0 to 38 decimals.
impl Arbitrary for Dynamic {
    type Parameters = ();
    type Strategy = Map<RangeInclusive<u8>, fn(u8) -> Self>;

    fn arbitrary_with((): ()) -> Self::Strategy {
        (0..=MAX_DECIMALS).prop_map(|decimals| Self::new(decimals).unwrap_or_default())
    }
}

/// Any of the nine modes.
impl Arbitrary for Rounding {
    type Parameters = ();
    type Strategy = Select<Self>;

    fn arbitrary_with((): ()) -> Select<Self> {
        select(Self::ALL)
    }
}
