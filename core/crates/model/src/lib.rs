//! Domain model: typed parameters/areas and aggregates (kit, pads, FX, ambience),
//! edit intents, and localizable value formatting (`Message { id, args }`).
//!
//! See docs/DEVELOPMENT.md §4.3, §8 and ADR-0008 (i18n in the core via Fluent).
#![forbid(unsafe_code)]

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn links_against_lower_crates() {
        assert_eq!(sysex::roland_checksum(&[]), 0x00);
        assert_eq!(device::SCHEMA_VERSION, 1);
    }
}
