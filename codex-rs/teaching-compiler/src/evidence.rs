//! Evidence references captured during teaching.
//!
//! Teaching input is untrusted external data. The compiler never stores
//! raw transcripts as workflow semantics: hosts attach digest-bearing
//! references to externally stored evidence (for example Codex rollout or
//! trace records) and the compiler carries those references through to
//! the candidate. Evidence payloads live outside this crate and are never
//! fetched, parsed, or trusted here.

use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;

/// Maximum byte length of an evidence label.
pub const MAX_LABEL_BYTES: usize = 128;
/// Maximum byte length of an evidence locator.
pub const MAX_LOCATOR_BYTES: usize = 512;
/// Length of a lowercase hex-encoded SHA-256 digest.
const SHA256_HEX_LEN: usize = 64;

/// A digest-bearing pointer to externally stored teaching evidence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingEvidence {
    label: String,
    locator: String,
    sha256: String,
}

impl TeachingEvidence {
    /// Validates and constructs an evidence reference.
    ///
    /// The digest must be 64 lowercase hexadecimal characters; the label
    /// and locator must be non-empty and within their byte bounds.
    pub fn new(
        label: impl Into<String>,
        locator: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Result<Self, TeachingCompilerError> {
        let label = label.into();
        let locator = locator.into();
        let sha256 = sha256.into();
        if label.is_empty() || label.len() > MAX_LABEL_BYTES {
            return Err(TeachingCompilerError::InvalidEvidence {
                reason: format!("label must be 1..={MAX_LABEL_BYTES} bytes"),
            });
        }
        if locator.is_empty() || locator.len() > MAX_LOCATOR_BYTES {
            return Err(TeachingCompilerError::InvalidEvidence {
                reason: format!("locator must be 1..={MAX_LOCATOR_BYTES} bytes"),
            });
        }
        if !is_lowercase_sha256_hex(&sha256) {
            return Err(TeachingCompilerError::InvalidEvidence {
                reason: "sha256 must be 64 lowercase hexadecimal characters".to_string(),
            });
        }
        Ok(Self {
            label,
            locator,
            sha256,
        })
    }

    /// A short human-facing label for the evidence.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Where the host stored the evidence payload (for example a rollout
    /// path or trace id). Never dereferenced by this crate.
    pub fn locator(&self) -> &str {
        &self.locator
    }

    /// The SHA-256 digest of the evidence payload, hex-encoded.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Whether `value` is exactly 64 lowercase hex characters.
fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::TeachingEvidence;

    #[test]
    fn accepts_valid_evidence() {
        let evidence =
            TeachingEvidence::new("open-list", "rollout://abc", "ab".repeat(32)).expect("evidence");
        assert_eq!(evidence.label(), "open-list");
        assert_eq!(evidence.locator(), "rollout://abc");
        assert_eq!(evidence.sha256().len(), 64);
    }

    #[test]
    fn rejects_bad_labels_and_locators() {
        assert!(TeachingEvidence::new("", "rollout://abc", "ab".repeat(32)).is_err());
        assert!(TeachingEvidence::new(&"x".repeat(129), "rollout://abc", "ab".repeat(32)).is_err());
        assert!(TeachingEvidence::new("label", "", "ab".repeat(32)).is_err());
    }

    #[test]
    fn rejects_bad_digests() {
        assert!(TeachingEvidence::new("label", "rollout://abc", "ab".repeat(31)).is_err());
        assert!(TeachingEvidence::new("label", "rollout://abc", "AB".repeat(32)).is_err());
        assert!(TeachingEvidence::new("label", "rollout://abc", "gg".repeat(32)).is_err());
    }

    #[test]
    fn round_trips_through_json() {
        let evidence =
            TeachingEvidence::new("label", "trace://xyz", "cd".repeat(32)).expect("evidence");
        let encoded = serde_json::to_string(&evidence).expect("serialize");
        let decoded: TeachingEvidence = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(evidence, decoded);
    }
}
