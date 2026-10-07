//! Decimals drawn uniformly from a range, through rand's [`SampleUniform`]: every step between the
//! bounds equally likely, as the integer beneath draws its own.
//!
//! # Examples
//! ```
//! use ddust::{D64, dec};
//! use rand::{Rng as _, rng};
//!
//! let mut rng = rng();
//! let price: D64<2> = rng.random_range(dec!(99.5)..dec!(100.5));
//! assert!(dec!(99.5) <= price && price < dec!(100.5), "a cent from 99.50 to 100.49");
//! ```

use rand::Rng;
use rand::distr::uniform::{Error, SampleBorrow, SampleUniform, UniformInt, UniformSampler};

use crate::{Decimal, Int, StaticScale};

/// Draws a decimal of a static scale uniformly from a range, by drawing its steps from the
/// integer's range: what [`SampleUniform`] names for a decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniformDecimal<I, S> {
    /// The steps' sampler.
    steps: UniformInt<I>,
    /// The scale every draw is at.
    scale: S,
}

/// Through [`UniformDecimal`].
impl<I, S: StaticScale> SampleUniform for Decimal<I, S>
where
    I: Int + SampleUniform<Sampler = UniformInt<I>>,
    UniformInt<I>: UniformSampler<X = I>,
{
    type Sampler = UniformDecimal<I, S>;
}

/// The steps from the bounds', drawn as the integer's are.
impl<I, S: StaticScale> UniformSampler for UniformDecimal<I, S>
where
    I: Int + SampleUniform<Sampler = UniformInt<I>>,
    UniformInt<I>: UniformSampler<X = I>,
{
    type X = Decimal<I, S>;

    fn new<B1, B2>(low: B1, high: B2) -> Result<Self, Error>
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let steps = UniformInt::new(low.borrow().steps(), high.borrow().steps())?;
        Ok(Self { steps, scale: S::INSTANCE })
    }

    fn new_inclusive<B1, B2>(low: B1, high: B2) -> Result<Self, Error>
    where
        B1: SampleBorrow<Self::X> + Sized,
        B2: SampleBorrow<Self::X> + Sized,
    {
        let steps = UniformInt::new_inclusive(low.borrow().steps(), high.borrow().steps())?;
        Ok(Self { steps, scale: S::INSTANCE })
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::X {
        Decimal::from_steps(self.steps.sample(rng), self.scale)
    }
}

#[cfg(test)]
mod tests {
    use rand::distr::uniform::{Error, UniformSampler as _};
    use rand::{Rng as _, rng};

    use super::UniformDecimal;
    use crate::{D8, D128, Fixed, dec};

    #[test]
    fn every_draw_is_in_its_range() {
        let mut rng = rng();
        for _ in 0..1_000 {
            let tick: D8<1> = rng.random_range(dec!(-0.1)..=dec!(0.1));
            assert!(
                [dec!(-0.1), dec!(0), dec!(0.1)].contains(&tick),
                "{tick} is a tenth from -0.1 to 0.1"
            );
        }
        let wide: D128<18> = rng.random_range(D128::<18>::MIN..D128::<18>::MAX);
        assert!(wide < D128::<18>::MAX, "below the bound");
    }

    #[test]
    fn an_empty_range_is_refused() {
        let empty = UniformDecimal::<i64, Fixed<2>>::new(dec!(1), dec!(1));
        assert_eq!(empty, Err(Error::EmptyRange));
    }
}
