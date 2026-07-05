//! Simulated module over FFI (cargo feature `simffi`) — device-mock plan, B1.
//!
//! Exposes [`devicesim::VirtualDevice`] as a UniFFI object so the native layer
//! can run the **full** app pipeline (identify → poll → edit → verify) against
//! the same profile-driven simulated module the Rust e2e harness drives — no
//! hardware needed. The native `SimulatedTransport` feeds host→device bytes in,
//! delivers the replies after the modelled latency, and can poke hardware-side
//! actions (kit dial, knob turns) for tests and dev use.
//!
//! Dev-only: never enabled in shipping builds (`just build-ios-release` builds
//! without this feature and asserts the symbols are absent).

use std::sync::{Arc, Mutex};

use devicesim::{EditValue, TimingProfile, VirtualDevice};

/// A simulated Roland module the native layer can drive like hardware.
///
/// Byte-level contract only: feed complete host→device MIDI messages to
/// [`respond`](Self::respond) and deliver the returned replies after
/// [`reply_delay_ms`](Self::reply_delay_ms) — exactly what a transport does with
/// a real module. State persists across messages (writes stick), so the
/// write→read-back→verify pipeline behaves as on hardware.
#[derive(uniffi::Object)]
pub struct VirtualDeviceHandle {
    device: Mutex<VirtualDevice>,
    timing: TimingProfile,
}

#[uniffi::export]
impl VirtualDeviceHandle {
    /// A virtual V31 (builtin profile, seeded kits) with synthetic latencies.
    #[uniffi::constructor]
    pub fn v31() -> Arc<Self> {
        Arc::new(Self {
            device: Mutex::new(VirtualDevice::v31()),
            timing: TimingProfile::synthetic(),
        })
    }

    /// Feed one complete host→device MIDI message; returns the module's replies
    /// (Identity Reply, RQ1 read-backs), in order. Writes are stored (and
    /// persist), unknown reads answer zeros — like the real module.
    pub fn respond(&self, host_msg: Vec<u8>) -> Vec<Vec<u8>> {
        self.device.lock().unwrap().respond(&host_msg)
    }

    /// How long the module takes to answer `request`, in milliseconds.
    pub fn reply_delay_ms(&self, request: Vec<u8>) -> u64 {
        self.timing.reply_delay_ms(&request)
    }

    /// The delay before an unsolicited hardware push lands, in milliseconds.
    pub fn push_delay_ms(&self) -> u64 {
        self.timing.unsolicited_push_ms
    }

    /// The user turns the kit dial on the module: updates the simulated state and
    /// returns the unsolicited DT1 push the module would transmit.
    pub fn hardware_select_kit(&self, index: u32) -> Vec<u8> {
        self.device.lock().unwrap().hardware_select_kit(index)
    }

    /// The user edits an integer parameter on the module: updates the simulated
    /// state and returns the unsolicited DT1 push (Transmit Edit Data).
    pub fn hardware_edit_int(&self, param_id: String, indices: Vec<u32>, value: i64) -> Vec<u8> {
        self.device
            .lock()
            .unwrap()
            .hardware_edit(&param_id, &indices, EditValue::Int(value))
    }

    /// Seed a kit's name and raw tempo (test/dev setup).
    pub fn seed_kit(&self, index: u32, name: String, tempo_raw: i64) {
        self.device
            .lock()
            .unwrap()
            .with_kit(index, &name, tempo_raw);
    }

    /// Make the given kit current without emitting a push (test/dev setup).
    pub fn set_current_kit(&self, index: u32) {
        self.device.lock().unwrap().set_current_kit(index);
    }

    /// Fault injection: an unresponsive module (no replies at all).
    pub fn set_responsive(&self, responsive: bool) {
        self.device.lock().unwrap().set_responsive(responsive);
    }

    /// Fault injection: the module ignores the next DT1 write (read-back then
    /// reports the old value → the edit pipeline detects the mismatch).
    pub fn reject_next_write(&self) {
        self.device.lock().unwrap().reject_next_write();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responds_to_an_identity_request_like_the_device() {
        let handle = VirtualDeviceHandle::v31();
        let replies = handle.respond(vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0][3..5], [0x06, 0x02], "an Identity Reply");
        assert!(handle.reply_delay_ms(vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]) > 0);
    }

    #[test]
    fn seeded_state_flows_through_pushes() {
        let handle = VirtualDeviceHandle::v31();
        handle.seed_kit(7, "Latin".to_string(), 1100);
        let push = handle.hardware_select_kit(7);
        assert!(!push.is_empty(), "a hardware kit change emits a DT1 push");
    }
}
