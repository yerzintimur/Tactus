//! The sans-I/O session: a pure state machine. Inputs are `on_connected`,
//! `on_disconnected`, `handle_midi_input`, `tick`, and (later) user intents; each
//! returns a `Vec<Effect>` the host performs. No I/O, threads, or timers here.
//! See ADR-0008 and docs/DEVELOPMENT.md §4.4, §7.

use crate::event::{
    ConnectionState, CoreEvent, DeviceInfo, Earcon, Effect, Speech, SpeechCategory, SpeechPriority,
    SpeechSource,
};
use crate::setlist::{END, SetlistState, StepWrite};
use crate::viewmodel::{self, KitRef, ParamKind, ParamValue, ParameterView, SetlistView, Snapshot};
use device::{DeviceProfile, FirmwareSupport, FirmwareVersion, ProfileRegistry};
use model::{
    LocalizedText, Localizer, Message, UiString, format_kit, format_parameter,
    format_parameter_label,
};
use std::collections::HashMap;
use sysex::SysexMessage;
use sysex::encoding::decode_ascii;

/// Device ID used to broadcast the Identity Request (any unit responds).
const IDENTITY_DEVICE_ID: u8 = 0x7F;
/// How often to poll the active kit (`Current`).
const POLL_INTERVAL_MS: u64 = 300;

/// An edit times out after this many ticks without a confirming read-back.
const EDIT_TIMEOUT_TICKS: u32 = 5;

/// Re-read the current kit's name every this many polls (~3 s), to notice its
/// slot's contents being replaced on the module — a kit copied or imported over
/// it changes everything about the kit while its *number* stays put, so the poll
/// on the number alone would never see it. Every poll would be twice the traffic
/// for an event that happens between songs, not during one.
const KIT_NAME_REFRESH_POLLS: u32 = 10;

/// What an outstanding RQ1 reply means when its DT1 comes back.
#[derive(Debug, Clone)]
enum Pending {
    CurrentKitNum,
    /// Kit-name read-back (the kit number lets us drop stale read-backs from
    /// rapid scrolling; the origin tags the announcement — ADR-0014).
    KitName(u32, KitOrigin),
    /// Periodic re-read of the current kit's name, to notice its slot's contents
    /// being replaced on the module. Silent unless the name actually changed.
    KitNameRefresh(u32),
    /// Tempo read-back for the given kit.
    Tempo(u32, KitOrigin),
    /// Read-back of an edit, awaiting verification.
    EditVerify(Edit),
    /// The bulk read of one set list (name + every step in a single reply).
    Setlist(u32),
    /// A kit's name, requested to label a set-list step.
    SetlistKitName(u32),
}

/// Why the current kit changed — decides how the resulting announcements are
/// tagged for the platform's router (ADR-0014).
#[derive(Debug, Clone, Copy)]
enum KitOrigin {
    /// First discovery after identify: the kit is part of the connection
    /// summary, not a `KitNav` barge-in that would clobber the connect line.
    Initial,
    /// An app-initiated selection (the user pressed next/previous kit).
    User,
    /// Changed on the module itself (unsolicited push or detected by the poll).
    Device,
}

impl KitOrigin {
    fn tags(self) -> (SpeechCategory, SpeechSource) {
        match self {
            KitOrigin::Initial => (SpeechCategory::Connection, SpeechSource::DeviceInitiated),
            KitOrigin::User => (SpeechCategory::KitNav, SpeechSource::UserInitiated),
            KitOrigin::Device => (SpeechCategory::KitNav, SpeechSource::DeviceInitiated),
        }
    }

    fn source(self) -> SpeechSource {
        self.tags().1
    }
}

/// An in-flight edit awaiting write → read-back → verify.
#[derive(Debug, Clone)]
struct Edit {
    param_id: String,
    /// Area/dim indices the edit was addressed with — an indexed parameter's
    /// value belongs to one slot (set list 3, step 5), not to the parameter.
    indices: Vec<u32>,
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
    /// The set list open for viewing/editing, if any (one at a time).
    setlist: Option<SetlistState>,
    /// Kit number → name, filled in as read-backs land. A set-list step is a kit
    /// *number*; a name is the only part of it a blind user can act on.
    kit_names: HashMap<u32, String>,
    /// Polls since the current kit's name was last re-read (see
    /// [`KIT_NAME_REFRESH_POLLS`]).
    polls_since_name_check: u32,
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
            setlist: None,
            kit_names: HashMap::new(),
            polls_since_name_check: 0,
        }
    }

    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.locale = locale.into();
    }

    /// The languages the core can render, for the platform's language picker.
    pub fn available_locales(&self) -> &'static [model::LocaleInfo] {
        model::AVAILABLE_LOCALES
    }

    /// The app's own interface text, localized here rather than in each platform's
    /// resources (ADR-0008) — in a nonvisual app a control label is speech too.
    /// `value` fills the `{ $value }` slot of the strings that take one.
    pub fn ui_string(&self, string: UiString, value: Option<String>) -> String {
        let mut message = Message::new(string.message_id());
        if let Some(value) = value {
            message = message.arg("value", value);
        }
        self.render(&message)
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
        self.setlist = None;
        self.kit_names.clear();
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
        self.setlist = None;
        self.kit_names.clear();
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
                self.setlist = None;
                self.kit_names.clear();

                let mut speech = self.render_spoken(
                    &Message::new("device.connected")
                        .device_arg("device", name.as_str())
                        .arg("firmware", fw.display()),
                );
                if !support.is_tested() {
                    speech.push_str(" ");
                    speech.push(&self.render_spoken(&Message::new("device.firmware_untested")));
                }

                let mut fx = vec![
                    Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Ready)),
                    Effect::Emit(CoreEvent::DeviceIdentified(info)),
                    Effect::Emit(CoreEvent::Earcon(Earcon::Connected)),
                    self.speak(
                        speech,
                        SpeechPriority::High,
                        SpeechCategory::Connection,
                        SpeechSource::DeviceInitiated,
                    ),
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
                self.setlist = None;
                self.kit_names.clear();
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
                let speech = self.render_spoken(&Message::new("device.unrecognized"));
                vec![
                    Effect::Emit(CoreEvent::ConnectionChanged(ConnectionState::Ready)),
                    Effect::Emit(CoreEvent::DeviceIdentified(info)),
                    Effect::Emit(CoreEvent::Earcon(Earcon::Connected)),
                    self.speak(
                        speech,
                        SpeechPriority::High,
                        SpeechCategory::Connection,
                        SpeechSource::DeviceInitiated,
                    ),
                ]
            }
        }
    }

    fn handle_dt1(&mut self, address: [u8; 4], data: &[u8]) -> Vec<Effect> {
        if let Some(pending) = self.pending.remove(&address) {
            self.handle_pending(pending, address, data)
        } else {
            self.handle_unsolicited(address, data)
        }
    }

    fn handle_pending(&mut self, pending: Pending, address: [u8; 4], data: &[u8]) -> Vec<Effect> {
        match pending {
            Pending::CurrentKitNum => {
                let decoded = self.profile.as_ref().and_then(|p| decode_kit_num(p, data));
                match decoded {
                    Some(num) => self.on_current_kit_read(num),
                    None => Vec::new(),
                }
            }
            Pending::KitName(number, origin) => {
                // Content gate (ADR-0014): a read-back for a kit we have already
                // scrolled past must not be announced or cached as the *current*
                // kit — that would report outdated state as fact. Kits the user
                // dwells on still get announced; the platform's interruption
                // handles announcement overlap.
                if Some(number) != self.current_kit {
                    return Vec::new();
                }
                let name = self.decode_text("kit.common.name", data);
                self.values.insert(
                    "kit.common.name".to_string(),
                    ParamValue::Text(name.clone()),
                );
                let speech = self.render_spoken(&format_kit(number + 1, &name));
                let (category, source) = origin.tags();
                vec![
                    Effect::Emit(CoreEvent::CurrentKitChanged { number, name }),
                    self.speak(speech, SpeechPriority::Default, category, source),
                ]
            }
            Pending::KitNameRefresh(kit) => self.handle_kit_name_refresh(kit, data),
            Pending::Tempo(kit, origin) => {
                if Some(kit) != self.current_kit {
                    return Vec::new();
                }
                self.speak_tempo(
                    data,
                    SpeechPriority::Low,
                    SpeechCategory::Info,
                    origin.source(),
                )
            }
            Pending::EditVerify(edit) => self.handle_edit_verify(edit, data),
            Pending::Setlist(index) => {
                if self.absorb_setlist(address, data) {
                    vec![Effect::Emit(CoreEvent::SetlistChanged { number: index })]
                } else {
                    Vec::new()
                }
            }
            Pending::SetlistKitName(kit) => {
                let name = self.decode_text("kit.common.name", data);
                self.kit_names.insert(kit, name);
                match self.setlist.as_ref().map(|s| s.index) {
                    Some(number) => vec![Effect::Emit(CoreEvent::SetlistChanged { number })],
                    None => Vec::new(),
                }
            }
        }
    }

    /// Unsolicited DT1 (e.g. a hardware edit pushed via Transmit Edit Data):
    /// best-effort match against the active kit's known addresses.
    fn handle_unsolicited(&mut self, address: [u8; 4], data: &[u8]) -> Vec<Effect> {
        // A set list edited on the module while the user has it open here.
        if self.absorb_setlist(address, data) {
            let number = self.setlist.as_ref().map(|s| s.index).unwrap_or_default();
            return vec![Effect::Emit(CoreEvent::SetlistChanged { number })];
        }
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
            // The current kit was renamed on the module — a device-initiated
            // parameter edit the screen reader cannot see.
            let name = self.decode_text("kit.common.name", data);
            self.values.insert(
                "kit.common.name".to_string(),
                ParamValue::Text(name.clone()),
            );
            self.announce_kit_identity(kit, name)
        } else if Some(address) == tempo_addr {
            self.speak_tempo(
                data,
                SpeechPriority::Low,
                SpeechCategory::ParamEdit,
                SpeechSource::DeviceInitiated,
            )
        } else {
            Vec::new()
        }
    }

    /// A periodic name re-read came back. Silence is the answer almost every time:
    /// this fires every few seconds, and announcing an unchanged name would make
    /// the app talk over the drummer for nothing.
    fn handle_kit_name_refresh(&mut self, kit: u32, data: &[u8]) -> Vec<Effect> {
        if Some(kit) != self.current_kit {
            return Vec::new();
        }
        let name = self.decode_text("kit.common.name", data);
        let Some(known) = self.text_value("kit.common.name") else {
            // Nothing to compare against yet — the kit-change flow is already
            // reading this name and will announce it. Just seed the cache.
            self.values
                .insert("kit.common.name".to_string(), ParamValue::Text(name));
            return Vec::new();
        };
        if known == name {
            return Vec::new();
        }

        // The slot holds something else now: a kit copied or imported over it on
        // the module, or a rename. Whatever else we cached for this slot describes
        // the kit that used to be here, so drop it and read the rest again.
        self.values.clear();
        self.values.insert(
            "kit.common.name".to_string(),
            ParamValue::Text(name.clone()),
        );
        self.kit_names.insert(kit, name.clone());
        let mut fx = self.announce_kit_identity(kit, name);
        fx.extend(self.request_read(
            "kit.common.tempo",
            &[kit],
            Pending::Tempo(kit, KitOrigin::Device),
        ));
        fx
    }

    /// Report what the active kit *is* now, after the module changed it under us.
    /// Not `KitNav` — the drummer didn't navigate anywhere, the kit changed
    /// beneath them — but device-initiated, so the screen reader can't see it and
    /// it must be spoken (ADR-0014).
    fn announce_kit_identity(&self, kit: u32, name: String) -> Vec<Effect> {
        let speech = self.render_spoken(&format_kit(kit + 1, &name));
        vec![
            Effect::Emit(CoreEvent::CurrentKitChanged { number: kit, name }),
            self.speak(
                speech,
                SpeechPriority::Default,
                SpeechCategory::ParamEdit,
                SpeechSource::DeviceInitiated,
            ),
        ]
    }

    /// A `Current` value arrived — via the poll, a kit-select confirmation read, or
    /// an unsolicited push. All three funnel here so a kit selection is confirmed by
    /// whatever `Current` read lands first.
    fn on_current_kit_read(&mut self, number: u32) -> Vec<Effect> {
        if Some(number) != self.current_kit {
            let origin = if self.current_kit.is_none() {
                KitOrigin::Initial
            } else if self.kit_select.is_some() {
                KitOrigin::User
            } else {
                KitOrigin::Device
            };
            return self.on_kit_changed(number, origin);
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

    fn on_kit_changed(&mut self, number: u32, origin: KitOrigin) -> Vec<Effect> {
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
        if let Some(e) = self.request_read(
            "kit.common.name",
            &[number],
            Pending::KitName(number, origin),
        ) {
            fx.push(e);
        }
        if let Some(e) = self.request_read(
            "kit.common.tempo",
            &[number],
            Pending::Tempo(number, origin),
        ) {
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
        let mut fx: Vec<Effect> = self
            .request_read("current.kit_num", &[], Pending::CurrentKitNum)
            .into_iter()
            .collect();
        self.polls_since_name_check += 1;
        let mut refreshed = false;
        if self.polls_since_name_check >= KIT_NAME_REFRESH_POLLS {
            self.polls_since_name_check = 0;
            if let Some(effect) = self.refresh_kit_name() {
                fx.push(effect);
                refreshed = true;
            }
        }
        // Two messages per poll at most: filling in a set-list step's name can
        // wait a tick rather than share one with the refresh (PROTOCOL §6 — a
        // burst is what a module drops).
        if !refreshed {
            fx.extend(self.request_missing_step_name());
        }
        fx
    }

    /// Re-read the current kit's name, unless a read of it is already outstanding
    /// — the kit-change flow's own name read is what announces a new kit, and
    /// replacing its pending slot would swallow that announcement.
    fn refresh_kit_name(&mut self) -> Option<Effect> {
        let kit = self.current_kit?;
        let addr = self
            .profile
            .as_ref()?
            .address_of("kit.common.name", &[kit])?;
        if self.pending.contains_key(&addr) {
            return None;
        }
        self.request_read("kit.common.name", &[kit], Pending::KitNameRefresh(kit))
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

    fn speak_tempo(
        &mut self,
        data: &[u8],
        priority: SpeechPriority,
        category: SpeechCategory,
        source: SpeechSource,
    ) -> Vec<Effect> {
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
        vec![self.speak(self.render_spoken(&message), priority, category, source)]
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
        let (addr, len, encoding, model_id, max_kit) = {
            let Some(p) = self.profile.as_ref() else {
                return self.fail_simple("edit.not_ready", "current.kit_num");
            };
            let (Some(addr), Some(def)) = (
                p.address_of("current.kit_num", &[]),
                p.parameter("current.kit_num"),
            ) else {
                return self.fail_simple("edit.not_ready", "current.kit_num");
            };
            (
                addr,
                def.len,
                def.encoding,
                p.model_id.clone(),
                p.max_kit_number(),
            )
        };
        // A slot the module doesn't have: the write would fit the field, so the
        // module would simply ignore it and the user would wait out an edit
        // timeout for an answer we already have. Say so now.
        if max_kit.is_some_and(|max| number > max) {
            return self.fail_simple("edit.out_of_range", "current.kit_num");
        }
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

    /// Step to the next kit, stopping at the module's last slot.
    ///
    /// The edge is a **boundary, not an error**: nothing is written, and the app
    /// says where the user is. Writing past the end instead would leave them with
    /// a second of silence and then an edit timeout — reporting a broken
    /// connection for a request the module was right to ignore.
    pub fn next_kit(&mut self) -> Vec<Effect> {
        self.step_kit(1)
    }

    /// Step to the previous kit; kit 1 is a boundary (see [`Session::next_kit`]).
    pub fn previous_kit(&mut self) -> Vec<Effect> {
        self.step_kit(-1)
    }

    fn step_kit(&mut self, delta: i64) -> Vec<Effect> {
        // Relative navigation needs a known position. Without one there is nothing
        // to step from, and assuming kit 1 would move the user's module blind.
        let Some(current) = self.current_kit else {
            return self.fail_simple("edit.not_ready", "current.kit_num");
        };
        let target = i64::from(current) + delta;
        if target < 0 {
            return self.announce_kit_edge("kit.at_first");
        }
        let max_kit = self
            .profile
            .as_ref()
            .and_then(DeviceProfile::max_kit_number);
        if max_kit.is_some_and(|max| target > i64::from(max)) {
            return self.announce_kit_edge("kit.at_last");
        }
        match u32::try_from(target) {
            Ok(number) => self.select_kit(number),
            // Unreachable while a kit number fits u32; refuse rather than wrap.
            Err(_) => self.fail_simple("edit.out_of_range", "current.kit_num"),
        }
    }

    /// The user asked to step past the first/last kit. Nothing changed — on the
    /// module or on screen — so the screen reader has nothing of its own to voice:
    /// say where they are. Tagged `KitNav`, so it interrupts like any other kit
    /// announcement instead of queueing behind a scroll (ADR-0014).
    fn announce_kit_edge(&mut self, message_id: &str) -> Vec<Effect> {
        let edge = self.render_spoken(&Message::new(message_id));
        vec![self.speak(
            edge,
            SpeechPriority::High,
            SpeechCategory::KitNav,
            SpeechSource::UserInitiated,
        )]
    }

    // ── set lists ──

    /// Open a set list (0-based) for viewing and editing.
    ///
    /// One RQ1 for the whole 160-byte block, not 33 for its parts: Roland asks for
    /// a pause between consecutive messages, and a burst of requests is exactly
    /// what a module drops. Whatever the reply covers is absorbed by
    /// [`Session::absorb_setlist`], so a module that splits it still works.
    pub fn read_setlist(&mut self, index: u32) -> Vec<Effect> {
        let (addr, size, capacity, model_id) = {
            let Some(p) = self.profile.as_ref() else {
                return self.fail_simple("edit.not_ready", "setlist.name");
            };
            let Some(capacity) = setlist_capacity(p) else {
                return self.fail_simple("edit.not_ready", "setlist.name");
            };
            if p.areas.get("setlist").and_then(|a| a.count) <= Some(index) {
                return self.fail_simple("edit.out_of_range", "setlist.name");
            }
            let (Some(addr), Some(size)) = (
                p.address_of("setlist.name", &[index]),
                setlist_block_size(p, capacity),
            ) else {
                return self.fail_simple("edit.not_ready", "setlist.name");
            };
            (addr, size, capacity, p.model_id.clone())
        };

        self.setlist = Some(SetlistState::new(index, capacity as usize));
        self.pending.insert(addr, Pending::Setlist(index));
        vec![
            Effect::SendMidi(sysex::build_rq1(
                self.device_id,
                &model_id,
                addr,
                sysex::address::from_linear(size as u32),
            )),
            Effect::ScheduleTick {
                after_ms: POLL_INTERVAL_MS,
            },
        ]
    }

    /// Point a step at a kit, or at `None` for the list's `END` terminator.
    pub fn set_setlist_step(&mut self, step: u32, kit: Option<u32>) -> Vec<Effect> {
        let raw = kit.map_or(END, i64::from);
        self.queue_step_writes(vec![StepWrite { step, raw }])
    }

    /// Add a kit to the end of the open set list, keeping it terminated.
    pub fn append_setlist_step(&mut self, kit: u32) -> Vec<Effect> {
        self.plan_setlist_edit(|s| s.append(i64::from(kit)))
    }

    /// Drop a step; the steps after it shift up.
    pub fn remove_setlist_step(&mut self, step: u32) -> Vec<Effect> {
        self.plan_setlist_edit(|s| s.remove(step as usize))
    }

    /// Exchange two steps — "move up" / "move down" in a list the user is reading.
    pub fn swap_setlist_steps(&mut self, a: u32, b: u32) -> Vec<Effect> {
        self.plan_setlist_edit(|s| s.swap(a as usize, b as usize))
    }

    /// Rename the open set list, verified by read-back.
    pub fn rename_setlist(&mut self, name: String) -> Vec<Effect> {
        let Some(index) = self.setlist.as_ref().map(|s| s.index) else {
            return self.fail_simple("edit.not_ready", "setlist.name");
        };
        self.set_value("setlist.name", &[index], EditValue::Text(name))
    }

    /// Turn a list operation into step writes, refusing when the cached list can't
    /// answer (nothing open, or the steps it depends on haven't been read).
    fn plan_setlist_edit(
        &mut self,
        plan: impl FnOnce(&SetlistState) -> Option<Vec<StepWrite>>,
    ) -> Vec<Effect> {
        let Some(state) = self.setlist.as_ref() else {
            return self.fail_simple("edit.not_ready", "setlist.step");
        };
        match plan(state) {
            Some(writes) => self.queue_step_writes(writes),
            None => self.fail_simple("edit.out_of_range", "setlist.step"),
        }
    }

    /// Queue step writes and send the first. The rest go out one at a time as the
    /// module confirms each — a reorder is several writes, and sending them in a
    /// burst risks the module dropping one and silently scrambling the order.
    fn queue_step_writes(&mut self, writes: Vec<StepWrite>) -> Vec<Effect> {
        let Some(state) = self.setlist.as_mut() else {
            return self.fail_simple("edit.not_ready", "setlist.step");
        };
        state.queue.extend(writes);
        self.send_next_step_write()
    }

    fn send_next_step_write(&mut self) -> Vec<Effect> {
        let Some(state) = self.setlist.as_mut() else {
            return Vec::new();
        };
        let (Some(write), index) = (state.queue.pop_front(), state.index) else {
            return Vec::new();
        };
        self.set_value(
            "setlist.step",
            &[index, write.step],
            EditValue::Int(write.raw),
        )
    }

    /// Absorb whatever part of the open set list a DT1 covers — the bulk read's
    /// reply, one slice of a reply the module chose to split, or an edit the user
    /// made on the module itself. Returns `true` if anything landed.
    fn absorb_setlist(&mut self, address: [u8; 4], data: &[u8]) -> bool {
        let Some(index) = self.setlist.as_ref().map(|s| s.index) else {
            return false;
        };
        let Some(profile) = self.profile.as_ref() else {
            return false;
        };
        let start = sysex::address::to_linear(address) as usize;
        let end = start + data.len();
        // The slice of `data` covering `field`, if the reply covers all of it.
        let covered = |addr: Option<[u8; 4]>, len: usize| -> Option<&[u8]> {
            let at = sysex::address::to_linear(addr?) as usize;
            (at >= start && at + len <= end).then(|| &data[at - start..at - start + len])
        };

        let name = profile.parameter("setlist.name").and_then(|def| {
            covered(profile.address_of("setlist.name", &[index]), def.len)
                .and_then(|bytes| def.encoding.decode_text(bytes))
        });
        let steps: Vec<(usize, i64)> = match profile.parameter("setlist.step") {
            Some(def) => (0..setlist_capacity(profile).unwrap_or(0))
                .filter_map(|step| {
                    let bytes =
                        covered(profile.address_of("setlist.step", &[index, step]), def.len)?;
                    Some((step as usize, def.encoding.decode_int(bytes)?))
                })
                .collect(),
            None => Vec::new(),
        };

        let Some(state) = self.setlist.as_mut() else {
            return false;
        };
        let mut landed = false;
        if let Some(name) = name {
            state.name = Some(name);
            landed = true;
        }
        for (step, raw) in steps {
            if let Some(slot) = state.steps.get_mut(step) {
                *slot = Some(raw);
                landed = true;
            }
        }
        landed
    }

    /// Ask for one kit name the open set list still needs. Called from the poll so
    /// a 12-step list fills in over a few ticks instead of firing a dozen requests
    /// at once — same reason [`Session::read_setlist`] reads in bulk.
    fn request_missing_step_name(&mut self) -> Option<Effect> {
        let wanted = self.setlist.as_ref()?.steps.iter().find_map(|slot| {
            let kit = u32::try_from((*slot)?).ok()?;
            (!self.kit_names.contains_key(&kit)).then_some(kit)
        })?;
        // The current kit's name is already read by the kit flow, and its address
        // is the one the poller uses — don't race it, just copy what we have.
        if Some(wanted) == self.current_kit {
            if let Some(ParamValue::Text(name)) = self.values.get("kit.common.name") {
                self.kit_names.insert(wanted, name.clone());
            }
            return None;
        }
        self.request_read(
            "kit.common.name",
            &[wanted],
            Pending::SetlistKitName(wanted),
        )
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
            // `None` means text was aimed at a numeric field — a profile/caller
            // error, not something the module should be asked to store.
            EditValue::Text(s) => match encoding.encode_text(s, len) {
                Some(bytes) => bytes,
                None => return self.fail_simple("edit.out_of_range", param_id),
            },
        };

        self.pending.insert(
            addr,
            Pending::EditVerify(Edit {
                param_id: param_id.to_string(),
                indices: indices.to_vec(),
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
                let actual = self.decode_text(&edit.param_id, data);
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
        let mut fx = vec![
            Effect::Emit(CoreEvent::EditConfirmed {
                param_id: edit.param_id.clone(),
                display: display.clone(),
            }),
            self.speak(
                self.spoken(display),
                SpeechPriority::Default,
                SpeechCategory::ParamEdit,
                SpeechSource::UserInitiated,
            ),
            Effect::Emit(CoreEvent::Earcon(Earcon::Confirmed)),
        ];
        fx.extend(self.confirm_setlist_step(edit, actual));
        fx
    }

    /// A confirmed set-list step: record the value the module reported and release
    /// the next queued write. A multi-step edit (reorder, remove) advances only on
    /// confirmation, so a rejected write stops the sequence instead of leaving the
    /// list half-rewritten.
    fn confirm_setlist_step(&mut self, edit: &Edit, actual: i64) -> Vec<Effect> {
        if edit.param_id != "setlist.step" {
            return Vec::new();
        }
        let [index, step] = edit.indices[..] else {
            return Vec::new();
        };
        let Some(state) = self.setlist.as_mut() else {
            return Vec::new();
        };
        if state.index != index {
            return Vec::new(); // a reply for a list the user has since left
        }
        if let Some(slot) = state.steps.get_mut(step as usize) {
            *slot = Some(actual);
        }
        let mut fx = vec![Effect::Emit(CoreEvent::SetlistChanged { number: index })];
        fx.extend(self.send_next_step_write());
        fx
    }

    fn confirm_text(&mut self, edit: &Edit, actual: String) -> Vec<Effect> {
        self.values
            .insert(edit.param_id.clone(), ParamValue::Text(actual.clone()));
        vec![
            Effect::Emit(CoreEvent::EditConfirmed {
                param_id: edit.param_id.clone(),
                display: actual.clone(),
            }),
            // A text parameter's value is the module's own text (a kit name).
            self.speak(
                self.spoken_device_text(actual),
                SpeechPriority::Default,
                SpeechCategory::ParamEdit,
                SpeechSource::UserInitiated,
            ),
            Effect::Emit(CoreEvent::Earcon(Earcon::Confirmed)),
        ]
    }

    /// Edit didn't take — announce the **actual** value, never the intended one.
    fn fail_mismatch(&self, edit: &Edit, actual_display: String) -> Vec<Effect> {
        let reason =
            self.render_spoken(&Message::new("edit.mismatch").arg("value", actual_display));
        self.emit_failure(&edit.param_id, reason)
    }

    fn fail_simple(&self, msg_id: &str, param_id: &str) -> Vec<Effect> {
        let reason = self.render_spoken(&Message::new(msg_id));
        self.emit_failure(param_id, reason)
    }

    fn emit_failure(&self, param_id: &str, reason: LocalizedText) -> Vec<Effect> {
        vec![
            Effect::Emit(CoreEvent::EditFailed {
                param_id: param_id.to_string(),
                reason: reason.text.clone(),
            }),
            self.speak(
                reason,
                SpeechPriority::High,
                SpeechCategory::Error,
                SpeechSource::UserInitiated,
            ),
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

    /// Decode a text field with the parameter's own encoding: a name is stored
    /// either one character per byte (kit name) or as a nibble pair per character
    /// (set-list name). Falls back to plain ASCII for a parameter we don't know.
    fn decode_text(&self, param_id: &str, data: &[u8]) -> String {
        self.profile
            .as_ref()
            .and_then(|p| p.parameter(param_id))
            .and_then(|def| def.encoding.decode_text(data))
            .unwrap_or_else(|| decode_ascii(data))
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
            setlist: self.setlist.as_ref().map(|state| SetlistView {
                number: state.index,
                display_number: state.index + 1,
                name: state.name.clone().unwrap_or_default(),
                // Only the steps the module confirmed, up to the list's END.
                steps: (0..state.length())
                    .filter_map(|step| state.kit_at(step))
                    .filter_map(|kit| u32::try_from(kit).ok())
                    .map(|number| KitRef {
                        number,
                        display_number: number + 1,
                        name: self.kit_names.get(&number).cloned().unwrap_or_default(),
                    })
                    .collect(),
                capacity: state.steps.len() as u32,
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

    /// Like [`Self::render`], but keeps the per-language runs needed for speech
    /// (ADR-0011). Display-only strings can stay flat.
    fn render_spoken(&self, message: &Message) -> LocalizedText {
        self.localizer.format_spans(message, &self.locale)
    }

    /// Wrap already-localized text as speech in the app's own language.
    fn spoken(&self, text: String) -> LocalizedText {
        LocalizedText::plain(text, self.localizer.language(&self.locale))
    }

    /// Wrap text that came from the module (a kit name) as speech in the
    /// module's language, so it is not read as if it were the app's (ADR-0011).
    fn spoken_device_text(&self, text: String) -> LocalizedText {
        LocalizedText::plain(text, model::DEVICE_CONTENT_LANG)
    }

    fn speak(
        &self,
        text: LocalizedText,
        priority: SpeechPriority,
        category: SpeechCategory,
        source: SpeechSource,
    ) -> Effect {
        Effect::Emit(CoreEvent::Speak(Speech {
            text: text.text,
            spans: text.spans,
            priority,
            category,
            source,
        }))
    }
}

fn decode_kit_num(profile: &DeviceProfile, data: &[u8]) -> Option<u32> {
    let def = profile.parameter("current.kit_num")?;
    u32::try_from(def.encoding.decode_int(data)?).ok()
}

fn rq_size(len: usize) -> [u8; 4] {
    sysex::address::from_linear(len as u32)
}

/// How many steps this module's set lists hold, per the profile's `step` dim.
fn setlist_capacity(profile: &DeviceProfile) -> Option<u32> {
    let def = profile.parameter("setlist.step")?;
    def.dims.iter().find(|d| d.name == "step").map(|d| d.count)
}

/// The byte size of one whole set list (name through last step), so it can be
/// requested in a single RQ1. Derived from the profile rather than hardcoded —
/// another module's set lists are a different size, or absent entirely.
fn setlist_block_size(profile: &DeviceProfile, capacity: u32) -> Option<usize> {
    let first = profile.address_of("setlist.name", &[0])?;
    let last = profile.address_of("setlist.step", &[0, capacity.checked_sub(1)?])?;
    let step_len = profile.parameter("setlist.step")?.len;
    let span = sysex::address::to_linear(last).checked_sub(sysex::address::to_linear(first))?;
    Some(span as usize + step_len)
}
