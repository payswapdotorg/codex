//! Pack Constitution contracts.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! A Pack Constitution contains domain-specific invariants and
//! non-negotiable rules that apply inside the Pack boundary, for example:
//! forbidden actions, required approvals, provenance requirements, data
//! integrity rules, domain state-machine invariants, non-destructive
//! behavior, and audit requirements.
//!
//! The Pack Constitution cannot weaken the Codex platform Constitution,
//! authorization, credential, evidence, or security invariants. That rule is
//! structural here: every [`PackConstitution`] must explicitly acknowledge
//! the full well-known [`PlatformInvariant`] list, there is no field that can
//! weaken or disable a platform invariant, and validation
//! ([`PackConstitution::validate`]) rejects any record that attempts to
//! acknowledge only a subset. Whether a rule's *prose* tries to subvert a
//! platform invariant is a review concern (red-team review in later Pack
//! phases), not something a structural contract can decide.
//!
//! Constitution content is content-addressed ([`PackConstitution::digest`])
//! and participates in the pack revision identity through the governing
//! policy digest ([`crate::PackPolicySet::digest`]).

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

use crate::PackContractError;

/// A non-empty constitutional rule statement.
///
/// Rule statements must be non-empty and must not start or end with
/// whitespace: silently trimming an authored rule would change constitutional
/// content.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConstitutionStatement(String);

/// A domain-defined rule class label for the [`ConstitutionRule::Domain`]
/// escape hatch.
///
/// Domain rule classes exist because the frozen rule-class list cannot
/// anticipate every mission domain (medical dosing, CAD geometry, accounting
/// closures all have bespoke invariant classes); a closed list would force
/// domains to abuse the nearest built-in class. The label is a validated
/// snake_case identifier that carries **no** additional semantics: consumers
/// outside the owning domain must treat `Domain` rules as opaque
/// non-negotiable statements.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DomainRuleClass(String);

/// One non-negotiable rule of a pack constitution.
///
/// The rule classes are typed: each variant names the class of invariant the
/// rule expresses. The only open-ended class is [`ConstitutionRule::Domain`],
/// whose escape hatch is documented on [`DomainRuleClass`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ConstitutionRule {
    /// An action that is forbidden inside the pack boundary.
    ForbiddenAction {
        /// The forbidden action.
        statement: ConstitutionStatement,
    },
    /// An action that requires approval before it may run.
    RequiredApproval {
        /// What requires approval, and from whom.
        statement: ConstitutionStatement,
    },
    /// A provenance requirement for pack content.
    ProvenanceRequirement {
        /// The provenance requirement.
        statement: ConstitutionStatement,
    },
    /// A data integrity rule.
    DataIntegrity {
        /// The integrity rule.
        statement: ConstitutionStatement,
    },
    /// A domain state-machine invariant the pack must uphold.
    DomainStateMachineInvariant {
        /// The state-machine invariant.
        statement: ConstitutionStatement,
    },
    /// A non-destructive behavior requirement.
    NonDestructiveBehavior {
        /// The non-destructive behavior requirement.
        statement: ConstitutionStatement,
    },
    /// An audit requirement.
    AuditRequirement {
        /// The audit requirement.
        statement: ConstitutionStatement,
    },
    /// A domain-defined rule class (see [`DomainRuleClass`]).
    Domain {
        /// The domain class the rule belongs to.
        class: DomainRuleClass,
        /// The rule statement.
        statement: ConstitutionStatement,
    },
}

/// A well-known platform invariant that no pack constitution may weaken.
///
/// These invariants are owned by the Codex platform, not by any pack. The
/// list is closed and typed on purpose: a pack cannot invent, rename, or
/// remove platform invariants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformInvariant {
    /// The Codex platform Constitution applies inside every pack boundary.
    PlatformConstitution,
    /// Authorization decisions remain owned by the platform authorization
    /// authority.
    Authorization,
    /// Credential issuance and management remain owned by the platform
    /// credential authority.
    Credentials,
    /// Evidence attestation remains owned by the platform evidence
    /// authority.
    Evidence,
    /// Platform security invariants (sandboxing, approval escalation,
    /// process hardening) remain owned by the platform.
    Security,
}

/// The complete well-known platform-invariant list.
///
/// Every pack constitution must acknowledge exactly this list. When the
/// platform adds an invariant, this list grows and previously recorded
/// constitutions fail validation until they are re-sealed against the new
/// list — a deliberate governance-visible event, never a silent pass-through.
pub const ALL_PLATFORM_INVARIANTS: [PlatformInvariant; 5] = [
    PlatformInvariant::PlatformConstitution,
    PlatformInvariant::Authorization,
    PlatformInvariant::Credentials,
    PlatformInvariant::Evidence,
    PlatformInvariant::Security,
];

/// The constitution of a pack: domain-specific invariants and non-negotiable
/// rules that apply inside the pack boundary.
///
/// A constitution cannot weaken platform invariants: the
/// `protected_invariants` acknowledgment is not configurable — it must always
/// be the complete [`ALL_PLATFORM_INVARIANTS`] list, and
/// [`PackConstitution::validate`] enforces that. There is no mechanism in
/// this record to disable, weaken, or waive any platform invariant.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackConstitution {
    /// The domain rules, in authored order (order is content).
    pub rules: Vec<ConstitutionRule>,
    /// The platform invariants this constitution explicitly acknowledges as
    /// unweakenable. Validation requires this to be exactly
    /// [`ALL_PLATFORM_INVARIANTS`].
    pub protected_invariants: BTreeSet<PlatformInvariant>,
}

impl PackConstitution {
    /// Builds a constitution from the given rules.
    ///
    /// The protected-invariant acknowledgment is set to the full well-known
    /// list; it is not caller-configurable. A constitution must declare at
    /// least one rule — a pack that wants no domain invariants should not
    /// carry a constitution record at all.
    pub fn new(rules: Vec<ConstitutionRule>) -> Result<Self, PackContractError> {
        let constitution = Self {
            rules,
            protected_invariants: ALL_PLATFORM_INVARIANTS.into_iter().collect(),
        };
        constitution.validate()?;
        Ok(constitution)
    }

    /// Validates the constitution record.
    ///
    /// A record is valid only when it declares at least one rule and
    /// acknowledges the complete platform-invariant list. A record that
    /// acknowledges only a subset is an attempt to weaken platform invariants
    /// and is rejected.
    pub fn validate(&self) -> Result<(), PackContractError> {
        if self.rules.is_empty() {
            return Err(PackContractError::PolicyConflict {
                reason: "a pack constitution must declare at least one rule".to_owned(),
            });
        }
        let expected: BTreeSet<PlatformInvariant> = ALL_PLATFORM_INVARIANTS.into_iter().collect();
        if self.protected_invariants != expected {
            return Err(PackContractError::PolicyConflict {
                reason: format!(
                    "a pack constitution may not weaken platform invariants: the \
                     acknowledgment must cover all {} well-known invariants",
                    ALL_PLATFORM_INVARIANTS.len()
                ),
            });
        }
        Ok(())
    }

    /// Canonical content digest of the complete constitution record.
    ///
    /// The digest covers the rules (in authored order) and the
    /// protected-invariant acknowledgment. Equal digests mean equal
    /// constitutions.
    pub fn digest(&self) -> Result<ContentDigest, PackContractError> {
        Ok(ContentDigest::of(self)?)
    }
}

impl ConstitutionStatement {
    /// Parses a constitutional rule statement.
    ///
    /// Statements must be non-empty and must not start or end with
    /// whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(PackContractError::InvalidIdentifier {
                kind: "constitution statement",
                value,
                reason: "must be non-empty",
            })
        } else if trimmed.len() != value.len() {
            Err(PackContractError::InvalidIdentifier {
                kind: "constitution statement",
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

impl TryFrom<String> for ConstitutionStatement {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ConstitutionStatement {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<ConstitutionStatement> for String {
    fn from(value: ConstitutionStatement) -> Self {
        value.0
    }
}

impl AsRef<str> for ConstitutionStatement {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ConstitutionStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for ConstitutionStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConstitutionStatement({:?})", self.0)
    }
}

impl DomainRuleClass {
    /// Parses a domain rule class label.
    ///
    /// Labels must be non-empty lowercase snake_case (ASCII letters, digits,
    /// `_`), must not start or end with `_`, and must not contain
    /// whitespace, so they stay stable across serializations and language
    /// bindings.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !value.starts_with('_')
            && !value.ends_with('_');
        if valid {
            Ok(Self(value))
        } else {
            Err(PackContractError::InvalidIdentifier {
                kind: "domain rule class",
                value,
                reason: "must be non-empty lowercase snake_case",
            })
        }
    }

    /// The class label.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for DomainRuleClass {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for DomainRuleClass {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<DomainRuleClass> for String {
    fn from(value: DomainRuleClass) -> Self {
        value.0
    }
}

impl AsRef<str> for DomainRuleClass {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for DomainRuleClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for DomainRuleClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DomainRuleClass({})", self.0)
    }
}

#[cfg(test)]
#[path = "constitution_tests.rs"]
mod tests;
