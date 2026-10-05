//! The pull-side view-model: a snapshot of the session's current observable state
//! (connection, device, active kit, and per-parameter values + presentation
//! metadata), built on demand for the UI. It complements the push-side
//! [`CoreEvent`](crate::CoreEvent) stream — the host listens to events for change
//! notifications and pulls a [`Snapshot`] when it needs the full current state
//! (e.g. opening an editor). See docs/DEVELOPMENT.md §6.
//!
//! Values are a **read-through cache of the module** — the last value the device
//! confirmed on read-back, refreshed by polling. They are never the *intended*
//! value of an in-flight edit; the device is the source of truth (ADR-0010).

use crate::event::{ConnectionState, DeviceInfo};
use device::ParameterDef;
use model::UiString;

/// A complete snapshot of the session's observable state.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub connection: ConnectionState,
    /// The identified module, once known (`None` while disconnected/identifying).
    pub device: Option<DeviceInfo>,
    /// The active kit, once known.
    pub current_kit: Option<KitRef>,
    /// The set list currently open for viewing/editing, if one has been read.
    pub setlist: Option<SetlistView>,
    /// What the drummer still has to do on the module itself before the app can
    /// do its whole job — only the switches the app cannot flip for them, and
    /// only until it sees them flipped. Empty once there is nothing left to say.
    pub setup_hints: Vec<SetupHint>,
    /// The active device's parameters with their last-known values + metadata.
    /// Empty until a profile is matched.
    pub parameters: Vec<ParameterView>,
}

/// A set list as the drummer arranged it: the kits, in order. Only the steps the
/// module has confirmed appear — the list ends at its `END` terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetlistView {
    /// 0-based wire number, and the 1-based one to show.
    pub number: u32,
    pub display_number: u32,
    /// The set list's own name (empty until read back).
    pub name: String,
    /// The kits in playing order; `steps[0]` is step 1.
    pub steps: Vec<KitRef>,
    /// How many steps this module's set lists hold.
    pub capacity: u32,
    /// The step the drummer is on (0-based), once they have stepped into the
    /// list. The app's own position — the module keeps none.
    pub position: Option<u32>,
}

/// A reference to a kit: the 0-based wire `number`, the 1-based `display_number`
/// shown to the user, and the kit's name (empty until read back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitRef {
    pub number: u32,
    pub display_number: u32,
    pub name: String,
}

/// One thing to do on the module, as interface text: the string and, when the
/// profile supplies it, the module's own menu path to put in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupHint {
    pub text: UiString,
    pub value: Option<String>,
}

/// How a parameter is presented/edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Numeric,
    Text,
}

/// A parameter's last device-confirmed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValue {
    Int(i64),
    Text(String),
}

/// A parameter, its last-known value, and everything the UI needs to present and
/// edit it without per-device code.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterView {
    pub param_id: String,
    /// Localized control/accessibility label (e.g. "Tempo") — never the value.
    pub label: String,
    pub kind: ParamKind,
    /// Last value the device confirmed (`None` if not read back yet).
    pub value: Option<ParamValue>,
    /// Localized presentation of `value` (e.g. "120.0 BPM"); `None` if unknown.
    pub display: Option<String>,
    /// Numeric editing metadata; present for [`ParamKind::Numeric`].
    pub numeric: Option<NumericInfo>,
}

/// Editing metadata for a numeric parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericInfo {
    /// Display divisor (1 if none): raw 1200 / scale 10 -> shown as 120.0.
    pub scale: i64,
    /// Raw unit token from the profile (e.g. "bpm"); the UI should prefer
    /// [`ParameterView::display`] for the fully localized presentation.
    pub unit: Option<String>,
    /// Inclusive value range, when the profile declares one.
    pub range: Option<NumericRange>,
}

/// The inclusive range of a numeric parameter, in both raw and display units.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericRange {
    pub raw_min: i64,
    pub raw_max: i64,
    /// Smallest raw increment (Roland parameters step by 1).
    pub raw_step: i64,
    /// The display range covers the *numbers* only: a sentinel the profile
    /// declares at an end of the raw range (-INF under the lowest level) is still
    /// inside `raw_min..=raw_max`, but it has no number — it reads through
    /// [`ParameterView::display`].
    pub display_min: f64,
    pub display_max: f64,
    /// Smallest display increment (`raw_step / scale`, e.g. 0.1 BPM).
    pub display_step: f64,
}

impl ParamKind {
    /// Classify a parameter from its wire encoding (text vs numeric).
    pub(crate) fn of(def: &ParameterDef) -> Self {
        if def.encoding.is_text() {
            ParamKind::Text
        } else {
            ParamKind::Numeric
        }
    }
}

/// Build numeric editing metadata from a parameter definition.
pub(crate) fn numeric_info(def: &ParameterDef) -> NumericInfo {
    let scale = def.scale.filter(|s| *s > 1).unwrap_or(1);
    let range = def.range.map(|r| {
        let s = scale as f64;
        // Raw stays raw — the offset is presentation only (see `display_offset`).
        let shown = |raw: i64| (raw + def.display_offset) as f64 / s;
        // A sentinel at an end of the range is not a quantity (−601 under the
        // lowest level is -INF, −1 before the first kit is END): the display
        // range covers the numbers only, and the sentinel speaks through `display`.
        let is_sentinel = |raw: i64| def.sentinel.as_ref().is_some_and(|x| x.raw == raw);
        let lowest = if is_sentinel(r.min) { r.min + 1 } else { r.min };
        let highest = if is_sentinel(r.max) { r.max - 1 } else { r.max };
        NumericRange {
            raw_min: r.min,
            raw_max: r.max,
            raw_step: 1,
            display_min: shown(lowest),
            display_max: shown(highest),
            display_step: 1.0 / s,
        }
    });
    NumericInfo {
        scale,
        unit: def.unit.clone(),
        range,
    }
}
