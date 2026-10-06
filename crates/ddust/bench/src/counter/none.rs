//! No counters: the platform lends none, so the harness reports time alone.

use std::io;
use std::time::Instant;

use super::Sample;

/// One set, which counts nothing.
#[derive(Debug)]
pub(crate) struct Counters;

#[expect(
    clippy::unused_self,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_ref_mut,
    clippy::missing_const_for_fn,
    reason = "the signatures every platform's counters share, which the harness calls alike"
)]
impl Counters {
    /// No counters.
    pub(crate) fn open() -> Self {
        Self
    }

    /// One set, empty.
    pub(crate) const fn sets(&self) -> usize {
        1
    }

    /// Times `routine`, called `calls` times.
    pub(crate) fn sample<F: FnMut()>(
        &mut self, _set: usize, calls: u64, routine: &mut F,
    ) -> io::Result<Sample> {
        let start = Instant::now();
        for _ in 0..calls {
            routine();
        }
        Ok(Sample { elapsed: start.elapsed(), ..Sample::default() })
    }

    /// What counts: nothing.
    pub(crate) fn backend(&self) -> String {
        "none: this platform lends no counters to a process, so time alone".to_owned()
    }
}
