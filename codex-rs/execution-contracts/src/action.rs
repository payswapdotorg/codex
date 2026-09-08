//! Normalized actions.
//!
//! An [`Action`] is the execution-plane request for one environment to do
//! one thing. Actions are the *currency* between the execution engine and
//! environment adapters: the engine (a later Work Order) derives actions
//! from workflow step semantics; adapters interpret them.
//!
//! The contract is deliberately an envelope, not a taxonomy: `operation`
//! names an environment-scoped operation, `target` is an opaque
//! environment-scoped selector, and `inputs` are bounded structured
//! parameters. Browser-, computer-, terminal-, and MCP-specific action
//! vocabularies belong to the adapters that own them (WO-006 and
//! WO-007); universal workflow semantics never reference action
//! vocabulary, so adding an operation never changes workflow semantics.
//!
//! Actions usually originate from model output and are therefore untrusted
//! input: the contract bounds their size and depth so a malformed action
//! cannot destabilize the runtime, and adapters validate semantics.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::ExecutionContractError;

/// Maximum serialized size of action inputs, in bytes.
pub const ACTION_INPUTS_MAX_BYTES: usize = 65_536;

/// Maximum nesting depth of action input values.
pub const ACTION_INPUTS_MAX_DEPTH: u8 = 16;

/// Maximum length of an action target selector, in bytes.
pub const ACTION_TARGET_MAX_BYTES: usize = 1024;

/// An environment-scoped operation name (for example `navigate`).
///
/// Operation ids use lowercase snake_case, matching semantic capability
/// ids. Their vocabulary is owned by the serving adapter.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OperationId(String);

/// An opaque, environment-scoped action target (for example a selector, a
/// URI, or a window identity).
///
/// Targets are interpreted only by the adapter that serves the action; the
/// contract bounds their size so untrusted targets cannot carry unbounded
/// payloads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionTarget(String);

/// Bounded, structured action parameters.
///
/// Keys are ordered (BTreeMap) so identical inputs always digest
/// identically. Values are untrusted input by default; size and depth are
/// bounded by [`ActionInputs::validate`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionInputs(BTreeMap<String, serde_json::Value>);

/// A normalized request for one environment to perform one operation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    /// The environment-scoped operation to perform.
    pub operation: OperationId,
    /// The opaque target the operation applies to, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ActionTarget>,
    /// Bounded structured parameters for the operation.
    #[serde(default)]
    pub inputs: ActionInputs,
}

impl Action {
    /// Creates an action with an operation, no target, and no inputs.
    pub fn new(operation: OperationId) -> Self {
        Self {
            operation,
            target: None,
            inputs: ActionInputs::default(),
        }
    }

    /// Validates the action's bounds.
    ///
    /// - the operation and target satisfy their identifier contracts;
    /// - the serialized inputs fit within [`ACTION_INPUTS_MAX_BYTES`];
    /// - input values nest no deeper than [`ACTION_INPUTS_MAX_DEPTH`].
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if let Some(target) = &self.target
            && target.0.len() > ACTION_TARGET_MAX_BYTES
        {
            return Err(ExecutionContractError::PayloadTooLarge {
                field: "action target",
                limit: ACTION_TARGET_MAX_BYTES,
            });
        }
        self.inputs.validate()
    }
}

impl ActionInputs {
    /// Inserts one parameter.
    ///
    /// Keys become part of the canonical serialization, so identical
    /// parameter sets always digest identically.
    pub fn insert(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.0.insert(key.into(), value);
    }

    /// Validates size and depth bounds.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        let serialized = serde_json::to_vec(&self.0)?;
        if serialized.len() > ACTION_INPUTS_MAX_BYTES {
            return Err(ExecutionContractError::PayloadTooLarge {
                field: "action inputs",
                limit: ACTION_INPUTS_MAX_BYTES,
            });
        }
        for value in self.0.values() {
            ensure_depth(value, ACTION_INPUTS_MAX_DEPTH, "action inputs")?;
        }
        Ok(())
    }

    /// The number of input parameters.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no input parameters.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl OperationId {
    /// Parses an operation identifier.
    ///
    /// Operation ids use lowercase snake_case, matching semantic
    /// capability ids, so they stay stable across serializations.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
            && !value.starts_with('_')
            && !value.ends_with('_');
        if valid {
            Ok(Self(value))
        } else {
            Err(ExecutionContractError::InvalidIdentifier {
                kind: "operation id",
                value,
                reason: "must be non-empty lowercase snake_case",
            })
        }
    }
}

impl TryFrom<String> for OperationId {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for OperationId {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<OperationId> for String {
    fn from(value: OperationId) -> Self {
        value.0
    }
}

impl AsRef<str> for OperationId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl ActionTarget {
    /// Parses an action target.
    ///
    /// Targets are opaque adapter-scoped selectors; the contract requires
    /// them to be non-empty, free of control characters, and bounded by
    /// [`ACTION_TARGET_MAX_BYTES`].
    pub fn parse(value: impl Into<String>) -> Result<Self, ExecutionContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= ACTION_TARGET_MAX_BYTES
            && !value.chars().any(char::is_control);
        if valid {
            Ok(Self(value))
        } else {
            Err(ExecutionContractError::InvalidIdentifier {
                kind: "action target",
                value,
                reason: "must be non-empty, bounded, without control characters",
            })
        }
    }
}

impl TryFrom<String> for ActionTarget {
    type Error = ExecutionContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ActionTarget {
    type Error = ExecutionContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ActionTarget> for String {
    fn from(value: ActionTarget) -> Self {
        value.0
    }
}

impl AsRef<str> for ActionTarget {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// Rejects values nested deeper than `limit`.
fn ensure_depth(
    value: &serde_json::Value,
    limit: u8,
    field: &'static str,
) -> Result<(), ExecutionContractError> {
    fn walk(value: &serde_json::Value, depth: u8) -> u8 {
        match value {
            serde_json::Value::Array(items) => items
                .iter()
                .map(|item| walk(item, depth + 1))
                .max()
                .unwrap_or(depth),
            serde_json::Value::Object(map) => map
                .values()
                .map(|item| walk(item, depth + 1))
                .max()
                .unwrap_or(depth),
            _ => depth,
        }
    }
    if walk(value, 0) > limit {
        return Err(ExecutionContractError::PayloadTooDeep {
            field,
            limit: ACTION_INPUTS_MAX_DEPTH,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "action_tests.rs"]
mod tests;
