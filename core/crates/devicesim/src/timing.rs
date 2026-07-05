//! Virtual time + reply-latency model.

use serde::{Deserialize, Serialize};

/// A monotonic virtual clock in milliseconds.
///
/// There is no wall-clock and no threads — the harness advances this explicitly as
/// it processes the timeline, so every scenario is fully deterministic and
/// reproducible (the same inputs always interleave the same way).
#[derive(Debug, Clone, Default)]
pub struct VirtualClock {
    now_ms: u64,
}

impl VirtualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now(&self) -> u64 {
        self.now_ms
    }

    /// Move the clock forward to `t` (never backward).
    pub fn advance_to(&mut self, t: u64) {
        if t > self.now_ms {
            self.now_ms = t;
        }
    }
}

/// How long a virtual module takes to answer, by request kind.
///
/// Synthetic defaults are used until a real session is recorded; a recorded
/// cassette's latencies will override them (Phase 3). Latency is what lets the
/// harness reproduce timing races — e.g. a poll's read-back landing on an edit's
/// verify slot (PROTOCOL §6, "bug B").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingProfile {
    /// Identity Request → Identity Reply.
    pub identity_reply_ms: u64,
    /// RQ1 (read) → DT1 (reply).
    pub rq1_reply_ms: u64,
    /// Delay before the module acknowledges a DT1 write, if it ever does (the V31
    /// does not — it is verified by a follow-up read). `None` = no ack.
    #[serde(default)]
    pub dt1_ack_ms: Option<u64>,
    /// Delay on an unsolicited hardware-edit push (Transmit Edit Data).
    #[serde(default)]
    pub unsolicited_push_ms: u64,
}

impl TimingProfile {
    /// Sane synthetic latencies, used before any hardware is recorded.
    pub fn synthetic() -> Self {
        Self {
            identity_reply_ms: 20,
            rq1_reply_ms: 8,
            dt1_ack_ms: None,
            unsolicited_push_ms: 4,
        }
    }

    /// Zero latency — replies are delivered in send order. Useful for tests that
    /// only care about content, not interleaving.
    pub fn instant() -> Self {
        Self {
            identity_reply_ms: 0,
            rq1_reply_ms: 0,
            dt1_ack_ms: None,
            unsolicited_push_ms: 0,
        }
    }

    /// The delay before the module's reply to `request` lands: an Identity
    /// Request (`F0 7E .. 06 01`) takes [`identity_reply_ms`](Self::identity_reply_ms),
    /// anything else [`rq1_reply_ms`](Self::rq1_reply_ms). (A DT1 write produces
    /// no reply, so its value is never observed.)
    pub fn reply_delay_ms(&self, request: &[u8]) -> u64 {
        let identity = request.len() >= 5 && request[1] == 0x7E && request[4] == 0x01;
        if identity {
            self.identity_reply_ms
        } else {
            self.rq1_reply_ms
        }
    }
}

impl Default for TimingProfile {
    fn default() -> Self {
        Self::synthetic()
    }
}
