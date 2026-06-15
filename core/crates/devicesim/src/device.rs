//! The virtual module: a profile-driven Roland SysEx responder.

use crate::state::DeviceState;
use device::{DeviceProfile, ProfileRegistry};
use sysex::{SysexMessage, address, build_dt1, encoding, parse};

// ── SysEx wire constants (only what the simulator needs to recognise). ──
const SYSEX_START: u8 = 0xF0;
const SYSEX_END: u8 = 0xF7;
const ROLAND_ID: u8 = 0x41;
const UNIVERSAL_NON_REALTIME: u8 = 0x7E;
const GENERAL_INFO: u8 = 0x06;
const IDENTITY_REQUEST: u8 = 0x01;
const IDENTITY_REPLY: u8 = 0x02;
const CMD_RQ1: u8 = 0x11;

/// A value to write into the device, before encoding (mirrors the engine's intent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditValue {
    Int(i64),
    Text(String),
}

/// A virtual Roland module.
///
/// Answers Identity Requests, RQ1 reads and DT1 writes consistently with a
/// [`DeviceProfile`]. It is deliberately *dumb*: reads return the bytes stored at
/// an address (or the RQ1's requested number of zero bytes if nothing was written
/// there yet), and writes store bytes verbatim. There is no per-parameter logic, so
/// a different profile (a future V51/V71) is simulated by data alone — see the
/// generality test below.
pub struct VirtualDevice {
    profile: DeviceProfile,
    device_id: u8,
    firmware: [u8; 4],
    state: DeviceState,
    /// When set, the next DT1 write is acknowledged but **not** stored, so the
    /// follow-up read-back reports the old value (models a rejected edit).
    reject_next: bool,
    /// When `false`, the module answers nothing (models a hung/disconnected unit,
    /// for identity-retry and edit-timeout scenarios).
    responsive: bool,
}

impl VirtualDevice {
    /// Build an un-seeded device for `profile`. Firmware defaults to the profile's
    /// first tested version (or zeros if none).
    pub fn from_profile(profile: DeviceProfile) -> Self {
        let device_id = profile.device_id_default.unwrap_or(0x10);
        let firmware = profile
            .firmware
            .tested
            .first()
            .map(|v| v.0)
            .unwrap_or([0, 0, 0, 0]);
        Self {
            profile,
            device_id,
            firmware,
            state: DeviceState::new(),
            reject_next: false,
            responsive: true,
        }
    }

    /// A V31 seeded like the hardware we test against: current kit 5 (index 4)
    /// "Jazz" at 120.0 BPM, and kit 1 (index 0) "Rock" at 140.0 BPM.
    pub fn v31() -> Self {
        let profile = ProfileRegistry::with_builtin()
            .match_model(&[1, 6, 1])
            .expect("built-in V31 profile")
            .clone();
        let mut dev = Self::from_profile(profile);
        dev.set_current_kit(4)
            .with_kit(4, "Jazz", 1200)
            .with_kit(0, "Rock", 1400);
        dev
    }

    // ── builders / seeding ──

    /// Seed a kit's name and tempo (raw, e.g. 1200 = 120.0 BPM).
    pub fn with_kit(&mut self, index: u32, name: &str, tempo_raw: i64) -> &mut Self {
        self.write_param(
            "kit.common.name",
            &[index],
            EditValue::Text(name.to_string()),
        );
        self.write_param("kit.common.tempo", &[index], EditValue::Int(tempo_raw));
        self
    }

    /// Set which kit the module currently has selected.
    pub fn set_current_kit(&mut self, index: u32) -> &mut Self {
        self.write_param("current.kit_num", &[], EditValue::Int(i64::from(index)))
    }

    /// Override the firmware reported in the Identity Reply.
    pub fn with_firmware(&mut self, bytes: [u8; 4]) -> &mut Self {
        self.firmware = bytes;
        self
    }

    /// Write a parameter's value directly into device memory (profile-encoded).
    pub fn write_param(&mut self, param_id: &str, indices: &[u32], value: EditValue) -> &mut Self {
        if let Some((addr, data)) = self.encode_param(param_id, indices, &value) {
            self.state.write(addr, data);
        }
        self
    }

    // ── test-controllable behaviour ──

    /// Make the next DT1 write a no-op at the storage layer (the edit "didn't take").
    pub fn reject_next_write(&mut self) -> &mut Self {
        self.reject_next = true;
        self
    }

    /// Toggle whether the module answers at all. An unresponsive module lets a test
    /// exercise identity-retry and edit-timeout paths.
    pub fn set_responsive(&mut self, responsive: bool) -> &mut Self {
        self.responsive = responsive;
        self
    }

    pub fn device_id(&self) -> u8 {
        self.device_id
    }

    pub fn profile(&self) -> &DeviceProfile {
        &self.profile
    }

    // ── the wire ──

    /// Respond to one inbound host message: zero or more whole SysEx replies. The
    /// reply bytes are computed *now* (a snapshot of current state), so a delayed
    /// delivery still carries the value the module had when the request arrived —
    /// which is exactly how a stale read-back arises on real hardware.
    pub fn respond(&mut self, host: &[u8]) -> Vec<Vec<u8>> {
        if !self.responsive {
            return Vec::new();
        }
        if is_identity_request(host) {
            return self.identity_reply().into_iter().collect();
        }
        if let Some((addr, size)) = self.parse_rq1(host) {
            let len = address::to_linear(size) as usize;
            let data = self
                .state
                .read(addr)
                .map(<[u8]>::to_vec)
                .unwrap_or_else(|| vec![0u8; len]);
            return vec![build_dt1(
                self.device_id,
                &self.profile.model_id,
                addr,
                &data,
            )];
        }
        if let Ok(SysexMessage::Dt1 { address, data, .. }) = parse(host, &self.profile.model_id) {
            if self.reject_next {
                self.reject_next = false;
            } else {
                self.state.write(address, data);
            }
        }
        Vec::new()
    }

    // ── unsolicited hardware-initiated pushes (Transmit Edit Data) ──

    /// Simulate selecting a kit on the module's own panel: update state and return
    /// the unsolicited DT1 the module pushes to the host.
    pub fn hardware_select_kit(&mut self, index: u32) -> Vec<u8> {
        self.push_param("current.kit_num", &[], EditValue::Int(i64::from(index)))
    }

    /// Simulate editing a parameter on the module's own panel.
    pub fn hardware_edit(&mut self, param_id: &str, indices: &[u32], value: EditValue) -> Vec<u8> {
        self.push_param(param_id, indices, value)
    }

    fn push_param(&mut self, param_id: &str, indices: &[u32], value: EditValue) -> Vec<u8> {
        match self.encode_param(param_id, indices, &value) {
            Some((addr, data)) => {
                self.state.write(addr, data.clone());
                build_dt1(self.device_id, &self.profile.model_id, addr, &data)
            }
            None => Vec::new(),
        }
    }

    // ── internals ──

    fn encode_param(
        &self,
        param_id: &str,
        indices: &[u32],
        value: &EditValue,
    ) -> Option<([u8; 4], Vec<u8>)> {
        let def = self.profile.parameter(param_id)?;
        let addr = self.profile.address_of(param_id, indices)?;
        let data = match value {
            EditValue::Int(v) => def.encoding.encode_int(*v, def.len)?,
            EditValue::Text(s) => encoding::encode_ascii(s, def.len),
        };
        Some((addr, data))
    }

    /// `F0 7E dd 06 02 mm ff ff nn nn vv vv vv vv F7` — built from the profile's
    /// identity fingerprint and the configured firmware bytes (15 bytes total, the
    /// single-manufacturer layout the parser expects).
    fn identity_reply(&self) -> Option<Vec<u8>> {
        let id = self.profile.identity.as_ref()?;
        let v = self.firmware;
        Some(vec![
            SYSEX_START,
            UNIVERSAL_NON_REALTIME,
            self.device_id,
            GENERAL_INFO,
            IDENTITY_REPLY,
            id.manufacturer,
            id.family[0],
            id.family[1],
            id.member[0],
            id.member[1],
            v[0],
            v[1],
            v[2],
            v[3],
            SYSEX_END,
        ])
    }

    /// Extract `(address, size)` from an RQ1 framed for this device's Model ID.
    /// RQ1 isn't a message `sysex::parse` decodes (it's host→device), so we read it
    /// off the wire by position. `None` if it isn't a well-formed RQ1 for us.
    fn parse_rq1(&self, b: &[u8]) -> Option<([u8; 4], [u8; 4])> {
        let model = &self.profile.model_id;
        let cmd_idx = 3 + model.len();
        // F0 41 dev <model> cmd <addr×4> <size×4> sum F7
        if b.len() < cmd_idx + 1 + 8 + 2 {
            return None;
        }
        if b[0] != SYSEX_START || b[1] != ROLAND_ID || b[b.len() - 1] != SYSEX_END {
            return None;
        }
        if &b[3..cmd_idx] != model.as_slice() || b[cmd_idx] != CMD_RQ1 {
            return None;
        }
        let a = cmd_idx + 1;
        let addr = [b[a], b[a + 1], b[a + 2], b[a + 3]];
        let size = [b[a + 4], b[a + 5], b[a + 6], b[a + 7]];
        Some((addr, size))
    }
}

/// `F0 7E dd 06 01 F7` — a Universal Non-realtime Identity Request.
fn is_identity_request(b: &[u8]) -> bool {
    b.len() >= 6
        && b[0] == SYSEX_START
        && b[1] == UNIVERSAL_NON_REALTIME
        && b[3] == GENERAL_INFO
        && b[4] == IDENTITY_REQUEST
        && b[b.len() - 1] == SYSEX_END
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysex::{Encoding, build_identity_request, build_rq1};

    fn rq1(dev: &VirtualDevice, addr: [u8; 4], len: u8) -> Vec<u8> {
        build_rq1(
            dev.device_id(),
            &dev.profile().model_id,
            addr,
            [0, 0, 0, len],
        )
    }

    #[test]
    fn answers_identity_request_with_profile_fingerprint() {
        let dev_id;
        let reply = {
            let mut dev = VirtualDevice::v31();
            dev_id = dev.device_id();
            dev.respond(&build_identity_request(0x7F))
        };
        assert_eq!(reply.len(), 1);
        assert_eq!(
            parse(&reply[0], &[]),
            Ok(SysexMessage::IdentityReply {
                device_id: dev_id,
                manufacturer_id: 0x41,
                family: [0x01, 0x06],
                member: [0x03, 0x00],
                version: [0, 2, 1, 0],
            })
        );
    }

    #[test]
    fn read_back_reflects_seeded_state() {
        let mut dev = VirtualDevice::v31();
        // current kit
        let addr = dev.profile().address_of("current.kit_num", &[]).unwrap();
        let reply = dev.respond(&rq1(&dev, addr, 4));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(Encoding::Nibble.decode_int(&data), Some(4));

        // kit 4 name "Jazz"
        let name_addr = dev.profile().address_of("kit.common.name", &[4]).unwrap();
        let reply = dev.respond(&rq1(&dev, name_addr, 16));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(encoding::decode_ascii(&data), "Jazz");
    }

    #[test]
    fn write_then_read_round_trips() {
        let mut dev = VirtualDevice::v31();
        let addr = dev.profile().address_of("kit.common.tempo", &[4]).unwrap();
        let data = Encoding::Nibble.encode_int(1300, 4).unwrap();
        let write = build_dt1(dev.device_id(), &dev.profile().model_id, addr, &data);
        assert!(dev.respond(&write).is_empty()); // a write yields no reply

        let reply = dev.respond(&rq1(&dev, addr, 4));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(Encoding::Nibble.decode_int(&data), Some(1300));
    }

    #[test]
    fn rejected_write_leaves_the_old_value() {
        let mut dev = VirtualDevice::v31();
        let addr = dev.profile().address_of("kit.common.tempo", &[4]).unwrap();
        dev.reject_next_write();
        let data = Encoding::Nibble.encode_int(1300, 4).unwrap();
        dev.respond(&build_dt1(
            dev.device_id(),
            &dev.profile().model_id,
            addr,
            &data,
        ));

        let reply = dev.respond(&rq1(&dev, addr, 4));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(Encoding::Nibble.decode_int(&data), Some(1200)); // unchanged
    }

    #[test]
    fn unseeded_address_reads_back_zeros_of_requested_size() {
        let mut dev = VirtualDevice::v31();
        // kit 7 (index 6) was never seeded.
        let addr = dev.profile().address_of("kit.common.name", &[6]).unwrap();
        let reply = dev.respond(&rq1(&dev, addr, 16));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(data, vec![0u8; 16]);
        assert_eq!(encoding::decode_ascii(&data), "");
    }

    #[test]
    fn unresponsive_module_answers_nothing() {
        let mut dev = VirtualDevice::v31();
        dev.set_responsive(false);
        assert!(dev.respond(&build_identity_request(0x7F)).is_empty());
        let addr = dev.profile().address_of("current.kit_num", &[]).unwrap();
        assert!(dev.respond(&rq1(&dev, addr, 4)).is_empty());
    }

    #[test]
    fn hardware_select_kit_updates_state_and_returns_a_push() {
        let mut dev = VirtualDevice::v31();
        let push = dev.hardware_select_kit(0);
        // The push is a DT1 to the Current address carrying the new kit.
        let Ok(SysexMessage::Dt1 { address, data, .. }) = parse(&push, &dev.profile().model_id)
        else {
            panic!("expected DT1 push");
        };
        assert_eq!(
            address,
            dev.profile().address_of("current.kit_num", &[]).unwrap()
        );
        assert_eq!(Encoding::Nibble.decode_int(&data), Some(0));
        // And a subsequent read reflects it.
        let addr = dev.profile().address_of("current.kit_num", &[]).unwrap();
        let reply = dev.respond(&rq1(&dev, addr, 4));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(Encoding::Nibble.decode_int(&data), Some(0));
    }

    /// Generality: the simulator has **no** V31-specific code. Drive it with a
    /// throwaway profile whose addresses/encodings differ, and it still answers
    /// reads from what writes stored — proving a new device is data, not code.
    #[test]
    fn works_for_an_arbitrary_profile_with_no_device_specific_code() {
        const SYNTH: &str = r#"{
            "schema_version": 1,
            "profile_id": "synth-mod",
            "display_name": "Synth Module",
            "model_id": [2, 9],
            "device_id_default": 17,
            "identity": { "manufacturer": 65, "family": [7, 7], "member": [1, 2] },
            "firmware": { "tested": [[3, 0, 0, 0]] },
            "areas": {
                "bank": { "address": [5, 0, 0, 0], "stride": [0, 2, 0, 0], "count": 8 }
            },
            "parameters": [
                { "id": "bank.level", "area": "bank", "offset": [0, 10], "len": 2, "encoding": "plain7", "range": { "min": 0, "max": 16383 } }
            ]
        }"#;
        let profile = DeviceProfile::from_json(SYNTH).unwrap();
        let mut dev = VirtualDevice::from_profile(profile);

        // Identity reply carries the synthetic fingerprint + firmware.
        let reply = dev.respond(&build_identity_request(0x7F));
        assert_eq!(
            parse(&reply[0], &[]),
            Ok(SysexMessage::IdentityReply {
                device_id: 17,
                manufacturer_id: 65,
                family: [7, 7],
                member: [1, 2],
                version: [3, 0, 0, 0],
            })
        );

        // Write a 2-byte plain7 value to bank 3 (index 2), read it straight back.
        dev.write_param("bank.level", &[2], EditValue::Int(2356));
        let addr = dev.profile().address_of("bank.level", &[2]).unwrap();
        let reply = dev.respond(&rq1(&dev, addr, 2));
        let Ok(SysexMessage::Dt1 { data, .. }) = parse(&reply[0], &dev.profile().model_id) else {
            panic!("expected DT1");
        };
        assert_eq!(Encoding::Plain7.decode_int(&data), Some(2356));
    }
}
