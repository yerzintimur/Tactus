//! Firmware version handling and the "tested vs untested" compatibility policy.
//!
//! The version is the 4-byte software-revision field from the SysEx Identity Reply
//! (see `sysex::SysexMessage::IdentityReply`). Policy: detect, announce when
//! untested, **never block** — see ADR-0009.

use serde::Deserialize;

/// A module's firmware version — the 4 version bytes from the Identity Reply.
///
/// Ordered lexicographically (newer firmware is assumed to compare greater).
/// Deserializes from a JSON array `[a, b, c, d]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct FirmwareVersion(pub [u8; 4]);

impl FirmwareVersion {
    pub fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Best-effort human string. The exact byte→version mapping is module-specific
    /// and **verified on hardware** (profile `version_format`); until then we show
    /// the raw dotted bytes.
    pub fn display(&self) -> String {
        let [a, b, c, d] = self.0;
        format!("{a}.{b}.{c}.{d}")
    }
}

/// How the connected firmware relates to what a profile was tested against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareSupport {
    /// Exactly a version we've verified.
    Tested,
    /// Newer than the newest tested version (e.g. a new firmware shipped).
    UntestedNewer,
    /// Older than the oldest tested version.
    UntestedOlder,
    /// No tested versions recorded, or it falls in a gap between them.
    Unknown,
}

impl FirmwareSupport {
    /// Classify `version` against a profile's `tested` list. Never errors — an
    /// untested result is informational only (ADR-0009).
    pub fn classify(tested: &[FirmwareVersion], version: FirmwareVersion) -> Self {
        if tested.is_empty() {
            return Self::Unknown;
        }
        if tested.contains(&version) {
            return Self::Tested;
        }
        // Safe: list is non-empty.
        let max = tested.iter().max().copied().unwrap();
        let min = tested.iter().min().copied().unwrap();
        if version > max {
            Self::UntestedNewer
        } else if version < min {
            Self::UntestedOlder
        } else {
            Self::Unknown
        }
    }

    /// Whether this is the fully-tested case. (All other cases still work — we just
    /// announce them; nothing is ever blocked.)
    pub fn is_tested(self) -> bool {
        matches!(self, Self::Tested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(b: [u8; 4]) -> FirmwareVersion {
        FirmwareVersion::new(b)
    }

    #[test]
    fn ordering_is_lexicographic() {
        assert!(v([1, 0, 0, 0]) < v([1, 1, 0, 0]));
        assert!(v([1, 0, 9, 0]) < v([1, 1, 0, 0]));
        assert_eq!(v([1, 0, 5, 0]).display(), "1.0.5.0");
    }

    #[test]
    fn empty_tested_is_unknown() {
        assert_eq!(
            FirmwareSupport::classify(&[], v([1, 0, 0, 0])),
            FirmwareSupport::Unknown
        );
    }

    #[test]
    fn classify_against_tested() {
        let tested = [v([1, 0, 0, 0]), v([1, 0, 5, 0])];
        assert_eq!(
            FirmwareSupport::classify(&tested, v([1, 0, 0, 0])),
            FirmwareSupport::Tested
        );
        assert_eq!(
            FirmwareSupport::classify(&tested, v([1, 0, 5, 0])),
            FirmwareSupport::Tested
        );
        assert_eq!(
            FirmwareSupport::classify(&tested, v([1, 1, 0, 0])),
            FirmwareSupport::UntestedNewer
        );
        assert_eq!(
            FirmwareSupport::classify(&tested, v([0, 9, 0, 0])),
            FirmwareSupport::UntestedOlder
        );
        // Between two tested versions but not equal to either => Unknown gap.
        assert_eq!(
            FirmwareSupport::classify(&tested, v([1, 0, 2, 0])),
            FirmwareSupport::Unknown
        );
    }

    #[test]
    fn is_tested_helper() {
        assert!(FirmwareSupport::Tested.is_tested());
        assert!(!FirmwareSupport::UntestedNewer.is_tested());
    }
}
