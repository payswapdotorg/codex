//! Content-addressed digests for workflow contract records.
//!
//! Workflow versions, definitions, and dependency locks are integrity-checked
//! with SHA-256 digests over a canonical JSON serialization (object keys
//! sorted lexicographically, matching the canonicalization approach used by
//! `codex-rs/config/src/fingerprint.rs`). Two records with equal digests are
//! byte-equivalent under that canonical form.

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest as _;
use sha2::Sha256;
use std::fmt;

use crate::WorkflowContractError;

/// Digest prefix used for every content digest produced by this crate.
const DIGEST_PREFIX: &str = "sha256:";

/// Length of a SHA-256 digest encoded as lowercase hex.
const SHA256_HEX_LENGTH: usize = 64;

/// A SHA-256 content digest in `sha256:<hex>` form.
///
/// Digests are the integrity backbone of immutable workflow versions: the
/// execution identity of a published version is derived from digests of its
/// definition and dependency lock, so any mutation of the covered content
/// produces a different digest and therefore a different version identity.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentDigest(String);

impl ContentDigest {
    /// Digests the canonical JSON serialization of `value`.
    ///
    /// The canonical form sorts object keys lexicographically so that
    /// struct field ordering and map iteration order cannot change the
    /// digest of semantically identical records.
    pub fn of<T>(value: &T) -> Result<Self, WorkflowContractError>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        let canonical = canonicalize_json(&json);
        let serialized = serde_json::to_vec(&canonical)?;
        let mut hasher = Sha256::new();
        hasher.update(serialized);
        let digest = hasher.finalize();
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(Self(format!("{DIGEST_PREFIX}{hex}")))
    }

    /// Returns the digest string, including the algorithm prefix.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for ContentDigest {
    type Error = WorkflowContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let reason = if !value.starts_with(DIGEST_PREFIX) {
            format!("must start with `{DIGEST_PREFIX}`")
        } else {
            let hex = &value[DIGEST_PREFIX.len()..];
            if hex.len() != SHA256_HEX_LENGTH {
                format!("hex part must be {SHA256_HEX_LENGTH} characters")
            } else if !hex
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
            {
                "hex part must be lowercase hexadecimal".to_string()
            } else {
                return Ok(Self(value));
            }
        };
        Err(WorkflowContractError::InvalidDigest { reason })
    }
}

impl TryFrom<&str> for ContentDigest {
    type Error = WorkflowContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<ContentDigest> for String {
    fn from(value: ContentDigest) -> Self {
        value.0
    }
}

impl AsRef<str> for ContentDigest {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Recursively sorts object keys so serialization order is canonical.
fn canonicalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(entry) = map.get(&key) {
                    sorted.insert(key, canonicalize_json(entry));
                }
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonicalize_json).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
#[path = "digest_tests.rs"]
mod tests;
