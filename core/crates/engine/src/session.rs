//! The sans-I/O session: a pure state machine. Inputs are `on_connected`,
//! `on_disconnected`, `handle_midi_input`, `tick`, and (later) user intents; each
//! returns a `Vec<Effect>` the host performs. No I/O, threads, or timers here.
//! See ADR-0008 and docs/DEVELOPMENT.md §4.4, §7.

use crate::event::{
    ConnectionState, CoreEvent, DeviceInfo, Earcon, Effect, Speech, SpeechPriority,
};
use device::{DeviceProfile, FirmwareSupport, FirmwareVersion, ProfileRegistry};
use model::{Localizer, Message, format_kit, format_parameter};
use std::collections::HashMap;
use sysex::SysexMessage;
use sysex::encoding::decode_ascii;

/// Device ID used to broadcast the Identity Request (any unit responds).
const IDENTITY_DEVICE_ID: u8 = 0x7F;
/// How often to poll the active kit (`Current`).
const POLL_INTERVAL_MS: u64 = 300;

/// What an outstanding RQ1 reply means when its DT1 comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    CurrentKitNum,
    KitName(u32),
    Tempo(u32),
}

/// The connection + read state machine for one device.
pub struct Session {
    registry: ProfileRegistry,
    localizer: Localizer,
    locale: String,
    reassembler: sysex::SysexReassembler,
    state: ConnectionState,
    device_id: u8,
    profile: Option<DeviceProfile>,
    current_kit: Option<u32>,
    pending: HashMap<[u8; 4], Pending>,
}

impl Session {
    /// Create a session for the given UI/speech locale (e.g. "en" or "ru").
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            registry: ProfileRegistry::with_builtin(),
            localizer: Localizer::new(),
            locale: locale.into(),
            reassembler: sysex::SysexReassembler::new(),
            state: ConnectionState::Disconnected,
            device_id: IDENTITY_DEVICE_ID,
            profile: None,
            current_kit: None,
            pending: HashMap::new(),
        }
    }

    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.locale = locale.into();
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// The transport opened — start identifying the module.
    pub fn on_connected(&mut self) -> Vec<Effect> {
        self.state = ConnectionState::Identifying;
        self.profile = None;
        self.current_kit = None;
        self.pending.clear();
        vec![
            Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Identifying)),
            Effect::SendMidi(sysex::build_identity_request(IDENTITY_DEVICE_ID)),
            Effect::ScheduleTick {
                after_ms: POLL_INTERVAL_MS * 3,
            },
        ]
    }

    /// The transport closed — reset to disconnected.
    pub fn on_disconnected(&mut self) -> Vec<Effect> {
        self.state = ConnectionState::Disconnected;
        self.profile = None;
        self.current_kit = None;
        self.pending.clear();
        vec![
            Effect::Emit(CoreEvent::Earcon(Earcon::Disconnected)),
            Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Disconnected)),
        ]
    }

    /// Periodic tick. While identifying, retry the handshake; while ready, poll
    /// the current kit. (`now_ms` is reserved for timeouts, task #9.)
    pub fn tick(&mut self, now_ms: u64) -> Vec<Effect> {
        let _ = now_ms;
        match self.state {
            ConnectionState::Identifying => vec![
                Effect::SendMidi(sysex::build_identity_request(IDENTITY_DEVICE_ID)),
                Effect::ScheduleTick {
                    after_ms: POLL_INTERVAL_MS * 3,
                },
            ],
            ConnectionState::Ready => {
                let mut fx = self.poll_current();
                fx.push(Effect::ScheduleTick {
                    after_ms: POLL_INTERVAL_MS,
                });
                fx
            }
            ConnectionState::Disconnected => vec![],
        }
    }

    /// Feed inbound MIDI bytes (may be fragmented across calls).
    pub fn handle_midi_input(&mut self, bytes: &[u8]) -> Vec<Effect> {
        let model_id: Vec<u8> = self
            .profile
            .as_ref()
            .map(|p| p.model_id.clone())
            .unwrap_or_default();
        let messages = self.reassembler.push_slice(bytes);
        let mut effects = Vec::new();
        for msg in messages {
            match sysex::parse(&msg, &model_id) {
                Ok(SysexMessage::IdentityReply {
                    device_id,
                    manufacturer_id,
                    family,
                    member,
                    version,
                }) => {
                    effects.extend(self.handle_identity(
                        device_id,
                        manufacturer_id,
                        family,
                        member,
                        version,
                    ));
                }
                Ok(SysexMessage::Dt1 { address, data, .. }) => {
                    effects.extend(self.handle_dt1(address, &data));
                }
                _ => {}
            }
        }
        effects
    }

    // ── internals ──

    fn handle_identity(
        &mut self,
        device_id: u8,
        manufacturer: u8,
        family: [u8; 2],
        member: [u8; 2],
        version: [u8; 4],
    ) -> Vec<Effect> {
        let fw = FirmwareVersion::new(version);
        match self
            .registry
            .match_identity(manufacturer, family, member)
            .cloned()
        {
            Some(profile) => {
                let support = profile.firmware_support(fw);
                let name = profile.display_name.clone();
                let info = DeviceInfo {
                    model_id: profile.model_id.clone(),
                    device_id,
                    name: name.clone(),
                    firmware: fw.display(),
                    firmware_support: support,
                    profile_id: profile.profile_id.clone(),
                    recognized: true,
                };
                self.profile = Some(profile);
                self.device_id = device_id;
                self.state = ConnectionState::Ready;
                self.current_kit = None;
                self.pending.clear();

                let mut speech = self.render(
                    &Message::new("device.connected")
                        .arg("device", name.as_str())
                        .arg("firmware", fw.display()),
                );
                if !support.is_tested() {
                    speech.push(' ');
                    speech.push_str(&self.render(&Message::new("device.firmware_untested")));
                }

                let mut fx = vec![
                    Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Ready)),
                    Effect::Emit(CoreEvent::DeviceIdentified(info)),
                    Effect::Emit(CoreEvent::Earcon(Earcon::Connected)),
                    self.speak(speech, SpeechPriority::High),
                ];
                fx.extend(self.poll_current());
                fx.push(Effect::ScheduleTick {
                    after_ms: POLL_INTERVAL_MS,
                });
                fx
            }
            None => {
                self.profile = None;
                self.device_id = device_id;
                self.state = ConnectionState::Ready;
                let info = DeviceInfo {
                    model_id: Vec::new(),
                    device_id,
                    name: "Unknown device".to_string(),
                    firmware: fw.display(),
                    firmware_support: FirmwareSupport::Unknown,
                    profile_id: String::new(),
                    recognized: false,
                };
                let speech = self.render(&Message::new("device.unrecognized"));
                vec![
                    Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Ready)),
                    Effect::Emit(CoreEvent::DeviceIdentified(info)),
                    Effect::Emit(CoreEvent::Earcon(Earcon::Connected)),
                    self.speak(speech, SpeechPriority::High),
                ]
            }
        }
    }

    fn handle_dt1(&mut self, address: [u8; 4], data: &[u8]) -> Vec<Effect> {
        if let Some(pending) = self.pending.remove(&address) {
            self.handle_pending(pending, data)
        } else {
            self.handle_unsolicited(address, data)
        }
    }

    fn handle_pending(&mut self, pending: Pending, data: &[u8]) -> Vec<Effect> {
        match pending {
            Pending::CurrentKitNum => {
                let decoded = self.profile.as_ref().and_then(|p| decode_kit_num(p, data));
                match decoded {
                    Some(num) if Some(num) != self.current_kit => self.on_kit_changed(num),
                    _ => Vec::new(),
                }
            }
            Pending::KitName(number) => {
                let name = decode_ascii(data);
                let speech = self.render(&format_kit(number + 1, &name));
                vec![
                    Effect::Emit(CoreEvent::CurrentKitChanged { number, name }),
                    self.speak(speech, SpeechPriority::Default),
                ]
            }
            Pending::Tempo(_) => self.speak_tempo(data, SpeechPriority::Low),
        }
    }

    /// Unsolicited DT1 (e.g. a hardware edit pushed via Transmit Edit Data):
    /// best-effort match against the active kit's known addresses.
    fn handle_unsolicited(&mut self, address: [u8; 4], data: &[u8]) -> Vec<Effect> {
        let Some(kit) = self.current_kit else {
            return Vec::new();
        };
        let (cur_addr, name_addr, tempo_addr) = {
            let Some(p) = self.profile.as_ref() else {
                return Vec::new();
            };
            (
                p.address_of("current.kit_num", &[]),
                p.address_of("kit.common.name", &[kit]),
                p.address_of("kit.common.tempo", &[kit]),
            )
        };

        if Some(address) == cur_addr {
            let decoded = self.profile.as_ref().and_then(|p| decode_kit_num(p, data));
            match decoded {
                Some(num) if Some(num) != self.current_kit => self.on_kit_changed(num),
                _ => Vec::new(),
            }
        } else if Some(address) == name_addr {
            let name = decode_ascii(data);
            let speech = self.render(&format_kit(kit + 1, &name));
            vec![
                Effect::Emit(CoreEvent::CurrentKitChanged { number: kit, name }),
                self.speak(speech, SpeechPriority::Default),
            ]
        } else if Some(address) == tempo_addr {
            self.speak_tempo(data, SpeechPriority::Low)
        } else {
            Vec::new()
        }
    }

    fn on_kit_changed(&mut self, number: u32) -> Vec<Effect> {
        self.current_kit = Some(number);
        let mut fx = vec![Effect::Emit(CoreEvent::Earcon(Earcon::KitChanged))];
        if let Some(e) = self.request_read("kit.common.name", &[number], Pending::KitName(number)) {
            fx.push(e);
        }
        if let Some(e) = self.request_read("kit.common.tempo", &[number], Pending::Tempo(number)) {
            fx.push(e);
        }
        fx
    }

    fn poll_current(&mut self) -> Vec<Effect> {
        self.request_read("current.kit_num", &[], Pending::CurrentKitNum)
            .into_iter()
            .collect()
    }

    /// Build an RQ1 for `param_id` at `indices` and remember what its reply means.
    fn request_read(
        &mut self,
        param_id: &str,
        indices: &[u32],
        pending: Pending,
    ) -> Option<Effect> {
        let (addr, len, model_id) = {
            let p = self.profile.as_ref()?;
            let addr = p.address_of(param_id, indices)?;
            let len = p.parameter(param_id)?.len;
            (addr, len, p.model_id.clone())
        };
        self.pending.insert(addr, pending);
        Some(Effect::SendMidi(sysex::build_rq1(
            self.device_id,
            &model_id,
            addr,
            rq_size(len),
        )))
    }

    fn speak_tempo(&self, data: &[u8], priority: SpeechPriority) -> Vec<Effect> {
        let message = {
            let Some(p) = self.profile.as_ref() else {
                return Vec::new();
            };
            let Some(def) = p.parameter("kit.common.tempo") else {
                return Vec::new();
            };
            let Some(raw) = def.encoding.decode_int(data) else {
                return Vec::new();
            };
            format_parameter(def, raw)
        };
        vec![self.speak(self.render(&message), priority)]
    }

    fn render(&self, message: &Message) -> String {
        self.localizer.format(message, &self.locale)
    }

    fn speak(&self, text: String, priority: SpeechPriority) -> Effect {
        Effect::Emit(CoreEvent::Speak(Speech { text, priority }))
    }
}

fn decode_kit_num(profile: &DeviceProfile, data: &[u8]) -> Option<u32> {
    let def = profile.parameter("current.kit_num")?;
    u32::try_from(def.encoding.decode_int(data)?).ok()
}

fn rq_size(len: usize) -> [u8; 4] {
    [0, 0, 0, (len.min(0x7F)) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysex::Encoding;

    /// A fake V31 that answers Identity Requests and RQ1 reads consistently with
    /// the embedded profile.
    struct FakeModule {
        profile: DeviceProfile,
        device_id: u8,
        version: [u8; 4],
        current_kit: u32,
        kits: HashMap<u32, (String, i64)>,
    }

    impl FakeModule {
        fn v31() -> Self {
            let profile = ProfileRegistry::with_builtin()
                .match_model(&[1, 6, 1])
                .unwrap()
                .clone();
            let mut kits = HashMap::new();
            kits.insert(4, ("Jazz".to_string(), 1200));
            kits.insert(0, ("Rock".to_string(), 1400));
            Self {
                profile,
                device_id: 0x10,
                version: [0, 2, 0, 0],
                current_kit: 4,
                kits,
            }
        }

        fn identity_reply(&self) -> Vec<u8> {
            let v = self.version;
            vec![
                0xF0,
                0x7E,
                self.device_id,
                0x06,
                0x02,
                0x41,
                0x01,
                0x06,
                0x03,
                0x00,
                v[0],
                v[1],
                v[2],
                v[3],
                0xF7,
            ]
        }

        fn respond(&self, midi: &[u8]) -> Vec<Vec<u8>> {
            if midi.len() == 6
                && midi[0] == 0xF0
                && midi[1] == 0x7E
                && midi[3] == 0x06
                && midi[4] == 0x01
            {
                return vec![self.identity_reply()];
            }
            if midi.len() >= 15 && midi[0] == 0xF0 && midi[1] == 0x41 && midi[6] == 0x11 {
                let addr = [midi[7], midi[8], midi[9], midi[10]];
                if let Some(dt1) = self.data_at(addr) {
                    return vec![dt1];
                }
            }
            Vec::new()
        }

        fn data_at(&self, addr: [u8; 4]) -> Option<Vec<u8>> {
            let p = &self.profile;
            if Some(addr) == p.address_of("current.kit_num", &[]) {
                let data = Encoding::Nibble.encode_int(i64::from(self.current_kit), 4)?;
                return Some(sysex::build_dt1(self.device_id, &p.model_id, addr, &data));
            }
            if Some(addr) == p.address_of("kit.common.name", &[self.current_kit]) {
                let (name, _) = self.kits.get(&self.current_kit)?;
                let data = sysex::encoding::encode_ascii(name, 16);
                return Some(sysex::build_dt1(self.device_id, &p.model_id, addr, &data));
            }
            if Some(addr) == p.address_of("kit.common.tempo", &[self.current_kit]) {
                let (_, tempo) = self.kits.get(&self.current_kit)?;
                let data = Encoding::Nibble.encode_int(*tempo, 4)?;
                return Some(sysex::build_dt1(self.device_id, &p.model_id, addr, &data));
            }
            None
        }
    }

    /// Run the session against the fake until no more MIDI is sent; collect events.
    fn drive(session: &mut Session, fake: &FakeModule, initial: Vec<Effect>) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        let mut effects = initial;
        for _ in 0..50 {
            let mut midi_out = Vec::new();
            for e in effects.drain(..) {
                match e {
                    Effect::SendMidi(bytes) => midi_out.push(bytes),
                    Effect::Emit(ev) => events.push(ev),
                    Effect::ScheduleTick { .. } => {}
                }
            }
            if midi_out.is_empty() {
                break;
            }
            let mut next = Vec::new();
            for out in midi_out {
                for reply in fake.respond(&out) {
                    next.extend(session.handle_midi_input(&reply));
                }
            }
            effects = next;
        }
        events
    }

    #[test]
    fn connect_identify_and_read_current_kit() {
        let fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        let events = drive(&mut s, &fake, init);

        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::DeviceIdentified(d) if d.recognized && d.name == "Roland V31")));
        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::CurrentKitChanged { number, name } if *number == 4 && name == "Jazz")));
        // Untested firmware (profile's tested list is empty) -> connect speech still High.
        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::Speak(sp) if sp.text.contains("Roland V31") && sp.priority == SpeechPriority::High)));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "Kit 5: Jazz"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "120.0 BPM"))
        );
        assert_eq!(s.state(), ConnectionState::Ready);
    }

    #[test]
    fn speaks_russian_kit_label() {
        let fake = FakeModule::v31();
        let mut s = Session::new("ru");
        let init = s.on_connected();
        let events = drive(&mut s, &fake, init);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "Кит 5: Jazz"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "120.0 уд/мин"))
        );
    }

    #[test]
    fn unknown_module_degrades_without_crashing() {
        let mut s = Session::new("en");
        let _ = s.on_connected();
        // Identity Reply for a different Roland device (family 09 09).
        let reply = vec![
            0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x09, 0x09, 0x00, 0x00, 0, 0, 0, 0, 0xF7,
        ];
        let effects = s.handle_midi_input(&reply);
        let recognized_false = effects.iter().any(|e| {
            matches!(e,
            Effect::Emit(CoreEvent::DeviceIdentified(d)) if !d.recognized)
        });
        assert!(recognized_false);
        assert_eq!(s.state(), ConnectionState::Ready);
    }

    #[test]
    fn hardware_kit_change_is_picked_up() {
        let fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        let _ = drive(&mut s, &fake, init); // now Ready, current kit = 4

        // The module pushes an unsolicited Current change to kit index 0.
        let data = Encoding::Nibble.encode_int(0, 4).unwrap();
        let dt1 = sysex::build_dt1(0x10, &[1, 6, 1], [0, 0, 0, 0], &data);
        let effects = s.handle_midi_input(&dt1);
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::Emit(CoreEvent::Earcon(Earcon::KitChanged))))
        );
    }
}
