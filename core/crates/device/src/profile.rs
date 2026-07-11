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
    #[serde(default)]
    pub capabilities: Capabilities,
    /// Named parameter areas (Current, Setup, Kit, …) keyed by name.
    pub areas: BTreeMap<String, AreaDef>,
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
    /// Named catalog files (instruments / fx / ambience), relative paths.
    #[serde(default)]
    pub catalogs: BTreeMap<String, String>,
}

/// Firmware the profile was tested against + the version-byte format.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FirmwareConfig {
    #[serde(default)]
    pub tested: Vec<FirmwareVersion>,
    /// How to render the 4 version bytes (verified on hardware).
    #[serde(default)]
    pub version_format: Option<String>,
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
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub i18n_key: Option<String>,
    /// Enum value labels (raw = range.min + position), verbatim from the docs.
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Provenance: `"Block/Parameter Name"` in the parsed address map
    /// (profiles/maps/…), cross-checked by tests.
    #[serde(default)]
    pub doc: Option<String>,
}

/// One repeat dimension of a parameter: how many instances and how far apart.
#[derive(Debug, Clone, Deserialize)]
pub struct DimDef {
    /// What the dimension ranges over ("unit", "layer", "pad", "fx" …).
    pub name: String,
    pub count: u32,
    /// Right-aligned address step between instances (7-bit bytes, like offsets).
    pub stride: Vec<u8>,
}

/// Inclusive valid range of a parameter's raw value.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ValueRange {
    pub min: i64,
    pub max: i64,
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
