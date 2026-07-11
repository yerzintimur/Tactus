//! Instrument catalog: maps an instrument *number* (device state) to a *name*
//! (our data). The catalog can lag behind Roland Cloud expansions, so an unknown
//! number degrades gracefully to "Instrument #N (unknown)" — never fails (ADR-0010).

use crate::i18n::Message;
use std::collections::HashMap;

/// Number → name lookup for one module's instruments.
#[derive(Debug, Clone, Default)]
pub struct InstrumentCatalog {
    names: HashMap<u32, String>,
}

impl InstrumentCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the number→name lookup from parsed catalog data (the *preset*
    /// bank; expansion-pack bank numbering is confirmed on hardware first).
    pub fn from_data(data: &device::catalogs::Instruments) -> Self {
        let mut cat = Self::new();
        for inst in &data.preset {
            cat.insert(inst.number, inst.name.clone());
        }
        cat
    }

    /// The built-in V31 catalog (embedded derived data).
    pub fn v31() -> Self {
        let json = device::builtin_catalog_json("roland-v31", "instruments")
            .expect("built-in V31 instrument catalog");
        let data = device::catalogs::Instruments::from_json(json)
            .expect("built-in V31 instrument catalog must be valid");
        Self::from_data(&data)
    }

    pub fn insert(&mut self, number: u32, name: impl Into<String>) {
        self.names.insert(number, name.into());
    }

    pub fn name(&self, number: u32) -> Option<&str> {
        self.names.get(&number).map(String::as_str)
    }

    /// A localizable label for an instrument number: its name if known, else a
    /// graceful "unknown" fallback that still tells the user the number.
    pub fn label(&self, number: u32) -> Message {
        match self.name(number) {
            Some(name) => Message::new("instrument.name").arg("name", name),
            None => Message::new("instrument.unknown").arg("number", number),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Localizer;

    #[test]
    fn known_instrument_uses_name() {
        let mut cat = InstrumentCatalog::new();
        cat.insert(35, "DW Concrete S");
        let loc = Localizer::new();
        assert_eq!(loc.format(&cat.label(35), "en"), "DW Concrete S");
    }

    #[test]
    fn builtin_v31_catalog_speaks_real_names() {
        let cat = InstrumentCatalog::v31();
        let loc = Localizer::new();
        // Real derived data: 35 really is the DW Concrete snare on the V31.
        assert_eq!(loc.format(&cat.label(35), "en"), "DW Concrete S");
        assert_eq!(loc.format(&cat.label(0), "en"), "OFF");
        // Numbers past the preset bank still degrade gracefully.
        assert_eq!(
            loc.format(&cat.label(9999), "en"),
            "Instrument #9999 (unknown)"
        );
    }

    #[test]
    fn unknown_instrument_falls_back() {
        let cat = InstrumentCatalog::new(); // empty (e.g. a Roland Cloud expansion)
        let loc = Localizer::new();
        assert_eq!(
            loc.format(&cat.label(234), "en"),
            "Instrument #234 (unknown)"
        );
        assert_eq!(
            loc.format(&cat.label(234), "ru"),
            "Инструмент №234 (неизвестен)"
        );
    }
}
