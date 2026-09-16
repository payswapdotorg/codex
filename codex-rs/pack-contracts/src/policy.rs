//! Pack Policy contracts.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! A [`PackPolicy`] record carries governed policy content whose identity is
//! a content-addressed [`crate::PackPolicyId`]. Policy authority is explicit:
//! policies constrain behavior inside the pack boundary, they do not grant
//! authority over workflow transitions, credentials, or evidence. Every
//! policy record carries a [`PolicyAuthorityBoundary`] that must explicitly
//! acknowledge all [`RetainedAuthority`] values — there is no field that can
//! transfer those authorities to a policy, and validation rejects records
//! that try to acknowledge only a subset.
//!
//! The complete governing policy content of a pack revision — its
//! constitution plus its governing policies — is carried by
//! [`PackPolicySet`], whose digest is the `policy_digest` input of
//! [`crate::PackRevisionIdentity`].
//!
//! Assurance and determinism dimensions are policy-scoped and owned by the
//! `assurance` module (PACK-003); this module owns the general governing
//! policy records. A governing policy may reference assurance policies by
//! [`crate::PackPolicyId`] only.

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

use crate::PackConstitution;
use crate::PackContractError;
use crate::PackPolicyId;

/// A non-empty policy statement.
///
/// Statements must be non-empty and must not start or end with whitespace:
/// silently trimming an authored statement would change policy content.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PolicyStatement(String);

/// What a pack policy governs inside the pack boundary.
///
/// The scope names governed surfaces; it never defines them. The system
/// state, its lifecycle, and its candidate/promoted distinction are owned by
/// the `system_state` module (PACK-002), and assurance dimensions are owned
/// by the `assurance` module (PACK-003). A policy scoped to
/// [`PackPolicyScope::SystemStateChanges`] constrains how those changes may
/// happen; it does not acquire the right to perform them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackPolicyScope {
    /// The entire pack boundary.
    EntirePack,
    /// Changes to the pack's mission model.
    MissionModel,
    /// Changes to the pack's constitution.
    ConstitutionChanges,
    /// Updates to the pack's dependencies.
    DependencyUpdates,
    /// Changes to the pack's system state (candidate construction and
    /// promotion proposals).
    SystemStateChanges,
}

/// An authority a pack policy explicitly does not hold.
///
/// Pack policies constrain behavior; they never grant these authorities.
/// The list mirrors the frozen authority model: workflow transitions belong
/// to the Workflow Control Plane, credentials to the credential authority,
/// and evidence to the evidence authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetainedAuthority {
    /// Durable workflow transitions remain owned by the Workflow Control
    /// Plane.
    WorkflowTransitions,
    /// Credential management remains owned by the platform credential
    /// authority.
    Credentials,
    /// Evidence attestation remains owned by the platform evidence
    /// authority.
    Evidence,
}

/// The complete retained-authority list.
///
/// Every pack policy must acknowledge exactly this list. A policy record
/// that acknowledges only a subset is an attempt to claim a retained
/// authority and is rejected by validation.
pub const ALL_RETAINED_AUTHORITIES: [RetainedAuthority; 3] = [
    RetainedAuthority::WorkflowTransitions,
    RetainedAuthority::Credentials,
    RetainedAuthority::Evidence,
];

/// The explicit authority boundary of a pack policy.
///
/// A policy must say what it governs and must acknowledge every authority it
/// does not hold. The acknowledgment is not configurable: validation
/// requires the complete [`ALL_RETAINED_AUTHORITIES`] list, and no field of
/// any policy type can transfer a retained authority to a policy.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyAuthorityBoundary {
    /// What this policy governs (constrains) inside the pack boundary.
    pub governs: PolicyStatement,
    /// The authorities this policy does not hold; must always be the
    /// complete [`ALL_RETAINED_AUTHORITIES`] list.
    pub retained_authorities: BTreeSet<RetainedAuthority>,
}

impl PolicyAuthorityBoundary {
    /// Builds a boundary for the given governed surface, acknowledging every
    /// retained authority.
    pub fn new(governs: impl Into<String>) -> Result<Self, PackContractError> {
        let boundary = Self {
            governs: PolicyStatement::parse(governs)?,
            retained_authorities: ALL_RETAINED_AUTHORITIES.into_iter().collect(),
        };
        boundary.validate()?;
        Ok(boundary)
    }

    /// Validates the boundary: the retained-authority acknowledgment must be
    /// complete.
    pub fn validate(&self) -> Result<(), PackContractError> {
        let expected: BTreeSet<RetainedAuthority> = ALL_RETAINED_AUTHORITIES.into_iter().collect();
        if self.retained_authorities != expected {
            return Err(PackContractError::PolicyConflict {
                reason: format!(
                    "a pack policy may not claim workflow-transition, credential, or \
                     evidence authority: the acknowledgment must cover all {} retained \
                     authorities",
                    ALL_RETAINED_AUTHORITIES.len()
                ),
            });
        }
        Ok(())
    }
}

/// A general governing policy record inside a pack.
///
/// A policy constrains behavior inside the pack boundary for its
/// [`PackPolicyScope`]. Its identity is content-addressed:
/// [`PackPolicy::policy_id`] digests the complete record into a
/// [`PackPolicyId`], so any statement, scope, boundary, or reference change
/// produces a different policy identity.
///
/// Policies never grant authority. In particular they cannot grant
/// workflow-transition, credential, or evidence authority (see
/// [`PolicyAuthorityBoundary`]), and they cannot weaken platform invariants
/// (see [`crate::PlatformInvariant`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackPolicy {
    /// What the policy governs inside the pack.
    pub scope: PackPolicyScope,
    /// The policy statements, in authored order (order is content).
    pub statements: Vec<PolicyStatement>,
    /// The explicit authority boundary of this policy.
    pub authority_boundary: PolicyAuthorityBoundary,
    /// Policies referenced by this record, by content-addressed identity —
    /// for example assurance policies owned by the `assurance` module
    /// (PACK-003). References are identities only; referenced content never
    /// becomes part of this record.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub referenced_policies: BTreeSet<PackPolicyId>,
}

impl PackPolicy {
    /// Builds a governing policy record.
    ///
    /// The authority boundary is constructed with the complete
    /// retained-authority acknowledgment; it is not caller-configurable.
    pub fn new(
        scope: PackPolicyScope,
        statements: Vec<PolicyStatement>,
        governs: impl Into<String>,
        referenced_policies: BTreeSet<PackPolicyId>,
    ) -> Result<Self, PackContractError> {
        let policy = Self {
            scope,
            statements,
            authority_boundary: PolicyAuthorityBoundary::new(governs)?,
            referenced_policies,
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Validates the policy record.
    ///
    /// A record is valid only when it declares at least one statement and
    /// its authority boundary acknowledges every retained authority.
    pub fn validate(&self) -> Result<(), PackContractError> {
        if self.statements.is_empty() {
            return Err(PackContractError::PolicyConflict {
                reason: "a pack policy must declare at least one statement".to_owned(),
            });
        }
        self.authority_boundary.validate()
    }

    /// Computes the content-addressed identity of this policy record.
    ///
    /// The digest covers the complete record — scope, statements, authority
    /// boundary, and referenced policies. Equal content yields equal policy
    /// identities; any change yields a different identity.
    pub fn policy_id(&self) -> Result<PackPolicyId, PackContractError> {
        Ok(PackPolicyId::from_digest(ContentDigest::of(self)?))
    }
}

/// The complete governing policy content of a pack revision: its constitution
/// plus its governing policies.
///
/// This is the record whose digest becomes the `policy_digest` of a
/// [`crate::PackRevisionIdentity`]: a revision pins one exact governing
/// policy set. The set contains the constitution and the general governing
/// policies; assurance policies enter only as
/// [`PackPolicy::referenced_policies`] identities, and their content is
/// pinned through the system-state digest (owned by the `system_state`
/// module, PACK-002).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackPolicySet {
    /// The pack constitution.
    pub constitution: PackConstitution,
    /// The governing policies, in authored order (order is content).
    pub policies: Vec<PackPolicy>,
}

impl PackPolicySet {
    /// Builds a governing policy set from a validated constitution and
    /// validated governing policies.
    ///
    /// Every policy must be distinct: two records with the same
    /// [`PackPolicyId`] are a duplication error, not a merge.
    pub fn new(
        constitution: PackConstitution,
        policies: Vec<PackPolicy>,
    ) -> Result<Self, PackContractError> {
        let set = Self {
            constitution,
            policies,
        };
        set.validate()?;
        Ok(set)
    }

    /// Validates the policy set end to end: the constitution validates, every
    /// policy validates, and no policy appears twice.
    pub fn validate(&self) -> Result<(), PackContractError> {
        self.constitution.validate()?;
        let mut seen = BTreeSet::new();
        for policy in &self.policies {
            policy.validate()?;
            let policy_id = policy.policy_id()?;
            if !seen.insert(policy_id) {
                return Err(PackContractError::PolicyConflict {
                    reason: "duplicate governing policy in the policy set".to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Canonical content digest of the complete governing policy set.
    ///
    /// This is the digest a pack revision records as its `policy_digest`:
    /// equal digests mean equal governing policy content.
    pub fn digest(&self) -> Result<ContentDigest, PackContractError> {
        Ok(ContentDigest::of(self)?)
    }
}

impl PolicyStatement {
    /// Parses a policy statement.
    ///
    /// Statements must be non-empty and must not start or end with
    /// whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(PackContractError::InvalidIdentifier {
                kind: "policy statement",
                value,
                reason: "must be non-empty",
            })
        } else if trimmed.len() != value.len() {
            Err(PackContractError::InvalidIdentifier {
                kind: "policy statement",
                value,
                reason: "must not start or end with whitespace",
            })
        } else {
            Ok(Self(value))
        }
    }

    /// The statement text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for PolicyStatement {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for PolicyStatement {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<PolicyStatement> for String {
    fn from(value: PolicyStatement) -> Self {
        value.0
    }
}

impl AsRef<str> for PolicyStatement {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PolicyStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for PolicyStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PolicyStatement({:?})", self.0)
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
