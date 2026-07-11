//! Catalog data: the derived JSON files referenced by a profile's `catalogs`
//! map (instruments / drum kits / FX types), parsed into typed data. Generated
//! by tools/parse_datalist.py from Roland's Data List; committed under
//! profiles/catalogs/. The model layer turns these into speech.

use serde::Deserialize;

/// The instrument catalog: preset instruments plus expansion packs.
#[derive(Debug, Clone, Deserialize)]
pub struct Instruments {
    pub preset: Vec<InstrumentEntry>,
    #[serde(default)]
    pub expansions: Vec<ExpansionPack>,
}

/// One instrument: its device number, doc group, display name, and remark
/// flags (M mic, P positional, X cross-stick, O overtone, L lo-cut).
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentEntry {
    pub number: u32,
    pub group: String,
    pub name: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

/// A downloadable instrument expansion (EXV) pack; numbering restarts at 1
/// per pack (bank selection semantics to be confirmed on hardware).
#[derive(Debug, Clone, Deserialize)]
pub struct ExpansionPack {
    pub id: String,
    pub title: String,
    pub instruments: Vec<InstrumentEntry>,
}

/// The preset drum kits as shipped (the module's kit names are the live
/// source of truth; this is reference data for seeding and tests).
#[derive(Debug, Clone, Deserialize)]
pub struct DrumKits {
    pub kits: Vec<KitEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KitEntry {
    /// 1-based, as displayed (the wire value is 0-based).
    pub number: u32,
    pub name: String,
    #[serde(default)]
    pub sub_name: String,
}

/// The Bus FX type list; the raw `kit.fx.type` value indexes into `types`.
#[derive(Debug, Clone, Deserialize)]
pub struct FxTypes {
    pub types: Vec<String>,
}

impl Instruments {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl DrumKits {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl FxTypes {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::builtin_catalog_json;

    #[test]
    fn v31_instruments_parse_and_contain_known_entries() {
        let json = builtin_catalog_json("roland-v31", "instruments").unwrap();
        let cat = Instruments::from_json(json).unwrap();
        assert_eq!(cat.preset[0].name, "OFF");
        let snare = &cat.preset[35];
        assert_eq!(snare.number, 35);
        assert_eq!(snare.name, "DW Concrete S");
        assert_eq!(snare.group, "SNARE");
        assert_eq!(snare.flags, ["M", "P", "X", "O"]);
        assert_eq!(cat.expansions.len(), 3);
        assert_eq!(cat.expansions[0].id, "EXV001");
    }

    #[test]
    fn v31_drum_kits_have_200_entries() {
        let json = builtin_catalog_json("roland-v31", "drum-kits").unwrap();
        let kits = DrumKits::from_json(json).unwrap();
        assert_eq!(kits.kits.len(), 200);
        assert_eq!(kits.kits[4].name, "Massive Metal"); // kit 5, 1-based
        assert_eq!(kits.kits[8].sub_name, "Mid-O Style");
    }

    #[test]
    fn v31_fx_types_cover_the_full_enum() {
        let json = builtin_catalog_json("roland-v31", "fx-types").unwrap();
        let fx = FxTypes::from_json(json).unwrap();
        assert_eq!(fx.types.len(), 95); // matches kit.fx.type range 0..=94
        assert_eq!(fx.types[0], "THRU");
        assert_eq!(fx.types[94], "JD-MULTI");
    }
}
