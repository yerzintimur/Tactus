//! The sans-I/O session: a pure state machine. Inputs are `on_connected`,
//! `on_disconnected`, `handle_midi_input`, `tick`, and (later) user intents; each
//! returns a `Vec<Effect>` the host performs. No I/O, threads, or timers here.
//! See ADR-0008 and docs/DEVELOPMENT.md §4.4, §7.

use crate::event::{
    ConnectionState, CoreEvent, DeviceInfo, Earcon, Effect, Speech, SpeechPriority,
};
use crate::viewmodel::{self, KitRef, ParamKind, ParamValue, ParameterView, Snapshot};
use device::{DeviceProfile, FirmwareSupport, FirmwareVersion, ProfileRegistry};
use model::{Localizer, Message, format_kit, format_parameter, format_parameter_label};
use std::collections::HashMap;
use sysex::SysexMessage;
use sysex::encoding::decode_ascii;

/// Device ID used to broadcast the Identity Request (any unit responds).
const IDENTITY_DEVICE_ID: u8 = 0x7F;
/// How often to poll the active kit (`Current`).
const POLL_INTERVAL_MS: u64 = 300;

/// An edit times out after this many ticks without a confirming read-back.
const EDIT_TIMEOUT_TICKS: u32 = 5;

/// What an outstanding RQ1 reply means when its DT1 comes back.
#[derive(Debug, Clone)]
enum Pending {
    CurrentKitNum,
    KitName(u32),
    Tempo,
    /// Read-back of an edit, awaiting verification.
    EditVerify(Edit),
}

/// An in-flight edit awaiting write → read-back → verify.
#[derive(Debug, Clone)]
struct Edit {
    param_id: String,
    intended: EditValue,
    age: u32,
    is_kit_select: bool,
}

/// The intended value of an edit (raw, pre-encoding).
#[derive(Debug, Clone)]
enum EditValue {
    Int(i64),
    Text(String),
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
    /// The identified module (cached so `snapshot` can report it after the
    /// one-shot `DeviceIdentified` event).
    device_info: Option<DeviceInfo>,
    current_kit: Option<u32>,
    /// Read-through cache of the active kit's last device-confirmed parameter
    /// values, keyed by `param_id`. Refreshed by polling / edit read-backs and
    /// cleared on kit change / disconnect. Never holds intended (unverified)
    /// values — the device is the source of truth (ADR-0010).
    values: HashMap<String, ParamValue>,
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
            device_info: None,
            current_kit: None,
            values: HashMap::new(),
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
        self.device_info = None;
        self.current_kit = None;
        self.values.clear();
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
        self.device_info = None;
        self.current_kit = None;
        self.values.clear();
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
                let mut fx = self.age_edits();
                fx.extend(self.poll_current());
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
                self.device_info = Some(info.clone());
                self.device_id = device_id;
                self.state = ConnectionState::Ready;
                self.current_kit = None;
                self.values.clear();
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
                self.current_kit = None;
                self.values.clear();
                self.pending.clear();
                let info = DeviceInfo {
                    model_id: Vec::new(),
                    device_id,
                    name: "Unknown device".to_string(),
                    firmware: fw.display(),
                    firmware_support: FirmwareSupport::Unknown,
                    profile_id: String::new(),
                    recognized: false,
                };
                self.device_info = Some(info.clone());
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
                self.values.insert(
                    "kit.common.name".to_string(),
                    ParamValue::Text(name.clone()),
                );
                let speech = self.render(&format_kit(number + 1, &name));
                vec![
                    Effect::Emit(CoreEvent::CurrentKitChanged { number, name }),
                    self.speak(speech, SpeechPriority::Default),
                ]
            }
            Pending::Tempo => self.speak_tempo(data, SpeechPriority::Low),
            Pending::EditVerify(edit) => self.handle_edit_verify(edit, data),
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
            self.values.insert(
                "kit.common.name".to_string(),
                ParamValue::Text(name.clone()),
            );
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
        // New kit: the previous kit's cached values no longer apply. The name and
        // tempo reads below repopulate the cache for the new kit.
        self.values.clear();
        self.values.insert(
            "current.kit_num".to_string(),
            ParamValue::Int(i64::from(number)),
        );
        let mut fx = vec![Effect::Emit(CoreEvent::Earcon(Earcon::KitChanged))];
        if let Some(e) = self.request_read("kit.common.name", &[number], Pending::KitName(number)) {
            fx.push(e);
        }
        if let Some(e) = self.request_read("kit.common.tempo", &[number], Pending::Tempo) {
            fx.push(e);
        }
        fx
    }

    fn poll_current(&mut self) -> Vec<Effect> {
        // Don't poll over an in-flight edit: a Current poll would clobber an edit's
        // pending read-back (they key the same address) and lose the verification.
        if self
            .pending
            .values()
            .any(|p| matches!(p, Pending::EditVerify(_)))
        {
            return Vec::new();
        }
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

    fn speak_tempo(&mut self, data: &[u8], priority: SpeechPriority) -> Vec<Effect> {
        let (message, raw) = {
            let Some(p) = self.profile.as_ref() else {
                return Vec::new();
            };
            let Some(def) = p.parameter("kit.common.tempo") else {
                return Vec::new();
            };
            let Some(raw) = def.encoding.decode_int(data) else {
                return Vec::new();
            };
            (format_parameter(def, raw), raw)
        };
        self.values
            .insert("kit.common.tempo".to_string(), ParamValue::Int(raw));
        vec![self.speak(self.render(&message), priority)]
    }

    // ── edits: write → read-back → verify (no blind writes) ──

    /// Switch the active kit (0-based number), verified by read-back.
    pub fn select_kit(&mut self, number: u32) -> Vec<Effect> {
        self.set_value(
            "current.kit_num",
            &[],
            EditValue::Int(i64::from(number)),
            true,
        )
    }

    /// Set a numeric parameter to a raw value, verified by read-back.
    pub fn set_parameter(
        &mut self,
        param_id: String,
        indices: Vec<u32>,
        value: i64,
    ) -> Vec<Effect> {
        self.set_value(&param_id, &indices, EditValue::Int(value), false)
    }

    /// Rename a kit, verified by read-back.
    pub fn rename_kit(&mut self, number: u32, name: String) -> Vec<Effect> {
        self.set_value("kit.common.name", &[number], EditValue::Text(name), false)
    }

    fn set_value(
        &mut self,
        param_id: &str,
        indices: &[u32],
        intended: EditValue,
        is_kit_select: bool,
    ) -> Vec<Effect> {
        let (addr, len, encoding, model_id) = {
            let Some(p) = self.profile.as_ref() else {
                return self.fail_simple("edit.not_ready", param_id);
            };
            let (Some(addr), Some(def)) = (p.address_of(param_id, indices), p.parameter(param_id))
            else {
                return self.fail_simple("edit.not_ready", param_id);
            };
            (addr, def.len, def.encoding, p.model_id.clone())
        };

        let data = match &intended {
            EditValue::Int(v) => match encoding.encode_int(*v, len) {
                Some(bytes) => bytes,
                None => return self.fail_simple("edit.out_of_range", param_id),
            },
            EditValue::Text(s) => sysex::encoding::encode_ascii(s, len),
        };

        self.pending.insert(
            addr,
            Pending::EditVerify(Edit {
                param_id: param_id.to_string(),
                intended,
                age: 0,
                is_kit_select,
            }),
        );
        vec![
            Effect::SendMidi(sysex::build_dt1(self.device_id, &model_id, addr, &data)),
            Effect::SendMidi(sysex::build_rq1(
                self.device_id,
                &model_id,
                addr,
                rq_size(len),
            )),
            Effect::ScheduleTick {
                after_ms: POLL_INTERVAL_MS,
            },
        ]
    }

    fn handle_edit_verify(&mut self, edit: Edit, data: &[u8]) -> Vec<Effect> {
        match &edit.intended {
            EditValue::Int(intended) => {
                let actual = self
                    .profile
                    .as_ref()
                    .and_then(|p| p.parameter(&edit.param_id))
                    .and_then(|def| def.encoding.decode_int(data));
                match actual {
                    Some(a) if a == *intended => self.confirm_int(&edit, a),
                    Some(a) => {
                        let display = self.render_int_value(&edit.param_id, a);
                        self.fail_mismatch(&edit, display)
                    }
                    // Couldn't decode the read-back — treat as "value unknown".
                    None => self.fail_simple("edit.timeout", &edit.param_id),
                }
            }
            EditValue::Text(intended) => {
                let actual = decode_ascii(data);
                if &actual == intended {
                    self.confirm_text(&edit, actual)
                } else {
                    self.fail_mismatch(&edit, actual)
                }
            }
        }
    }

    fn confirm_int(&mut self, edit: &Edit, actual: i64) -> Vec<Effect> {
        if edit.is_kit_select {
            // The kit-change flow (earcon + read name/tempo + speak) is the confirmation.
            return self.on_kit_changed(u32::try_from(actual).unwrap_or(0));
        }
        self.values
            .insert(edit.param_id.clone(), ParamValue::Int(actual));
        let display = self.render_int_value(&edit.param_id, actual);
        vec![
            Effect::Emit(CoreEvent::EditConfirmed {
                param_id: edit.param_id.clone(),
                display: display.clone(),
            }),
            self.speak(display, SpeechPriority::Default),
            Effect::Emit(CoreEvent::Earcon(Earcon::Confirmed)),
        ]
    }

    fn confirm_text(&mut self, edit: &Edit, actual: String) -> Vec<Effect> {
        self.values
            .insert(edit.param_id.clone(), ParamValue::Text(actual.clone()));
        vec![
            Effect::Emit(CoreEvent::EditConfirmed {
                param_id: edit.param_id.clone(),
                display: actual.clone(),
            }),
            self.speak(actual, SpeechPriority::Default),
            Effect::Emit(CoreEvent::Earcon(Earcon::Confirmed)),
        ]
    }

    /// Edit didn't take — announce the **actual** value, never the intended one.
    fn fail_mismatch(&self, edit: &Edit, actual_display: String) -> Vec<Effect> {
        let reason = self.render(&Message::new("edit.mismatch").arg("value", actual_display));
        self.emit_failure(&edit.param_id, reason)
    }

    fn fail_simple(&self, msg_id: &str, param_id: &str) -> Vec<Effect> {
        let reason = self.render(&Message::new(msg_id));
        self.emit_failure(param_id, reason)
    }

    fn emit_failure(&self, param_id: &str, reason: String) -> Vec<Effect> {
        vec![
            Effect::Emit(CoreEvent::EditFailed {
                param_id: param_id.to_string(),
                reason: reason.clone(),
            }),
            self.speak(reason, SpeechPriority::High),
            Effect::Emit(CoreEvent::Earcon(Earcon::Error)),
        ]
    }

    /// Age in-flight edits on each tick; fire a timeout for any that expired.
    fn age_edits(&mut self) -> Vec<Effect> {
        let mut expired = Vec::new();
        for (addr, pending) in self.pending.iter_mut() {
            if let Pending::EditVerify(edit) = pending {
                edit.age += 1;
                if edit.age >= EDIT_TIMEOUT_TICKS {
                    expired.push(*addr);
                }
            }
        }
        let mut fx = Vec::new();
        for addr in expired {
            if let Some(Pending::EditVerify(edit)) = self.pending.remove(&addr) {
                fx.extend(self.fail_simple("edit.timeout", &edit.param_id));
            }
        }
        fx
    }

    /// Localize a numeric parameter's value for speech (e.g. 1300 -> "130.0 BPM").
    fn render_int_value(&self, param_id: &str, value: i64) -> String {
        match self.profile.as_ref().and_then(|p| p.parameter(param_id)) {
            Some(def) => self.render(&format_parameter(def, value)),
            None => value.to_string(),
        }
    }

    // ── pull-side view-model ──

    /// Build a snapshot of the current observable state for the UI. Complements
    /// the `CoreEvent` stream: the host pulls this when it needs the full current
    /// state (e.g. opening an editor). Parameter values are the last device-
    /// confirmed read-backs, never intent. See [`crate::viewmodel`].
    pub fn snapshot(&self) -> Snapshot {
        let parameters = self
            .profile
            .as_ref()
            .map(|p| self.build_parameter_views(p))
            .unwrap_or_default();
        Snapshot {
            connection: self.state,
            device: self.device_info.clone(),
            current_kit: self.current_kit.map(|number| KitRef {
                number,
                display_number: number + 1,
                name: self.text_value("kit.common.name").unwrap_or_default(),
            }),
            parameters,
        }
    }

    fn build_parameter_views(&self, profile: &DeviceProfile) -> Vec<ParameterView> {
        profile
            .parameters
            .iter()
            .map(|def| {
                let kind = ParamKind::of(def);
                let value = self.values.get(&def.id).cloned();
                let display = value.as_ref().map(|v| match v {
                    ParamValue::Int(raw) => self.render(&format_parameter(def, *raw)),
                    ParamValue::Text(text) => text.clone(),
                });
                let numeric =
                    matches!(kind, ParamKind::Numeric).then(|| viewmodel::numeric_info(def));
                ParameterView {
                    param_id: def.id.clone(),
                    label: self.render(&format_parameter_label(def)),
                    kind,
                    value,
                    display,
                    numeric,
                }
            })
            .collect()
    }

    /// The last text value cached for `param_id`, if any.
    fn text_value(&self, param_id: &str) -> Option<String> {
        match self.values.get(param_id) {
            Some(ParamValue::Text(s)) => Some(s.clone()),
            _ => None,
        }
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

        fn respond(&mut self, midi: &[u8]) -> Vec<Vec<u8>> {
            if midi.len() == 6
                && midi[0] == 0xF0
                && midi[1] == 0x7E
                && midi[3] == 0x06
                && midi[4] == 0x01
            {
                return vec![self.identity_reply()];
            }
            if midi.len() >= 12 && midi[0] == 0xF0 && midi[1] == 0x41 {
                let addr = [midi[7], midi[8], midi[9], midi[10]];
                match midi[6] {
                    0x11 => return self.data_at(addr).into_iter().collect(), // RQ1 read
                    0x12 => self.apply_write(addr, &midi[11..midi.len() - 2]), // DT1 write
                    _ => {}
                }
            }
            Vec::new()
        }

        /// Apply a DT1 write so later read-backs reflect it (so verify can confirm).
        fn apply_write(&mut self, addr: [u8; 4], data: &[u8]) {
            let p = self.profile.clone();
            if Some(addr) == p.address_of("current.kit_num", &[]) {
                if let Some(v) = Encoding::Nibble.decode_int(data) {
                    self.current_kit = u32::try_from(v).unwrap_or(0);
                }
            } else if Some(addr) == p.address_of("kit.common.tempo", &[self.current_kit]) {
                if let Some(v) = Encoding::Nibble.decode_int(data)
                    && let Some(kit) = self.kits.get_mut(&self.current_kit)
                {
                    kit.1 = v;
                }
            } else if Some(addr) == p.address_of("kit.common.name", &[self.current_kit]) {
                let name = sysex::encoding::decode_ascii(data);
                if let Some(kit) = self.kits.get_mut(&self.current_kit) {
                    kit.0 = name;
                }
            }
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
    fn drive(session: &mut Session, fake: &mut FakeModule, initial: Vec<Effect>) -> Vec<CoreEvent> {
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
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        let events = drive(&mut s, &mut fake, init);

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
        let mut fake = FakeModule::v31();
        let mut s = Session::new("ru");
        let init = s.on_connected();
        let events = drive(&mut s, &mut fake, init);
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
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        let _ = drive(&mut s, &mut fake, init); // now Ready, current kit = 4

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

    #[test]
    fn set_parameter_confirmed_by_readback() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready, kit 4, tempo 1200

        let fx = s.set_parameter("kit.common.tempo".to_string(), vec![4], 1300);
        let events = drive(&mut s, &mut fake, fx);
        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::EditConfirmed { display, .. } if display == "130.0 BPM")));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "130.0 BPM"))
        );
    }

    #[test]
    fn edit_mismatch_announces_the_actual_value() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init);

        let _ = s.set_parameter("kit.common.tempo".to_string(), vec![4], 1300);
        // Module did NOT apply the write: read-back still reports 1200.
        let addr = ProfileRegistry::with_builtin()
            .match_model(&[1, 6, 1])
            .unwrap()
            .address_of("kit.common.tempo", &[4])
            .unwrap();
        let dt1 = sysex::build_dt1(
            0x10,
            &[1, 6, 1],
            addr,
            &Encoding::Nibble.encode_int(1200, 4).unwrap(),
        );
        let effects = s.handle_midi_input(&dt1);
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::Emit(CoreEvent::EditFailed { .. })))
        );
        // Announces the TRUTH (still 120.0 BPM), never the intended 130.0.
        assert!(effects.iter().any(|e| matches!(e,
            Effect::Emit(CoreEvent::Speak(sp))
                if sp.text.contains("120.0 BPM") && sp.priority == SpeechPriority::High)));
    }

    #[test]
    fn edit_times_out_without_a_readback() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init);

        let _ = s.set_parameter("kit.common.tempo".to_string(), vec![4], 1300);
        let mut timed_out = false;
        for i in 0..=EDIT_TIMEOUT_TICKS {
            let fx = s.tick(u64::from(i) * 300);
            if fx
                .iter()
                .any(|e| matches!(e, Effect::Emit(CoreEvent::EditFailed { .. })))
            {
                timed_out = true;
            }
        }
        assert!(timed_out);
    }

    #[test]
    fn out_of_range_edit_is_rejected_without_sending() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init);

        // 999999 needs more than 4 nibbles (max 65535) -> rejected, nothing sent.
        let fx = s.set_parameter("kit.common.tempo".to_string(), vec![4], 999_999);
        assert!(!fx.iter().any(|e| matches!(e, Effect::SendMidi(_))));
        assert!(fx.iter().any(|e| matches!(e,
            Effect::Emit(CoreEvent::EditFailed { reason, .. }) if reason.contains("range"))));
    }

    #[test]
    fn select_kit_confirmed_reads_the_new_kit() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready on kit 4 ("Jazz")

        let fx = s.select_kit(0); // switch to kit index 0 ("Rock")
        let events = drive(&mut s, &mut fake, fx);
        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::CurrentKitChanged { number, name } if *number == 0 && name == "Rock")));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CoreEvent::Speak(sp) if sp.text == "Kit 1: Rock"))
        );
    }

    #[test]
    fn rename_kit_confirmed_by_readback() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready, kit 4

        let fx = s.rename_kit(4, "Funk".to_string());
        let events = drive(&mut s, &mut fake, fx);
        assert!(events.iter().any(|e| matches!(e,
            CoreEvent::EditConfirmed { display, .. } if display == "Funk")));
    }

    #[test]
    fn disconnect_resets_and_earcons() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready

        let fx = s.on_disconnected();
        assert_eq!(s.state(), ConnectionState::Disconnected);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Emit(CoreEvent::Earcon(Earcon::Disconnected))))
        );
    }

    #[test]
    fn poll_does_not_clobber_an_in_flight_edit() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready

        let _ = s.select_kit(0); // leaves an edit-verify pending on Current
        // A tick must NOT issue a Current poll (which would clobber that pending).
        let tick_fx = s.tick(1000);
        assert!(
            !tick_fx.iter().any(|e| matches!(e, Effect::SendMidi(_))),
            "poll must be skipped while an edit is in flight"
        );
    }

    #[test]
    fn snapshot_before_connect_is_disconnected_and_empty() {
        let s = Session::new("en");
        let snap = s.snapshot();
        assert_eq!(snap.connection, ConnectionState::Disconnected);
        assert!(snap.device.is_none());
        assert!(snap.current_kit.is_none());
        assert!(snap.parameters.is_empty());
    }

    #[test]
    fn snapshot_reports_device_kit_and_parameter_metadata() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // Ready, kit 4 = "Jazz", tempo 1200

        let snap = s.snapshot();
        assert_eq!(snap.connection, ConnectionState::Ready);
        assert!(
            snap.device
                .as_ref()
                .is_some_and(|d| d.recognized && d.name == "Roland V31")
        );

        let kit = snap.current_kit.expect("current kit known");
        assert_eq!(
            (kit.number, kit.display_number, kit.name.as_str()),
            (4, 5, "Jazz")
        );

        let tempo = snap
            .parameters
            .iter()
            .find(|p| p.param_id == "kit.common.tempo")
            .expect("tempo view");
        assert_eq!(tempo.label, "Tempo"); // localized label, value-free
        assert_eq!(tempo.kind, ParamKind::Numeric);
        assert_eq!(tempo.value, Some(ParamValue::Int(1200)));
        assert_eq!(tempo.display.as_deref(), Some("120.0 BPM"));
        let num = tempo.numeric.as_ref().expect("numeric info");
        assert_eq!(num.scale, 10);
        assert_eq!(num.unit.as_deref(), Some("bpm"));
        let range = num.range.as_ref().expect("declared range");
        assert_eq!(
            (range.raw_min, range.raw_max, range.raw_step),
            (200, 2600, 1)
        );
        assert!((range.display_min - 20.0).abs() < 1e-9);
        assert!((range.display_max - 260.0).abs() < 1e-9);
        assert!((range.display_step - 0.1).abs() < 1e-9);

        // The kit name is exposed as a Text parameter (no numeric metadata).
        let name = snap
            .parameters
            .iter()
            .find(|p| p.param_id == "kit.common.name")
            .expect("name view");
        assert_eq!(name.kind, ParamKind::Text);
        assert_eq!(name.value, Some(ParamValue::Text("Jazz".to_string())));
        assert!(name.numeric.is_none());
    }

    #[test]
    fn snapshot_reflects_a_confirmed_edit() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init);

        let fx = s.set_parameter("kit.common.tempo".to_string(), vec![4], 1300);
        drive(&mut s, &mut fake, fx);

        let tempo = s
            .snapshot()
            .parameters
            .into_iter()
            .find(|p| p.param_id == "kit.common.tempo")
            .unwrap();
        assert_eq!(tempo.value, Some(ParamValue::Int(1300)));
        assert_eq!(tempo.display.as_deref(), Some("130.0 BPM"));
    }

    #[test]
    fn snapshot_localizes_labels_and_values() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("ru");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init);

        let tempo = s
            .snapshot()
            .parameters
            .into_iter()
            .find(|p| p.param_id == "kit.common.tempo")
            .unwrap();
        assert_eq!(tempo.label, "Темп");
        assert_eq!(tempo.display.as_deref(), Some("120.0 уд/мин"));
    }

    #[test]
    fn snapshot_clears_stale_values_on_kit_change() {
        let mut fake = FakeModule::v31();
        let mut s = Session::new("en");
        let init = s.on_connected();
        drive(&mut s, &mut fake, init); // kit 4, tempo 1200

        let fx = s.select_kit(0); // switch to kit 0 ("Rock", tempo 1400)
        drive(&mut s, &mut fake, fx);

        let snap = s.snapshot();
        assert_eq!(snap.current_kit.unwrap().name, "Rock");
        let tempo = snap
            .parameters
            .into_iter()
            .find(|p| p.param_id == "kit.common.tempo")
            .unwrap();
        assert_eq!(tempo.value, Some(ParamValue::Int(1400)));
    }
}
