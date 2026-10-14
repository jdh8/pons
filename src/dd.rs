//! Access to the double-dummy solver with a configurable thread pool
//!
//! ddss keeps one process-wide DDS thread pool.  Asked for no cap, it sizes
//! the pool to every online CPU (`sysconf(_SC_NPROCESSORS_ONLN)`), ignoring
//! CPU affinity, `RAYON_NUM_THREADS` and any worker count a script passes —
//! on a shared box that is far more threads than the job was given.  Open the
//! solver through [`solver`][crate::dd::solver] instead of `Solver::lock(None)` so the
//! [`THREADS_VAR`][crate::dd::THREADS_VAR] environment variable caps every solve in the process.

use core::num::NonZero;
use ddss::Solver;
use std::sync::LazyLock;

/// Environment variable capping the DDS thread pool: a positive integer, or
/// unset, empty or `0` for every online CPU
pub const THREADS_VAR: &str = "DDSS_THREADS";

static THREADS: LazyLock<Option<NonZero<usize>>> = LazyLock::new(|| {
    let value = std::env::var(THREADS_VAR).unwrap_or_default();
    parse_threads(&value).unwrap_or_else(|| {
        panic!("{THREADS_VAR}={value:?} is not a thread count (a positive integer, 0 or empty)")
    })
});

/// Parses a [`THREADS_VAR`] value: `Some(None)` for no cap, `None` if invalid
fn parse_threads(value: &str) -> Option<Option<NonZero<usize>>> {
    let value = value.trim();
    if value.is_empty() {
        return Some(None);
    }
    value.parse().ok().map(NonZero::new)
}

/// The DDS thread cap from [`THREADS_VAR`], read once per process
///
/// `None` lets ddss use every online CPU.
///
/// # Panics
///
/// If [`THREADS_VAR`] is set to anything but a non-negative integer, so a
/// typo fails loudly instead of silently taking the whole machine.
#[must_use]
pub fn threads() -> Option<NonZero<usize>> {
    *THREADS
}

/// Locks the DDS solver with the pool capped by [`threads`]
///
/// # Panics
///
/// See [`threads`].
#[must_use]
pub fn solver() -> Solver {
    Solver::lock(threads())
}

#[cfg(test)]
mod tests;
