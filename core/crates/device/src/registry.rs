//! Holds the known device profiles and resolves one from a connected module's
//! Model ID (auto-detection). See ADR-0007 and docs/DEVELOPMENT.md §3.2.

use crate::profile::DeviceProfile;

/// Built-in profile JSON, embedded at compile time from `profiles/`.
const ROLAND_V31_JSON: &str = include_str!("../../../../profiles/roland-v31.json");

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

    /// Find the profile whose Model ID matches the connected module's Identity
    /// Reply. `None` => unknown module (the engine falls back to a degraded mode).
    pub fn match_model(&self, model_id: &[u8]) -> Option<&DeviceProfile> {
        self.profiles.iter().find(|p| p.model_id == model_id)
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
    fn registers_and_matches_by_model_id() {
        let mut reg = ProfileRegistry::new();
        reg.register(DeviceProfile::from_json(PROFILE).unwrap());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.match_model(&[1, 6, 1]).unwrap().profile_id, "test-mod");
        assert!(reg.match_model(&[9, 9, 9]).is_none()); // unknown module
    }
}
