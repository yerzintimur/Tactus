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
    /// Tempo read-back for the given kit (the kit lets us drop stale read-backs
    /// from rapid scrolling).
    Tempo(u32),
    /// Read-back of an edit, awaiting verification.
    EditVerify(Edit),
}

/// An in-flight edit awaiting write → read-back → verify.
#[derive(Debug, Clone)]
struct Edit {
    param_id: String,
    intended: EditValue,
    age: u32,
}

/// An in-flight kit selection. Deliberately *not* an [`Edit`]: the edit pipeline
/// verifies via a read-back keyed by address, and the kit number lives at the same
/// address the poller reads — a stale in-flight poll reply would land on the verify
/// slot and read as a spurious mismatch (PROTOCOL §6). Instead the selection is
/// confirmed by the regular `Current` read path, which tolerates stale values.
#[derive(Debug, Clone)]
struct KitSelect {
    intended: u32,
    age: u32,
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
    /// An app-initiated kit selection awaiting confirmation via the `Current` read.
    kit_select: Option<KitSelect>,
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
            kit_select: None,
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
        self.kit_select = None;
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
        self.kit_select = None;
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
                self.kit_select = None;

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
                self.kit_select = None;
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
                    Some(num) => self.on_current_kit_read(num),
                    None => Vec::new(),
                }
            }
            Pending::KitName(number) => {
                // Drop stale read-backs from rapid kit-scrolling: only announce the
                // kit the device has settled on (the one matching our state). This
                // keeps a fast dial through kits from speaking every name in between.
                if Some(number) != self.current_kit {
                    return Vec::new();
                }
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
            Pending::Tempo(kit) => {
                if Some(kit) != self.current_kit {
                    return Vec::new();
                }
                self.speak_tempo(data, SpeechPriority::Low)
            }
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
                Some(num) => self.on_current_kit_read(num),
                None => Vec::new(),
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

    /// A `Current` value arrived — via the poll, a kit-select confirmation read, or
    /// an unsolicited push. All three funnel here so a kit selection is confirmed by
    /// whatever `Current` read lands first.
    fn on_current_kit_read(&mut self, number: u32) -> Vec<Effect> {
        if Some(number) != self.current_kit {
            return self.on_kit_changed(number);
        }
        // Unchanged. A read matching an in-flight selection's target means we were
        // already on that kit — settle silently. Any *other* unchanged read while a
        // selection is in flight is a stale reply from before the write landed:
        // ignore it and let the next `Current` read confirm. (This is what makes
        // the shared-address race harmless — PROTOCOL §6.)
        if self
            .kit_select
            .as_ref()
            .is_some_and(|ks| ks.intended == number)
        {
            self.kit_select = None;
        }
        Vec::new()
    }

    fn on_kit_changed(&mut self, number: u32) -> Vec<Effect> {
        // The device settled on a kit: any in-flight selection is resolved by
        // announcing the *actual* kit below (the device is the source of truth).
        self.kit_select = None;
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
        if let Some(e) = self.request_read("kit.common.tempo", &[number], Pending::Tempo(number)) {
            fx.push(e);
        }
        fx
    }

    fn poll_current(&mut self) -> Vec<Effect> {
        // Don't poll over an in-flight edit: if the poll surfaced a kit change
        // mid-verify, the kit-change flow would clear the value cache and issue
        // name/tempo reads around the verify — keep the edit exchange atomic.
        // (A kit *selection* is the opposite: polling is exactly how it gets
        // confirmed, so it never suppresses the poll.)
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

    /// Switch the active kit (0-based number). Not routed through the edit-verify
    /// pipeline: the kit number lives at the same address the poller reads, so an
    /// address-keyed verify slot is racy — a stale in-flight poll reply would land
    /// on it and read as a spurious mismatch (PROTOCOL §6). Instead: write, then
    /// confirm via the regular `Current` read. The kit-change flow (earcon +
    /// name/tempo reads + announcing the actual kit) is the audible confirmation —
    /// still write → read back → verify, never a blind write; if the module never
    /// lands on a new kit, [`Session::age_edits`] reports a timeout.
    pub fn select_kit(&mut self, number: u32) -> Vec<Effect> {
        let (addr, len, encoding, model_id) = {
            let Some(p) = self.profile.as_ref() else {
                return self.fail_simple("edit.not_ready", "current.kit_num");
            };
            let (Some(addr), Some(def)) = (
                p.address_of("current.kit_num", &[]),
                p.parameter("current.kit_num"),
            ) else {
                return self.fail_simple("edit.not_ready", "current.kit_num");
            };
            (addr, def.len, def.encoding, p.model_id.clone())
        };
        let Some(data) = encoding.encode_int(i64::from(number), len) else {
            return self.fail_simple("edit.out_of_range", "current.kit_num");
        };
        self.kit_select = Some(KitSelect {
            intended: number,
            age: 0,
        });
        self.pending.insert(addr, Pending::CurrentKitNum);
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

    /// Set a numeric parameter to a raw value, verified by read-back.
    pub fn set_parameter(
        &mut self,
        param_id: String,
        indices: Vec<u32>,
        value: i64,
    ) -> Vec<Effect> {
        self.set_value(&param_id, &indices, EditValue::Int(value))
    }

    /// Rename a kit, verified by read-back.
    pub fn rename_kit(&mut self, number: u32, name: String) -> Vec<Effect> {
        self.set_value("kit.common.name", &[number], EditValue::Text(name))
    }

    fn set_value(&mut self, param_id: &str, indices: &[u32], intended: EditValue) -> Vec<Effect> {
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

    /// Age in-flight edits (and any kit selection) on each tick; fire a timeout
    /// for any that expired.
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
        // A kit selection the device never confirms (no `Current` read lands on a
        // new kit) times out the same way — a failed select is audible, not silent.
        if let Some(ks) = self.kit_select.as_mut() {
            ks.age += 1;
            if ks.age >= EDIT_TIMEOUT_TICKS {
                self.kit_select = None;
                fx.extend(self.fail_simple("edit.timeout", "current.kit_num"));
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
