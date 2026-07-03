//! Engine outputs: the events the host observes and the effects it must perform.
//! The engine is sans-I/O — every input method returns a `Vec<Effect>` and the
//! host executes them (send MIDI, schedule a tick, forward events). See ADR-0008.

pub use device::FirmwareSupport;

/// Connection lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Identifying,
    Ready,
}

/// Identity of the connected module (from its Identity Reply + matched profile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub model_id: Vec<u8>,
    pub device_id: u8,
    pub name: String,
    pub firmware: String,
    pub firmware_support: FirmwareSupport,
    pub profile_id: String,
    /// `false` => unknown module, running in degraded mode.
    pub recognized: bool,
}

/// How important a spoken message is (maps to platform announcement priorities).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechPriority {
    Low,
    Default,
    High,
}

/// What an announcement is about. The platform's router keys on this (ADR-0014):
/// `KitNav` interrupts the previous announcement, a `UserInitiated` `ParamEdit` is
/// suppressed while a screen reader voices the focused control, etc. The core only
/// *tags*; the platform decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechCategory {
    /// Connection lifecycle: connected / unrecognized / firmware notes, plus the
    /// initial kit read that completes the connection summary (ADR-0014 — the
    /// first kit is part of the summary, not a `KitNav` barge-in).
    Connection,
    /// The current kit changed — app- or device-side navigation. Newer
    /// announcements in this category preempt older ones (interruption, not
    /// debouncing).
    KitNav,
    /// A parameter value changed: an edit confirmation or a hardware edit push.
    ParamEdit,
    /// A failure the user must hear (edit rejected / timed out / out of range).
    Error,
    /// Supplementary detail (e.g. the tempo tail after a kit change).
    Info,
}

/// Who caused the change being announced (ADR-0014). The platform suppresses
/// `UserInitiated` edit announcements the screen reader already voices via the
/// focused control; `DeviceInitiated` changes are always announced — the screen
/// reader cannot see them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechSource {
    DeviceInitiated,
    UserInitiated,
}

/// A spoken message (already localized by the core).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Speech {
    pub text: String,
    pub priority: SpeechPriority,
    pub category: SpeechCategory,
    pub source: SpeechSource,
}

/// A short non-speech audio cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Earcon {
    Connected,
    Disconnected,
    KitChanged,
    Confirmed,
    Error,
}

/// Something the host should surface to the user (forwarded to the UI/listener).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreEvent {
    ConnectionChanged(ConnectionState),
    DeviceIdentified(DeviceInfo),
    CurrentKitChanged {
        number: u32,
        name: String,
    },
    /// An edit was applied and verified by read-back (`display` = the actual value).
    EditConfirmed {
        param_id: String,
        display: String,
    },
    /// An edit could not be confirmed (mismatch, timeout, or out of range);
    /// `reason` is a localized, spoken-friendly explanation.
    EditFailed {
        param_id: String,
        reason: String,
    },
    Speak(Speech),
    Earcon(Earcon),
    Error(String),
}

/// A side effect the host must perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send these raw MIDI bytes to the module.
    SendMidi(Vec<u8>),
    /// Call `tick` again after roughly this many milliseconds.
    ScheduleTick { after_ms: u64 },
    /// Forward this event to the UI / listener.
    Emit(CoreEvent),
}
