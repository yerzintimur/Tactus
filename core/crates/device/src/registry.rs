//! Holds the known device profiles and resolves one from a connected module's
//! Model ID (auto-detection). See ADR-0007 and docs/DEVELOPMENT.md §3.2.

use crate::profile::DeviceProfile;

/// Built-in profile JSON, embedded at compile time from the workspace-root
/// `profiles/` dir. `WORKSPACE_DIR` is defined in `.cargo/config.toml`.
const ROLAND_V31_JSON: &str =
    include_str!(concat!(env!("WORKSPACE_DIR"), "/profiles/roland-v31.json"));

/// Embedded catalog JSON for built-in profiles: (profile id, catalog name in
/// the profile's `catalogs` map) → file contents. Downloadable profile packs
/// will carry their catalogs alongside the profile instead.
const BUILTIN_CATALOGS: &[(&str, &str, &str)] = &[
    (
        "roland-v31",
        "drum-kits",
        include_str!(concat!(
            env!("WORKSPACE_DIR"),
            "/profiles/catalogs/roland-v31/drum-kits.json"
        )),
    ),
    (
        "roland-v31",
        "instruments",
        include_str!(concat!(
            env!("WORKSPACE_DIR"),
            "/profiles/catalogs/roland-v31/instruments.json"
        )),
    ),
    (
        "roland-v31",
        "fx-types",
        include_str!(concat!(
            env!("WORKSPACE_DIR"),
            "/profiles/catalogs/roland-v31/fx-types.json"
        )),
    ),
];

/// The embedded catalog JSON for a built-in profile, by the name used in the
/// profile's `catalogs` map (e.g. `("roland-v31", "instruments")`).
pub fn builtin_catalog_json(profile_id: &str, catalog: &str) -> Option<&'static str> {
    BUILTIN_CATALOGS
        .iter()
        .find(|(p, c, _)| *p == profile_id && *c == catalog)
        .map(|(_, _, json)| *json)
}

/// A collection of device profiles, looked up by Model ID.
///
/// Built-in profiles are registered at startup (task #6 embeds the V31 profile);
/// downloadable "profile packs" for future modules can be `register`ed at runtime.
#[derive(Debug, Default)]
pub struct ProfileRegistry {
    profiles: Vec<DeviceProfile>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry preloaded with all built-in profiles (currently the V31).
    ///
    /// Panics only if a compiled-in profile is malformed — a build-time bug caught
    /// by tests, not a runtime condition.
    pub fn with_builtin() -> Self {
        let mut reg = Self::new();
        reg.register(
            DeviceProfile::from_json(ROLAND_V31_JSON).expect("built-in V31 profile must be valid"),
        );
        reg
    }

    /// Add a profile (built-in or downloaded).
    pub fn register(&mut self, profile: DeviceProfile) {
        self.profiles.push(profile);
    }

    /// Find the profile whose **Model ID** matches (used for DT1/RQ1 framing).
    pub fn match_model(&self, model_id: &[u8]) -> Option<&DeviceProfile> {
        self.profiles.iter().find(|p| p.model_id == model_id)
    }

    /// Find the profile that matches a module's **Identity Reply** fingerprint
    /// (manufacturer + family + member). This is how we auto-detect on connect.
    /// `None` => unknown module (the engine falls back to a degraded mode).
    pub fn match_identity(
        &self,
        manufacturer: u8,
        family: [u8; 2],
        member: [u8; 2],
    ) -> Option<&DeviceProfile> {
        self.profiles.iter().find(|p| {
            p.identity.as_ref().is_some_and(|id| {
                id.manufacturer == manufacturer && id.family == family && id.member == member
            })
        })
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r#"{
        "schema_version": 1,
        "profile_id": "test-mod",
        "display_name": "Test Module",
        "model_id": [1, 6, 1],
        "areas": { "current": { "address": [0, 0, 0, 0] } }
    }"#;

    #[test]
    fn empty_registry_matches_nothing() {
        let reg = ProfileRegistry::new();
        assert!(reg.is_empty());
        assert!(reg.match_model(&[1, 6, 1]).is_none());
    }

    #[test]
    fn every_declared_catalog_is_embedded() {
        // The profile's catalogs map and the embedded table must not drift.
        let reg = ProfileRegistry::with_builtin();
        let v31 = reg.match_model(&[1, 6, 1]).unwrap();
        assert!(!v31.catalogs.is_empty());
        for name in v31.catalogs.keys() {
            assert!(
                builtin_catalog_json(&v31.profile_id, name).is_some(),
                "catalog {name:?} declared in the profile but not embedded"
            );
        }
    }

    #[test]
    fn registers_and_matches_by_model_id() {
        let mut reg = ProfileRegistry::new();
        reg.register(DeviceProfile::from_json(PROFILE).unwrap());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.match_model(&[1, 6, 1]).unwrap().profile_id, "test-mod");
        assert!(reg.match_model(&[9, 9, 9]).is_none()); // unknown module
    }
}
