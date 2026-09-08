//! Canonical serialization and content digests for adapter artifacts.
//!
//! Every artifact this adapter records (lifecycle transitions, bindings,
//! observations, actions, results, recovery notes) is serialized through
//! one deterministic canonical form before digesting, so the same logical
//! payload always yields the same SHA-256 digest regardless of map
//! insertion order or platform differences.
//!
//! The digest is computed over the canonical UTF-8 bytes. The evidence
//! plane allocates its own `ContentDigest` objects from exactly the same
//! bytes (see [`crate::bridge::EvidenceRecord::digest_input_bytes`]); this
//! crate never constructs evidence-plane digest types itself.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A canonical, ordering-stable value used for digesting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Canonical {
    /// JSON `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A 64-bit signed integer; out-of-range JSON numbers degrade to
    /// their deterministic decimal string form.
    Number(i64),
    /// A UTF-8 string.
    String(String),
    /// An ordered sequence.
    Sequence(Vec<Canonical>),
    /// A key-sorted mapping.
    Mapping(BTreeMap<String, Canonical>),
}

impl From<&Value> for Canonical {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => Canonical::Null,
            Value::Bool(flag) => Canonical::Bool(*flag),
            Value::Number(number) => number
                .as_i64()
                .map(Canonical::Number)
                .unwrap_or_else(|| Canonical::String(number.to_string())),
            Value::String(text) => Canonical::String(text.clone()),
            Value::Array(items) => Canonical::Sequence(items.iter().map(Canonical::from).collect()),
            Value::Object(map) => {
                let mut out = BTreeMap::new();
                for (key, item) in map {
                    out.insert(key.clone(), Canonical::from(item));
                }
                Canonical::Mapping(out)
            }
        }
    }
}

impl Canonical {
    /// Renders the canonical form: mapping keys are sorted, sequence order
    /// is preserved, and strings use deterministic JSON escaping.
    pub fn to_canonical_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Canonical::Null => out.push_str("null"),
            Canonical::Bool(true) => out.push_str("true"),
            Canonical::Bool(false) => out.push_str("false"),
            Canonical::Number(number) => out.push_str(&number.to_string()),
            Canonical::String(text) => write_json_string(text, out),
            Canonical::Sequence(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Canonical::Mapping(map) => {
                out.push('{');
                for (index, (key, value)) in map.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_json_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Writes `value` as a deterministic JSON string literal.
fn write_json_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Canonicalizes `payload` and digests the canonical bytes.
///
/// Returns `(canonical text, sha256 hex)`. Serialization cannot fail for
/// the plain derived payloads used by this crate; if it ever did, the
/// canonical form falls back to `null` deterministically.
pub fn canonical_digest<T: Serialize + ?Sized>(payload: &T) -> (String, String) {
    let value = serde_json::to_value(payload).unwrap_or(Value::Null);
    let canonical = Canonical::from(&value);
    let text = canonical.to_canonical_string();
    let hex = sha256_hex(text.as_bytes());
    (text, hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn canonical_form_sorts_mapping_keys() {
        let value: Value =
            serde_json::from_str(r#"{"b":1,"a":{"d":true,"c":"x"}}"#).expect("valid json");
        let canonical = Canonical::from(&value);
        assert_eq!(
            canonical.to_canonical_string(),
            r#"{"a":{"c":"x","d":true},"b":1}"#
        );
    }

    #[test]
    fn canonical_digest_is_insertion_order_independent() {
        let first = json!({"alpha": 1, "beta": [true, "s"], "gamma": null});
        let second = json!({"gamma": null, "beta": [true, "s"], "alpha": 1});
        assert_eq!(canonical_digest(&first).1, canonical_digest(&second).1);
    }

    #[test]
    fn json_string_escaping_is_deterministic() {
        let value = json!({"quote": "\"\\\n\t\u{1}"});
        let canonical = Canonical::from(&value).to_canonical_string();
        assert_eq!(canonical, "{\"quote\":\"\\\"\\\\\\n\\t\\u0001\"}");
    }
}
