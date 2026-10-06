//! How measurements are written: a table a person reads as the run goes, and a TOML file a run
//! commits, which nothing transcribes by hand.

use core::fmt::{self, Write as _};
use std::io;

use crate::counters::Event;
use crate::harness::{Config, Figure, Measurement};

/// The width of the name column.
const NAME_WIDTH: usize = 72;

/// Writes the table's header.
///
/// # Errors
/// When `out` refuses the line.
pub fn write_header<W: io::Write>(out: &mut W) -> io::Result<()> {
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
pub fn write_row<W: io::Write>(out: &mut W, measurement: &Measurement) -> io::Result<()> {
    let median = |event| measurement.event(event).map(|figure| figure.median);
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
        cell(median(Event::FrontendStalls), 3),
        cell(median(Event::BackendStalls), 3),
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
pub fn write_toml(
    out: &mut String, backend: &str, config: &Config, measurements: &[Measurement],
) -> fmt::Result {
    writeln!(out, "counters = {}", quoted(backend))?;
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
    use crate::counters::Event;
    use crate::harness::{Config, Figure, Measurement};

    /// A measurement of one nanosecond and three cycles an operation.
    fn measurement() -> Measurement {
        let figure = |median| Figure { median, low: median, high: median };
        Measurement {
            name: "add/predictable/i64".to_owned(),
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
        write_row(&mut row, &measurement()).expect("a Vec takes the row");
        let row = String::from_utf8(row).expect("ASCII");
        assert!(row.starts_with("add/predictable/i64"), "{row}");
        assert!(row.contains(" 3.000 ") && row.contains(" 2.00 "), "cycles, and IPC: {row}");
        assert!(row.contains("n/a"), "what no set counted: {row}");
    }

    #[test]
    fn a_saved_run_holds_every_figure() {
        let mut text = String::new();
        write_toml(&mut text, "none", &Config::QUICK, &[measurement()]).expect("a String takes it");
        assert!(text.contains("[[measurement]]\nname = \"add/predictable/i64\""), "{text}");
        assert!(text.contains("cycles = { median = 3, low = 3, high = 3 }"), "{text}");
    }

    #[test]
    fn a_string_is_escaped() {
        assert_eq!(quoted(r#"a "b" \c"#), r#""a \"b\" \\c""#, "quotes and backslashes");
    }
}
