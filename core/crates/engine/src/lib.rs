//! Sans-I/O session engine: a pure state machine driven by inbound events
//! (`on_connected`, `handle_midi_input`, `tick`, user intents) that emits effects
//! (send MIDI, speak, schedule tick, emit events). The native layer owns all I/O.
//!
//! See ADR-0008 and docs/DEVELOPMENT.md §4.4, §7.
#![forbid(unsafe_code)]

mod event;
mod session;

pub use event::{
    ConnectionState, CoreEvent, DeviceInfo, Earcon, Effect, FirmwareSupport, Speech, SpeechPriority,
};
pub use session::Session;

/// Crate version, exposed so the FFI layer can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
