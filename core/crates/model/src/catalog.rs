//! Catalogs: the names behind the numbers a module stores. The module keeps an
//! instrument as a bank + number and an FX type as an index (device state); the
//! names are our data, derived from Roland's Data List and keyed as the module
//! keys them. A catalog can lag behind the module — a Roland Cloud pack we have
//! not catalogued, a firmware that added effects — so an unknown value degrades
//! to a graceful "unknown" that still says the number: never a wrong name, never
//! a failure (ADR-0010).

use crate::i18n::Message;
use device::DeviceProfile;
use device::catalogs::{FxTypes, Instruments};
use std::collections::HashMap;

/// Bank + number → name for one module's instruments.
///
/// On the V31 the preset list is one bank, each expansion pack another, and
/// SYNTH WAVE a bank of its own (PROTOCOL §5): a number alone names nothing —
/// 97 is a tom in the presets and "TR-808 Kick 1" in the Electronic pack.
#[derive(Debug, Clone, Default)]
pub struct InstrumentCatalog {
    /// Bank → number → name.
    banks: HashMap<u32, HashMap<u32, String>>,
    /// Banks that are one built-in instrument, whatever the number says.
    builtin: HashMap<u32, String>,
}

impl InstrumentCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the lookup from parsed catalog data: the preset list under its
    /// bank, every expansion pack under its own, the built-in banks by name.
    pub fn from_data(data: &Instruments) -> Self {
        let mut cat = Self::new();
        for inst in &data.preset {
            cat.insert(data.preset_bank, inst.number, inst.name.clone());
        }
        for pack in &data.expansions {
            if let Some(bank) = pack.bank {
                for inst in &pack.instruments {
                    cat.insert(bank, inst.number, inst.name.clone());
                }
            }
        }
        for builtin in &data.builtin_banks {
            cat.builtin.insert(builtin.bank, builtin.name.clone());
        }
        cat
    }

    /// The built-in V31 catalog (embedded derived data).
    pub fn v31() -> Self {
        let json = device::builtin_catalog_json("roland-v31", "instruments")
            .expect("built-in V31 instrument catalog");
        let data =
            Instruments::from_json(json).expect("built-in V31 instrument catalog must be valid");
        Self::from_data(&data)
    }

    pub fn insert(&mut self, bank: u32, number: u32, name: impl Into<String>) {
        self.banks
            .entry(bank)
            .or_default()
            .insert(number, name.into());
    }

    /// The name of instrument `number` in `bank`, if catalogued.
    pub fn name(&self, bank: u32, number: u32) -> Option<&str> {
        if let Some(name) = self.builtin.get(&bank) {
            return Some(name);
        }
        self.banks.get(&bank)?.get(&number).map(String::as_str)
    }

    /// A localizable label: the name if known; otherwise an honest "unknown" that
    /// still carries the number — and the bank, when the bank itself is unknown
    /// here, so a drummer with an uncatalogued pack can tell us which one.
    pub fn label(&self, bank: u32, number: u32) -> Message {
        match self.name(bank, number) {
            // Instrument names are the module's own English text (ADR-0011).
            Some(name) => Message::new("instrument.name").device_arg("name", name),
            None if self.banks.contains_key(&bank) => {
                Message::new("instrument.unknown").arg("number", number)
            }
            None => Message::new("instrument.unknown_bank")
                .arg("number", number)
                .arg("bank", bank),
        }
    }
}

/// Every value catalog a profile names, loaded and ready to answer "what is raw
/// value N of this parameter called?" — what [`crate::format_parameter`] needs.
#[derive(Debug, Clone, Default)]
pub struct Catalogs {
    instruments: HashMap<String, InstrumentCatalog>,
    lists: HashMap<String, Vec<String>>,
}

impl Catalogs {
    /// No catalogs: every catalogued value falls back to its number.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load the built-in catalogs the profile names. The kind of each is read
    /// from its shape: an instrument catalog has banks of numbered instruments,
    /// a plain list (FX types) is indexed by the raw value. Anything else — the
    /// kit reference list — is not a value lookup and is skipped.
    pub fn for_profile(profile: &DeviceProfile) -> Self {
        let mut cats = Self::empty();
        for key in profile.catalogs.keys() {
            let Some(json) = device::builtin_catalog_json(&profile.profile_id, key) else {
                continue;
            };
            if let Ok(data) = Instruments::from_json(json) {
                cats.instruments
                    .insert(key.clone(), InstrumentCatalog::from_data(&data));
            } else if let Ok(list) = FxTypes::from_json(json) {
                cats.lists.insert(key.clone(), list.types);
            }
        }
        cats
    }

    /// The label for `raw` in `catalog` — with `bank` when the catalog is
    /// banked. `None` when there is nothing honest to say: the catalog is not
    /// loaded, the value is past the list, or a banked catalog was asked
    /// without its bank (guessing "preset" would name the wrong instrument on
    /// an expansion kit). Callers then fall back to the number.
    pub fn label(&self, catalog: &str, bank: Option<u32>, raw: i64) -> Option<Message> {
        let number = u32::try_from(raw).ok()?;
        if let Some(cat) = self.instruments.get(catalog) {
            return Some(cat.label(bank?, number));
        }
        let name = self.lists.get(catalog)?.get(number as usize)?;
        Some(Message::new("param.enum_value").device_arg("value", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Localizer;

    #[test]
    fn known_instrument_uses_name() {
        let mut cat = InstrumentCatalog::new();
        cat.insert(0, 35, "DW Concrete S");
        let loc = Localizer::new();
        assert_eq!(loc.format(&cat.label(0, 35), "en"), "DW Concrete S");
    }

    /// The real derived data, keyed as the module keys it (PROTOCOL §5): the
    /// presets under bank 0, each pack under 2006 + its number, SYNTH WAVE as a
    /// bank of its own.
    #[test]
    fn builtin_v31_catalog_speaks_real_names_per_bank() {
        let cat = InstrumentCatalog::v31();
        let loc = Localizer::new();
        assert_eq!(loc.format(&cat.label(0, 35), "en"), "DW Concrete S");
        assert_eq!(loc.format(&cat.label(0, 0), "en"), "OFF");
        // Kit 46 "UK Wet Booth" and kit 49 "TR-808", as read from the module.
        assert_eq!(loc.format(&cat.label(2007, 1), "en"), "Cm Vintage K");
        assert_eq!(loc.format(&cat.label(2008, 97), "en"), "TR-808 Kick 1");
        // Kit 69 "Jazz Brushes": the pack in slot 2 is still bank 2006 + 3.
        assert_eq!(loc.format(&cat.label(2009, 1), "en"), "Pl MapleBrush S");
        // The same number in another bank is another instrument.
        assert_ne!(cat.name(0, 97), cat.name(2008, 97));
        // Kit 9 "Pure Analog": every unit is SYNTH WAVE, number 0.
        assert_eq!(loc.format(&cat.label(1, 0), "en"), "SYNTH WAVE");
        // Numbers past a known list still degrade gracefully.
        assert_eq!(
            loc.format(&cat.label(0, 9999), "en"),
            "Instrument #9999 (unknown)"
        );
    }

    /// A pack we have not catalogued (two were installed on the test unit): say
    /// so with both numbers, so the drummer can tell us which pack it is.
    #[test]
    fn unknown_bank_says_the_bank() {
        let cat = InstrumentCatalog::v31();
        let loc = Localizer::new();
        assert_eq!(
            loc.format(&cat.label(2032, 3), "en"),
            "Instrument #3 in bank 2032 (unknown)"
        );
        assert_eq!(
            loc.format(&cat.label(2032, 3), "ru"),
            "Инструмент №3 в банке 2032 (неизвестен)"
        );
    }

    #[test]
    fn unknown_instrument_falls_back() {
        let mut cat = InstrumentCatalog::new();
        cat.insert(0, 1, "TM SL Maple K"); // bank 0 known, 234 not catalogued
        let loc = Localizer::new();
        assert_eq!(
            loc.format(&cat.label(0, 234), "en"),
            "Instrument #234 (unknown)"
        );
        assert_eq!(
            loc.format(&cat.label(0, 234), "ru"),
            "Инструмент №234 (неизвестен)"
        );
    }
}
