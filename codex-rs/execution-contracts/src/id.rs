//! Validated identifiers for the execution contract surface.
//!
//! Identifiers follow the WO-003 convention of validated string newtypes:
//! they are cheap to clone, stable across serializations, and reject
//! malformed values at construction instead of during execution.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::ExecutionContractError;
use codex_workflow_contracts::CapabilityId;

/// Identity of one registered environment adapter (for example
/// `codex-browser-use`).
///
/// Adapter ids use path-segment-safe tokens (the same rules as Codex skill
/// and plugin names) so they can appear in logs, evidence locators, and
/// debug surfaces without escaping.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AdapterId(String);

/// Identity of one capability binding: the pairing of an adapter with a
/// semantic capability.
///
/// Binding ids are derived deterministically as `<adapter>:<capability>`,
/// so registering the same adapter providing the same capability always
/// addresses the same binding identity, across processes and restarts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CapabilityBindingId(String);

/// Opaque handle of one concrete resource instance (for example
/// `profile-default`, `github-acme`).
///
/// Resource ids never carry credentials: they name an instance whose
/// secret material stays inside the adapter that owns the resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceId(String);

/// Opaque session handle allocated by an environment adapter when a
/// binding is prepared for execution.
///
/// Session handles identify adapter-internal execution contexts (for
/// example a browser session or MCP connection). They are opaque to the
/// contract: only the allocating adapter interprets them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SessionHandle(String);

impl AdapterId {
    /// Parses an adapter identifier.
    ///
    /// Follows the Codex plugin/skill naming rules: non-empty ASCII
    /// letters, digits, `_`, `-`, and `.` separating non-empty segments.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        validate_name_segment(&value, "adapter id")?;
        Ok(Self(value))
    }
}

impl TryFrom<String> for AdapterId {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for AdapterId {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<AdapterId> for String {
    fn from(value: AdapterId) -> Self {
        value.0
    }
}

impl AsRef<str> for AdapterId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl CapabilityBindingId {
    /// Derives the binding identity for `adapter` providing `capability`.
    ///
    /// The derivation is deterministic, so identical adapter/capability
    /// pairs always produce identical binding identities.
    pub fn derive(adapter: &AdapterId, capability: &CapabilityId) -> Self {
        Self(format!("{adapter}:{}", capability.as_ref()))
    }

    /// Parses a binding identifier in `<adapter>:<capability>` form.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        let (adapter, capability) =
            value
                .split_once(':')
                .ok_or_else(|| ExecutionContractError::InvalidIdentifier {
                    kind: "capability binding id",
                    value: value.clone(),
                    reason: "must be `<adapter>:<capability>`",
                })?;
        AdapterId::parse(adapter.to_owned())?;
        CapabilityId::parse(capability.to_owned())?;
        Ok(Self(value))
    }
}

impl TryFrom<String> for CapabilityBindingId {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CapabilityBindingId {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<CapabilityBindingId> for String {
    fn from(value: CapabilityBindingId) -> Self {
        value.0
    }
}

impl AsRef<str> for CapabilityBindingId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for CapabilityBindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl ResourceId {
    /// Parses a resource instance identifier.
    ///
    /// Resource ids use path-segment-safe tokens, matching adapter ids.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        validate_name_segment(&value, "resource id")?;
        Ok(Self(value))
    }
}

impl TryFrom<String> for ResourceId {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ResourceId {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ResourceId> for String {
    fn from(value: ResourceId) -> Self {
        value.0
    }
}

impl AsRef<str> for ResourceId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl SessionHandle {
    /// Parses a session handle.
    ///
    /// Session handles are adapter-allocated opaque tokens; the contract
    /// only requires them to be non-empty, free of control characters, and
    /// reasonably bounded.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        let valid = !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control);
        if valid {
            Ok(Self(value))
        } else {
            Err(ExecutionContractError::InvalidIdentifier {
                kind: "session handle",
                value,
                reason: "must be non-empty, at most 256 characters, without control characters",
            })
        }
    }
}

impl TryFrom<String> for SessionHandle {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for SessionHandle {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<SessionHandle> for String {
    fn from(value: SessionHandle) -> Self {
        value.0
    }
}

impl AsRef<str> for SessionHandle {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for SessionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Validates path-segment-safe name tokens shared by adapter and resource
/// ids, mirroring the WO-003 skill/plugin name rules.
fn validate_name_segment(value: &str, kind: &'static str) -> Result<(), ExecutionContractError> {
    let characters_ok = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        });
    let dots_ok = !value.starts_with('.') && !value.ends_with('.') && !value.contains("..");
    if characters_ok && dots_ok {
        Ok(())
    } else {
        Err(ExecutionContractError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "must be a non-empty path-segment-safe token",
        })
    }
}

#[cfg(test)]
#[path = "id_tests.rs"]
mod tests;
