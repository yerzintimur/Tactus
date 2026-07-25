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

/// What an announcement is about — the platform's router keys on this (ADR-0014):
/// `KitNav` interrupts the previous announcement; a `UserInitiated` `ParamEdit` is
/// suppressed while a screen reader voices the focused control.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SpeechCategory {
    /// Connection lifecycle, incl. the initial kit of the connection summary.
    Connection,
    /// The current kit changed (navigation) — newer preempts older.
    KitNav,
    /// A parameter value changed (edit confirmation or hardware edit push).
    ParamEdit,
    /// A failure the user must hear.
    Error,
    /// Supplementary detail (e.g. the tempo tail after a kit change).
    Info,
}

/// Who caused the change being announced (ADR-0014).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SpeechSource {
    DeviceInitiated,
    UserInitiated,
}

/// A language the app can be shown in, for the platform's language picker.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LocaleOption {
    /// Language subtag to pass back to `set_locale` ("en", "ru").
    pub code: String,
    /// The language's name in that language — recognisable to a user who cannot
    /// read the interface's current language.
    pub endonym: String,
}

impl From<&engine::LocaleInfo> for LocaleOption {
    fn from(info: &engine::LocaleInfo) -> Self {
        Self {
            code: info.code.to_string(),
            endonym: info.endonym.to_string(),
        }
    }
}

/// A piece of the app's own interface text, localized by the core (ADR-0008).
/// Mirrors `engine::UiString`; the two conversions below are exhaustive, so a
/// variant cannot be added on one side alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum UiString {
    SectionLanguage,
    LanguageSystem,
    SectionConnection,
    LabelStatus,
    LabelDevice,
    LabelFirmware,
    StatusDisconnected,
    StatusIdentifying,
    StatusReady,
    ConnectPrompt,
    FirmwareNewer,
    FirmwareOlder,
    FirmwareUnknown,
    SectionKit,
    LabelCurrentKit,
    ValueCurrentKit,
    ButtonPreviousKit,
    ButtonNextKit,
    ButtonRenameKit,
    HintRenameKit,
    TitleRenameKit,
    LabelKitName,
    ButtonSave,
    ButtonCancel,
    SectionTempo,
    LabelTempo,
    ValueUpdating,
    HintTempoAdjust,
    ValueUnknown,
}

impl From<UiString> for engine::UiString {
    fn from(s: UiString) -> Self {
        match s {
            UiString::SectionLanguage => engine::UiString::SectionLanguage,
            UiString::LanguageSystem => engine::UiString::LanguageSystem,
            UiString::SectionConnection => engine::UiString::SectionConnection,
            UiString::LabelStatus => engine::UiString::LabelStatus,
            UiString::LabelDevice => engine::UiString::LabelDevice,
            UiString::LabelFirmware => engine::UiString::LabelFirmware,
            UiString::StatusDisconnected => engine::UiString::StatusDisconnected,
            UiString::StatusIdentifying => engine::UiString::StatusIdentifying,
            UiString::StatusReady => engine::UiString::StatusReady,
            UiString::ConnectPrompt => engine::UiString::ConnectPrompt,
            UiString::FirmwareNewer => engine::UiString::FirmwareNewer,
            UiString::FirmwareOlder => engine::UiString::FirmwareOlder,
            UiString::FirmwareUnknown => engine::UiString::FirmwareUnknown,
            UiString::SectionKit => engine::UiString::SectionKit,
            UiString::LabelCurrentKit => engine::UiString::LabelCurrentKit,
            UiString::ValueCurrentKit => engine::UiString::ValueCurrentKit,
            UiString::ButtonPreviousKit => engine::UiString::ButtonPreviousKit,
            UiString::ButtonNextKit => engine::UiString::ButtonNextKit,
            UiString::ButtonRenameKit => engine::UiString::ButtonRenameKit,
            UiString::HintRenameKit => engine::UiString::HintRenameKit,
            UiString::TitleRenameKit => engine::UiString::TitleRenameKit,
            UiString::LabelKitName => engine::UiString::LabelKitName,
            UiString::ButtonSave => engine::UiString::ButtonSave,
            UiString::ButtonCancel => engine::UiString::ButtonCancel,
            UiString::SectionTempo => engine::UiString::SectionTempo,
            UiString::LabelTempo => engine::UiString::LabelTempo,
            UiString::ValueUpdating => engine::UiString::ValueUpdating,
            UiString::HintTempoAdjust => engine::UiString::HintTempoAdjust,
            UiString::ValueUnknown => engine::UiString::ValueUnknown,
        }
    }
}

impl From<engine::UiString> for UiString {
    fn from(s: engine::UiString) -> Self {
        match s {
            engine::UiString::SectionLanguage => Self::SectionLanguage,
            engine::UiString::LanguageSystem => Self::LanguageSystem,
            engine::UiString::SectionConnection => Self::SectionConnection,
            engine::UiString::LabelStatus => Self::LabelStatus,
            engine::UiString::LabelDevice => Self::LabelDevice,
            engine::UiString::LabelFirmware => Self::LabelFirmware,
            engine::UiString::StatusDisconnected => Self::StatusDisconnected,
            engine::UiString::StatusIdentifying => Self::StatusIdentifying,
            engine::UiString::StatusReady => Self::StatusReady,
            engine::UiString::ConnectPrompt => Self::ConnectPrompt,
            engine::UiString::FirmwareNewer => Self::FirmwareNewer,
            engine::UiString::FirmwareOlder => Self::FirmwareOlder,
            engine::UiString::FirmwareUnknown => Self::FirmwareUnknown,
            engine::UiString::SectionKit => Self::SectionKit,
            engine::UiString::LabelCurrentKit => Self::LabelCurrentKit,
            engine::UiString::ValueCurrentKit => Self::ValueCurrentKit,
            engine::UiString::ButtonPreviousKit => Self::ButtonPreviousKit,
            engine::UiString::ButtonNextKit => Self::ButtonNextKit,
            engine::UiString::ButtonRenameKit => Self::ButtonRenameKit,
            engine::UiString::HintRenameKit => Self::HintRenameKit,
            engine::UiString::TitleRenameKit => Self::TitleRenameKit,
            engine::UiString::LabelKitName => Self::LabelKitName,
            engine::UiString::ButtonSave => Self::ButtonSave,
            engine::UiString::ButtonCancel => Self::ButtonCancel,
            engine::UiString::SectionTempo => Self::SectionTempo,
            engine::UiString::LabelTempo => Self::LabelTempo,
            engine::UiString::ValueUpdating => Self::ValueUpdating,
            engine::UiString::HintTempoAdjust => Self::HintTempoAdjust,
            engine::UiString::ValueUnknown => Self::ValueUnknown,
        }
    }
}

/// One run of a spoken message in a single language (ADR-0011).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TextSpan {
    pub text: String,
    /// BCP-47 language tag, e.g. "ru" for the sentence and "en" for a kit name.
    pub lang: String,
}

/// A spoken message (already localized by the core).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Speech {
    pub text: String,
    /// The language of each run of `text`, concatenating back to it exactly. A
    /// single span means one language throughout and the platform can announce
    /// `text` as-is; more than one means the announcement must be tagged per
    /// range so the screen reader pronounces each part correctly.
    pub spans: Vec<TextSpan>,
    pub priority: SpeechPriority,
    pub category: SpeechCategory,
    pub source: SpeechSource,
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

// ── pull-side view-model (snapshot) ──

/// A complete snapshot of the session's observable state, pulled on demand
/// (e.g. when opening an editor). Complements the `CoreEvent` push stream.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Snapshot {
    pub connection: ConnectionState,
    pub device: Option<DeviceInfo>,
    pub current_kit: Option<KitRef>,
    pub parameters: Vec<ParameterView>,
}

/// A reference to a kit: 0-based wire `number`, 1-based `display_number`, name.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KitRef {
    pub number: u32,
    pub display_number: u32,
    pub name: String,
}

/// How a parameter is presented/edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ParamKind {
    Numeric,
    Text,
}

/// A parameter's last device-confirmed value.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ParamValue {
    Int { value: i64 },
    Text { value: String },
}

/// A parameter, its last-known value, and the metadata the UI needs to present
/// and edit it without per-device code.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ParameterView {
    pub param_id: String,
    /// Localized control/accessibility label (never carries the value).
    pub label: String,
    pub kind: ParamKind,
    /// Last value the device confirmed (`None` if not read back yet).
    pub value: Option<ParamValue>,
    /// Localized presentation of `value` (e.g. "120.0 BPM").
    pub display: Option<String>,
    /// Numeric editing metadata; present for `ParamKind::Numeric`.
    pub numeric: Option<NumericInfo>,
}

/// Editing metadata for a numeric parameter.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct NumericInfo {
    pub scale: i64,
    pub unit: Option<String>,
    pub range: Option<NumericRange>,
}

/// The inclusive range of a numeric parameter, in raw and display units.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct NumericRange {
    pub raw_min: i64,
    pub raw_max: i64,
    pub raw_step: i64,
    pub display_min: f64,
    pub display_max: f64,
    pub display_step: f64,
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

impl From<engine::SpeechCategory> for SpeechCategory {
    fn from(c: engine::SpeechCategory) -> Self {
        match c {
            engine::SpeechCategory::Connection => Self::Connection,
            engine::SpeechCategory::KitNav => Self::KitNav,
            engine::SpeechCategory::ParamEdit => Self::ParamEdit,
            engine::SpeechCategory::Error => Self::Error,
            engine::SpeechCategory::Info => Self::Info,
        }
    }
}

impl From<engine::SpeechSource> for SpeechSource {
    fn from(s: engine::SpeechSource) -> Self {
        match s {
            engine::SpeechSource::DeviceInitiated => Self::DeviceInitiated,
            engine::SpeechSource::UserInitiated => Self::UserInitiated,
        }
    }
}

impl From<engine::TextSpan> for TextSpan {
    fn from(s: engine::TextSpan) -> Self {
        Self {
            text: s.text,
            lang: s.lang,
        }
    }
}

impl From<engine::Speech> for Speech {
    fn from(s: engine::Speech) -> Self {
        Self {
            text: s.text,
            spans: s.spans.into_iter().map(Into::into).collect(),
            priority: s.priority.into(),
            category: s.category.into(),
            source: s.source.into(),
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

impl From<engine::Snapshot> for Snapshot {
    fn from(s: engine::Snapshot) -> Self {
        Self {
            connection: s.connection.into(),
            device: s.device.map(Into::into),
            current_kit: s.current_kit.map(Into::into),
            parameters: s.parameters.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<engine::KitRef> for KitRef {
    fn from(k: engine::KitRef) -> Self {
        Self {
            number: k.number,
            display_number: k.display_number,
            name: k.name,
        }
    }
}

impl From<engine::ParamKind> for ParamKind {
    fn from(k: engine::ParamKind) -> Self {
        match k {
            engine::ParamKind::Numeric => Self::Numeric,
            engine::ParamKind::Text => Self::Text,
        }
    }
}

impl From<engine::ParamValue> for ParamValue {
    fn from(v: engine::ParamValue) -> Self {
        match v {
            engine::ParamValue::Int(value) => Self::Int { value },
            engine::ParamValue::Text(value) => Self::Text { value },
        }
    }
}

impl From<engine::ParameterView> for ParameterView {
    fn from(p: engine::ParameterView) -> Self {
        Self {
            param_id: p.param_id,
            label: p.label,
            kind: p.kind.into(),
            value: p.value.map(Into::into),
            display: p.display,
            numeric: p.numeric.map(Into::into),
        }
    }
}

impl From<engine::NumericInfo> for NumericInfo {
    fn from(n: engine::NumericInfo) -> Self {
        Self {
            scale: n.scale,
            unit: n.unit,
            range: n.range.map(Into::into),
        }
    }
}

impl From<engine::NumericRange> for NumericRange {
    fn from(r: engine::NumericRange) -> Self {
        Self {
            raw_min: r.raw_min,
            raw_max: r.raw_max,
            raw_step: r.raw_step,
            display_min: r.display_min,
            display_max: r.display_max,
            display_step: r.display_step,
        }
    }
}
