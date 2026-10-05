//! The `DeviceProfile` schema: a data-only description of one Roland module type
//! (Model ID, capabilities, address map, catalogs). Generic code consumes this;
//! there is no per-module code. See ADR-0007 and docs/DEVELOPMENT.md §3.

use crate::firmware::{FirmwareSupport, FirmwareVersion};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use sysex::Encoding;

/// A complete description of one module type. Parsed from JSON (`profiles/*.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceProfile {
    /// Schema version of this profile document (see `crate::SCHEMA_VERSION`).
    pub schema_version: u32,
    pub profile_id: String,
    pub display_name: String,
    #[serde(default)]
    pub family: Option<String>,
    /// SysEx Model ID — the key used to auto-detect this module from its Identity Reply.
    pub model_id: Vec<u8>,
    #[serde(default)]
    pub device_id_default: Option<u8>,
    /// How to recognise this module from its Identity Reply (manufacturer +
    /// device family/member codes). Distinct from `model_id`, which frames DT1/RQ1.
    #[serde(default)]
    pub identity: Option<Identity>,
    /// Citation of the source documents the data was derived from.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub firmware: FirmwareConfig,
    /// Whether this map has been exercised on a real module. Absent means it has
    /// not: the engine then reads and announces, but never writes (ADR-0016).
    #[serde(default)]
    pub verification: Verification,
    #[serde(default)]
    pub capabilities: Capabilities,
    /// Named parameter areas (Current, Setup, Kit, …) keyed by name.
    pub areas: BTreeMap<String, AreaDef>,
    /// What each position of a repeat dimension is called, keyed by the
    /// `DimDef::name` the parameters use ("unit" → kick, snare head, snare rim
    /// …). Device data: which pads a module has, and in what order, is a fact
    /// about the module (PROTOCOL §5), so it lives here and not in code.
    #[serde(default)]
    pub dimensions: BTreeMap<String, DimensionDef>,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    /// Named catalog files (instruments / fx / ambience), relative paths.
    #[serde(default)]
    pub catalogs: BTreeMap<String, String>,
}

/// What stands behind a profile's address map: a module that has run it, or
/// only the documents it was derived from.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Verification {
    /// `true` once writes, read-backs and persistence have been confirmed on the
    /// module itself. Until then the profile is read-only: a wrong offset would
    /// pass our own read-back (it reads the address it wrote) while changing a
    /// neighbouring parameter on someone's instrument (ADR-0016).
    #[serde(default)]
    pub on_hardware: bool,
    /// The evidence: bench sessions with dates, or the sources a profile built
    /// without the module was checked against.
    #[serde(default)]
    pub basis: Option<String>,
}

/// Firmware the profile was tested against + the version-byte format.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FirmwareConfig {
    #[serde(default)]
    pub tested: Vec<FirmwareVersion>,
    /// How the module's own screen renders the 4 version bytes — a template for
    /// [`FirmwareVersion::display_as`], verified on hardware. Absent: dotted bytes.
    #[serde(default)]
    pub version_format: Option<String>,
    /// For a module whose Identity Reply carries a *code* rather than the version
    /// itself: the version each code stands for, keyed by the dotted bytes
    /// (`"0.0.0.2"` → `"2.00"` on the TD-17). A code not listed falls through to
    /// `version_format`, so a newer firmware still shows something true.
    #[serde(default)]
    pub version_names: BTreeMap<String, String>,
}

impl FirmwareConfig {
    /// `version` as this module shows it on its own screen, so the app and the
    /// module's manual agree on what to call the firmware.
    pub fn display(&self, version: FirmwareVersion) -> String {
        if let Some(name) = self.version_names.get(&version.display()) {
            return name.clone();
        }
        match self.version_format.as_deref() {
            Some(format) => version.display_as(format),
            None => version.display(),
        }
    }
}

/// The Universal Identity Reply fingerprint that identifies this module
/// (`F0 7E dd 06 02 <manufacturer> <family×2> <member×2> <version×4> F7`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Identity {
    pub manufacturer: u8,
    pub family: [u8; 2],
    pub member: [u8; 2],
}

/// Coarse module capabilities, so the UI/engine can adapt without hardcoding.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub kit_count: u32,
    #[serde(default)]
    pub fx_slots: u32,
    #[serde(default)]
    pub features: Vec<String>,
    /// Where on the module's own screen a feature is switched, by feature name
    /// — the path the app tells the drummer when it cannot flip the switch
    /// itself (Transmit Edit Data is not in the address map). Written for
    /// speech: the menu names in order, comma-separated.
    #[serde(default)]
    pub menus: BTreeMap<String, String>,
}

/// A top-level parameter area: a base address, plus an optional repeat (stride +
/// count) for indexed areas like kits/triggers.
#[derive(Debug, Clone, Deserialize)]
pub struct AreaDef {
    pub address: [u8; 4],
    #[serde(default)]
    pub stride: Option<[u8; 4]>,
    #[serde(default)]
    pub count: Option<u32>,
}

/// One parameter: where it is (area + offset), how big, how encoded, and how to
/// present it (scale/unit/i18n — used by the model layer, not here).
#[derive(Debug, Clone, Deserialize)]
pub struct ParameterDef {
    pub id: String,
    pub area: String,
    /// Right-aligned offset within the area/unit (1–4 bytes, 7-bit each).
    pub offset: Vec<u8>,
    pub len: usize,
    #[serde(deserialize_with = "de_encoding")]
    pub encoding: Encoding,
    /// Repeat dimensions inside the area (per-layer, per-pad, per-FX-slot …),
    /// outermost first. `address_of` consumes one index per dimension, after
    /// the area index.
    #[serde(default)]
    pub dims: Vec<DimDef>,
    #[serde(default)]
    pub range: Option<ValueRange>,
    /// Divisor applied for display (e.g. tempo 1200 -> 120.0). Presentation only.
    #[serde(default)]
    pub scale: Option<i64>,
    /// Added to the raw value for display, before `scale`. Roland numbers kits
    /// and set-list steps from 0 on the wire and from 1 on screen — the doc's
    /// value column says `0 - 199`, its display column `1 - 200`. Presentation
    /// only: everything on the wire stays raw.
    #[serde(default)]
    pub display_offset: i64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub i18n_key: Option<String>,
    /// A raw value that means something other than a number on this parameter
    /// (a set-list step's `END`, a level's `-INF`). Device data, so it belongs
    /// in the profile — speaking it as a quantity would be a lie.
    #[serde(default)]
    pub sentinel: Option<Sentinel>,
    /// The catalog — a key of the profile's `catalogs` map — that names this
    /// parameter's values: an instrument number, an FX type. The wire value
    /// stays a number; the model layer speaks the name.
    #[serde(default)]
    pub catalog: Option<String>,
    /// The parameter whose value, at the same indices, selects the bank within
    /// `catalog`: a V31 instrument number names nothing without its `Inst Bank`.
    #[serde(default)]
    pub catalog_bank: Option<String>,
    /// Enum value labels (raw = range.min + position), verbatim from the docs.
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Provenance: `"Block/Parameter Name"` in the parsed address map
    /// (profiles/maps/…), cross-checked by tests.
    #[serde(default)]
    pub doc: Option<String>,
}

/// A parameter found at an address: its definition and the indices (the area's,
/// then one per `dims` entry) that put it there — the inverse of
/// [`DeviceProfile::address_of`].
#[derive(Debug, Clone)]
pub struct Located<'a> {
    pub param: &'a ParameterDef,
    pub indices: Vec<u32>,
}

impl Located<'_> {
    /// The indices that belong to the parameter's own `dims`, one each in
    /// declaration order — what is left after the area's index, if its area has
    /// one. Pairs with [`ParameterDef::dims`] to name the position.
    pub fn dim_indices(&self) -> &[u32] {
        let start = self.indices.len().saturating_sub(self.param.dims.len());
        &self.indices[start..]
    }
}

impl ParameterDef {
    /// The documented label for a raw enum value (`labels[raw - range.min]`), or
    /// `None` if the parameter isn't an enum or the value is outside the list.
    /// The words are Roland's own — the model speaks them verbatim rather than
    /// translating the module's vocabulary (ADR-0011).
    pub fn enum_label(&self, raw: i64) -> Option<&str> {
        let labels = self.labels.as_ref()?;
        let min = self.range.map_or(0, |r| r.min);
        let index = usize::try_from(raw.checked_sub(min)?).ok()?;
        labels.get(index).map(String::as_str)
    }
}

/// One repeat dimension of a parameter: how many instances and how far apart.
#[derive(Debug, Clone, Deserialize)]
pub struct DimDef {
    /// What the dimension ranges over ("unit", "layer", "pad", "fx" …) — the key
    /// into [`DeviceProfile::dimensions`] that names its positions.
    pub name: String,
    pub count: u32,
    /// Right-aligned address step between instances (7-bit bytes, like offsets).
    pub stride: Vec<u8>,
}

/// The names of one dimension's positions, shared by every parameter that
/// repeats over it. Either a label key per position (pads, layers, FX slots) or
/// one key that takes the 1-based number (set-list steps).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DimensionDef {
    /// i18n key for each position, in index order; as long as the dimension's
    /// `count` when present.
    #[serde(default)]
    pub labels: Vec<String>,
    /// i18n key of a numbered label ("Step { $number }") for positions without a
    /// name of their own.
    #[serde(default)]
    pub i18n_key: Option<String>,
    /// Provenance in the vendor documents (ADR-0004).
    #[serde(default)]
    pub doc: Option<String>,
}

/// How to call one position of a dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimLabel<'a> {
    /// The position has a name of its own: the i18n key of that name.
    Named(&'a str),
    /// The position is only a number: the i18n key of a numbered phrase and the
    /// 1-based number to put in it.
    Numbered { key: &'a str, number: u32 },
}

/// Inclusive valid range of a parameter's raw value.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ValueRange {
    pub min: i64,
    pub max: i64,
}

/// A raw value with a meaning of its own, spoken through its own message rather
/// than as a number (`-1` on a set-list step is the list's end, not kit zero).
#[derive(Debug, Clone, Deserialize)]
pub struct Sentinel {
    pub raw: i64,
    pub i18n_key: String,
}

impl DeviceProfile {
    /// Parse a profile from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Look up a parameter definition by id.
    pub fn parameter(&self, id: &str) -> Option<&ParameterDef> {
        self.parameters.iter().find(|p| p.id == id)
    }

    /// Resolve the absolute 4-byte address of a parameter.
    ///
    /// `indices` are consumed in order: one for the area (when it repeats,
    /// e.g. the 0-based kit number), then one per `dims` entry (layer, unit,
    /// pad …). Missing indices default to 0; an out-of-range index is `None`
    /// (never a silent write to a neighbouring address).
    pub fn address_of(&self, param_id: &str, indices: &[u32]) -> Option<[u8; 4]> {
        let param = self.parameter(param_id)?;
        let area = self.areas.get(&param.area)?;
        let mut idx = indices.iter().copied();
        let mut base = area.address;
        if let Some(stride) = area.stride {
            let index = idx.next().unwrap_or(0);
            if area.count.is_some_and(|c| index >= c) {
                return None;
            }
            base = sysex::address::with_stride(base, stride, index);
        }
        for dim in &param.dims {
            let index = idx.next().unwrap_or(0);
            if index >= dim.count {
                return None;
            }
            base = sysex::address::with_stride(base, pad_stride(&dim.stride), index);
        }
        Some(sysex::address::add_offset(base, &param.offset))
    }

    /// Which parameter starts at `address`, and at which indices — for a DT1 the
    /// module sends on its own: with Transmit Edit Data on, a knob turned on the
    /// panel arrives as a write to the parameter's own address, with the index of
    /// the kit being edited (PROTOCOL §6). The inverse of [`Self::address_of`].
    ///
    /// `None` when no parameter *starts* there: an address inside a parameter,
    /// or one the profile does not describe, is never guessed at.
    pub fn locate(&self, address: [u8; 4]) -> Option<Located<'_>> {
        let target = sysex::address::to_linear(address);
        self.parameters.iter().find_map(|param| {
            let area = self.areas.get(&param.area)?;
            let offset = sysex::address::to_linear(pad_stride(&param.offset));
            // Peel the grid outermost first: the area's repeat, then each dim.
            let mut rem = target
                .checked_sub(sysex::address::to_linear(area.address))?
                .checked_sub(offset)?;
            let mut indices = Vec::new();
            if let Some(stride) = area.stride {
                let stride = sysex::address::to_linear(stride);
                let index = if stride == 0 { 0 } else { rem / stride };
                if area.count.is_some_and(|c| index >= c) {
                    return None;
                }
                indices.push(index);
                rem -= index * stride;
            }
            for dim in &param.dims {
                let stride = sysex::address::to_linear(pad_stride(&dim.stride));
                let index = if stride == 0 { 0 } else { rem / stride };
                if index >= dim.count {
                    return None;
                }
                indices.push(index);
                rem -= index * stride;
            }
            (rem == 0).then_some(Located { param, indices })
        })
    }

    /// How the profile calls position `index` (0-based) of `dim`: a name when
    /// the dimension lists one, a numbered phrase when it only counts, `None`
    /// when the profile says nothing — the caller then says nothing rather than
    /// reading out a raw index.
    pub fn dim_label(&self, dim: &DimDef, index: u32) -> Option<DimLabel<'_>> {
        let def = self.dimensions.get(&dim.name)?;
        if let Some(key) = def.labels.get(index as usize) {
            return Some(DimLabel::Named(key));
        }
        let key = def.i18n_key.as_deref()?;
        Some(DimLabel::Numbered {
            key,
            number: index + 1,
        })
    }

    /// Whether the engine may send a DT1 to a module running this profile: only
    /// once the map has been confirmed on hardware (ADR-0016). Reads and
    /// announcements are always allowed.
    pub fn allows_writes(&self) -> bool {
        self.verification.on_hardware
    }

    /// The highest kit number the module accepts (0-based), if the profile says.
    ///
    /// The wire truth is the `current.kit_num` range — that is what the module
    /// actually stores at that address; `capabilities.kit_count` states the same
    /// fact as a count and covers profiles that don't pin a range. `None` means
    /// the profile declares neither: navigation then stays unclamped and the
    /// module remains the only authority (ADR-0010), as in degraded mode.
    pub fn max_kit_number(&self) -> Option<u32> {
        self.parameter("current.kit_num")
            .and_then(|p| p.range)
            .and_then(|r| u32::try_from(r.max).ok())
            .or_else(|| self.capabilities.kit_count.checked_sub(1))
    }

    /// Classify the connected firmware against this profile's tested set (ADR-0009).
    pub fn firmware_support(&self, version: FirmwareVersion) -> FirmwareSupport {
        FirmwareSupport::classify(&self.firmware.tested, version)
    }
}

/// Left-pad a right-aligned stride (1–4 bytes) to the full 4-byte form.
fn pad_stride(stride: &[u8]) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (slot, &b) in out.iter_mut().rev().zip(stride.iter().rev()) {
        *slot = b & 0x7F;
    }
    out
}

/// Deserialize an encoding tag ("plain7" | "nibble" | "signed" | "signed_nibble"
/// | "ascii") into the shared `sysex::Encoding`.
fn de_encoding<'de, D: Deserializer<'de>>(d: D) -> Result<Encoding, D::Error> {
    let s = String::deserialize(d)?;
    match s.as_str() {
        "plain7" => Ok(Encoding::Plain7),
        "nibble" => Ok(Encoding::Nibble),
        "signed" => Ok(Encoding::Signed),
        "signed_nibble" => Ok(Encoding::SignedNibble),
        "ascii" => Ok(Encoding::Ascii),
        "ascii_nibble" => Ok(Encoding::AsciiNibble),
        other => Err(serde::de::Error::custom(format!(
            "unknown encoding: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A small synthetic profile (the real V31 profile is authored in task #6).
    const PROFILE: &str = r#"{
        "schema_version": 1,
        "profile_id": "test-mod",
        "display_name": "Test Module",
        "model_id": [1, 6, 1],
        "device_id_default": 16,
        "firmware": { "tested": [[1, 0, 0, 0]] },
        "capabilities": { "kit_count": 200, "fx_slots": 4, "features": ["transmit_edit_data"] },
        "areas": {
            "current": { "address": [0, 0, 0, 0] },
            "kit": { "address": [4, 0, 0, 0], "stride": [0, 4, 0, 0], "count": 200 }
        },
        "parameters": [
            { "id": "current.kit_num", "area": "current", "offset": [0, 0], "len": 4, "encoding": "nibble", "range": { "min": 0, "max": 199 } },
            { "id": "kit.common.name", "area": "kit", "offset": [0, 0], "len": 16, "encoding": "ascii" },
            { "id": "kit.snare_eq", "area": "kit", "offset": [0, 82, 33], "len": 1, "encoding": "plain7" }
        ]
    }"#;

    fn profile() -> DeviceProfile {
        DeviceProfile::from_json(PROFILE).expect("valid profile")
    }

    #[test]
    fn parses_fields_and_encodings() {
        let p = profile();
        assert_eq!(p.profile_id, "test-mod");
        assert_eq!(p.model_id, vec![1, 6, 1]);
        assert_eq!(p.capabilities.kit_count, 200);
        assert_eq!(
            p.parameter("kit.common.name").unwrap().encoding,
            Encoding::Ascii
        );
        assert_eq!(
            p.parameter("current.kit_num").unwrap().encoding,
            Encoding::Nibble
        );
    }

    #[test]
    fn kit_bound_prefers_the_wire_range_and_falls_back_to_the_capability() {
        let p = profile();
        assert_eq!(p.max_kit_number(), Some(199));

        // No range on the wire parameter → the declared count still bounds it.
        let without_range = PROFILE.replace(r#", "range": { "min": 0, "max": 199 }"#, "");
        let p = DeviceProfile::from_json(&without_range).expect("valid profile");
        assert_eq!(p.max_kit_number(), Some(199));

        // A profile that declares neither doesn't get to clamp anything.
        let silent = without_range.replace(r#""kit_count": 200"#, r#""kit_count": 0"#);
        let p = DeviceProfile::from_json(&silent).expect("valid profile");
        assert_eq!(p.max_kit_number(), None);
    }

    #[test]
    fn resolves_current_address() {
        let p = profile();
        assert_eq!(p.address_of("current.kit_num", &[]), Some([0, 0, 0, 0]));
    }

    #[test]
    fn resolves_kit_addresses_with_stride_and_offset() {
        let p = profile();
        // kit 1 (index 0): base 04 00 00 00 + offset 00 52 21 = 04 00 52 21 (golden G1).
        assert_eq!(
            p.address_of("kit.snare_eq", &[0]),
            Some([0x04, 0x00, 0x52, 0x21])
        );
        // kit 200 (index 199): base 0A 1C 00 00 + offset 00 52 21.
        assert_eq!(
            p.address_of("kit.snare_eq", &[199]),
            Some([0x0A, 0x1C, 0x52, 0x21])
        );
        // kit name of kit 200: base 0A 1C 00 00 + offset 00 00 = 0A 1C 00 00.
        assert_eq!(
            p.address_of("kit.common.name", &[199]),
            Some([0x0A, 0x1C, 0x00, 0x00])
        );
    }

    #[test]
    fn unknown_parameter_is_none() {
        let p = profile();
        assert!(p.parameter("nope").is_none());
        assert_eq!(p.address_of("nope", &[0]), None);
    }

    const DIMS_PROFILE: &str = r#"{
        "schema_version": 1,
        "profile_id": "test-dims",
        "display_name": "Test Dims",
        "model_id": [1, 6, 1],
        "areas": {
            "kit": { "address": [4, 0, 0, 0], "stride": [0, 4, 0, 0], "count": 200 }
        },
        "parameters": [
            { "id": "kit.unit.layer.instrument", "area": "kit",
              "offset": [0, 80, 1], "len": 4, "encoding": "nibble",
              "dims": [
                  { "name": "layer", "count": 3, "stride": [0, 64, 0] },
                  { "name": "unit", "count": 28, "stride": [0, 2, 0] }
              ] }
        ]
    }"#;

    #[test]
    fn resolves_dims_addresses() {
        let p = DeviceProfile::from_json(DIMS_PROFILE).unwrap();
        // Kit 1, layer A, unit 1: kit base + Kit Unit LayerA 1 (00 50 00) + leaf 01.
        assert_eq!(
            p.address_of("kit.unit.layer.instrument", &[0, 0, 0]),
            Some([0x04, 0x00, 0x50, 0x01])
        );
        // Layer B, unit 2 lands on the doc's "Kit Unit LayerB 2" row (01 12 00):
        // 00 50 00 + 00 40 00 + 00 02 00 = 01 12 00 in 7-bit arithmetic.
        assert_eq!(
            p.address_of("kit.unit.layer.instrument", &[0, 1, 1]),
            Some([0x04, 0x01, 0x12, 0x01])
        );
        // Missing dim indices default to 0.
        assert_eq!(
            p.address_of("kit.unit.layer.instrument", &[0]),
            Some([0x04, 0x00, 0x50, 0x01])
        );
    }

    #[test]
    fn out_of_range_indices_are_none() {
        let p = DeviceProfile::from_json(DIMS_PROFILE).unwrap();
        assert_eq!(p.address_of("kit.unit.layer.instrument", &[200]), None); // kit
        assert_eq!(p.address_of("kit.unit.layer.instrument", &[0, 3, 0]), None); // layer
        assert_eq!(p.address_of("kit.unit.layer.instrument", &[0, 0, 28]), None); // unit
    }

    #[test]
    fn firmware_support_uses_tested_list() {
        let p = profile();
        assert_eq!(
            p.firmware_support(FirmwareVersion::new([1, 0, 0, 0])),
            FirmwareSupport::Tested
        );
        assert_eq!(
            p.firmware_support(FirmwareVersion::new([2, 0, 0, 0])),
            FirmwareSupport::UntestedNewer
        );
    }

    #[test]
    fn rejects_unknown_encoding() {
        let bad = PROFILE.replace("\"nibble\"", "\"weird\"");
        assert!(DeviceProfile::from_json(&bad).is_err());
    }
}
