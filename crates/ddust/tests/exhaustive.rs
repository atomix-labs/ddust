//! Every pair of 8-bit values, signed and unsigned, through every operation and every rounding
//! mode, against an exact reference: integer arithmetic in an `i128`, and each mode by its
//! definition rather than its table; and random pairs at 32 and 64 bits, where an `i128` is still
//! exact.

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, reason = "the reference's own arithmetic, in an i128")]
mod tests {
    use ddust::round::Rounding;
    use ddust::{D8, Decimal, Dynamic, Fixed, Int, UD8};
    use proptest::prelude::*;
    use proptest::sample::select;

    /// Every rounding mode.
    const MODES: [Rounding; 9] = [
        Rounding::Floor,
        Rounding::Ceil,
        Rounding::Trunc,
        Rounding::Expand,
        Rounding::HalfFloor,
        Rounding::HalfCeil,
        Rounding::HalfTrunc,
        Rounding::HalfExpand,
        Rounding::HalfEven,
    ];

    /// `n / d` rounded by `mode`, by its definition, for a `d` that is not zero.
    fn divide(n: i128, d: i128, mode: Rounding) -> i128 {
        let (q, r) = (n / d, n % d);
        if r == 0 {
            return q;
        }
        // The exact quotient lies strictly between q and q + step, where step is its sign.
        let negative = (n < 0) != (d < 0);
        let step = if negative { -1 } else { 1 };
        let (twice, whole) = ((2 * r).abs(), d.abs());
        let away = match mode {
            Rounding::Floor => negative,
            Rounding::Ceil => !negative,
            Rounding::Trunc => false,
            Rounding::Expand => true,
            Rounding::HalfFloor => twice > whole || (twice == whole && negative),
            Rounding::HalfCeil => twice > whole || (twice == whole && !negative),
            Rounding::HalfTrunc => twice > whole,
            Rounding::HalfExpand => twice >= whole,
            Rounding::HalfEven => twice > whole || (twice == whole && q % 2 != 0),
        };
        if away { q + step } else { q }
    }

    /// `10^k`.
    const fn pow10(k: u8) -> i128 {
        let mut power = 1;
        let mut left = k;
        while left > 0 {
            power *= 10;
            left -= 1;
        }
        power
    }

    /// The value as the integer `I`, or `None` past its range.
    fn fit<I: Int + TryFrom<i128>>(value: i128) -> Option<I> {
        I::try_from(value).ok()
    }

    /// Every operation on every pair of `I` values at `a` and `b` decimals, results at `a`'s scale
    /// or `to`, against the reference.
    fn check_pairs<I>(values: &[I], a: u8, b: u8, to: u8)
    where
        I: Int + Into<i128> + TryFrom<i128>,
    {
        for &x in values {
            for &y in values {
                check_exact(x, y, a, b);
                check_rounded(x, y, a, b, to);
            }
        }
    }

    /// The exact operations on `x` at `a` decimals and `y` at `b`: sums and differences in each
    /// family, the order, the remainder and the product.
    fn check_exact<I>(x: I, y: I, a: u8, b: u8)
    where
        I: Int + Into<i128> + TryFrom<i128>,
    {
        let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
        let (left, right) = (Decimal::from_steps(x, scale(a)), Decimal::from_steps(y, scale(b)));
        let (xi, yi): (i128, i128) = (x.into(), y.into());
        // Lined up at the finer scale, exactly: at one scale, the integer's own arithmetic.
        let finer = a.max(b);
        let (xl, yl) = (xi * pow10(finer - a), yi * pow10(finer - b));
        assert_eq!(
            left.checked_add(right).map(Decimal::steps),
            fit::<I>(xl + yl),
            "{xi}@{a} + {yi}@{b}"
        );
        assert_eq!(
            left.checked_sub(right).map(Decimal::steps),
            fit::<I>(xl - yl),
            "{xi}@{a} - {yi}@{b}"
        );
        assert_eq!(left.cmp(&right), xl.cmp(&yl), "{xi}@{a} vs {yi}@{b}");
        // The families, which read the exact sum's sign and range.
        for (exact, saturated, (wrapped, flagged)) in [
            (xl + yl, left.saturating_add(right), left.overflowing_add(right)),
            (xl - yl, left.saturating_sub(right), left.overflowing_sub(right)),
        ] {
            let held = if exact < 0 { I::MIN } else { I::MAX };
            assert_eq!(
                saturated.steps(),
                fit::<I>(exact).unwrap_or(held),
                "{xi}@{a}, {yi}@{b}: held"
            );
            assert_eq!(flagged, fit::<I>(exact).is_none(), "{xi}@{a}, {yi}@{b}: flagged");
            let modulus = 1_i128 << (8 * size_of::<I>());
            let low: i128 = wrapped.steps().into();
            assert_eq!(
                low.rem_euclid(modulus),
                exact.rem_euclid(modulus),
                "{xi}@{a}, {yi}@{b}: wrapped"
            );
        }
        if yl != 0 {
            // At one scale, the integer's own: the minimum by -1 overflows its quotient.
            let overflows = a == b && x == I::MIN && yi == -1;
            let remainder = if overflows { None } else { fit::<I>(xl % yl) };
            assert_eq!(
                left.checked_rem(right).map(Decimal::steps),
                remainder,
                "{xi}@{a} % {yi}@{b}"
            );
        }
        // The exact product: steps times steps, scales summed.
        assert_eq!(left.checked_mul(right).map(Decimal::steps), fit::<I>(xi * yi), "{xi} × {yi}");
    }

    /// The rounded operations on `x` at `a` decimals and `y` at `b`, in every mode: products and
    /// quotients at `a`'s scale and at `to`, a ratio, and a count's parts.
    fn check_rounded<I>(x: I, y: I, a: u8, b: u8, to: u8)
    where
        I: Int + Into<i128> + TryFrom<i128>,
    {
        let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
        let (left, right) = (Decimal::from_steps(x, scale(a)), Decimal::from_steps(y, scale(b)));
        let (xi, yi): (i128, i128) = (x.into(), y.into());
        for mode in MODES {
            let own = divide(xi * yi, pow10(b), mode);
            assert_eq!(
                left.checked_mul_round(right, mode).map(Decimal::steps),
                fit::<I>(own),
                "{xi}@{a} × {yi}@{b} at {a}, {mode:?}"
            );
            // A product at `to` decimals: (x · y) at a + b, moved to `to`.
            let product = if a + b >= to {
                divide(xi * yi, pow10(a + b - to), mode)
            } else {
                xi * yi * pow10(to - a - b)
            };
            let got = left.checked_mul_round_to(right, scale(to), mode).map(Decimal::steps);
            assert_eq!(got, fit::<I>(product), "{xi}@{a} × {yi}@{b} at {to}, {mode:?}");
            if yi == 0 {
                continue;
            }
            let own = divide(xi * pow10(b), yi, mode);
            assert_eq!(
                left.checked_div_round(right, mode).map(Decimal::steps),
                fit::<I>(own),
                "{xi}@{a} / {yi}@{b} at {a}, {mode:?}"
            );
            // `x × y / y` with one rounding: x again.
            let ratio = Decimal::from_steps(y, scale(b));
            assert_eq!(
                left.checked_mul_div_round(ratio, ratio, mode),
                Some(left),
                "{xi} × {yi} / {yi}, {mode:?}"
            );
            // A quotient at `to` decimals: x · 10^(to + b - a) / y.
            let (up, down) = ((to + b).saturating_sub(a), a.saturating_sub(to + b));
            let quotient = divide(xi * pow10(up), yi * pow10(down), mode);
            let got = left.checked_div_round_to(right, scale(to), mode).map(Decimal::steps);
            assert_eq!(got, fit::<I>(quotient), "{xi}@{a} / {yi}@{b} at {to}, {mode:?}");
            // One of `y` parts, at `a`.
            let got = left.checked_div_int_round(y, mode).map(Decimal::steps);
            assert_eq!(got, fit::<I>(divide(xi, yi, mode)), "{xi}@{a} in {yi} parts, {mode:?}");
        }
    }

    /// Every value of `I` at `decimals` rounded, rescaled and written, against the reference.
    fn check_values<I>(values: &[I], decimals: u8)
    where
        I: Int + Into<i128> + TryFrom<i128>,
    {
        let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
        for &x in values {
            let value = Decimal::from_steps(x, scale(decimals));
            let xi: i128 = x.into();
            let (trunc, fract): (i128, i128) =
                (value.trunc().steps().into(), value.fract().steps().into());
            assert_eq!(
                (trunc, fract),
                (xi / pow10(decimals) * pow10(decimals), xi % pow10(decimals)),
                "{xi}@{decimals}'s parts"
            );
            for mode in MODES {
                let int: i128 = value.to_int(mode).into();
                assert_eq!(
                    int,
                    divide(xi, pow10(decimals), mode),
                    "{xi}@{decimals} to an integer, {mode:?}"
                );
                let whole = divide(xi, pow10(decimals), mode) * pow10(decimals);
                assert_eq!(
                    value.checked_round(mode).map(Decimal::steps),
                    fit::<I>(whole),
                    "{xi}@{decimals} round, {mode:?}"
                );
                for to in 0..=3 {
                    let moved = if to >= decimals {
                        xi * pow10(to - decimals)
                    } else {
                        divide(xi, pow10(decimals - to), mode)
                    };
                    let got = value.rescale_round(scale(to), mode).ok().map(Decimal::steps);
                    assert_eq!(got, fit::<I>(moved), "{xi}@{decimals} to {to}, {mode:?}");
                }
                for step in [1_i128, 3, 7, 25] {
                    let Some(size) = fit::<I>(step) else { continue };
                    let multiple = divide(xi, step, mode) * step;
                    let got = value
                        .checked_round_to(Decimal::from_steps(size, scale(decimals)), mode)
                        .map(Decimal::steps);
                    assert_eq!(
                        got,
                        fit::<I>(multiple),
                        "{xi}@{decimals} to a step of {step}, {mode:?}"
                    );
                }
            }
            // Text: written, then read back at the same scale.
            let text = value.to_string();
            assert_eq!(
                Decimal::<I, Dynamic>::from_ascii(text.as_bytes(), scale(decimals)),
                Ok(value),
                "{text}"
            );
            assert_eq!(
                value.to_f64(),
                text.parse::<f64>().expect("a number"),
                "{text} as a double"
            );
        }
    }

    #[test]
    fn every_pair_of_i8_values_agrees_with_the_reference() {
        let values: Vec<i8> = (i8::MIN..=i8::MAX).collect();
        for (a, b, to) in [(0, 0, 0), (1, 1, 1), (0, 2, 1), (2, 0, 3), (1, 2, 0)] {
            check_pairs(&values, a, b, to);
        }
    }

    #[test]
    fn every_pair_of_u8_values_agrees_with_the_reference() {
        let values: Vec<u8> = (u8::MIN..=u8::MAX).collect();
        for (a, b, to) in [(0, 0, 0), (1, 1, 1), (0, 2, 1), (2, 0, 3), (1, 2, 0)] {
            check_pairs(&values, a, b, to);
        }
    }

    #[test]
    fn every_8_bit_value_rounds_rescales_and_writes_as_the_reference() {
        let (signed, unsigned): (Vec<i8>, Vec<u8>) =
            ((i8::MIN..=i8::MAX).collect(), (u8::MIN..=u8::MAX).collect());
        for decimals in 0..=3 {
            check_values(&signed, decimals);
            check_values(&unsigned, decimals);
        }
    }

    #[test]
    fn a_static_scale_agrees_with_a_run_time_one() {
        for x in i8::MIN..=i8::MAX {
            for y in i8::MIN..=i8::MAX {
                let (fixed, dynamic) = (
                    D8::<1>::from_steps(x, Fixed),
                    Decimal::from_steps(x, Dynamic::new(1).expect("at most 38")),
                );
                let (other, other_dynamic) = (
                    D8::<1>::from_steps(y, Fixed),
                    Decimal::from_steps(y, Dynamic::new(1).expect("at most 38")),
                );
                assert_eq!(
                    fixed.checked_add(other).map(D8::steps),
                    dynamic.checked_add(other_dynamic).map(Decimal::steps)
                );
                assert_eq!(
                    fixed.checked_div(other).map(D8::steps),
                    dynamic.checked_div(other_dynamic).map(Decimal::steps)
                );
            }
        }
        for x in u8::MIN..=u8::MAX {
            let fixed = UD8::<2>::from_steps(x, Fixed);
            assert_eq!(
                fixed.to_string(),
                Decimal::from_steps(x, Dynamic::new(2).expect("at most 38")).to_string()
            );
        }
    }

    /// A run-time scale of at most 6 decimals, and a mode.
    fn scales_and_mode() -> impl Strategy<Value = (u8, u8, u8, Rounding)> {
        (0_u8..=6, 0_u8..=6, 0_u8..=6, select(MODES.to_vec()))
    }

    /// One pair of values with one mode, as [`check_pairs`] checks every pair: the sum, the
    /// difference and the order, the product and the quotient at `to`, and a count's parts.
    fn check_one<I>(x: I, y: I, a: u8, b: u8, to: u8, mode: Rounding) -> Result<(), TestCaseError>
    where
        I: Int + Into<i128> + TryFrom<i128>,
    {
        let scale = |decimals| Dynamic::new(decimals).expect("at most 38");
        let (left, right) = (Decimal::from_steps(x, scale(a)), Decimal::from_steps(y, scale(b)));
        let (xi, yi): (i128, i128) = (x.into(), y.into());
        let finer = a.max(b);
        let (xl, yl) = (xi * pow10(finer - a), yi * pow10(finer - b));
        prop_assert_eq!(left.checked_add(right).map(Decimal::steps), fit::<I>(xl + yl));
        prop_assert_eq!(left.checked_sub(right).map(Decimal::steps), fit::<I>(xl - yl));
        prop_assert_eq!(left.cmp(&right), xl.cmp(&yl));
        // Two u64 magnitudes may multiply past even an i128, where the reference has no answer.
        let Some(exact) = xi.checked_mul(yi) else { return Ok(()) };
        // A product past an i128 is past every narrower integer.
        let product = if a + b >= to {
            Some(divide(exact, pow10(a + b - to), mode))
        } else {
            exact.checked_mul(pow10(to - a - b))
        };
        prop_assert_eq!(
            left.checked_mul_round_to(right, scale(to), mode).map(Decimal::steps),
            product.and_then(fit::<I>)
        );
        if yi != 0 {
            let (up, down) = ((to + b).saturating_sub(a), a.saturating_sub(to + b));
            let quotient = divide(xi * pow10(up), yi * pow10(down), mode);
            prop_assert_eq!(
                left.checked_div_round_to(right, scale(to), mode).map(Decimal::steps),
                fit::<I>(quotient)
            );
            prop_assert_eq!(
                left.checked_div_int_round(y, mode).map(Decimal::steps),
                fit::<I>(divide(xi, yi, mode))
            );
        }
        Ok(())
    }

    proptest! {
        #[test]
        fn random_i32_pairs_agree_with_the_reference(x: i32, y: i32, (a, b, to, mode) in scales_and_mode()) {
            check_one(x, y, a, b, to, mode)?;
        }

        #[test]
        fn random_i64_pairs_agree_with_the_reference(x: i64, y: i64, (a, b, to, mode) in scales_and_mode()) {
            check_one(x, y, a, b, to, mode)?;
        }

        #[test]
        fn random_u64_pairs_agree_with_the_reference(x: u64, y: u64, (a, b, to, mode) in scales_and_mode()) {
            check_one(x, y, a, b, to, mode)?;
        }
    }
}
