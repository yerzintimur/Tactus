//! Device profiles: everything specific to a particular Roland module lives here
//! as *data* (`DeviceProfile`), never hardcoded in logic. The registry resolves a
//! profile from the module's Identity Reply (Model ID).
//!
//! See ADR-0007 and docs/DEVELOPMENT.md §3, §4.2.
#![forbid(unsafe_code)]

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `DeviceProfile` JSON schema version (see docs/DEVELOPMENT.md §3.1).
pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn links_against_sysex() {
        // Smoke: the dependency edge device -> sysex is wired.
        assert_eq!(sysex::roland_checksum(&[0x00]), 0x00);
    }
}
