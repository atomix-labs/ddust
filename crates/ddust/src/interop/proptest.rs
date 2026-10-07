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
        select(Self::MODES)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use proptest::prelude::*;
    use proptest::strategy::ValueTree as _;
    use proptest::test_runner::TestRunner;

    use crate::round::Rounding;
    use crate::{Decimal, Dynamic};

    proptest! {
        #[test]
        fn a_drawn_value_reads_back_from_its_text(value: Decimal<i64, Dynamic>) {
            let read: Decimal<i64, Dynamic> = format!("{value:#}").parse().expect("its own text");
            prop_assert_eq!((read.steps(), read.decimals()), (value.steps(), value.decimals()));
        }
    }

    #[test]
    fn every_mode_is_drawn() {
        let mut runner = TestRunner::deterministic();
        let strategy = any::<Rounding>();
        let mut seen = [false; 9];
        for _ in 0..200 {
            let mode = strategy.new_tree(&mut runner).expect("a mode").current();
            let place =
                Rounding::MODES.iter().position(|&each| each == mode).expect("one of the nine");
            *seen.get_mut(place).expect("nine places") = true;
        }
        assert!(seen.iter().all(|&drawn| drawn), "each of the nine modes, in 200 draws");
    }
}
