//! Improvement candidates: proposed changes with full provenance.
//!
//! An [`ImprovementCandidate`] is the contract type of the learning plane:
//! a **proposed** change to a workflow, anchored to the immutable published
//! version it was derived from and carrying the execution evidence that
//! produced it. Candidates are never authority: they are reviewable
//! proposals that live or die by the validation pipeline, the approval
//! plane, and the governed publication path.
//!
//! Provenance is reference-shaped by construction: evidence is cited as
//! [`EvidenceReference`]s (locator + digest) and runs as content-addressed
//! fingerprints ([`RunProvenance`]). No evidence payload, transcript, or
//! external content is embedded, so candidates stay bounded regardless of
//! how much execution evidence backs them.

use std::collections::BTreeMap;

use codex_eval_compat::WorkflowRunRecord;
use codex_execution_contracts::BindingPolicy;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_triggers::ScheduleSpec;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowEvolutionError;

/// The identity of one improvement candidate.
///
/// Candidate ids are content-addressed by the generator (kind, incumbent
/// version, provenance digests), so regenerating candidates from the same
/// evidence yields the same identities and duplicates are detectable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CandidateId(String);

impl CandidateId {
    /// Parses and validates a candidate identity: non-empty, ASCII
    /// lowercase letters, digits, or `-`, at most 96 characters.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowEvolutionError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 96
            && value.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: value,
                reason:
                    "candidate ids must be non-empty lowercase/digits/hyphen, at most 96 characters"
                        .to_string(),
            })
        }
    }

    /// Derives a content-addressed candidate id from a namespace prefix,
    /// the change kind key, and the digest of the provenance projection.
    pub(crate) fn derive(
        prefix: &str,
        kind_key: &str,
        provenance_digest: &ContentDigest,
    ) -> Result<Self, WorkflowEvolutionError> {
        let digest = provenance_digest.as_str().trim_start_matches("sha256:");
        // Sixteen hex characters keep ids short while collisions remain
        // negligible for a bounded ledger.
        let short = &digest[..16];
        Self::parse(format!("{prefix}-{kind_key}-{short}"))
    }
}

impl TryFrom<String> for CandidateId {
    type Error = WorkflowEvolutionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<CandidateId> for String {
    fn from(value: CandidateId) -> Self {
        value.0
    }
}

impl AsRef<str> for CandidateId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CandidateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The category of a proposed change.
///
/// The five categories are exactly the improvement surfaces the Work Order
/// names: workflow definitions, capability bindings, recovery policies,
/// dependency choices, and scheduling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    /// A delta to the workflow definition (graph, descriptions, roles,
    /// triggers, declared dependencies).
    DefinitionDelta,
    /// A change to the declared capability bindings of step nodes.
    CapabilityBinding,
    /// An adjustment to the binding/recovery policy applied to future
    /// runs.
    RecoveryPolicy,
    /// A change to the resolved dependency lock (a dependency choice).
    DependencyChoice,
    /// A tuning of the trigger schedule.
    ScheduleTuning,
}

impl ChangeKind {
    /// The stable key of this category (used in candidate ids and
    /// summaries).
    pub fn key(self) -> &'static str {
        match self {
            Self::DefinitionDelta => "definition-delta",
            Self::CapabilityBinding => "capability-binding",
            Self::RecoveryPolicy => "recovery-policy",
            Self::DependencyChoice => "dependency-choice",
            Self::ScheduleTuning => "schedule-tuning",
        }
    }

    /// Whether this category changes version-covered content (definition
    /// or dependency lock) rather than control-plane settings.
    pub fn affects_version_content(self) -> bool {
        matches!(
            self,
            Self::DefinitionDelta | Self::CapabilityBinding | Self::DependencyChoice
        )
    }
}

/// The proposed change itself, per category.
///
/// Payloads are complete successor values, not diffs: replaying a candidate
/// never depends on reconstructing an intermediate state.
#[allow(clippy::large_enum_variant)] // the definition payload dominates; boxing would tax every match for one variant
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ProposedChange {
    /// Replace the workflow definition with the given successor
    /// definition.
    Definition {
        /// The successor definition content.
        definition: WorkflowDefinition,
    },
    /// Replace the declared capability requirements of the given step
    /// nodes.
    Bindings {
        /// Requirements per step node; nodes not listed keep their
        /// incumbent declarations.
        bindings: BTreeMap<IrNodeId, Vec<CapabilityRequirement>>,
    },
    /// Apply a different binding policy to future runs.
    Recovery {
        /// The successor binding policy.
        policy: BindingPolicy,
    },
    /// Resolve the workflow's dependencies with the given successor lock.
    Dependency {
        /// The successor dependency lock.
        lock: DependencyLock,
    },
    /// Tune the trigger schedule of the workflow.
    Schedule {
        /// The successor schedule specification.
        spec: ScheduleSpec,
    },
}

impl ProposedChange {
    /// The category of this change.
    pub fn kind(&self) -> ChangeKind {
        match self {
            Self::Definition { .. } => ChangeKind::DefinitionDelta,
            Self::Bindings { .. } => ChangeKind::CapabilityBinding,
            Self::Recovery { .. } => ChangeKind::RecoveryPolicy,
            Self::Dependency { .. } => ChangeKind::DependencyChoice,
            Self::Schedule { .. } => ChangeKind::ScheduleTuning,
        }
    }

    /// Applies the change over `incumbent` on behalf of the candidate
    /// identified by `owner`, producing the successor definition and
    /// dependency lock.
    ///
    /// The incumbent record is only read: the result is new content, never
    /// a mutation. Binding changes must target existing step nodes;
    /// definition deltas must keep the incumbent's workflow identity.
    pub(crate) fn applied_over(
        &self,
        owner: &CandidateId,
        incumbent: &WorkflowVersion,
    ) -> Result<(WorkflowDefinition, DependencyLock), WorkflowEvolutionError> {
        let invalid = |reason: String| WorkflowEvolutionError::InvalidCandidate {
            candidate: owner.to_string(),
            reason,
        };
        match self {
            Self::Definition { definition } => {
                if definition.id != incumbent.definition.id {
                    return Err(invalid(format!(
                        "definition delta changes workflow identity from `{}` to `{}`",
                        incumbent.definition.id, definition.id
                    )));
                }
                Ok((definition.clone(), incumbent.dependency_lock.clone()))
            }
            Self::Bindings { bindings } => {
                let mut definition = incumbent.definition.clone();
                for (node_id, requirements) in bindings {
                    match definition.ir.nodes.get_mut(node_id) {
                        Some(WorkflowIrNode::Step(step)) => {
                            step.capabilities = requirements.clone();
                        }
                        Some(_) => {
                            return Err(invalid(format!(
                                "binding change targets node `{node_id}`, which is not a step node"
                            )));
                        }
                        None => {
                            return Err(invalid(format!(
                                "binding change targets node `{node_id}`, which does not exist"
                            )));
                        }
                    }
                }
                Ok((definition, incumbent.dependency_lock.clone()))
            }
            Self::Recovery { .. } | Self::Schedule { .. } => {
                // Control-plane adjustments do not change version-covered
                // content: the successor version re-publishes the incumbent
                // content and the lineage carries the adjustment.
                Ok((
                    incumbent.definition.clone(),
                    incumbent.dependency_lock.clone(),
                ))
            }
            Self::Dependency { lock } => Ok((incumbent.definition.clone(), lock.clone())),
        }
    }
}

/// The content-addressed identity of one recorded run.
///
/// Runs are cited by the immutable version they pinned, the semantic
/// fingerprint of their record, and their settled status — enough to
/// identify the equivalence class without embedding any run-local data.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunProvenance {
    /// The immutable version the run pinned.
    pub version: WorkflowVersionId,
    /// The semantic fingerprint digest of the run record.
    pub fingerprint: ContentDigest,
    /// The settled instance status of the run.
    pub status: WorkflowInstanceStatus,
}

impl RunProvenance {
    /// Projects one evaluation run record into its provenance form.
    pub fn of_record(record: &WorkflowRunRecord) -> Result<Self, WorkflowEvolutionError> {
        Ok(Self {
            version: record.version.clone(),
            fingerprint: record.fingerprint_digest()?,
            status: record.status,
        })
    }
}

/// The evidence stream a candidate was generated from.
///
/// References only: locators and digests allocated by the evidence plane,
/// plus the content-addressed identities of the runs. No payload, transcript,
/// or external content is retained.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateProvenance {
    /// The execution evidence references backing the candidate.
    pub evidence: Vec<EvidenceReference>,
    /// The runs whose records the candidate was derived from.
    pub runs: Vec<RunProvenance>,
}

impl CandidateProvenance {
    /// The canonical provenance projection used for candidate id
    /// derivation: kind-independent, order-stable, reference-shaped.
    pub(crate) fn projection(
        &self,
        kind: ChangeKind,
        incumbent: &WorkflowVersionId,
    ) -> serde_json::Value {
        serde_json::json!({
            "kind": kind.key(),
            "incumbent": incumbent.to_string(),
            "evidence": self
                .evidence
                .iter()
                .map(|reference| serde_json::json!({
                    "kind": reference.kind,
                    "locator": reference.locator,
                    "digest": reference.digest.as_str(),
                }))
                .collect::<Vec<_>>(),
            "runs": self
                .runs
                .iter()
                .map(|run| serde_json::json!({
                    "version": run.version.to_string(),
                    "fingerprint": run.fingerprint.as_str(),
                    "status": run.status,
                }))
                .collect::<Vec<_>>(),
        })
    }
}

/// One proposed improvement, with provenance from evidence to change.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImprovementCandidate {
    /// The candidate's identity.
    pub id: CandidateId,
    /// The immutable published version the candidate evolves.
    pub incumbent: PublishedVersionRef,
    /// The proposed change.
    pub change: ProposedChange,
    /// Why the generator proposes this change (bounded, derived text).
    pub rationale: String,
    /// The evidence stream the candidate was generated from.
    pub provenance: CandidateProvenance,
}

impl ImprovementCandidate {
    /// The category of the proposed change.
    pub fn change_kind(&self) -> ChangeKind {
        self.change.kind()
    }

    /// The workflow this candidate evolves.
    pub fn workflow(&self) -> &WorkflowDefinitionId {
        &self.incumbent.identity.workflow
    }

    /// Applies this candidate's change over `incumbent`, producing the
    /// successor definition and dependency lock.
    ///
    /// The incumbent record is only read: the result is new content, never
    /// a mutation.
    pub fn applied_over(
        &self,
        incumbent: &WorkflowVersion,
    ) -> Result<(WorkflowDefinition, DependencyLock), WorkflowEvolutionError> {
        self.change.applied_over(&self.id, incumbent)
    }

    /// Validates the candidate's structural invariants.
    ///
    /// - the incumbent reference must verify against its recomputed digest;
    /// - the rationale must stay bounded;
    /// - a definition delta must carry the incumbent's workflow identity.
    pub fn validate(&self) -> Result<(), WorkflowEvolutionError> {
        self.incumbent.verify()?;
        if self.rationale.len() > 512 {
            return Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: self.id.to_string(),
                reason: "rationale must stay bounded (at most 512 characters)".to_string(),
            });
        }
        if let ProposedChange::Definition { definition } = &self.change
            && definition.id != self.incumbent.identity.workflow
        {
            return Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: self.id.to_string(),
                reason: format!(
                    "definition delta changes workflow identity from `{}` to `{}`",
                    self.incumbent.identity.workflow, definition.id
                ),
            });
        }
        Ok(())
    }
}

/// A semantic-version bump direction for a successor version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionBump {
    /// Patch-level successor.
    Patch,
    /// Minor-level successor.
    Minor,
    /// Major-level successor.
    Major,
}

impl VersionBump {
    /// Applies the bump to `version`, clearing pre-release and build
    /// metadata: a governed successor is always a plain release version.
    pub fn apply(self, version: &SemanticVersion) -> SemanticVersion {
        match self {
            Self::Patch => SemanticVersion::new(version.major, version.minor, version.patch + 1),
            Self::Minor => SemanticVersion::new(version.major, version.minor + 1, 0),
            Self::Major => SemanticVersion::new(version.major + 1, 0, 0),
        }
    }
}

#[cfg(test)]
#[path = "candidate_tests.rs"]
mod tests;
