//! UniFFI-exported mirror types for the engine's outputs, with `From` conversions.
//!
//! The `engine` crate stays FFI-agnostic (no `uniffi` dependency); this crate is
//! the boundary and owns the UniFFI annotations. Keeping the mirror also lets the
//! foreign API stay stable/ergonomic independently of internal refactors.
//!
//! Per ADR-0013 these stay as generic as practical so a future declarative
//! view-model / renderer is additive.

/// Connection lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConnectionState {
    Disconnected,
    Identifying,
    Ready,
}

/// How the connected firmware relates to what the profile was tested against.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FirmwareSupport {
    Tested,
    UntestedNewer,
    UntestedOlder,
    Unknown,
}

/// Identity of the connected module.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceInfo {
    pub model_id: Vec<u8>,
    pub device_id: u8,
    pub name: String,
    pub firmware: String,
    pub firmware_support: FirmwareSupport,
    pub profile_id: String,
    pub recognized: bool,
}

/// Announcement priority (maps to platform announcement priorities).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SpeechPriority {
    Low,
    Default,
    High,
}

/// A spoken message (already localized by the core).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Speech {
    pub text: String,
    pub priority: SpeechPriority,
}

/// A short non-speech audio cue.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Earcon {
    Connected,
    Disconnected,
    KitChanged,
    Confirmed,
    Error,
}

/// Something the host should surface to the user.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CoreEvent {
    ConnectionChanged { state: ConnectionState },
    DeviceIdentified { device: DeviceInfo },
    CurrentKitChanged { number: u32, name: String },
    EditConfirmed { param_id: String, display: String },
    EditFailed { param_id: String, reason: String },
    Speak { speech: Speech },
    Earcon { earcon: Earcon },
    Error { message: String },
}

/// A side effect the host must perform (the engine is sans-I/O — see ADR-0008).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Effect {
    /// Send these raw MIDI bytes to the module.
    SendMidi { bytes: Vec<u8> },
    /// Call `tick` again after roughly this many milliseconds.
    ScheduleTick { after_ms: u64 },
    /// Forward this event to the UI / listener.
    Emit { event: CoreEvent },
}

// ── conversions from engine types ──

impl From<engine::ConnectionState> for ConnectionState {
    fn from(s: engine::ConnectionState) -> Self {
        match s {
            engine::ConnectionState::Disconnected => Self::Disconnected,
            engine::ConnectionState::Identifying => Self::Identifying,
            engine::ConnectionState::Ready => Self::Ready,
        }
    }
}

impl From<engine::FirmwareSupport> for FirmwareSupport {
    fn from(s: engine::FirmwareSupport) -> Self {
        match s {
            engine::FirmwareSupport::Tested => Self::Tested,
            engine::FirmwareSupport::UntestedNewer => Self::UntestedNewer,
            engine::FirmwareSupport::UntestedOlder => Self::UntestedOlder,
            engine::FirmwareSupport::Unknown => Self::Unknown,
        }
    }
}

impl From<engine::DeviceInfo> for DeviceInfo {
    fn from(d: engine::DeviceInfo) -> Self {
        Self {
            model_id: d.model_id,
            device_id: d.device_id,
            name: d.name,
            firmware: d.firmware,
            firmware_support: d.firmware_support.into(),
            profile_id: d.profile_id,
            recognized: d.recognized,
        }
    }
}

impl From<engine::SpeechPriority> for SpeechPriority {
    fn from(p: engine::SpeechPriority) -> Self {
        match p {
            engine::SpeechPriority::Low => Self::Low,
            engine::SpeechPriority::Default => Self::Default,
            engine::SpeechPriority::High => Self::High,
        }
    }
}

impl From<engine::Speech> for Speech {
    fn from(s: engine::Speech) -> Self {
        Self {
            text: s.text,
            priority: s.priority.into(),
        }
    }
}

impl From<engine::Earcon> for Earcon {
    fn from(e: engine::Earcon) -> Self {
        match e {
            engine::Earcon::Connected => Self::Connected,
            engine::Earcon::Disconnected => Self::Disconnected,
            engine::Earcon::KitChanged => Self::KitChanged,
            engine::Earcon::Confirmed => Self::Confirmed,
            engine::Earcon::Error => Self::Error,
        }
    }
}

impl From<engine::CoreEvent> for CoreEvent {
    fn from(e: engine::CoreEvent) -> Self {
        match e {
            engine::CoreEvent::ConnectionChanged(state) => Self::ConnectionChanged {
                state: state.into(),
            },
            engine::CoreEvent::DeviceIdentified(device) => Self::DeviceIdentified {
                device: device.into(),
            },
            engine::CoreEvent::CurrentKitChanged { number, name } => {
                Self::CurrentKitChanged { number, name }
            }
            engine::CoreEvent::EditConfirmed { param_id, display } => {
                Self::EditConfirmed { param_id, display }
            }
            engine::CoreEvent::EditFailed { param_id, reason } => {
                Self::EditFailed { param_id, reason }
            }
            engine::CoreEvent::Speak(speech) => Self::Speak {
                speech: speech.into(),
            },
            engine::CoreEvent::Earcon(earcon) => Self::Earcon {
                earcon: earcon.into(),
            },
            engine::CoreEvent::Error(message) => Self::Error { message },
        }
    }
}

impl From<engine::Effect> for Effect {
    fn from(e: engine::Effect) -> Self {
        match e {
            engine::Effect::SendMidi(bytes) => Self::SendMidi { bytes },
            engine::Effect::ScheduleTick { after_ms } => Self::ScheduleTick { after_ms },
            engine::Effect::Emit(event) => Self::Emit {
                event: event.into(),
            },
        }
    }
}
