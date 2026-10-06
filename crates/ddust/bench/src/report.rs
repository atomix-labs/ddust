//! How measurements are written: a table a person reads as the run goes, and a TOML file a run
//! commits, which nothing transcribes by hand.

use core::fmt::{self, Write as _};
use std::io;

use crate::counter::Event;
use crate::harness::{Config, Figure, Measurement};

/// The width of the name column.
const NAME_WIDTH: usize = 72;

/// Writes the table's header.
///
/// # Errors
/// When `out` refuses the line.
pub(crate) fn write_header<W: io::Write>(out: &mut W) -> io::Result<()> {
    writeln!(
        out,
        "{:<NAME_WIDTH$} {:>9} {:>8} {:>8} {:>5} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>5} {:>6}",
        "per operation",
        "ns",
        "cycles",
        "instr",
        "IPC",
        "branches",
        "br-miss",
        "L1i-miss",
        "L1d-miss",
        "fe-stall",
        "be-stall",
        "GHz",
        "spread",
    )
}

/// Writes `measurement`'s row: each figure's median, the instructions per cycle and the clock its
/// cycles and time imply, the time's spread, and a warning for samples whose counters were shared.
///
/// # Errors
/// When `out` refuses the line.
pub(crate) fn write_row<W: io::Write>(out: &mut W, measurement: &Measurement) -> io::Result<()> {
    let median = |event| measurement.figure(event).map(|figure| figure.median);
    let cell = |value: Option<f64>, decimals: usize| {
        value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.decimals$}"))
    };
    let (cycles, instructions) = (median(Event::Cycles), median(Event::Instructions));
    let ipc = cycles.zip(instructions).map(|(cycles, instructions)| instructions / cycles);
    let clock = cycles.map(|cycles| cycles / measurement.nanoseconds.median);
    write!(
        out,
        "{:<NAME_WIDTH$} {:>9.4} {:>8} {:>8} {:>5} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>5} {:>5.1}%",
        measurement.name,
        measurement.nanoseconds.median,
        cell(cycles, 3),
        cell(instructions, 2),
        cell(ipc, 2),
        cell(median(Event::Branches), 3),
        cell(median(Event::BranchMisses), 4),
        cell(median(Event::L1iMisses), 4),
        cell(median(Event::L1dMisses), 4),
        cell(median(Event::StalledCyclesFrontend), 3),
        cell(median(Event::StalledCyclesBackend), 3),
        cell(clock, 2),
        measurement.nanoseconds.spread() * 100.0,
    )?;
    if measurement.multiplexed_samples > 0 {
        write!(out, "  shared counters in {} samples", measurement.multiplexed_samples)?;
    }
    writeln!(out)
}

/// Writes every measurement as TOML: what counted, how long each ran, and each figure.
///
/// # Errors
/// Never, writing to a `String`; the result is `fmt`'s.
pub(crate) fn write_toml(
    out: &mut String, backend: &str, config: &Config, measurements: &[Measurement],
) -> fmt::Result {
    writeln!(out, "backend = {}", quoted(backend))?;
    writeln!(out, "warm-up-ms = {}", config.warm_up.as_millis())?;
    writeln!(out, "sample-us = {}", config.sample_time.as_micros())?;
    writeln!(out, "samples-per-set = {}", config.samples_per_set)?;
    for measurement in measurements {
        writeln!(out)?;
        writeln!(out, "[[measurement]]")?;
        writeln!(out, "name = {}", quoted(&measurement.name))?;
        writeln!(out, "operations = {}", measurement.operations)?;
        writeln!(out, "calls = {}", measurement.calls)?;
        writeln!(out, "multiplexed-samples = {}", measurement.multiplexed_samples)?;
        writeln!(out, "nanoseconds = {}", inline(measurement.nanoseconds))?;
        for (event, figure) in &measurement.events {
            writeln!(out, "{event} = {}", inline(*figure))?;
        }
    }
    Ok(())
}

/// `figure` as an inline table.
fn inline(figure: Figure) -> String {
    format!("{{ median = {}, low = {}, high = {} }}", figure.median, figure.low, figure.high)
}

/// `text` as a TOML basic string.
fn quoted(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len().saturating_add(2));
    quoted.push('"');
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::{quoted, write_row, write_toml};
    use crate::counter::Event;
    use crate::harness::{Config, Figure, Measurement};

    /// A measurement of one nanosecond and three cycles an operation.
    fn measurement() -> Measurement {
        let figure = |median| Figure { median, low: median, high: median };
        Measurement {
            name: "add/narrow/predictable/i64".to_owned(),
            operations: 1024,
            calls: 7,
            nanoseconds: figure(1.0),
            events: vec![(Event::Cycles, figure(3.0)), (Event::Instructions, figure(6.0))],
            multiplexed_samples: 0,
        }
    }

    #[test]
    fn a_row_shows_what_was_counted_and_marks_the_rest() {
        let mut row = Vec::new();
        write_row(&mut row, &measurement()).expect("a Vec refuses no write");
        let row = String::from_utf8(row).expect("the row is ASCII: digits, letters and spaces");
        let cells =
            ["1.0000", "3.000", "6.00", "2.00", "n/a", "n/a", "n/a", "n/a", "n/a", "n/a", "3.00"];
        let [ns, cycles, instructions, ipc, branches, misses, l1i, l1d, front, back, clock] = cells;
        let expected = format!(
            "{:<72} {ns:>9} {cycles:>8} {instructions:>8} {ipc:>5} {branches:>8} {misses:>8} {l1i:>8} \
             {l1d:>8} {front:>8} {back:>8} {clock:>5} {:>6}\n",
            "add/narrow/predictable/i64", "0.0%",
        );
        assert_eq!(row, expected, "each figure in its column, n/a for what no set counted");
    }

    #[test]
    fn a_saved_run_holds_every_figure() {
        let mut text = String::new();
        write_toml(&mut text, "none", &Config::QUICK, &[measurement()])
            .expect("a String refuses no write");
        let expected = "backend = \"none\"\nwarm-up-ms = 1\nsample-us = 100\nsamples-per-set = 1\n\n\
                        [[measurement]]\nname = \"add/narrow/predictable/i64\"\noperations = 1024\n\
                        calls = 7\nmultiplexed-samples = 0\n\
                        nanoseconds = { median = 1, low = 1, high = 1 }\n\
                        cycles = { median = 3, low = 3, high = 3 }\n\
                        instructions = { median = 6, low = 6, high = 6 }\n";
        assert_eq!(text, expected, "every figure, in the order written");
    }

    #[test]
    fn a_string_is_escaped() {
        assert_eq!(quoted(r#"a "b" \c"#), r#""a \"b\" \\c""#, "quotes and backslashes");
    }
}
