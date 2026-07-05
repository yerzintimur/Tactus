//! UniFFI public API for the Tactus core — built as `libtactus`, consumed by the
//! native apps via generated Swift/Kotlin bindings.
//!
//! Design: every method returns `Vec<Effect>` and the host performs them (send
//! MIDI, schedule a tick, forward events). This is faithful to the sans-I/O engine
//! (ADR-0008) — no callback interfaces needed; the host drives execution.
//!
//! This is the one crate that can't `#![forbid(unsafe_code)]`: UniFFI's generated
//! scaffolding uses `unsafe extern "C"`. Our own code here stays unsafe-free.

#[cfg(feature = "simffi")]
mod sim;
mod types;

#[cfg(feature = "simffi")]
pub use sim::VirtualDeviceHandle;
pub use types::{
    ConnectionState, CoreEvent, DeviceInfo, Earcon, Effect, FirmwareSupport, KitRef, NumericInfo,
    NumericRange, ParamKind, ParamValue, ParameterView, Snapshot, Speech, SpeechCategory,
    SpeechPriority, SpeechSource,
};

use std::sync::{Arc, Mutex};

uniffi::setup_scaffolding!();

/// The Tactus session — a thin, thread-safe wrapper over the sans-I/O engine.
#[derive(uniffi::Object)]
pub struct TactusSession {
    inner: Mutex<engine::Session>,
}

#[uniffi::export]
impl TactusSession {
    /// Create a session for the given UI/speech locale (e.g. "en" or "ru").
    #[uniffi::constructor]
    pub fn new(locale: String) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(engine::Session::new(locale)),
        })
    }

    /// Change the UI/speech locale.
    pub fn set_locale(&self, locale: String) {
        self.locked().set_locale(locale);
    }

    /// The transport opened.
    pub fn on_connected(&self) -> Vec<Effect> {
        self.run(engine::Session::on_connected)
    }

    /// The transport closed.
    pub fn on_disconnected(&self) -> Vec<Effect> {
        self.run(engine::Session::on_disconnected)
    }

    /// Feed inbound MIDI bytes (may be fragmented across calls).
    pub fn handle_midi_input(&self, bytes: Vec<u8>) -> Vec<Effect> {
        self.run(|s| s.handle_midi_input(&bytes))
    }

    /// Drive timers/polling; `now_ms` is a monotonic millisecond clock.
    pub fn tick(&self, now_ms: u64) -> Vec<Effect> {
        self.run(|s| s.tick(now_ms))
    }

    /// Switch the active kit (0-based), verified by read-back.
    pub fn select_kit(&self, number: u32) -> Vec<Effect> {
        self.run(|s| s.select_kit(number))
    }

    /// Set a numeric parameter (raw value), verified by read-back.
    pub fn set_parameter(&self, param_id: String, indices: Vec<u32>, value: i64) -> Vec<Effect> {
        self.run(|s| s.set_parameter(param_id, indices, value))
    }

    /// Rename a kit, verified by read-back.
    pub fn rename_kit(&self, number: u32, name: String) -> Vec<Effect> {
        self.run(|s| s.rename_kit(number, name))
    }

    /// Pull the current observable state (connection, device, active kit, and the
    /// active device's parameters with last-known values + presentation metadata).
    /// Complements the event stream — call it when (re)building UI such as an editor.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot::from(self.locked().snapshot())
    }
}

impl TactusSession {
    fn locked(&self) -> std::sync::MutexGuard<'_, engine::Session> {
        self.inner
            .lock()
            .expect("Tactus session mutex is not poisoned")
    }

    /// Run an engine call under the lock and map its effects to FFI types.
    fn run(&self, f: impl FnOnce(&mut engine::Session) -> Vec<engine::Effect>) -> Vec<Effect> {
        let mut guard = self.locked();
        f(&mut guard).into_iter().map(Effect::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_connected_requests_identity() {
        let session = TactusSession::new("en".to_string());
        let effects = session.on_connected();
        assert!(effects.iter().any(|e| matches!(e, Effect::SendMidi { .. })));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::Emit {
                event: CoreEvent::ConnectionChanged {
                    state: ConnectionState::Identifying
                }
            }
        )));
    }

    #[test]
    fn identity_reply_yields_recognized_device() {
        let session = TactusSession::new("en".to_string());
        let _ = session.on_connected();
        // V31 Identity Reply: Roland 0x41, family 01 06, member 03 00.
        let reply = vec![
            0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x01, 0x06, 0x03, 0x00, 0x00, 0x02, 0x00, 0x00,
            0xF7,
        ];
        let effects = session.handle_midi_input(reply);
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::Emit { event: CoreEvent::DeviceIdentified { device } }
                if device.recognized && device.name == "Roland V31"
        )));
    }

    #[test]
    fn wrapper_surface_and_failure_conversions() {
        let session = TactusSession::new("en".to_string());
        let _ = session.on_connected();
        // While identifying, a tick retries the handshake.
        assert!(
            session
                .tick(0)
                .iter()
                .any(|e| matches!(e, Effect::SendMidi { .. }))
        );

        // An edit before a device is identified -> EditFailed + Speak + Earcon
        // (exercises those From conversions through the FFI boundary).
        let fx = session.set_parameter("kit.common.tempo".to_string(), vec![0], 1200);
        assert!(fx.iter().any(|e| matches!(
            e,
            Effect::Emit {
                event: CoreEvent::EditFailed { .. }
            }
        )));
        assert!(fx.iter().any(|e| matches!(
            e,
            Effect::Emit {
                event: CoreEvent::Speak { .. }
            }
        )));
        assert!(fx.iter().any(|e| matches!(
            e,
            Effect::Emit {
                event: CoreEvent::Earcon { .. }
            }
        )));

        // Smoke the rest of the surface — no panics; exercises the wrappers.
        let _ = session.select_kit(0);
        let _ = session.rename_kit(0, "X".to_string());
        session.set_locale("ru".to_string());
        let fx = session.on_disconnected();
        assert!(fx.iter().any(|e| matches!(
            e,
            Effect::Emit {
                event: CoreEvent::Earcon {
                    earcon: Earcon::Disconnected
                }
            }
        )));
    }

    #[test]
    fn snapshot_crosses_the_ffi_boundary() {
        let session = TactusSession::new("en".to_string());
        assert_eq!(session.snapshot().connection, ConnectionState::Disconnected);

        let _ = session.on_connected();
        assert_eq!(session.snapshot().connection, ConnectionState::Identifying);

        // V31 Identity Reply -> Ready + recognized device.
        let reply = vec![
            0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x01, 0x06, 0x03, 0x00, 0x00, 0x02, 0x00, 0x00,
            0xF7,
        ];
        let _ = session.handle_midi_input(reply);

        let snap = session.snapshot();
        assert_eq!(snap.connection, ConnectionState::Ready);
        assert!(
            snap.device
                .is_some_and(|d| d.recognized && d.name == "Roland V31")
        );
        // The profile's parameters are surfaced with metadata even before a value
        // has been read back (value stays None until polling fills the cache).
        let tempo = snap
            .parameters
            .iter()
            .find(|p| p.param_id == "kit.common.tempo")
            .expect("tempo view from the V31 profile");
        assert_eq!(tempo.label, "Tempo");
        assert!(matches!(tempo.kind, ParamKind::Numeric));
        assert!(tempo.value.is_none());
        assert!(
            tempo
                .numeric
                .as_ref()
                .and_then(|n| n.range.as_ref())
                .is_some()
        );
    }
}
