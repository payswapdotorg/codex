//! Shared identity foundations for Pack contracts.
//!
//! These types are the cross-module identity backbone of the Pack contract
//! surface:
//!
//! - [`PackId`] names a stable pack lineage. It is a validated slug, not a
//!   content address: the same lineage produces many revisions over time.
//! - [`PackRevisionId`] is the immutable, content-addressed identity of one
//!   pack revision. Revisions are addressed by the digest of their identity
//!   tuple; any change to covered content produces a different revision id.
//! - [`PackPolicyId`] is the content-addressed identity of a governed policy
//!   record. It deliberately covers every kind of policy artifact inside a
//!   pack (governing policy, constitution-derived policy, assurance policy)
//!   so policy references stay unambiguous across modules.
//! - [`PackProvenance`] records lineage and attribution for a revision:
//!   which parent revision it builds on and which principal produced it.
//!   Provenance is metadata; it never grants authority.
//!
//! Revision identity tuples, semantic-version compatibility contracts, and
//! the full pack dependency model are owned by the PACK-001 work order and
//! build on top of these foundations.

use std::fmt;

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;

use crate::PackContractError;

/// Maximum length of a [`PackId`] slug.
const PACK_ID_MAX_LENGTH: usize = 128;

/// A stable identity for a pack lineage.
///
/// A `PackId` names the lineage, not the content: revisions of the same pack
/// share one `PackId` while carrying distinct [`PackRevisionId`] identities.
/// The slug charset (lowercase alphanumeric plus hyphen) keeps the identity
/// usable in URLs, file systems, and display surfaces without escaping.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackId(String);

/// The immutable, content-addressed identity of a single pack revision.
///
/// The digest is produced by the PACK-001 revision identity tuple (mission
/// revision, policy revision, system-state digest, dependency lock, parent
/// revision), mirroring how
/// [`codex_workflow_contracts::WorkflowVersionId`] addresses published
/// workflow versions. Equal content yields equal identities; any covered
/// mutation yields a different identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackRevisionId(ContentDigest);

/// The content-addressed identity of a governed policy record inside a pack.
///
/// Every policy artifact that participates in pack governance — the general
/// [`crate::PackPolicy`] records, constitution-derived policy, and the
/// assurance policies owned by the `assurance` module — digests its content
/// into a `PackPolicyId`. A shared identity type keeps policy references
/// unambiguous and prevents duplicate identity machinery.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackPolicyId(ContentDigest);

/// Lineage and attribution for a pack revision.
///
/// Provenance is descriptive metadata: it records which parent revision a
/// candidate builds on and which principal produced it. It never grants
/// authority — promotion and rollback remain control-plane decisions, and an
/// LLM-produced candidate carries the same provenance shape as a
/// user-produced one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackProvenance {
    /// The revision this revision builds on, or `None` for a root revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision: Option<PackRevisionId>,
    /// The principal that produced this revision.
    pub producer: ProvenanceProducer,
}

/// The principal that produced a pack revision.
///
/// Producers are descriptive. None of these variants carry authority: a
/// `ControlPlane` producer records that the control plane performed the
/// transition, it does not re-derive the control plane's right to do so.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ProvenanceProducer {
    /// A human user or organization principal.
    User {
        /// Stable identifier of the user or organization.
        subject: String,
    },
    /// An LLM agent or worker that proposed the revision.
    Agent {
        /// Description of the proposing agent or worker.
        agent: String,
    },
    /// A control plane performing a governed transition.
    ControlPlane {
        /// Name of the control plane that performed the transition.
        plane: String,
    },
}

impl PackId {
    /// Parses a pack lineage identity.
    ///
    /// Valid slugs are 1–128 characters of lowercase ASCII alphanumeric or
    /// hyphen, must not start or end with a hyphen, and must not contain
    /// whitespace or other characters that would require escaping in URLs or
    /// file names.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let valid_length = !value.is_empty() && value.len() <= PACK_ID_MAX_LENGTH;
        let valid_charset = value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        let valid_edges = !value.starts_with('-') && !value.ends_with('-');
        if valid_length && valid_charset && valid_edges {
            Ok(Self(value))
        } else {
            Err(PackContractError::InvalidIdentifier {
                kind: "pack id",
                value,
                reason: "must be 1-128 chars of [a-z0-9-] and must not start or end with '-'",
            })
        }
    }

    /// The lineage slug.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for PackId {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for PackId {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<PackId> for String {
    fn from(value: PackId) -> String {
        value.0
    }
}

impl AsRef<str> for PackId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackId({self})")
    }
}

impl PackRevisionId {
    /// Wraps a content digest as a pack revision identity.
    pub const fn from_digest(digest: ContentDigest) -> Self {
        Self(digest)
    }

    /// The underlying content digest.
    pub fn digest(&self) -> &ContentDigest {
        &self.0
    }
}

impl TryFrom<String> for PackRevisionId {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ContentDigest::try_from(value)
            .map(Self)
            .map_err(|error| PackContractError::InvalidDigest {
                reason: error.to_string(),
            })
    }
}

impl TryFrom<&str> for PackRevisionId {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<PackRevisionId> for String {
    fn from(value: PackRevisionId) -> String {
        value.0.into()
    }
}

impl AsRef<str> for PackRevisionId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PackRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl PackPolicyId {
    /// Wraps a content digest as a policy record identity.
    pub const fn from_digest(digest: ContentDigest) -> Self {
        Self(digest)
    }

    /// The underlying content digest.
    pub fn digest(&self) -> &ContentDigest {
        &self.0
    }
}

impl TryFrom<String> for PackPolicyId {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ContentDigest::try_from(value)
            .map(Self)
            .map_err(|error| PackContractError::InvalidDigest {
                reason: error.to_string(),
            })
    }
}

impl TryFrom<&str> for PackPolicyId {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<PackPolicyId> for String {
    fn from(value: PackPolicyId) -> String {
        value.0.into()
    }
}

impl AsRef<str> for PackPolicyId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PackPolicyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl PackProvenance {
    /// Records provenance for a revision building on a parent revision.
    pub fn child_of(parent_revision: PackRevisionId, producer: ProvenanceProducer) -> Self {
        Self {
            parent_revision: Some(parent_revision),
            producer,
        }
    }

    /// Records provenance for a root revision (no parent).
    pub fn root(producer: ProvenanceProducer) -> Self {
        Self {
            parent_revision: None,
            producer,
        }
    }
}

impl ProvenanceProducer {
    /// A human user or organization principal.
    pub fn user(subject: impl Into<String>) -> Result<Self, PackContractError> {
        validate_non_empty("provenance subject", subject.into())
            .map(|subject| Self::User { subject })
    }

    /// An LLM agent or worker that proposed the revision.
    pub fn agent(agent: impl Into<String>) -> Result<Self, PackContractError> {
        validate_non_empty("provenance agent", agent.into()).map(|agent| Self::Agent { agent })
    }

    /// A control plane that performed the governed transition.
    pub fn control_plane(plane: impl Into<String>) -> Result<Self, PackContractError> {
        validate_non_empty("provenance plane", plane.into())
            .map(|plane| Self::ControlPlane { plane })
    }
}

/// Validates a non-empty, whitespace-free principal string.
fn validate_non_empty(kind: &'static str, value: String) -> Result<String, PackContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(PackContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must be non-empty",
        })
    } else if trimmed.len() != value.len() {
        Err(PackContractError::InvalidIdentifier {
            kind,
            value,
            reason: "must not start or end with whitespace",
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
