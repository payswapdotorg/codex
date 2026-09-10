//! The candidate ledger and retention policy.
//!
//! The ledger is the bounded, in-memory record of improvement candidates
//! and their governance state. It exists for tests, seeding, and hosts
//! without a durable control plane; the Codex application owns durable
//! candidate storage and mirrors these semantics.
//!
//! # Retention
//!
//! Retention is bounded by [`RetentionPolicy`] and enforced on every
//! insert, never deferred:
//!
//! - **candidates** are capped; when a new candidate exceeds the cap, the
//!   oldest entry is evicted first-in-first-out. Eviction is safe for
//!   governance: published lineage and the install registry keep the
//!   immutable audit trail, candidates are regenerable from evidence by
//!   the deterministic generator, and eviction is the documented
//!   policy-driven bound the Work Order requires.
//! - **evidence provenance** per candidate is capped loudly: recording a
//!   candidate whose provenance exceeds the cap is refused, never
//!   silently truncated.
//!
//! # Ledger entries
//!
//! Each entry tracks one candidate's governance state: the validation
//! report and staged succession after validation, the approval record
//! after a decision, and the promotion outcome after publication. Entries
//! with an outcome are immutable history inside the bound.

use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use serde::Deserialize;
use serde::Serialize;

use crate::CandidateId;
use crate::ImprovementCandidate;
use crate::WorkflowEvolutionError;
use crate::scrub;
use crate::validate::ValidationReport;

/// The bounded retention policy of the evolution plane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionPolicy {
    /// Maximum candidates retained in the ledger.
    pub max_candidates: usize,
    /// Maximum evidence references retained per candidate provenance.
    pub max_evidence_per_candidate: usize,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_candidates: 64,
            max_evidence_per_candidate: 128,
        }
    }
}

/// The staged succession produced by a successful validation: the
/// immutable revision minted for the candidate, and the sealed successor
/// version anchored there.
#[derive(Clone, Debug)]
pub struct StagedSuccession {
    /// The immutable source revision the successor anchors at.
    pub revision: ImmutableSourceRevision,
    /// The proposed successor semantic version.
    pub successor_version: SemanticVersion,
    /// The sealed successor version record.
    pub version: WorkflowVersion,
    /// The successor's published-version reference.
    pub reference: PublishedVersionRef,
}

/// The governance outcome of one candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum CandidateOutcome {
    /// Still proposed: nothing has been decided.
    Pending,
    /// Promoted: published as a successor version through the forge.
    Promoted {
        /// The published successor version identity.
        successor: WorkflowVersionId,
    },
}

/// One candidate's ledger entry: the proposal plus its governance state.
#[derive(Clone, Debug)]
pub struct LedgerEntry {
    /// The recorded candidate.
    pub candidate: ImprovementCandidate,
    /// The validation report, once validated.
    pub validation: Option<ValidationReport>,
    /// The staged succession, once validated.
    pub staged: Option<StagedSuccession>,
    /// The approval decision, once decided.
    pub approval: Option<crate::ApprovalRecord>,
    /// The governance outcome.
    pub outcome: CandidateOutcome,
}

/// The retention-bounded candidate ledger.
#[derive(Clone, Debug)]
pub struct EvolutionLedger {
    policy: RetentionPolicy,
    entries: Vec<LedgerEntry>,
}

impl EvolutionLedger {
    /// Creates an empty ledger under `policy`.
    ///
    /// The policy's bounds must be non-zero: a zero cap would retain
    /// nothing and silently discard governance state.
    pub fn new(policy: RetentionPolicy) -> Result<Self, WorkflowEvolutionError> {
        if policy.max_candidates == 0 || policy.max_evidence_per_candidate == 0 {
            return Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: "retention policy".to_string(),
                reason: "retention caps must be positive".to_string(),
            });
        }
        Ok(Self {
            policy,
            entries: Vec::new(),
        })
    }

    /// The ledger's retention policy.
    pub fn policy(&self) -> &RetentionPolicy {
        &self.policy
    }

    /// All retained entries, oldest first.
    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }

    /// One entry, by candidate id.
    pub fn entry(&self, candidate: &CandidateId) -> Option<&LedgerEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.candidate.id == candidate)
    }

    /// One entry, mutably, by candidate id.
    pub(crate) fn entry_mut(&mut self, candidate: &CandidateId) -> Option<&mut LedgerEntry> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.candidate.id == candidate)
    }

    /// Records one candidate.
    ///
    /// Recording is the retention and credential gate:
    ///
    /// - the candidate is structurally validated first;
    /// - its change payload and rationale must pass the credential check
    ///   (credentials never enter candidates);
    /// - its provenance must fit the evidence-reference cap (loudly
    ///   refused, never truncated);
    /// - duplicate identities are refused (candidates are
    ///   content-addressed);
    /// - the candidate cap is enforced by evicting the oldest entry.
    pub fn record(
        &mut self,
        candidate: ImprovementCandidate,
    ) -> Result<(), WorkflowEvolutionError> {
        candidate.validate()?;
        let change = serde_json::to_value(&candidate.change)?;
        scrub::require_clean_json(&change, "candidate change")?;
        scrub::require_clean_text(&candidate.rationale, "candidate rationale")?;
        if candidate.provenance.evidence.len() > self.policy.max_evidence_per_candidate {
            return Err(WorkflowEvolutionError::ProvenanceTooLarge {
                candidate: candidate.id.to_string(),
                count: candidate.provenance.evidence.len(),
                cap: self.policy.max_evidence_per_candidate,
            });
        }
        if self.entry(&candidate.id).is_some() {
            return Err(WorkflowEvolutionError::CandidateAlreadyKnown {
                candidate: candidate.id.to_string(),
            });
        }
        self.entries.push(LedgerEntry {
            candidate,
            validation: None,
            staged: None,
            approval: None,
            outcome: CandidateOutcome::Pending,
        });
        // The hard bound: oldest entries leave first. Published governance
        // lives in lineage and the install registry, and candidates are
        // regenerable from evidence by the deterministic generator, so
        // eviction is the documented policy-driven retention, not a
        // silent drop of audit history.
        while self.entries.len() > self.policy.max_candidates {
            self.entries.remove(0);
        }
        Ok(())
    }

    /// Records a validation outcome and its staged succession.
    pub(crate) fn record_validation(
        &mut self,
        candidate: &CandidateId,
        report: ValidationReport,
        staged: StagedSuccession,
    ) -> Result<(), WorkflowEvolutionError> {
        let entry =
            self.entry_mut(candidate)
                .ok_or_else(|| WorkflowEvolutionError::CandidateNotFound {
                    candidate: candidate.to_string(),
                })?;
        entry.validation = Some(report);
        entry.staged = Some(staged);
        Ok(())
    }

    /// Records an approval decision on the entry.
    pub(crate) fn record_approval(
        &mut self,
        candidate: &CandidateId,
        approval: crate::ApprovalRecord,
    ) -> Result<(), WorkflowEvolutionError> {
        let entry =
            self.entry_mut(candidate)
                .ok_or_else(|| WorkflowEvolutionError::CandidateNotFound {
                    candidate: candidate.to_string(),
                })?;
        entry.approval = Some(approval);
        Ok(())
    }

    /// Records the promotion outcome on the entry.
    pub(crate) fn record_promotion(
        &mut self,
        candidate: &CandidateId,
        successor: WorkflowVersionId,
    ) -> Result<(), WorkflowEvolutionError> {
        let entry =
            self.entry_mut(candidate)
                .ok_or_else(|| WorkflowEvolutionError::CandidateNotFound {
                    candidate: candidate.to_string(),
                })?;
        entry.outcome = CandidateOutcome::Promoted { successor };
        Ok(())
    }
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
