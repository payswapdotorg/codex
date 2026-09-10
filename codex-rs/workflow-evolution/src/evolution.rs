//! The evolution governor: governed publication, lineage, and rollback.
//!
//! [`EvolutionGovernor`] composes the learning plane into one governed
//! lifecycle over a single installed workflow:
//!
//! ```text
//! record candidate (scrub + retention gates)
//!   -> validate (mint revision, seal successor, replay, differential, policy)
//!   -> decide approval (explicit human/policy decision only)
//!   -> publish successor (forge release + immutable lineage record)
//!   -> upgrade installation (explicit, reviewable update plan + decision)
//!   -> rollback (lineage-referenced predecessor, same reviewable path)
//! ```
//!
//! # Invariants enforced here
//!
//! - **The predecessor is never mutated.** Every operation that reads the
//!   incumbent re-verifies its integrity; publication only ever *adds* a
//!   successor version, a release, and a lineage record. There is no API
//!   path that writes to a published version record.
//! - **No promotion without explicit approval.** Publishing requires a
//!   recorded approval that binds to this candidate and this validation
//!   digest; failed or missing validation and rejected or stale approvals
//!   all refuse publication.
//! - **Rollback is explicit and recorded.** Moving between versions —
//!   forward or back — goes through the forge's reviewable update
//!   (propose + decide), never a silent write.
//! - **Lineage is append-only history.** Every successor records its
//!   predecessor, the candidate, the candidate's provenance, the
//!   validation digest and stage summaries, the approval, and any runtime
//!   adjustment the successor carries.
//! - **Runtime adjustments ride the version lineage.** Recovery-policy and
//!   schedule candidates publish a successor version with the incumbent's
//!   content (a governed re-release) and carry the adjustment in lineage;
//!   the effective adjustment always derives from the currently installed
//!   version, so a version rollback uniformly rolls the adjustment back
//!   too.
//!
//! The governor holds in-memory state (forge, releases, install registry,
//! known version records, ledger, lineage). Durable hosts implement the
//! ports and own these records the same way the workflow application owns
//! its seams.

use std::collections::BTreeMap;

use codex_execution_contracts::BindingPolicy;
use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRelease;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::InMemoryForge;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_forge::UpdateDecision;
use codex_workflow_forge::WorkflowUpdateRecord;
use codex_workflow_forge::publish_release;
use codex_workflow_triggers::ScheduleSpec;
use serde::Deserialize;
use serde::Serialize;

use crate::ApprovalPort;
use crate::ApprovalRecord;
use crate::ApprovalRequest;
use crate::CandidateId;
use crate::CandidateProvenance;
use crate::ImprovementCandidate;
use crate::ProposedChange;
use crate::RetentionPolicy;
use crate::StageSummary;
use crate::WorkflowEvolutionError;
use crate::ledger::EvolutionLedger;
use crate::scrub;
use crate::validate::PolicyPort;
use crate::validate::ReplayPort;
use crate::validate::ValidationPipeline;
use crate::validate::ValidationReport;

/// The commit author the governor mints successor revisions under.
pub const EVOLUTION_AUTHOR: &str = "workflow-evolution";

/// A runtime adjustment carried by a successor version's lineage: the
/// control-plane setting future runs or schedules should apply.
///
/// Adjustments are never workflow semantics: the successor version's
/// definition and lock stay exactly the incumbent's, and the adjustment
/// takes effect only through the control plane reading the lineage of the
/// version it has installed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum AppliedAdjustment {
    /// The binding policy future runs should apply.
    Recovery {
        /// The successor binding policy.
        policy: BindingPolicy,
    },
    /// The trigger schedule the control plane should install.
    Schedule {
        /// The successor schedule specification.
        spec: ScheduleSpec,
    },
}

/// The immutable lineage record of one governed succession: predecessor to
/// successor, with the full decision trail.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLineage {
    /// The workflow that evolved.
    pub workflow: WorkflowDefinitionId,
    /// The installed predecessor version the successor evolved from.
    pub predecessor: PublishedVersionRef,
    /// The published successor version.
    pub successor: PublishedVersionRef,
    /// The candidate that proposed the change.
    pub candidate: CandidateId,
    /// The candidate's evidence provenance, carried into the record.
    pub provenance: CandidateProvenance,
    /// The digest of the validation report that gated the succession.
    pub validation_digest: ContentDigest,
    /// The validation stage summaries.
    pub validation_stages: Vec<StageSummary>,
    /// The approval that authorized the succession.
    pub approval: ApprovalRecord,
    /// The runtime adjustment the successor carries, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment: Option<AppliedAdjustment>,
    /// The release tag the successor was published under.
    pub release_tag: String,
}

/// The outcome of publishing a successor version.
#[derive(Clone, Debug)]
pub struct SuccessorPublication {
    /// The release that pins the successor.
    pub release: WorkflowRelease,
    /// The successor's published-version reference.
    pub successor: PublishedVersionRef,
    /// The lineage record connecting predecessor to successor.
    pub lineage: VersionLineage,
}

/// The governed-evolution facade over one installed workflow.
pub struct EvolutionGovernor<R: ReplayPort, P: PolicyPort> {
    workflow: WorkflowDefinitionId,
    repository: codex_workflow_contracts::WorkflowRepositoryId,
    branch: DevelopmentRef,
    forge: InMemoryForge,
    releases: Vec<WorkflowRelease>,
    installs: InstallRegistry,
    versions: BTreeMap<WorkflowVersionId, WorkflowVersion>,
    pipeline: ValidationPipeline<R, P>,
    approvals: Box<dyn ApprovalPort>,
    ledger: EvolutionLedger,
    lineages: Vec<VersionLineage>,
}

impl<R: ReplayPort, P: PolicyPort> EvolutionGovernor<R, P> {
    /// Creates the governor over one installed incumbent.
    ///
    /// The incumbent must verify end to end and must be exactly what
    /// `installs` has installed; the governor mints successor revisions
    /// on a simulated forge seeded for the incumbent's repository.
    pub fn over_incumbent(
        incumbent: WorkflowVersion,
        installs: InstallRegistry,
        replay: R,
        policy: P,
        approvals: Box<dyn ApprovalPort>,
        retention: RetentionPolicy,
    ) -> Result<Self, WorkflowEvolutionError> {
        incumbent.verify_integrity()?;
        let workflow = incumbent.definition.id.clone();
        let installed = installs.installed(&workflow).ok_or_else(|| {
            WorkflowEvolutionError::IncumbentNotInstalled {
                workflow: workflow.clone(),
            }
        })?;
        if installed.installed.version_id != incumbent.version_id {
            return Err(WorkflowEvolutionError::IncumbentNotInstalled {
                workflow: workflow.clone(),
            });
        }
        let repository = incumbent.identity.repository.clone();
        let branch = DevelopmentRef::branch("main")?;
        let mut forge = InMemoryForge::new(ForgeKind::parse("github")?);
        forge.seed_repository(
            repository.clone(),
            branch.clone(),
            vec![Attribution {
                name: EVOLUTION_AUTHOR.to_string(),
                contact: None,
            }],
            Vec::new(),
        )?;
        let mut versions = BTreeMap::new();
        versions.insert(incumbent.version_id.clone(), incumbent);
        Ok(Self {
            workflow,
            repository,
            branch,
            forge,
            releases: Vec::new(),
            installs,
            versions,
            pipeline: ValidationPipeline::new(replay, policy),
            approvals,
            ledger: EvolutionLedger::new(retention)?,
            lineages: Vec::new(),
        })
    }

    /// The workflow this governor evolves.
    pub fn workflow(&self) -> &WorkflowDefinitionId {
        &self.workflow
    }

    /// The currently installed incumbent version record, re-verified.
    pub fn incumbent(&self) -> Result<&WorkflowVersion, WorkflowEvolutionError> {
        let installed = self.installs.installed(&self.workflow).ok_or_else(|| {
            WorkflowEvolutionError::IncumbentNotInstalled {
                workflow: self.workflow.clone(),
            }
        })?;
        let version = self
            .versions
            .get(&installed.installed.version_id)
            .ok_or_else(|| WorkflowEvolutionError::IncumbentNotInstalled {
                workflow: self.workflow.clone(),
            })?;
        version.verify_integrity()?;
        Ok(version)
    }

    /// The retention-bounded candidate ledger.
    pub fn ledger(&self) -> &EvolutionLedger {
        &self.ledger
    }

    /// The immutable lineage records, oldest first.
    pub fn lineages(&self) -> &[VersionLineage] {
        &self.lineages
    }

    /// The version record this governor knows for `version`, when any.
    ///
    /// Known records are the construction incumbent and every published
    /// successor; each still verifies when read back.
    pub fn version_record(
        &self,
        version: &WorkflowVersionId,
    ) -> Result<Option<&WorkflowVersion>, WorkflowEvolutionError> {
        match self.versions.get(version) {
            Some(record) => {
                record.verify_integrity()?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// The releases published through this governor, oldest first.
    pub fn releases(&self) -> &[WorkflowRelease] {
        &self.releases
    }

    /// The install registry backing upgrade and rollback transitions.
    pub fn install_registry(&self) -> &InstallRegistry {
        &self.installs
    }

    /// Records one improvement candidate.
    ///
    /// The ledger enforces the credential and retention gates; nothing is
    /// validated or approved yet.
    pub fn record_candidate(
        &mut self,
        candidate: ImprovementCandidate,
    ) -> Result<(), WorkflowEvolutionError> {
        if *candidate.workflow() != self.workflow {
            return Err(WorkflowEvolutionError::InvalidCandidate {
                candidate: candidate.id.to_string(),
                reason: format!(
                    "candidate evolves `{}`, not this governor's workflow `{}`",
                    candidate.workflow(),
                    self.workflow
                ),
            });
        }
        self.ledger.record(candidate)
    }

    /// Validates one recorded candidate for a proposed successor version.
    ///
    /// Steps: re-check freshness against the installed incumbent, apply
    /// the change, refuse no-op evolutions, mint an immutable revision on
    /// the forge, seal the successor version at that revision, then run
    /// the three-stage pipeline (replay, differential, policy). The report
    /// and the staged succession are recorded whether or not the gates
    /// passed — failed validation is governance evidence too.
    ///
    /// Re-validating a candidate mints a fresh revision and therefore a
    /// fresh successor identity: prior approvals (which bind to the
    /// validation digest) no longer match and must be re-decided.
    pub async fn validate_candidate(
        &mut self,
        candidate: &CandidateId,
        successor: &SemanticVersion,
    ) -> Result<ValidationReport, WorkflowEvolutionError> {
        let entry = self.ledger.entry(candidate).ok_or_else(|| {
            WorkflowEvolutionError::CandidateNotFound {
                candidate: candidate.to_string(),
            }
        })?;
        if entry.outcome != crate::CandidateOutcome::Pending {
            return Err(WorkflowEvolutionError::CandidateAlreadyPromoted {
                candidate: candidate.to_string(),
            });
        }
        let recorded = entry.candidate.clone();
        let incumbent = self.incumbent()?.clone();
        if recorded.incumbent.version_id != incumbent.version_id {
            return Err(WorkflowEvolutionError::StaleCandidate {
                candidate: candidate.to_string(),
                expected: recorded.incumbent.version_id.clone(),
                actual: incumbent.version_id.clone(),
            });
        }
        let (definition, lock) = recorded.applied_over(&incumbent)?;
        if recorded.change_kind().affects_version_content()
            && definition == incumbent.definition
            && lock == incumbent.dependency_lock
        {
            return Err(WorkflowEvolutionError::NoopEvolution {
                candidate: candidate.to_string(),
            });
        }
        // Mint the immutable revision the successor anchors at: the
        // development-side commit that carries the candidate. Publication
        // (the release) is the governed act and happens only after
        // approval.
        let commit = self.forge.commit(
            &self.repository,
            &self.branch,
            format!("evolve: {candidate}"),
            EVOLUTION_AUTHOR,
        )?;
        let version = WorkflowVersion::seal(
            definition,
            self.repository.clone(),
            commit.revision.clone(),
            successor.clone(),
            lock,
            incumbent.provenance.clone(),
        )?;
        let reference = PublishedVersionRef::of(&version);
        let report = self
            .pipeline
            .validate(&recorded, &incumbent, &version)
            .await;
        let report = report?;
        self.ledger.record_validation(
            candidate,
            report.clone(),
            crate::StagedSuccession {
                revision: commit.revision,
                successor_version: successor.clone(),
                version,
                reference,
            },
        )?;
        Ok(report)
    }

    /// Builds the approval request for a validated candidate.
    ///
    /// Only candidates whose validation passed explicitly are
    /// approvable: the request binds to the validation digest of the
    /// staged successor.
    pub fn request_approval(
        &self,
        candidate: &CandidateId,
    ) -> Result<ApprovalRequest, WorkflowEvolutionError> {
        let entry = self.passed_entry(candidate)?;
        let staged =
            entry
                .staged
                .as_ref()
                .ok_or_else(|| WorkflowEvolutionError::ValidationNotRun {
                    candidate: candidate.to_string(),
                })?;
        let report =
            entry
                .validation
                .as_ref()
                .ok_or_else(|| WorkflowEvolutionError::ValidationNotRun {
                    candidate: candidate.to_string(),
                })?;
        Ok(ApprovalRequest {
            candidate: candidate.clone(),
            workflow: self.workflow.clone(),
            incumbent: self.incumbent()?.version_id.clone(),
            validation_digest: report.digest(&staged.reference)?,
        })
    }

    /// Records an explicit approval decision for a validated candidate.
    ///
    /// The decision crosses the approval port (the only authorization
    /// path); its note text is scrubbed by the port before recording.
    pub fn decide_approval(
        &mut self,
        candidate: &CandidateId,
        decision: crate::ApprovalDecision,
    ) -> Result<ApprovalRecord, WorkflowEvolutionError> {
        let request = self.request_approval(candidate)?;
        let record = self.approvals.decide(&request, decision)?;
        self.ledger.record_approval(candidate, record.clone())?;
        Ok(record)
    }

    /// Publishes a validated and approved candidate as a successor
    /// version.
    ///
    /// Steps: refuse unless the validation passed and the approval
    /// matches (candidate, validation digest, incumbent); cut the release
    /// through the forge at the staged revision; record the immutable
    /// lineage (predecessor, successor, candidate provenance, validation
    /// evidence, approval, adjustment); mark the candidate promoted.
    ///
    /// The predecessor is untouched: publication only adds records. The
    /// installation is not moved either — that is the separate,
    /// reviewable [`Self::upgrade_installation`] transition.
    pub fn publish_successor(
        &mut self,
        candidate: &CandidateId,
        release_tag: &str,
    ) -> Result<SuccessorPublication, WorkflowEvolutionError> {
        let entry = self.passed_entry(candidate)?;
        let staged =
            entry
                .staged
                .clone()
                .ok_or_else(|| WorkflowEvolutionError::ValidationNotRun {
                    candidate: candidate.to_string(),
                })?;
        let report =
            entry
                .validation
                .clone()
                .ok_or_else(|| WorkflowEvolutionError::ValidationNotRun {
                    candidate: candidate.to_string(),
                })?;
        let approval =
            entry
                .approval
                .clone()
                .ok_or_else(|| WorkflowEvolutionError::ApprovalRequired {
                    candidate: candidate.to_string(),
                })?;
        if !approval.approves() {
            return Err(WorkflowEvolutionError::ApprovalRejected {
                candidate: candidate.to_string(),
                approver: approval.decision.approver().to_string(),
            });
        }
        if approval.request.candidate != *candidate
            || approval.request.validation_digest != report.digest(&staged.reference)?
        {
            return Err(WorkflowEvolutionError::ApprovalMismatch {
                candidate: candidate.to_string(),
            });
        }
        let incumbent = self.incumbent()?;
        if approval.request.incumbent != incumbent.version_id {
            return Err(WorkflowEvolutionError::ApprovalMismatch {
                candidate: candidate.to_string(),
            });
        }
        let predecessor = PublishedVersionRef::of(incumbent);
        let release = publish_release(
            &self.repository,
            &self.releases,
            release_tag,
            &staged.revision,
            std::slice::from_ref(&staged.reference),
        )?;
        if self
            .lineages
            .iter()
            .any(|lineage| lineage.successor.version_id == staged.reference.version_id)
        {
            return Err(WorkflowEvolutionError::LineageAlreadyRecorded {
                successor: staged.reference.version_id,
            });
        }
        let lineage = VersionLineage {
            workflow: self.workflow.clone(),
            predecessor,
            successor: staged.reference.clone(),
            candidate: candidate.clone(),
            provenance: entry.candidate.provenance.clone(),
            validation_digest: report.digest(&staged.reference)?,
            validation_stages: report.summaries(),
            approval,
            adjustment: adjustment_of(&entry.candidate.change),
            release_tag: release_tag.to_string(),
        };
        self.releases.push(release.clone());
        self.lineages.push(lineage.clone());
        self.versions
            .insert(staged.reference.version_id.clone(), staged.version.clone());
        self.ledger
            .record_promotion(candidate, staged.reference.version_id.clone())?;
        Ok(SuccessorPublication {
            release,
            successor: staged.reference,
            lineage,
        })
    }

    /// Moves the installation to a promoted candidate's successor version.
    ///
    /// The upgrade is an explicit, reviewable transition through the forge
    /// install registry (propose + decide); applying it changes which
    /// version *future* instances use, and the decision is recorded in
    /// the registry's history either way.
    pub fn upgrade_installation(
        &mut self,
        candidate: &CandidateId,
        decision: UpdateDecision,
        note: Option<String>,
    ) -> Result<WorkflowUpdateRecord, WorkflowEvolutionError> {
        let entry = self.ledger.entry(candidate).ok_or_else(|| {
            WorkflowEvolutionError::CandidateNotFound {
                candidate: candidate.to_string(),
            }
        })?;
        let successor = match &entry.outcome {
            crate::CandidateOutcome::Promoted { successor } => successor.clone(),
            crate::CandidateOutcome::Pending => {
                return Err(WorkflowEvolutionError::CandidateNotPromoted {
                    candidate: candidate.to_string(),
                });
            }
        };
        if let Some(note) = &note {
            scrub::require_clean_text(note, "upgrade note")?;
        }
        let successor_ref = self
            .lineages
            .iter()
            .find(|lineage| lineage.successor.version_id == successor)
            .map(|lineage| lineage.successor.clone())
            .ok_or_else(|| WorkflowEvolutionError::CandidateNotPromoted {
                candidate: candidate.to_string(),
            })?;
        let plan = self
            .installs
            .propose_update(&self.workflow, &successor_ref, note)?;
        Ok(self.installs.decide_update(&plan, decision)?)
    }

    /// Rolls the installation back to the current version's recorded
    /// predecessor.
    ///
    /// Rollback is the same explicit, reviewable transition as an
    /// upgrade: the predecessor comes from lineage (never a guess), the
    /// move goes through propose + decide, and both the decision and the
    /// from/to identities land in the registry's history. A rejected
    /// rollback keeps the installation and is recorded too.
    pub fn rollback_to_predecessor(
        &mut self,
        decision: UpdateDecision,
        note: Option<String>,
    ) -> Result<WorkflowUpdateRecord, WorkflowEvolutionError> {
        let incumbent = self.incumbent()?;
        let lineage = self
            .lineages
            .iter()
            .rev()
            .find(|lineage| lineage.successor.version_id == incumbent.version_id)
            .ok_or_else(|| WorkflowEvolutionError::NoPredecessorToRollback {
                workflow: self.workflow.clone(),
            })?;
        let predecessor = lineage.predecessor.clone();
        if let Some(note) = &note {
            scrub::require_clean_text(note, "rollback note")?;
        }
        let plan = self
            .installs
            .propose_update(&self.workflow, &predecessor, note)?;
        Ok(self.installs.decide_update(&plan, decision)?)
    }

    /// The runtime adjustment effective for the currently installed
    /// version, when any.
    ///
    /// The effective adjustment is whatever lineage the installed version
    /// was published with — so upgrading applies an adjustment and
    /// rolling back removes it, uniformly and recorded.
    pub fn effective_adjustment(
        &self,
    ) -> Result<Option<AppliedAdjustment>, WorkflowEvolutionError> {
        let incumbent = self.incumbent()?;
        let adjustment = self
            .lineages
            .iter()
            .rev()
            .find(|lineage| lineage.successor.version_id == incumbent.version_id)
            .and_then(|lineage| lineage.adjustment.clone());
        Ok(adjustment)
    }

    /// The ledger entry of a validated-and-passed candidate.
    fn passed_entry(
        &self,
        candidate: &CandidateId,
    ) -> Result<&crate::LedgerEntry, WorkflowEvolutionError> {
        let entry = self.ledger.entry(candidate).ok_or_else(|| {
            WorkflowEvolutionError::CandidateNotFound {
                candidate: candidate.to_string(),
            }
        })?;
        let report =
            entry
                .validation
                .as_ref()
                .ok_or_else(|| WorkflowEvolutionError::ValidationNotRun {
                    candidate: candidate.to_string(),
                })?;
        if !report.passed() {
            return Err(WorkflowEvolutionError::ValidationFailed {
                candidate: candidate.to_string(),
                failed: report.failed_stages().to_vec().join(", "),
            });
        }
        Ok(entry)
    }
}

/// Projects a proposed change into the runtime adjustment its successor
/// lineage carries.
fn adjustment_of(change: &ProposedChange) -> Option<AppliedAdjustment> {
    match change {
        ProposedChange::Recovery { policy } => Some(AppliedAdjustment::Recovery {
            policy: policy.clone(),
        }),
        ProposedChange::Schedule { spec } => {
            Some(AppliedAdjustment::Schedule { spec: spec.clone() })
        }
        _ => None,
    }
}
