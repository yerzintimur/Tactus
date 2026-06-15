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
use sysex::Encoding;

/// A complete snapshot of the session's observable state.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub connection: ConnectionState,
    /// The identified module, once known (`None` while disconnected/identifying).
    pub device: Option<DeviceInfo>,
    /// The active kit, once known.
    pub current_kit: Option<KitRef>,
    /// The active device's parameters with their last-known values + metadata.
    /// Empty until a profile is matched.
    pub parameters: Vec<ParameterView>,
}

/// A reference to a kit: the 0-based wire `number`, the 1-based `display_number`
/// shown to the user, and the kit's name (empty until read back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitRef {
    pub number: u32,
    pub display_number: u32,
    pub name: String,
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
    pub display_min: f64,
    pub display_max: f64,
    /// Smallest display increment (`raw_step / scale`, e.g. 0.1 BPM).
    pub display_step: f64,
}

impl ParamKind {
    /// Classify a parameter from its wire encoding (ASCII text vs numeric).
    pub(crate) fn of(def: &ParameterDef) -> Self {
        match def.encoding {
            Encoding::Ascii => ParamKind::Text,
            _ => ParamKind::Numeric,
        }
    }
}

/// Build numeric editing metadata from a parameter definition.
pub(crate) fn numeric_info(def: &ParameterDef) -> NumericInfo {
    let scale = def.scale.filter(|s| *s > 1).unwrap_or(1);
    let range = def.range.map(|r| {
        let s = scale as f64;
        NumericRange {
            raw_min: r.min,
            raw_max: r.max,
            raw_step: 1,
            display_min: r.min as f64 / s,
            display_max: r.max as f64 / s,
            display_step: 1.0 / s,
        }
    });
    NumericInfo {
        scale,
        unit: def.unit.clone(),
        range,
    }
}
