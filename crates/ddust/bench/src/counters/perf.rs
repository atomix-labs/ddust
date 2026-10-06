//! perf's counters on Linux, through `perf_event_open`, in user space alone.
//!
//! A set is a group of the cycle counter and two events, the most a Graviton4 guest schedules at
//! once: more, and the kernel shares the counters between them, and scales what each counted into a
//! guess. The harness rotates through the sets, and each sample says whether its set had the
//! counters to itself the whole time it ran.

use core::fmt;
use std::io;
use std::time::Instant;

use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Hardware};
use perf_event::{Builder, Counter, Group};

use super::{Event, Sample};

/// The sets the harness rotates through, each counting cycles beside these.
const SETS: [[Event; 2]; 4] = [
    [Event::Instructions, Event::BranchMisses],
    [Event::L1iMisses, Event::L1dMisses],
    [Event::FrontendStalls, Event::BackendStalls],
    [Event::Branches, Event::Instructions],
];

/// One set, open and stopped: its group, and each event's counter in it.
struct Set {
    /// The group, which starts, stops and reads its counters together.
    group: Group,
    /// Each event the set counts, cycles first.
    counters: Vec<(Event, Counter)>,
}

/// Every set the CPU counts.
pub(crate) struct Counters {
    /// The sets that opened and count, in rotation order; none when perf lends no counters.
    sets: Vec<Set>,
    /// What the kernel refused, if anything: an event, a set, or every counter, and why.
    refused: Vec<String>,
}

impl Counters {
    /// Every set the CPU counts, leaving out what the kernel refuses.
    pub(crate) fn open() -> Self {
        let mut counters = Self { sets: Vec::new(), refused: Vec::new() };
        for events in SETS {
            match open_set(events, &mut counters.refused) {
                Ok(set) => counters.sets.push(set),
                Err(error) => counters.refused.push(format!("{events:?}: {error}")),
            }
        }
        counters
    }

    /// How many sets the harness rotates through: at least one, which may count nothing.
    pub(crate) fn sets(&self) -> usize {
        self.sets.len().max(1)
    }

    /// Runs `routine` `iterations` times with set `set` counting, and what it counted.
    pub(crate) fn sample<F: FnMut()>(
        &mut self, set: usize, iterations: u64, routine: &mut F,
    ) -> io::Result<Sample> {
        let Some(open) = self.sets.get_mut(set) else {
            let start = Instant::now();
            for _ in 0..iterations {
                routine();
            }
            return Ok(Sample { elapsed: start.elapsed(), ..Sample::default() });
        };
        open.group.reset()?;
        open.group.enable()?;
        let start = Instant::now();
        for _ in 0..iterations {
            routine();
        }
        let elapsed = start.elapsed();
        open.group.disable()?;
        let data = open.group.read()?;
        let counts = open.counters.iter().map(|(event, counter)| (*event, data[counter])).collect();
        Ok(Sample { elapsed, counts, multiplexed: data.time_enabled() != data.time_running() })
    }

    /// What counts, for a run's manifest: the interface, the sets, and what the kernel refused.
    pub(crate) fn backend(&self) -> String {
        let sets: Vec<String> = self
            .sets
            .iter()
            .map(|set| {
                let names: Vec<&str> = set.counters.iter().map(|(event, _)| event.name()).collect();
                format!("[{}]", names.join(", "))
            })
            .collect();
        let mut backend = format!(
            "perf_event_open through perf-event2 0.7.4, user space only, perf's generic events; sets {}",
            sets.join(" ")
        );
        if !self.refused.is_empty() {
            backend.push_str("; refused: ");
            backend.push_str(&self.refused.join("; "));
        }
        backend
    }
}

/// What counts.
impl fmt::Debug for Counters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Counters").field("backend", &self.backend()).finish()
    }
}

/// Opens one set, cycles and `events`, and checks it counts: an event the kernel refuses is left
/// out, and named in `refused`.
fn open_set(events: [Event; 2], refused: &mut Vec<String>) -> io::Result<Set> {
    let mut group = Group::new()?;
    let mut counters = vec![(Event::Cycles, group.add(&builder(Event::Cycles))?)];
    for event in events {
        match group.add(&builder(event)) {
            Ok(counter) => counters.push((event, counter)),
            Err(error) => refused.push(format!("{event}: {error}")),
        }
    }
    // A group the PMU cannot schedule opens, then never runs: one empty sample shows it.
    group.enable()?;
    group.disable()?;
    let data = group.read()?;
    if data.time_running().is_none_or(|running| running.is_zero()) {
        return Err(io::Error::other("the group opened, but the CPU never scheduled it"));
    }
    Ok(Set { group, counters })
}

/// The event's builder: perf's generic name, which the kernel maps to the CPU's own event.
fn builder(event: Event) -> Builder<'static> {
    let miss = |which| Cache { which, operation: CacheOp::READ, result: CacheResult::MISS };
    match event {
        Event::Cycles => Builder::new(Hardware::CPU_CYCLES),
        Event::Instructions => Builder::new(Hardware::INSTRUCTIONS),
        Event::Branches => Builder::new(Hardware::BRANCH_INSTRUCTIONS),
        Event::BranchMisses => Builder::new(Hardware::BRANCH_MISSES),
        Event::L1iMisses => Builder::new(miss(CacheId::L1I)),
        Event::L1dMisses => Builder::new(miss(CacheId::L1D)),
        Event::FrontendStalls => Builder::new(Hardware::STALLED_CYCLES_FRONTEND),
        Event::BackendStalls => Builder::new(Hardware::STALLED_CYCLES_BACKEND),
    }
}
