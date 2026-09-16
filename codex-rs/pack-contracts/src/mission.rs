//! Mission, value model, and context model contracts for Packs.
//!
//! Owned by the PACK-001 work order (Contract Foundation).
//!
//! Every Pack has an explicit mission model that is user/organization
//! authority. The mission model distinguishes:
//!
//! ```text
//! Mission
//! Value Model
//! Context Model
//! Hard Constraints
//! User/Organization Preferences
//! Success Measures
//! ```
//!
//! An LLM-generated architecture is a proposal and may not silently redefine
//! the mission. That authority rule is structural here: every [`Mission`]
//! carries an explicit [`MissionAuthor`], and a mission proposed by an agent
//! (`AgentProposal`) is a distinct, clearly-labeled record — never silent
//! user authority. Whether an agent-proposed mission has been accepted as
//! authority is a governed transition owned by the pack lifecycle (PACK-002),
//! never by this record.
//!
//! Mission content is content-addressed: [`Mission::digest`] pins an exact
//! mission revision, and that digest participates in
//! [`crate::PackRevisionIdentity`]. The digest covers authorship on purpose:
//! a user-authored mission and an agent-proposed mission with identical text
//! are different mission revisions, because who speaks for the mission is
//! part of the mission's meaning.
//!
//! This module guarantees field-level integrity (non-empty statements,
//! validated identifiers). Mission completeness — for example, whether a
//! mission may be promoted without success measures — is a governance
//! decision owned by the pack lifecycle, not a contract invariant.

use codex_workflow_contracts::ContentDigest;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use crate::PackContractError;

/// Maximum length of a [`MissionId`] slug.
const MISSION_ID_MAX_LENGTH: usize = 128;

/// A stable identity for a mission lineage.
///
/// A `MissionId` names the mission lineage, not its content: revisions of the
/// same mission (for example an accepted rewrite of the statement) share one
/// `MissionId` while carrying distinct content digests. The slug rules match
/// [`crate::PackId`] so mission identities stay usable in URLs, file
/// systems, and display surfaces without escaping.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MissionId(String);

/// A non-empty mission statement or mission-model statement.
///
/// Statements are authored prose: they must be non-empty and must not start
/// or end with whitespace, because silently trimming an authored statement
/// would change mission content.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MissionStatement(String);

/// The principal behind mission authorship.
///
/// Either a user/organization subject or a description of the proposing
/// agent. Principals are validated non-empty strings; they are naming
/// metadata and never carry credentials.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MissionPrincipal(String);

/// Who authored the mission content.
///
/// Mission authority is explicit in the record:
///
/// - [`MissionAuthor::User`] records user/organization authorship. Only the
///   user/organization owns mission authority, so this is the only variant
///   that can represent authoritative mission content.
/// - [`MissionAuthor::AgentProposal`] records an LLM/agent proposal. An agent
///   proposal is never silent mission authority: it is a candidate that must
///   be accepted through the governed pack lifecycle before it can act as
///   the mission.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MissionAuthor {
    /// Authored by a user or organization principal.
    User {
        /// The authoring user or organization subject.
        subject: MissionPrincipal,
    },
    /// Proposed by an LLM agent or worker; never silent mission authority.
    AgentProposal {
        /// Description of the proposing agent or worker.
        agent: MissionPrincipal,
    },
}

/// The value model: what the mission considers valuable.
///
/// Objectives are ordered by priority as authored; the order is mission
/// content and is covered by the mission digest.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueModel {
    /// Objectives the pack optimizes for, most important first.
    pub objectives: Vec<ValueObjective>,
}

/// One objective inside the value model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueObjective {
    /// The objective statement.
    pub statement: MissionStatement,
}

/// The context model: the operating context the mission lives in.
///
/// Notes capture organization, environment, and situational context as
/// authored; like objectives, their order is content.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextModel {
    /// Notes describing the operating context.
    pub notes: Vec<ContextNote>,
}

/// One note inside the context model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextNote {
    /// The context note statement.
    pub statement: MissionStatement,
}

/// A non-negotiable hard constraint on the mission.
///
/// Hard constraints are inviolable inside the pack boundary; violating one is
/// a governance failure, not a preference trade-off. This differs from
/// [`UserPreference`], which records overridable guidance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardConstraint {
    /// The constraint statement.
    pub statement: MissionStatement,
}

/// A user/organization preference recorded in the mission model.
///
/// Preferences are overridable guidance: the pack may trade them off against
/// objectives and constraints. They never override [`HardConstraint`]s.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserPreference {
    /// The preference statement.
    pub statement: MissionStatement,
}

/// A measure of mission success.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuccessMeasure {
    /// The measure statement.
    pub statement: MissionStatement,
}

/// The complete mission model of a pack.
///
/// The record combines the six mission-model elements from the frozen
/// architecture with an explicit [`MissionAuthor`]. It is content-addressed
/// via [`Mission::digest`], which lets a
/// [`crate::PackRevisionIdentity`] pin an exact mission revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mission {
    /// The mission lineage identity.
    pub id: MissionId,
    /// The mission statement.
    pub statement: MissionStatement,
    /// Who authored this mission content.
    pub author: MissionAuthor,
    /// What the mission optimizes for.
    pub value_model: ValueModel,
    /// The operating context of the mission.
    pub context_model: ContextModel,
    /// Non-negotiable constraints.
    pub hard_constraints: Vec<HardConstraint>,
    /// Overridable user/organization preferences.
    pub preferences: Vec<UserPreference>,
    /// How mission success is measured.
    pub success_measures: Vec<SuccessMeasure>,
}

impl Mission {
    /// Canonical content digest of the complete mission record.
    ///
    /// The digest covers every field, including the author: a user-authored
    /// mission and an agent-proposed mission with identical text produce
    /// different digests, because mission authority is part of mission
    /// content. Equal digests mean equal mission revisions.
    pub fn digest(&self) -> Result<ContentDigest, PackContractError> {
        Ok(ContentDigest::of(self)?)
    }
}

impl MissionId {
    /// Parses a mission lineage identity.
    ///
    /// Valid slugs are 1–128 characters of lowercase ASCII alphanumeric or
    /// hyphen, and must not start or end with a hyphen (the same rules as
    /// [`crate::PackId`]).
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        let value = value.into();
        let valid_length = !value.is_empty() && value.len() <= MISSION_ID_MAX_LENGTH;
        let valid_charset = value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        let valid_edges = !value.starts_with('-') && !value.ends_with('-');
        if valid_length && valid_charset && valid_edges {
            Ok(Self(value))
        } else {
            Err(PackContractError::InvalidIdentifier {
                kind: "mission id",
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

impl TryFrom<String> for MissionId {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for MissionId {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<MissionId> for String {
    fn from(value: MissionId) -> Self {
        value.0
    }
}

impl AsRef<str> for MissionId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for MissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MissionId({self})")
    }
}

impl MissionStatement {
    /// Parses a mission statement.
    ///
    /// Statements must be non-empty and must not start or end with
    /// whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        parse_statement(value.into(), "mission statement").map(Self)
    }

    /// The statement text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for MissionStatement {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for MissionStatement {
    type Error = PackContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value.to_owned())
    }
}

impl From<MissionStatement> for String {
    fn from(value: MissionStatement) -> Self {
        value.0
    }
}

impl AsRef<str> for MissionStatement {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MissionStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for MissionStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MissionStatement({:?})", self.0)
    }
}

impl MissionPrincipal {
    /// Parses a mission authorship principal.
    ///
    /// Principals must be non-empty and must not start or end with
    /// whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, PackContractError> {
        parse_statement(value.into(), "mission principal").map(Self)
    }

    /// The principal string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for MissionPrincipal {
    type Error = PackContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<MissionPrincipal> for String {
    fn from(value: MissionPrincipal) -> Self {
        value.0
    }
}

impl AsRef<str> for MissionPrincipal {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl MissionAuthor {
    /// Records user/organization authorship — mission authority.
    pub fn user(subject: impl Into<String>) -> Result<Self, PackContractError> {
        MissionPrincipal::parse(subject).map(|subject| Self::User { subject })
    }

    /// Records an agent proposal — never silent mission authority.
    pub fn agent_proposal(agent: impl Into<String>) -> Result<Self, PackContractError> {
        MissionPrincipal::parse(agent).map(|agent| Self::AgentProposal { agent })
    }

    /// Whether this authorship is user/organization authority.
    pub fn is_user_authority(&self) -> bool {
        matches!(self, Self::User { .. })
    }
}

/// Validates authored statement text: non-empty, no edge whitespace.
fn parse_statement(value: String, kind: &'static str) -> Result<String, PackContractError> {
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
#[path = "mission_tests.rs"]
mod tests;
