//! Sans-I/O session engine: a pure state machine driven by inbound events
//! (`on_connected`, `handle_midi_input`, `tick`, user intents) that emits actions
//! (send MIDI, speak, schedule tick, update view-model). The native layer owns
//! all I/O.
//!
//! See ADR-0008 and docs/DEVELOPMENT.md §4.4, §7.
#![forbid(unsafe_code)]

/// Crate version, exposed so the FFI layer can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn links_against_model() {
        assert!(!model::VERSION.is_empty());
    }
}
