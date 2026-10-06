//! The CPU's counters, where the platform lends them: perf's on Linux, kperf's on macOS run as root
//! with the `kperf` feature, and none elsewhere, where the harness reports time alone.

use core::fmt;
use core::time::Duration;

cfg_select! {
    target_os = "linux" => {
        mod perf;
        pub(crate) use perf::Counters;
    },
    all(target_os = "macos", feature = "kperf") => {
        mod kperf;
        pub(crate) use kperf::Counters;
    },
    _ => {
        mod none;
        pub(crate) use none::Counters;
    },
}

/// An event the CPU counts, named after perf's generic events, which each platform maps to its
/// CPU's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Event {
    /// Core clock cycles.
    Cycles,
    /// Instructions retired.
    Instructions,
    /// Branch instructions retired.
    Branches,
    /// Branches the predictor got wrong.
    BranchMisses,
    /// Instruction fetches that missed the level 1 instruction cache.
    L1iMisses,
    /// Data loads that missed the level 1 data cache.
    L1dMisses,
    /// Cycles the front end delivered nothing to issue.
    StalledCyclesFrontend,
    /// Cycles the back end could not take what the front end delivered.
    StalledCyclesBackend,
}

impl Event {
    /// The event's name in a table's header and a result's key.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cycles => "cycles",
            Self::Instructions => "instructions",
            Self::Branches => "branches",
            Self::BranchMisses => "branch-misses",
            Self::L1iMisses => "l1i-misses",
            Self::L1dMisses => "l1d-misses",
            Self::StalledCyclesFrontend => "stalled-cycles-frontend",
            Self::StalledCyclesBackend => "stalled-cycles-backend",
        }
    }
}

/// The event's name.
impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What one sample measured: its time, and each event its set counted, over every call of it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Sample {
    /// The time the calls took, inside the span the counters ran.
    pub(crate) elapsed: Duration,
    /// Each event the set counted, and its count.
    pub(crate) counts: Vec<(Event, u64)>,
    /// Whether the set shared the CPU's counters with another while it ran, so its counts are
    /// scaled guesses: the harness reports every such sample.
    pub(crate) multiplexed: bool,
}
