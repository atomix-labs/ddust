//! Every contender's result, on every input the benches time, against the oracle's.
//!
//! A row whose results all agree declares itself exact; one that differs declares so, with its
//! reason in its adapter's docs. Either declaration that does not hold fails here, so a row the
//! book marks as differing does differ, and one it shows unmarked never does. The floors do the
//! bare instruction's arithmetic, not a decimal's, and the binary `fixed` holds no decimal value,
//! so neither is checked.

#[cfg(test)]
mod tests {
    use core::fmt::{Debug, Display};

    use ddust_bench::contender::{
        Buffer, CheckedAdd, Compare, Contender, DivRound, Format, FromF64, Kind, MulExact,
        MulRound, Parse, PreparedDivRound, RescaleRound, ToF64,
    };
    use ddust_bench::input::{self, Predictability, Width};
    use ddust_bench::{for_each_contender, oracle};

    /// What one row's results came to against the oracle's.
    #[derive(Default)]
    struct Tally {
        /// The results checked.
        checked: usize,
        /// The results that differ.
        differ: usize,
        /// The first that differs, described.
        first: Option<String>,
    }

    impl Tally {
        /// Counts one result: `got` against `expected`, described by `case` if it differs.
        fn count<T: PartialEq + Debug, D: Display>(&mut self, got: &T, expected: &T, case: D) {
            self.checked = self.checked.saturating_add(1);
            if got != expected {
                self.differ = self.differ.saturating_add(1);
                self.first
                    .get_or_insert_with(|| format!("{case}: {got:?}, the oracle {expected:?}"));
            }
        }

        /// Holds the row named `name`, of `kind`, to its declaration for `operation`: what does not
        /// hold is added to `failures`.
        fn verdict(
            self, operation: &str, name: &str, width: Width, kind: Kind, exact: bool,
            failures: &mut Vec<String>,
        ) {
            if matches!(kind, Kind::Floor | Kind::Binary) {
                return;
            }
            let row = format!("{operation} {name} at {} decimals", width.decimals());
            let first = self.first.unwrap_or_default();
            if self.checked == 0 {
                failures.push(format!("{row}: nothing checked"));
            } else if exact && self.differ > 0 {
                failures.push(format!(
                    "{row}: {} of {} differ; first {first}",
                    self.differ, self.checked
                ));
            } else if !exact && self.differ == 0 {
                failures
                    .push(format!("{row}: declared as differing, yet all {} agree", self.checked));
            } else if !exact {
                println!(
                    "{row}: {} of {} differ, as declared; first {first}",
                    self.differ, self.checked
                );
            }
        }
    }

    /// The values of `steps`, or why a contender cannot hold one.
    fn values<C: Contender>(steps: &[i128]) -> Vec<C::Value> {
        steps
            .iter()
            .map(|&steps| {
                C::from_steps(steps).unwrap_or_else(|| panic!("{} cannot hold {steps}", C::NAME))
            })
            .collect()
    }

    fn add<C: CheckedAdd>(failures: &mut Vec<String>) {
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::addends(C::WIDTH, predictability);
            for ((a, b), (x, y)) in values::<C>(&set.left)
                .iter()
                .zip(&values::<C>(&set.right))
                .zip(set.left.iter().zip(&set.right))
            {
                tally.count(
                    &C::checked_add(a, b).and_then(|sum| C::to_steps(&sum)),
                    &oracle::add(*x, *y),
                    format_args!("{x} + {y}"),
                );
            }
        }
        tally.verdict("add", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn compare<C: Compare>(failures: &mut Vec<String>) {
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::addends(C::WIDTH, predictability);
            for ((a, b), (x, y)) in values::<C>(&set.left)
                .iter()
                .zip(&values::<C>(&set.right))
                .zip(set.left.iter().zip(&set.right))
            {
                tally.count(
                    &C::is_less(a, b),
                    &oracle::compare(*x, *y).is_lt(),
                    format_args!("{x} < {y}"),
                );
            }
        }
        tally.verdict("compare", C::NAME, C::WIDTH, C::KIND, true, failures);
    }

    fn mul_round<C: MulRound>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::factors(C::WIDTH, predictability);
            for ((a, b), (x, y)) in values::<C>(&set.left)
                .iter()
                .zip(&values::<C>(&set.right))
                .zip(set.left.iter().zip(&set.right))
            {
                let expected = oracle::mul_round(*x, *y, decimals, C::MODE);
                tally.count(
                    &C::checked_mul_round(a, b).and_then(|product| C::to_steps(&product)),
                    &expected,
                    format_args!("{x} × {y}"),
                );
            }
        }
        tally.verdict("mul-round", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn div_round<C: DivRound>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::dividends_and_divisors(C::WIDTH, predictability);
            for ((a, b), (x, y)) in values::<C>(&set.left)
                .iter()
                .zip(&values::<C>(&set.right))
                .zip(set.left.iter().zip(&set.right))
            {
                let expected = oracle::div_round(*x, *y, decimals, C::MODE);
                tally.count(
                    &C::checked_div_round(a, b).and_then(|quotient| C::to_steps(&quotient)),
                    &expected,
                    format_args!("{x} / {y}"),
                );
            }
        }
        tally.verdict("div-round", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn prepared_div_round<C: PreparedDivRound>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::dividends_and_divisors(C::WIDTH, predictability);
            for ((a, b), (x, y)) in values::<C>(&set.left)
                .iter()
                .zip(&values::<C>(&set.right))
                .zip(set.left.iter().zip(&set.right))
            {
                // Each divisor prepared, as the timed rows prepare one.
                let expected = oracle::div_round(*x, *y, decimals, C::MODE);
                let quotient = C::prepare(b).and_then(|b| C::checked_div_round_prepared(a, &b));
                tally.count(
                    &quotient.and_then(|quotient| C::to_steps(&quotient)),
                    &expected,
                    format_args!("{x} / {y}, prepared"),
                );
            }
        }
        tally.verdict("div-round-prepared", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn rescale_round<C: RescaleRound>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let steps = input::steps(C::WIDTH, predictability);
            for (value, x) in values::<C>(&steps).iter().zip(&steps) {
                let expected = oracle::rescale_round(*x, decimals, 2, C::MODE);
                tally.count(
                    &C::rescale_round(value).and_then(|rounded| C::rounded_steps(&rounded)),
                    &expected,
                    format_args!("{x} to cents"),
                );
            }
        }
        tally.verdict("rescale-round", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn parse<C: Parse>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            for text in input::texts(C::WIDTH, predictability) {
                let got = C::parse(&text).and_then(|value| C::to_steps(&value));
                tally.count(&got, &oracle::parse(&text, decimals), format_args!("{text:?}"));
            }
        }
        // And one digit past the width's decimals, which a row refuses and never rounds.
        let past = format!("0.{}1", "0".repeat(usize::from(decimals)));
        let got = C::parse(&past).and_then(|value| C::to_steps(&value));
        tally.count(&got, &oracle::parse(&past, decimals), format_args!("{past:?}"));
        tally.verdict("parse", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn format<C: Format>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut buffer = Buffer::new();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let steps = input::steps(C::WIDTH, predictability);
            for (value, x) in values::<C>(&steps).iter().zip(&steps) {
                C::format(value, &mut buffer);
                tally.count(
                    &buffer.to_str().and_then(|text| oracle::parse(text, decimals)),
                    &Some(*x),
                    format_args!("{x} written {buffer:?}"),
                );
            }
        }
        tally.verdict("format", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn to_f64<C: ToF64>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let steps = input::steps(C::WIDTH, predictability);
            for (value, x) in values::<C>(&steps).iter().zip(&steps) {
                tally.count(
                    &C::to_f64(value).to_bits(),
                    &oracle::to_f64(*x, decimals).to_bits(),
                    format_args!("{x}"),
                );
            }
        }
        tally.verdict("to-f64", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn from_f64<C: FromF64>(failures: &mut Vec<String>) {
        let decimals = C::WIDTH.decimals();
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            for x in input::doubles(C::WIDTH, predictability) {
                let got = C::from_f64(x).and_then(|value| C::to_steps(&value));
                tally.count(&got, &oracle::from_f64(x, decimals, C::MODE), format_args!("{x:e}"));
            }
        }
        tally.verdict("from-f64", C::NAME, C::WIDTH, C::KIND, C::EXACT, failures);
    }

    fn mul_exact<C: MulExact>(failures: &mut Vec<String>) {
        let mut tally = Tally::default();
        for predictability in Predictability::ALL {
            let set = input::prices_and_quantities(predictability);
            for (&x, &y) in set.left.iter().zip(&set.right) {
                let (price, quantity) = (
                    C::price(x).expect("every row holds a price below 100,000"),
                    C::quantity(y).expect("every row holds a quantity below 100"),
                );
                let got = C::checked_mul(&price, &quantity)
                    .and_then(|product| C::product_steps(&product));
                tally.count(&got, &oracle::mul_exact(x, y), format_args!("{x} × {y}"));
            }
        }
        tally.verdict("mul-exact", C::NAME, C::WIDTH, C::KIND, true, failures);
    }

    /// A test that runs `$check` for every contender with `$operation`, and fails with every row
    /// whose declaration does not hold.
    macro_rules! every {
        ($test:ident : $operation:ident by $check:ident) => {
            #[test]
            fn $test() {
                let mut failures = Vec::new();
                macro_rules! check {
                    ($contender: ty,$function: ident) => {
                        $function::<$contender>(&mut failures);
                    };
                }
                for_each_contender!($operation, check, $check);
                assert!(failures.is_empty(), "{}", failures.join("\n"));
            }
        };
    }

    every!(every_sum_agrees: add by add);
    every!(every_order_agrees: compare by compare);
    every!(every_rounded_product_agrees: mul_round by mul_round);
    every!(every_rounded_quotient_agrees: div_round by div_round);
    every!(every_prepared_quotient_agrees: prepared_div_round by prepared_div_round);
    every!(every_value_to_cents_agrees: rescale_round by rescale_round);
    every!(every_text_reads_as_the_oracle_does: parse by parse);
    every!(every_text_written_reads_back: format by format);
    every!(every_double_is_the_nearest: to_f64 by to_f64);
    every!(every_double_converts_from_its_exact_value: from_f64 by from_f64);
    every!(every_notional_is_exact: mul_exact by mul_exact);
}
