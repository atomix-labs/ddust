//! ddust's benchmarks: a harness that times an operation and counts the CPU's events while it runs,
//! the predictability every contender reads, and the exact oracle each result is checked against.
//!
//! divan has no hardware counters, criterion takes one figure a pass, and gungraun counts a
//! routine once, cold. This harness takes each operation in steady state, and reads time,
//! cycles, instructions, branches and their misses, level 1 cache misses, and the cycles the front
//! and back ends stall, from the same samples:
//!
//! - **On Linux**, through `perf_event_open`, in sets of the cycle counter and two events, rotated
//!   so no set shares the counters; a sample that shared them is reported.
//! - **On macOS**, with the `kperf` feature and run as root, through Apple's kperf framework, every
//!   event but the stalls in one set.
//! - **Elsewhere**, and wherever the counters are refused, time alone, with each count `n/a`.
//!
//! # Examples
//! ```
//! use ddust_bench::{Config, Event, Harness};
//!
//! let mut harness = Harness::new(Config::QUICK);
//! let values: Vec<u64> = (0..1024).collect();
//! harness.measure("sum/example", 1024, || {
//!     core::hint::black_box(core::hint::black_box(&values).iter().sum::<u64>());
//! })?;
//! harness.finish()?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! # Crate Features
//!
//! | Feature | What it adds |
//! | ------- | ------------ |
//! | `kperf` | The CPU's counters on macOS, through darwin-kperf, for a run as root |

pub mod contender;
mod counter;
mod harness;
pub mod input;
pub mod oracle;
pub mod report;

pub use counter::Event;
pub use harness::{Config, Figure, Harness, Measurement};
