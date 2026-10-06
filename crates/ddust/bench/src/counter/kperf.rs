//! kperf's counters on macOS, through darwin-kperf: Apple's private framework, which counts for a
//! process run as root, every event below in one set.

use core::fmt;
use std::io;
use std::time::Instant;

use darwin_kperf::Sampler;
use darwin_kperf::event::Event as Kind;

use super::{Event, Sample};

/// The events the one set counts, as kperf names them, chip by chip.
const KINDS: [Kind; 6] = [
    Kind::FixedCycles,
    Kind::FixedInstructions,
    Kind::InstBranch,
    Kind::BranchMispredNonspec,
    Kind::L1DCacheMissLd,
    Kind::L1ICacheMissDemand,
];

/// The same events, as the harness names them.
const EVENTS: [Event; 6] = [
    Event::Cycles,
    Event::Instructions,
    Event::Branches,
    Event::BranchMisses,
    Event::L1dMisses,
    Event::L1iMisses,
];

/// The framework, loaded, or why it is not.
pub(crate) struct Counters {
    /// The sampler, when the framework loaded and the process may use it.
    sampler: Result<Sampler, String>,
}

impl Counters {
    /// The framework, if it loads: it needs root, and Instruments not holding the counters.
    pub(crate) fn open() -> Self {
        Self { sampler: Sampler::new().map_err(|error| error.to_string()) }
    }

    /// One set, which counts every event, or nothing without the framework.
    #[expect(clippy::unused_self, reason = "the signature every platform's counters share")]
    pub(crate) const fn sets(&self) -> usize {
        1
    }

    /// Runs `routine` `calls` times with the set counting, and what it counted.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "the signature every platform's counters share"
    )]
    pub(crate) fn sample<F: FnMut()>(
        &mut self, _set: usize, calls: u64, routine: &mut F,
    ) -> io::Result<Sample> {
        let Ok(sampler) = &self.sampler else {
            let start = Instant::now();
            for _ in 0..calls {
                routine();
            }
            return Ok(Sample { elapsed: start.elapsed(), ..Sample::default() });
        };
        let failed = |error: darwin_kperf::SamplerError| io::Error::other(error.to_string());
        let mut thread = sampler.thread(KINDS).map_err(failed)?;
        thread.start().map_err(failed)?;
        let before = thread.sample().map_err(failed)?;
        let start = Instant::now();
        for _ in 0..calls {
            routine();
        }
        let elapsed = start.elapsed();
        let after = thread.sample().map_err(failed)?;
        thread.stop().map_err(failed)?;
        let counts = EVENTS
            .into_iter()
            .zip(after.into_iter().zip(before).map(|(end, begin)| end.wrapping_sub(begin)))
            .collect();
        Ok(Sample { elapsed, counts, multiplexed: false })
    }

    /// What counts, for a run's manifest.
    pub(crate) fn backend(&self) -> String {
        match &self.sampler {
            Ok(_) => "kperf through darwin-kperf 0.1.1, this thread only; one set: FIXED_CYCLES, \
                      FIXED_INSTRUCTIONS, INST_BRANCH, BRANCH_MISPRED_NONSPEC, L1D_CACHE_MISS_LD, \
                      L1I_CACHE_MISS_DEMAND"
                .to_owned(),
            Err(why) => format!("none: kperf did not load ({why}); run as root for the counters"),
        }
    }
}

/// What counts.
impl fmt::Debug for Counters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Counters").field("backend", &self.backend()).finish()
    }
}
