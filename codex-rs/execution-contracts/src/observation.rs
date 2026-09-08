//! Normalized observations.
//!
//! An [`Observation`] is a normalized record of what an execution
//! environment observed: a page or accessibility snapshot for browser
//! environments, screen or window state for computer environments,
//! output for terminal environments, tool results for tool/MCP
//! environments. Adapters normalize their provider-specific observations
//! into this envelope; provider- and environment-specific observation
//! formats never enter workflow semantics.
//!
//! Observation payloads are **untrusted input by default** (web pages,
//! desktop applications, tool outputs, and model outputs can all be
//! hostile). The contract bounds payload size and depth so a hostile
//! environment cannot destabilize the runtime.
//!
//! Observations are evidence-bearing: they convert to
//! [`EvidenceReference`] values of kind [`EvidenceKind::Observation`] for
//! the evidence plane to store.

use serde::Deserialize;
use serde::Serialize;

use crate::AdapterId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;

/// Maximum serialized size of an observation payload, in bytes.
pub const OBSERVATION_MAX_BYTES: usize = 262_144;

/// Maximum nesting depth of an observation payload.
pub const OBSERVATION_MAX_DEPTH: u8 = 64;

/// A normalized observation of one execution environment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    /// The environment class that produced the observation.
    pub environment: ExecutionEnvironment,
    /// The adapter that produced the observation.
    pub adapter: AdapterId,
    /// The normalized observation payload: untrusted, bounded.
    pub payload: serde_json::Value,
}

impl Observation {
    /// Creates an observation from `environment`/`adapter` with `payload`.
    pub fn new(
        environment: ExecutionEnvironment,
        adapter: AdapterId,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            environment,
            adapter,
            payload,
        }
    }

    /// Validates the observation's bounds.
    ///
    /// - the environment must not be reserved;
    /// - the serialized payload fits within [`OBSERVATION_MAX_BYTES`];
    /// - the payload nests no deeper than [`OBSERVATION_MAX_DEPTH`].
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.environment.is_reserved() {
            return Err(ExecutionContractError::ReservedEnvironment {
                environment: self.environment,
                reason: "reserved environments cannot produce observations yet",
            });
        }
        let serialized = serde_json::to_vec(&self.payload)?;
        if serialized.len() > OBSERVATION_MAX_BYTES {
            return Err(ExecutionContractError::PayloadTooLarge {
                field: "observation payload",
                limit: OBSERVATION_MAX_BYTES,
            });
        }
        if depth_of(&self.payload) > OBSERVATION_MAX_DEPTH {
            return Err(ExecutionContractError::PayloadTooDeep {
                field: "observation payload",
                limit: OBSERVATION_MAX_DEPTH,
            });
        }
        Ok(())
    }

    /// Converts the observation into an evidence reference for the
    /// evidence plane.
    ///
    /// `locator` is the opaque evidence-plane address allocated when the
    /// payload is stored. The digest covers the canonical serialization of
    /// the observation, so evidence consumers can verify the referenced
    /// payload has not drifted.
    pub fn to_evidence_reference(
        &self,
        locator: impl Into<String>,
    ) -> Result<EvidenceReference, ExecutionContractError> {
        let digest = ContentDigest::of(self)?;
        Ok(EvidenceReference {
            kind: EvidenceKind::Observation,
            locator: validate_locator(locator)?,
            digest,
        })
    }
}

/// Measures the nesting depth of a JSON value.
fn depth_of(value: &serde_json::Value) -> u8 {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .map(depth_of)
            .max()
            .map_or(0, |depth| depth + 1),
        serde_json::Value::Object(map) => map
            .values()
            .map(depth_of)
            .max()
            .map_or(0, |depth| depth + 1),
        _ => 0,
    }
}

/// Validates an evidence locator supplied by the evidence plane.
pub(crate) fn validate_locator(
    locator: impl Into<String>,
) -> Result<String, ExecutionContractError> {
    let locator = locator.into();
    let valid =
        !locator.is_empty() && locator.len() <= 512 && !locator.chars().any(char::is_control);
    if valid {
        Ok(locator)
    } else {
        Err(ExecutionContractError::InvalidIdentifier {
            kind: "evidence locator",
            value: locator,
            reason: "must be non-empty, bounded, without control characters",
        })
    }
}

#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
