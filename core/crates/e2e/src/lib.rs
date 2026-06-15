//! End-to-end test harness: drive an [`engine::Session`] against a
//! [`devicesim::VirtualDevice`] over a [`devicesim::VirtualClock`], interleaving
//! delayed device replies and scheduled ticks on a single, deterministic timeline.
//!
//! This is what the engine's old synchronous `drive()` helper could **not** do — it
//! discarded every `ScheduleTick` and delivered replies immediately, so any bug
//! that depends on *when* a reply lands (a poll's read-back arriving on an edit's
//! verify slot — PROTOCOL §6, "bug B") was structurally inexpressible. Here, both
//! device replies and ticks are entries in one min-ordered queue keyed by virtual
//! time, with a per-entry sequence number breaking ties, so the interleaving is
//! reproducible.
//!
//! The crate ships nothing; it exists for the integration tests under `tests/`.
#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use devicesim::{TimingProfile, VirtualClock, VirtualDevice};
use engine::{ConnectionState, CoreEvent, Effect, Session, Snapshot};

/// What a queued timeline entry is.
enum DueKind {
    /// Bytes the device will deliver to the host (an RQ1 reply / identity reply /
    /// unsolicited push).
    DeviceReply(Vec<u8>),
    /// A `tick` the host scheduled.
    Tick,
}

/// One entry on the timeline, due at `at_ms`. `seq` (assigned at enqueue time)
/// orders entries that share a time, making the whole schedule deterministic.
struct Due {
    at_ms: u64,
    seq: u64,
    kind: DueKind,
}

// `BinaryHeap` is a max-heap; reverse the ordering so it yields the *earliest*
// (smallest `at_ms`, then smallest `seq`) entry first. Only the schedule key takes
// part in ordering — the payload doesn't.
impl PartialEq for Due {
    fn eq(&self, other: &Self) -> bool {
        self.at_ms == other.at_ms && self.seq == other.seq
    }
}
impl Eq for Due {}
impl Ord for Due {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .at_ms
            .cmp(&self.at_ms)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Due {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Couples a `Session`, a `VirtualDevice`, and a `VirtualClock` into a deterministic
/// end-to-end driver.
pub struct Harness {
    session: Session,
    device: VirtualDevice,
    clock: VirtualClock,
    timing: TimingProfile,
    queue: BinaryHeap<Due>,
    seq: u64,
    events: Vec<CoreEvent>,
}

impl Harness {
    /// Build a harness from explicit parts.
    pub fn new(session: Session, device: VirtualDevice, timing: TimingProfile) -> Self {
        Self {
            session,
            device,
            clock: VirtualClock::new(),
            timing,
            queue: BinaryHeap::new(),
            seq: 0,
            events: Vec::new(),
        }
    }

    /// A V31 device + a session in `locale`, with synthetic latencies.
    pub fn v31(locale: &str) -> Self {
        Self::new(
            Session::new(locale),
            VirtualDevice::v31(),
            TimingProfile::synthetic(),
        )
    }

    // ── observation ──

    pub fn now(&self) -> u64 {
        self.clock.now()
    }

    pub fn state(&self) -> ConnectionState {
        self.session.state()
    }

    pub fn snapshot(&self) -> Snapshot {
        self.session.snapshot()
    }

    /// All events emitted so far.
    pub fn events(&self) -> &[CoreEvent] {
        &self.events
    }

    /// Take and clear the events emitted so far.
    pub fn take_events(&mut self) -> Vec<CoreEvent> {
        std::mem::take(&mut self.events)
    }

    /// The spoken texts emitted so far, in order.
    pub fn spoken(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|e| match e {
                CoreEvent::Speak(s) => Some(s.text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Mutable access to the device, e.g. to make it unresponsive or seed it.
    pub fn device_mut(&mut self) -> &mut VirtualDevice {
        &mut self.device
    }

    // ── host actions ──

    /// Run an arbitrary host action and schedule its effects.
    pub fn act(&mut self, f: impl FnOnce(&mut Session) -> Vec<Effect>) -> &mut Self {
        let fx = f(&mut self.session);
        self.ingest(fx);
        self
    }

    /// Like [`Harness::act`], but also returns a copy of the produced effects so a
    /// test can assert on them directly (e.g. that a tick sent no MIDI).
    pub fn act_capturing(&mut self, f: impl FnOnce(&mut Session) -> Vec<Effect>) -> Vec<Effect> {
        let fx = f(&mut self.session);
        let captured = fx.clone();
        self.ingest(fx);
        captured
    }

    /// Whether anything (a device reply or a tick) is still scheduled.
    pub fn has_pending_io(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn connect(&mut self) -> &mut Self {
        self.act(Session::on_connected)
    }

    pub fn disconnect(&mut self) -> &mut Self {
        self.act(Session::on_disconnected)
    }

    /// Feed raw inbound MIDI directly (bypassing the device) — for crafted replies.
    pub fn feed(&mut self, bytes: &[u8]) -> &mut Self {
        let b = bytes.to_vec();
        self.act(move |s| s.handle_midi_input(&b))
    }

    /// Fire a single `tick` at the current virtual time.
    pub fn poll(&mut self) -> &mut Self {
        let now = self.now();
        self.act(move |s| s.tick(now))
    }

    pub fn select_kit(&mut self, number: u32) -> &mut Self {
        self.act(move |s| s.select_kit(number))
    }

    pub fn set_parameter(&mut self, param_id: &str, indices: Vec<u32>, value: i64) -> &mut Self {
        let id = param_id.to_string();
        self.act(move |s| s.set_parameter(id, indices, value))
    }

    pub fn rename_kit(&mut self, number: u32, name: &str) -> &mut Self {
        let name = name.to_string();
        self.act(move |s| s.rename_kit(number, name))
    }

    /// Simulate a kit selected on the module's own panel (unsolicited push).
    pub fn hardware_select_kit(&mut self, index: u32) -> &mut Self {
        let push = self.device.hardware_select_kit(index);
        let at = self.clock.now() + self.timing.unsolicited_push_ms;
        self.push(at, DueKind::DeviceReply(push));
        self
    }

    /// Simulate a parameter edited on the module's own panel (unsolicited push).
    pub fn hardware_edit(&mut self, param_id: &str, indices: &[u32], value: i64) -> &mut Self {
        let push = self
            .device
            .hardware_edit(param_id, indices, devicesim::EditValue::Int(value));
        let at = self.clock.now() + self.timing.unsolicited_push_ms;
        self.push(at, DueKind::DeviceReply(push));
        self
    }

    // ── running the clock ──

    /// Process the next due entry, advancing the clock to its time. Returns its
    /// time, or `None` if the timeline is empty.
    pub fn step(&mut self) -> Option<u64> {
        let due = self.queue.pop()?;
        self.clock.advance_to(due.at_ms);
        match due.kind {
            DueKind::DeviceReply(bytes) => {
                let fx = self.session.handle_midi_input(&bytes);
                self.ingest(fx);
            }
            DueKind::Tick => {
                let now = self.clock.now();
                let fx = self.session.tick(now);
                self.ingest(fx);
            }
        }
        Some(due.at_ms)
    }

    /// Settle the current exchange: keep stepping while any device reply is in
    /// flight, stopping once only future periodic ticks remain. This is the
    /// timing-aware replacement for the old `drive()` and is the right tool when
    /// the device is responsive and you just want the exchange to play out.
    pub fn run_to_idle(&mut self) -> &mut Self {
        let mut guard = 0;
        while self.has_device_reply() {
            self.step();
            guard += 1;
            assert!(
                guard < 1000,
                "harness did not settle (runaway device replies)"
            );
        }
        self
    }

    /// Advance virtual time by `ms`, firing every entry (ticks *and* replies) due at
    /// or before the new time. Use this for time-driven scenarios: polling cadence,
    /// identity retries, edit timeouts.
    pub fn advance(&mut self, ms: u64) -> &mut Self {
        let target = self.clock.now() + ms;
        while let Some(next) = self.queue.peek() {
            if next.at_ms > target {
                break;
            }
            self.step();
        }
        self.clock.advance_to(target);
        self
    }

    // ── internals ──

    /// Drain a batch of effects onto the timeline. `SendMidi` is answered by the
    /// device **now** (a snapshot — see [`VirtualDevice::respond`]) and the reply is
    /// enqueued at `now + latency`; `ScheduleTick` becomes a future tick; `Emit` is
    /// recorded.
    fn ingest(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::SendMidi(bytes) => {
                    let at = self.clock.now() + self.reply_latency(&bytes);
                    for reply in self.device.respond(&bytes) {
                        self.push(at, DueKind::DeviceReply(reply));
                    }
                }
                Effect::ScheduleTick { after_ms } => {
                    let at = self.clock.now() + after_ms;
                    self.push(at, DueKind::Tick);
                }
                Effect::Emit(event) => self.events.push(event),
            }
        }
    }

    fn push(&mut self, at_ms: u64, kind: DueKind) {
        let seq = self.seq;
        self.seq += 1;
        self.queue.push(Due { at_ms, seq, kind });
    }

    fn reply_latency(&self, request: &[u8]) -> u64 {
        // Identity Request (F0 7E .. 06 01) vs RQ1/DT1. A DT1 yields no reply, so
        // its latency is never observed.
        if request.len() >= 5 && request[1] == 0x7E && request[4] == 0x01 {
            self.timing.identity_reply_ms
        } else {
            self.timing.rq1_reply_ms
        }
    }

    fn has_device_reply(&self) -> bool {
        self.queue
            .iter()
            .any(|d| matches!(d.kind, DueKind::DeviceReply(_)))
    }
}
