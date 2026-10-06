//! Times a routine and counts the CPU's events while it runs, in steady state: warmed up, each
//! sample sized to last a set time, and samples taken round robin across the counters' sets, so
//! drift lands on every set alike.
//!
//! A bench passes its arguments through: `cargo bench --bench ops -- mul --quick --save run.toml`,
//! in `crates/ddust/bench/`, runs the measurements whose names hold `mul`, one sample a set, and
//! writes them to `run.toml` as well as the table it prints.

use core::time::Duration;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::time::Instant;
use std::{env, fs};

use crate::counters::{Counters, Event};
use crate::report;

/// How long a measurement runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// How long the routine runs before anything is recorded; it also sizes a sample.
    pub warm_up: Duration,
    /// How long a sample runs, roughly: the harness calls the routine as often as fits.
    pub sample_time: Duration,
    /// Samples of each set of counters; time is taken from every sample of every set.
    pub samples_per_set: usize,
}

impl Config {
    /// A run whose figures are cited: warmed for 200 ms, then 25 samples of 2 ms from each set, so
    /// 100 samples of time where perf rotates four sets.
    pub const STANDARD: Self = Self {
        warm_up: Duration::from_millis(200),
        sample_time: Duration::from_millis(2),
        samples_per_set: 25,
    };

    /// A smoke test: one short sample from each set, enough to show every routine runs.
    pub const QUICK: Self = Self {
        warm_up: Duration::from_millis(1),
        sample_time: Duration::from_micros(100),
        samples_per_set: 1,
    };
}

/// A figure per operation over a measurement's samples: the median, and the lowest and highest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Figure {
    /// The median sample.
    pub median: f64,
    /// The lowest sample.
    pub low: f64,
    /// The highest sample.
    pub high: f64,
}

impl Figure {
    /// The figure of `values`, or `None` for no values.
    fn of(values: &mut [f64]) -> Option<Self> {
        values.sort_by(f64::total_cmp);
        let (&low, &high) = (values.first()?, values.last()?);
        let middle = values.len() / 2;
        let median = if values.len() % 2 == 1 {
            *values.get(middle)?
        } else {
            f64::midpoint(*values.get(middle.checked_sub(1)?)?, *values.get(middle)?)
        };
        Some(Self { median, low, high })
    }

    /// The samples' range as a share of the median: 0.01 for 1%.
    #[must_use]
    pub fn spread(self) -> f64 {
        if self.median == 0.0 { 0.0 } else { (self.high - self.low) / self.median }
    }
}

/// What one measurement found, each figure per operation.
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Its name: the operation, the inputs and the contender, as `mul-round/unpredictable/ddust`.
    pub name: String,
    /// The operations one call of the routine performs.
    pub operations: u64,
    /// The calls one sample makes.
    pub calls: u64,
    /// Nanoseconds per operation, over every sample.
    pub nanoseconds: Figure,
    /// Each event counted, per operation, over the samples of its set.
    pub events: Vec<(Event, Figure)>,
    /// Samples whose set shared the counters while it ran: their counts are the kernel's guesses.
    pub multiplexed_samples: usize,
}

impl Measurement {
    /// The figure for `event`, if it was counted.
    #[must_use]
    pub fn event(&self, event: Event) -> Option<Figure> {
        self.events.iter().find(|(counted, _)| *counted == event).map(|(_, figure)| *figure)
    }
}

/// Runs measurements and reports them: a table as each finishes, and a file of them at the end
/// when the bench was asked to save one.
#[derive(Debug)]
pub struct Harness {
    /// The CPU's counters, or none.
    counters: Counters,
    /// How long each measurement runs.
    config: Config,
    /// The names to run, by any part of them: every measurement when empty.
    filters: Vec<String>,
    /// Where to write the measurements at the end, if anywhere.
    save: Option<PathBuf>,
    /// The measurements so far.
    measurements: Vec<Measurement>,
}

impl Harness {
    /// A harness with `config`, running every measurement and saving none.
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            counters: Counters::open(),
            config,
            filters: Vec::new(),
            save: None,
            measurements: Vec::new(),
        }
    }

    /// A harness set by the bench's arguments: any word a measurement's name must hold, `--quick`
    /// for [`Config::QUICK`], and `--save <path>` for the file to write. Cargo's own `--bench` is
    /// passed over.
    #[must_use]
    pub fn from_args() -> Self {
        let mut harness = Self::new(Config::STANDARD);
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--quick" => harness.config = Config::QUICK,
                "--save" => harness.save = arguments.next().map(PathBuf::from),
                "--bench" => {},
                word => harness.filters.push(word.to_owned()),
            }
        }
        harness
    }

    /// Whether a measurement named `name` runs: when it holds one of the filters, or there are
    /// none.
    #[must_use]
    pub fn runs(&self, name: &str) -> bool {
        self.filters.is_empty() || self.filters.iter().any(|filter| name.contains(filter.as_str()))
    }

    /// Measures `routine`, which performs `operations` operations a call, unless the filters pass
    /// it over, and prints its row.
    ///
    /// # Errors
    /// When a counter fails to start, stop or read, or the row cannot be written.
    pub fn measure<F: FnMut()>(
        &mut self, name: &str, operations: u64, mut routine: F,
    ) -> io::Result<()> {
        if !self.runs(name) {
            return Ok(());
        }
        if self.measurements.is_empty() {
            report::write_header(&mut io::stdout().lock())?;
        }
        let calls = self.calls(&mut routine);
        let total = calls.saturating_mul(operations);
        let mut nanoseconds = Vec::new();
        let mut counts: Vec<(Event, Vec<f64>)> = Vec::new();
        let mut multiplexed_samples = 0_usize;
        for _ in 0..self.config.samples_per_set {
            for set in 0..self.counters.sets() {
                let sample = self.counters.sample(set, calls, &mut routine)?;
                nanoseconds.push(sample.elapsed.as_secs_f64() * 1e9 / float(total));
                multiplexed_samples =
                    multiplexed_samples.saturating_add(usize::from(sample.multiplexed));
                for (event, count) in sample.counts {
                    let value = float(count) / float(total);
                    match counts.iter_mut().find(|(counted, _)| *counted == event) {
                        Some((_, values)) => values.push(value),
                        None => counts.push((event, vec![value])),
                    }
                }
            }
        }
        let mut events: Vec<(Event, Figure)> = counts
            .into_iter()
            .filter_map(|(event, mut values)| Figure::of(&mut values).map(|figure| (event, figure)))
            .collect();
        events.sort_by_key(|(event, _)| *event);
        let measurement = Measurement {
            name: name.to_owned(),
            operations,
            calls,
            nanoseconds: Figure::of(&mut nanoseconds)
                .ok_or_else(|| io::Error::other("no samples"))?,
            events,
            multiplexed_samples,
        };
        report::write_row(&mut io::stdout().lock(), &measurement)?;
        self.measurements.push(measurement);
        Ok(())
    }

    /// Writes the measurements to the file the arguments named, if any.
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn finish(self) -> io::Result<()> {
        io::stdout().lock().flush()?;
        let Some(path) = self.save else { return Ok(()) };
        let mut text = String::new();
        report::write_toml(&mut text, &self.counters.backend(), &self.config, &self.measurements)
            .map_err(io::Error::other)?;
        fs::write(path, text)
    }

    /// Warms `routine` up, and the calls a sample makes to last about the configured time.
    fn calls<F: FnMut()>(&self, routine: &mut F) -> u64 {
        let start = Instant::now();
        let mut calls = 0_u64;
        while start.elapsed() < self.config.warm_up || calls == 0 {
            routine();
            calls = calls.saturating_add(1);
        }
        let warm = start.elapsed().as_nanos().max(1);
        let wanted = self.config.sample_time.as_nanos().saturating_mul(u128::from(calls));
        u64::try_from(wanted.checked_div(warm).unwrap_or(1)).unwrap_or(u64::MAX).max(1)
    }
}

/// A count as a float, for the arithmetic of a figure.
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "a count past 2^53 rounds by less than a part in a billion, far below any run's noise"
)]
const fn float(count: u64) -> f64 {
    count as f64
}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use super::{Config, Figure, Harness};

    #[test]
    fn a_figure_is_the_median_and_the_ends() {
        let figure = Figure::of(&mut [4.0, 1.0, 3.0, 2.0]).expect("four values");
        assert_eq!(
            (figure.median, figure.low, figure.high),
            (2.5, 1.0, 4.0),
            "the middle two's mean"
        );
        assert_eq!(
            Figure::of(&mut [3.0, 1.0, 2.0]).map(|figure| figure.median),
            Some(2.0),
            "the middle"
        );
        assert_eq!(Figure::of(&mut []), None, "no values, no figure");
    }

    #[test]
    fn a_routine_is_measured_per_operation() {
        let mut harness = Harness::new(Config::QUICK);
        let mut sum = 0_u64;
        harness
            .measure("sum/test", 8, || {
                for value in 0..8_u64 {
                    sum = black_box(sum.wrapping_add(value));
                }
            })
            .expect("the counters, or time alone");
        let measurement = harness.measurements.first().expect("one measurement");
        assert!(measurement.nanoseconds.median > 0.0, "it took time");
        assert!(measurement.calls >= 1, "and was called");
    }

    #[test]
    fn a_filter_passes_over_what_it_does_not_name() {
        let mut harness = Harness::new(Config::QUICK);
        harness.filters.push("mul".to_owned());
        assert!(harness.runs("mul-round/predictable/ddust"), "named");
        assert!(!harness.runs("add/predictable/ddust"), "passed over");
    }
}
