//! Identity-bearing references from a pack system state to external
//! contract surfaces.
//!
//! Every reference type pins an immutable, content-addressed identity:
//!
//! - [`WorkflowVersionRef`] pins an exact
//!   [`codex_workflow_contracts::WorkflowVersionId`]. The identity is the
//!   pinned version id; an optional role is descriptive only.
//! - [`CapabilityRef`] reuses the Universal
//!   [`codex_workflow_contracts::CapabilityId`] semantic identity and adds an
//!   optional descriptive constraint.
//! - [`PolicyRef`] pins a [`crate::PackPolicyId`] plus the content digest the
//!   pack state was validated against.
//! - [`EvaluationRef`] is an opaque content-addressed evaluation identity.
//!
//! References never reinterpret the things they point at: a workflow version
//! reference pins a published version but never models workflow semantics,
//! transitions, or state.

use std::fmt;

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use super::validate_descriptive_text;
use crate::PackContractError;
use crate::PackPolicyId;

/// A pinned reference to an immutable published workflow version.
///
/// The pinned [`WorkflowVersionId`] is the identity of the reference; the
/// optional role describes why the pack state includes the workflow (for
/// example `"user onboarding"`). Updating the pin — or anything else in the
/// enclosing state — produces a new system-state digest and therefore a new
/// candidate revision; a pack revision can never silently alter a referenced
/// workflow version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowVersionRef {
    /// The pinned immutable workflow version identity.
    pub workflow_version_id: WorkflowVersionId,
    /// Descriptive role of the pinned workflow within the pack system state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

impl WorkflowVersionRef {
    /// Pins an exact immutable workflow version with no descriptive role.
    pub fn pin(workflow_version_id: WorkflowVersionId) -> Self {
        Self {
            workflow_version_id,
            role: None,
        }
    }

    /// Pins an exact immutable workflow version with a descriptive role.
    ///
    /// The role is validated descriptive text; it never participates in the
    /// pinned identity.
    pub fn pin_with_role(
        workflow_version_id: WorkflowVersionId,
        role: impl Into<String>,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            workflow_version_id,
            role: Some(validate_descriptive_text(
                "workflow version role",
                role.into(),
            )?),
        })
    }
}

/// A reference to a semantic capability the system state requires.
///
/// The capability identity is the Universal
/// [`codex_workflow_contracts::CapabilityId`] — capabilities are stable
/// semantic contracts, and this module never redefines them. The optional
/// constraint describes how the capability must be satisfied (for example
/// `"headless browser session required"`); it is descriptive text, not a
/// binding or an execution instruction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityRef {
    /// The required semantic capability, by Universal capability identity.
    pub capability: CapabilityId,
    /// Descriptive constraint on how the capability must be satisfied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<String>,
}

impl CapabilityRef {
    /// References a required capability with no constraint.
    pub fn requiring(capability: CapabilityId) -> Self {
        Self {
            capability,
            constraint: None,
        }
    }

    /// References a required capability with a descriptive constraint.
    pub fn constrained(
        capability: CapabilityId,
        constraint: impl Into<String>,
    ) -> Result<Self, PackContractError> {
        Ok(Self {
            capability,
            constraint: Some(validate_descriptive_text(
                "capability constraint",
                constraint.into(),
            )?),
        })
    }
}

/// A reference to a governing or assurance policy pinned by the system state.
///
/// The [`PackPolicyId`] addresses the policy record; the
/// `validated_content_digest` pins the exact policy content the pack state
/// was validated against. The two digests are deliberately separate: policy
/// record identity is owned by the policy contracts (PACK-001/PACK-003), and
/// this reference records what the pack state was actually validated against
/// rather than re-deriving it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyRef {
    /// The content-addressed identity of the referenced policy record.
    pub policy_id: PackPolicyId,
    /// Digest of the exact policy content the pack state was validated
    /// against.
    pub validated_content_digest: ContentDigest,
}

impl PolicyRef {
    /// References a policy by identity, pinning the content digest the pack
    /// state was validated against.
    pub fn validated_against(
        policy_id: PackPolicyId,
        validated_content_digest: ContentDigest,
    ) -> Self {
        Self {
            policy_id,
            validated_content_digest,
        }
    }
}

/// An opaque content-addressed reference to an evaluation record.
///
/// The system state does not interpret evaluation content; it pins the
/// digest so a revision can be tied to the exact evaluation it was assessed
/// by. Evaluation semantics and evidence authority live elsewhere.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EvaluationRef(ContentDigest);

impl EvaluationRef {
    /// Wraps a content digest as an evaluation reference.
    pub const fn from_digest(digest: ContentDigest) -> Self {
        Self(digest)
    }

    /// The underlying content digest.
    pub fn digest(&self) -> &ContentDigest {
        &self.0
    }
}

impl TryFrom<String> for EvaluationRef {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ContentDigest::try_from(value)
            .map(Self)
            .map_err(|error| PackContractError::InvalidDigest {
                reason: error.to_string(),
            })
    }
}

impl TryFrom<&str> for EvaluationRef {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<EvaluationRef> for String {
    fn from(value: EvaluationRef) -> Self {
        value.0.into()
    }
}

impl AsRef<str> for EvaluationRef {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for EvaluationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}
