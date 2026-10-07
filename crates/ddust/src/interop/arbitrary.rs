//! arbitrary's `Arbitrary`, for fuzzing: any steps at any scale, and any mode.

use arbitrary::{Arbitrary, Result, Unstructured, size_hint};

use crate::round::Rounding;
use crate::scale::MAX_DECIMALS;
use crate::{Decimal, Dynamic, Fixed, Int, Scale};

/// Any steps, at any scale its scale type takes.
impl<'a, I: Int + Arbitrary<'a>, S: Scale + Arbitrary<'a>> Arbitrary<'a> for Decimal<I, S> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self::from_steps(I::arbitrary(u)?, S::arbitrary(u)?))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::and(I::size_hint(depth), S::size_hint(depth))
    }
}

/// The one scale, from no byte.
impl<const D: u8> Arbitrary<'_> for Fixed<D> {
    fn arbitrary(_u: &mut Unstructured<'_>) -> Result<Self> {
        Ok(Self)
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

/// Any scale, from 0 to 38 decimals.
impl Arbitrary<'_> for Dynamic {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        Ok(Self::new(u.int_in_range(0..=MAX_DECIMALS)?).unwrap_or_default())
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}

/// Any of the nine modes.
impl Arbitrary<'_> for Rounding {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        u.choose(Self::ALL).copied()
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(4))
    }
}

#[cfg(test)]
mod tests {
    use arbitrary::{Arbitrary as _, Unstructured};

    use crate::round::Rounding;
    use crate::{Dynamic, Scale as _};

    #[test]
    fn every_scale_is_drawn_and_no_other() {
        let mut seen = [false; 39];
        for byte in 0..=u8::MAX {
            let scale = Dynamic::arbitrary(&mut Unstructured::new(&[byte])).expect("one byte");
            *seen.get_mut(usize::from(scale.decimals())).expect("at most 38 decimals") = true;
        }
        assert!(seen.iter().all(|&drawn| drawn), "each of 0 to 38 decimals");
    }

    #[test]
    fn each_byte_draws_the_mode_at_its_place() {
        for (byte, &mode) in (0_u8..).zip(Rounding::ALL) {
            let drawn = Rounding::arbitrary(&mut Unstructured::new(&[byte])).expect("one byte");
            assert_eq!(drawn, mode, "the mode at place {byte}");
        }
    }
}
