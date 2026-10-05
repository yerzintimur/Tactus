//! The engine's clock-facing numbers in one place: how often it polls, how long
//! it waits, how many polls between re-reads. The defaults are what the V31 was
//! measured against (PROTOCOL §6); a platform or a test can hand in others
//! through [`crate::Session::with_timings`] — the engine itself never sleeps,
//! it only asks for the next tick (ADR-0008).

/// Engine timings, in the units the engine counts in: milliseconds for the ticks
/// it schedules, polls for what it counts on those ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timings {
    /// How often to poll the active kit (`Current`) while connected. The V31
    /// answers reads again ~400 ms after a panel kit change, so a much faster
    /// poll buys little; a much slower one makes a panel kit change feel late.
    pub poll_interval_ms: u64,
    /// How long to wait for an Identity Reply before asking again.
    pub identity_retry_ms: u64,
    /// An edit (or a kit selection) times out after this many polls without the
    /// confirming read-back — the failure is announced, never left silent.
    pub edit_timeout_ticks: u32,
    /// Re-read the current kit's name every this many polls, to notice its slot
    /// being replaced on the module while the kit *number* stays put. Every poll
    /// would double the traffic for an event that happens between songs.
    pub kit_name_refresh_polls: u32,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 300,
            identity_retry_ms: 900,
            edit_timeout_ticks: 5,
            kit_name_refresh_polls: 10,
        }
    }
}
