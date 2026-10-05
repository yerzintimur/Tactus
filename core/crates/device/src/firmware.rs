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

    /// The raw bytes, dotted — what to show when no profile says how the module
    /// itself renders them (an unrecognised device, or a profile without a
    /// `version_format`).
    pub fn display(&self) -> String {
        let [a, b, c, d] = self.0;
        format!("{a}.{b}.{c}.{d}")
    }

    /// The version as the module's own screen shows it, from a profile's
    /// `version_format`: a template where `{0}`–`{3}` stand for the four bytes
    /// and everything else is copied through. The V31 renders `00 02 01 00` as
    /// "0.2.10" — the last two bytes make one component — so its profile says
    /// `"{0}.{1}.{2}{3}"`. `"raw4"` is the dotted default by name. A template
    /// naming no byte at all is a mistake, not a format: the dotted bytes are
    /// shown instead, so the version is never silently blank.
    pub fn display_as(&self, format: &str) -> String {
        if format == "raw4" {
            return self.display();
        }
        let mut out = String::new();
        let mut named = false;
        let mut rest = format;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    match after[..end]
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| self.0.get(i))
                    {
                        Some(byte) => {
                            out.push_str(&byte.to_string());
                            named = true;
                        }
                        None => out.push_str(&rest[start..start + end + 2]),
                    }
                    rest = &after[end + 1..];
                }
                None => {
                    out.push_str(rest);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        if named { out } else { self.display() }
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

    /// The V31's own screen says "0.2.10" for `00 02 01 00` (verified on the
    /// module 2026-06-15, PROTOCOL §6): the profile's template reproduces it.
    #[test]
    fn display_follows_the_profiles_template() {
        let fw = v([0, 2, 1, 0]);
        assert_eq!(fw.display_as("{0}.{1}.{2}{3}"), "0.2.10");
        assert_eq!(fw.display_as("raw4"), "0.2.1.0");
        assert_eq!(fw.display_as("{0}.{1}.{2}.{3}"), "0.2.1.0");
        // Literal text comes through; a byte past the four stays as written.
        assert_eq!(fw.display_as("v{1}.{2} ({4})"), "v2.1 ({4})");
        // A template that names no byte shows the bytes, never nothing.
        assert_eq!(fw.display_as(""), "0.2.1.0");
        assert_eq!(fw.display_as("{x}"), "0.2.1.0");
        assert_eq!(fw.display_as("{1"), "0.2.1.0");
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
