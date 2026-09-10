//! Credential scrubbing and checking.
//!
//! The Work Order forbids unbounded retention of sensitive credentials:
//! credentials never enter evidence, candidates, approvals, or lineage
//! records. This module is the enforcement step — a bounded, documented
//! detector of credential-shaped content applied at every boundary where
//! host- or model-derived text could enter the governed-evolution records:
//!
//! - candidate recording (change payloads and rationale);
//! - approval decisions (notes, reasons, approver identities);
//! - upgrade and rollback notes.
//!
//! Detection has two layers:
//!
//! 1. **key names** — JSON object keys that name secret fields
//!    (`password`, `api_key`, `access_token`, ...) with non-empty values;
//! 2. **value shapes** — string values that look like credentials: PEM
//!    blocks, AWS access-key ids, OpenAI-style `sk-` keys, GitHub tokens,
//!    Slack tokens, and `user:password@` URL credentials.
//!
//! Findings record the path and the reason only — never the matched
//! content, so the guard itself cannot leak what it detected.

use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowEvolutionError;

/// One scrubbing finding: where a credential-shaped value was found and
/// why it was flagged. The matched content is deliberately not recorded.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScrubFinding {
    /// The JSON path of the flagged value (for example `$.provenance[0]`).
    pub path: String,
    /// Why the value was flagged.
    pub reason: &'static str,
}

/// The outcome of one scrubbing pass: the redacted copy and every finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrubReport {
    /// The scrubbed copy of the value (flagged values replaced with
    /// `[redacted-credential]`).
    pub clean: serde_json::Value,
    /// Every finding, in traversal order.
    pub findings: Vec<ScrubFinding>,
}

impl ScrubReport {
    /// Whether anything was flagged.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// The marker replacing flagged values during redaction.
pub const REDACTED_MARKER: &str = "[redacted-credential]";

/// The normalized names of secret-bearing JSON keys.
const SECRET_KEYS: [&str; 14] = [
    "password",
    "passwd",
    "secret",
    "apikey",
    "accesstoken",
    "refreshtoken",
    "idtoken",
    "authtoken",
    "token",
    "authorization",
    "credentials",
    "privatekey",
    "clientsecret",
    "sessiontoken",
];

/// Normalizes a key for comparison: lowercase, separators stripped.
fn normalized_key(key: &str) -> String {
    key.chars()
        .filter(|character| *character != '-' && *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether a string value carries a credential shape.
fn credential_shape(value: &str) -> Option<&'static str> {
    if value.len() < 16 {
        return None;
    }
    if value.contains("-----BEGIN ") {
        return Some("PEM private-key block");
    }
    if let Some((_, rest)) = value.split_once("://")
        && let Some((before_at, _)) = rest.split_once('@')
        && before_at.contains(':')
    {
        return Some("URL-embedded credential");
    }
    // Token-shaped credentials can appear anywhere in free text, so the
    // scan is per whitespace-separated word (prefix matching per word
    // avoids flagging ordinary hyphenated words like `task-runner`).
    value.split_whitespace().find_map(token_shape)
}

/// Whether one whitespace-delimited word carries a token-shaped
/// credential prefix.
fn token_shape(token: &str) -> Option<&'static str> {
    let token = token.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric() && character != '_' && character != '-'
    });
    if let Some(rest) = token.strip_prefix("AKIA")
        && rest.len() == 16
        && rest
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    {
        return Some("AWS access key id");
    }
    if let Some(rest) = token.strip_prefix("sk-")
        && rest.chars().filter(char::is_ascii_alphanumeric).count() >= 20
    {
        return Some("OpenAI-style secret key");
    }
    for prefix in ["ghp_", "gho_", "ghu_", "ghs_", "github_pat_"] {
        if let Some(rest) = token.strip_prefix(prefix)
            && rest
                .chars()
                .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
                .count()
                >= 20
        {
            return Some("GitHub token");
        }
    }
    for prefix in ["xoxb-", "xoxa-", "xoxp-", "xoxr-", "xoxs-"] {
        if let Some(rest) = token.strip_prefix(prefix)
            && rest.len() >= 10
        {
            return Some("Slack token");
        }
    }
    None
}

/// Whether a JSON object key names a secret field.
fn secret_key(key: &str) -> bool {
    SECRET_KEYS.contains(&normalized_key(key).as_str())
}

/// Scrubs a JSON value: returns the redacted copy plus every finding.
///
/// Redaction replaces flagged values with [`REDACTED_MARKER`]; nothing is
/// dropped silently — every replacement is a recorded finding.
pub fn scrub_json(value: &serde_json::Value) -> ScrubReport {
    let mut findings = Vec::new();
    let clean = scrub_value(value, "$", &mut findings);
    ScrubReport { clean, findings }
}

/// Scrubs one value at `path`, recording findings.
fn scrub_value(
    value: &serde_json::Value,
    path: &str,
    findings: &mut Vec<ScrubFinding>,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut scrubbed = serde_json::Map::new();
            for (key, inner) in map {
                let child_path = format!("{path}.{key}");
                let empty = matches!(inner, serde_json::Value::String(text) if text.is_empty());
                if secret_key(key) && !empty && !matches!(inner, serde_json::Value::Null) {
                    findings.push(ScrubFinding {
                        path: child_path,
                        reason: "secret-bearing key",
                    });
                    scrubbed.insert(
                        key.clone(),
                        serde_json::Value::String(REDACTED_MARKER.to_string()),
                    );
                } else {
                    scrubbed.insert(key.clone(), scrub_value(inner, &child_path, findings));
                }
            }
            serde_json::Value::Object(scrubbed)
        }
        serde_json::Value::Array(items) => {
            let scrubbed = items
                .iter()
                .enumerate()
                .map(|(index, item)| scrub_value(item, &format!("{path}[{index}]"), findings))
                .collect();
            serde_json::Value::Array(scrubbed)
        }
        serde_json::Value::String(text) => match credential_shape(text) {
            Some(reason) => {
                findings.push(ScrubFinding {
                    path: path.to_string(),
                    reason,
                });
                serde_json::Value::String(REDACTED_MARKER.to_string())
            }
            None => value.clone(),
        },
        _ => value.clone(),
    }
}

/// The checking gate over a JSON value: refuses (never redacts) when any
/// credential-shaped content is present.
///
/// Candidates and other records that would retain credentials are rejected
/// outright — redaction would mean the credential already entered the
/// record, which the invariant forbids.
pub fn require_clean_json(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<(), WorkflowEvolutionError> {
    let report = scrub_json(value);
    if report.is_clean() {
        Ok(())
    } else {
        Err(WorkflowEvolutionError::CredentialContamination {
            field,
            findings: report.findings.len(),
        })
    }
}

/// The checking gate over a plain free-text field.
pub(crate) fn require_clean_text(
    text: &str,
    field: &'static str,
) -> Result<(), WorkflowEvolutionError> {
    match credential_shape(text) {
        None => Ok(()),
        Some(_) => Err(WorkflowEvolutionError::CredentialContamination { field, findings: 1 }),
    }
}

#[cfg(test)]
#[path = "scrub_tests.rs"]
mod tests;
