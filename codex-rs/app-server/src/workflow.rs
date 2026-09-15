//! The workflow control plane service (RWO-001).
//!
//! [`WorkflowControlPlane`] mounts the workflow family's existing engine
//! ports behind one service shared by the app-server handlers and the CLI
//! workflow subcommand group. It owns no engine semantics: teaching,
//! compilation, validation, simulation, approval, publication, and the
//! durable instance lifecycle all delegate to the frozen crates
//! (`codex-teaching-compiler`, `codex-workflow-app`,
//! `codex-workflow-durable`, `codex-workflow-contracts`). The fork path
//! (RWO-005) likewise delegates to the frozen distribution port's `fork`:
//! the derived release is the engine's sealed record, never a mount-side
//! re-derivation.
//!
//! State layout, following the RWO-001 bound of in-memory doubles first:
//!
//! - Teaching sessions and compiled candidates live in process memory
//!   (the in-memory double policy): teaching is one process's work, and
//!   the durable teaching artifacts are the immutable versions that
//!   publication seals.
//! - Workflow versions, instances, evidence, and run positions live in the
//!   file-backed durable stores from `codex-workflow-durable` (MWO-001)
//!   under the control-plane root, so instances survive kill/restart.
//!
//! Ordinary Codex compatibility: nothing here executes unless a workflow
//! method is called; the durable root is opened lazily on first use.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use codex_app_server_protocol as rpc;
use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::CandidateOrigin;
use codex_teaching_compiler::CandidateStatus;
use codex_teaching_compiler::RecordOrigin;
use codex_teaching_compiler::Severity;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::SimulationOutcome;
use codex_teaching_compiler::SimulationReport;
use codex_teaching_compiler::StepOrigin;
use codex_teaching_compiler::TeachingEvidence;
use codex_teaching_compiler::TeachingMode;
use codex_teaching_compiler::TeachingSession;
use codex_teaching_compiler::TrajectoryEvent;
use codex_teaching_compiler::ValidationSummary;
use codex_teaching_compiler::WorkflowCandidate;
use codex_teaching_compiler::compile;
use codex_workflow_app::LifecycleDeps;
use codex_workflow_app::PublishRequest;
use codex_workflow_app::RecordingEventSink;
use codex_workflow_app::RunPositionStore;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::ScriptedActionSource;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::WorkflowInstanceStore;
use codex_workflow_app::WorkflowLifecycle;
use codex_workflow_app::WorkflowVersionStore;
use codex_workflow_app::publish;
use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::TriggerSource;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowInstance;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_distribution::CompatibilitySpec;
use codex_workflow_distribution::DistributionPort;
use codex_workflow_distribution::ForkRequest;
use codex_workflow_distribution::InMemoryAccessPolicy;
use codex_workflow_distribution::InMemoryEntitlements;
use codex_workflow_distribution::InMemoryMarketplace;
use codex_workflow_distribution::LicenseTerms;
use codex_workflow_distribution::MarketplacePrincipal;
use codex_workflow_distribution::PublicationMetadata;
use codex_workflow_distribution::PublicationScope;
use codex_workflow_distribution::PublishSubmission;
use codex_workflow_distribution::SourceLineage;
use codex_workflow_distribution::UpgradePolicySetting;
use codex_workflow_distribution::WorkflowDistributionError;
use codex_workflow_durable::DurableRunPositionStore;
use codex_workflow_durable::DurableStores;
use codex_workflow_forge::PublishedVersionRef;
use codex_workflow_triggers::InstanceControl;
use codex_workflow_triggers::ResumeDirective;
use codex_workflow_triggers::WorkflowTriggerError;
use thiserror::Error;
use uuid::Uuid;

use crate::workflow::approval::DurableBackedApprovalSource;

mod approval;

/// The default workflow name when a teaching session does not provide one.
pub const DEFAULT_WORKFLOW_NAME: &str = "taught-workflow";
/// The default canonical repository identity when publish does not provide one.
pub const DEFAULT_REPOSITORY: &str = "local/workflows/taught";
/// The default semantic version when publish does not provide one.
pub const DEFAULT_SEMANTIC_VERSION: &str = "1.0.0";
/// The default attribution label the mount's approval source records.
const MOUNT_APPROVER: &str = "workflow-mount";
/// The default owning principal of a fork made through the mount when the
/// caller names none (opaque, credential-free).
const DEFAULT_FORK_OWNER: &str = "cli-user";
/// The default license identifier recorded on a fork made through the
/// mount when the caller declares none: an explicit custom-license marker
/// that makes no claim about the upstream's terms.
const DEFAULT_FORK_LICENSE: &str = "LicenseRef-Unspecified";

/// Errors surfaced by the workflow control plane mount.
#[derive(Debug, Error)]
pub enum WorkflowControlPlaneError {
    /// A control-plane record was not found.
    #[error("{0}")]
    NotFound(String),
    /// The request is invalid for the current lifecycle state.
    #[error("{0}")]
    InvalidRequest(String),
    /// An engine or store operation failed.
    #[error("workflow engine error: {0}")]
    Engine(#[from] WorkflowAppError),
    /// A teaching-session or compiler operation failed.
    #[error("teaching compiler error: {0}")]
    Teaching(#[from] codex_teaching_compiler::TeachingCompilerError),
    /// A trigger-plane operation failed.
    #[error("workflow trigger error: {0}")]
    Trigger(#[from] WorkflowTriggerError),
    /// A durable store operation failed.
    #[error("workflow durable store error: {0}")]
    Durable(#[from] codex_workflow_durable::DurableStoreError),
    /// A distribution-plane operation failed. Refusals (an unknown
    /// upstream, an invalid fork record such as empty carried
    /// attribution, or an already-published identity) surface as request
    /// errors; integrity failures surface as internal errors.
    #[error("workflow distribution error: {0}")]
    Distribution(#[from] WorkflowDistributionError),
}

/// The in-memory state of the control plane mount.
#[derive(Default)]
struct WorkflowState {
    sessions: BTreeMap<String, TeachingSessionEntry>,
    candidates: BTreeMap<String, CandidateEntry>,
    stores: Option<Arc<DurableStores>>,
    reconciled: bool,
}

/// One open or closed teaching session.
struct TeachingSessionEntry {
    name: String,
    session: TeachingSession,
}

/// One compiled workflow candidate.
struct CandidateEntry {
    name: String,
    candidate: WorkflowCandidate,
}

/// The workflow teaching and control-plane service mounted behind
/// user-facing surfaces.
///
/// All methods are additive delegations to the frozen engine ports. The
/// durable stores under `root` are opened lazily on first use and cached
/// for the process lifetime (the stores are shared-state handles over the
/// same files; one handle per process preserves the single-writer
/// contract). The read path re-opens the root first (see
/// [`Self::refresh_stores`]): a list or get must observe the store's
/// current content — including records another control-plane process
/// wrote after this plane's handle was opened — and the one-shot startup
/// sweep runs over that refreshed view.
pub struct WorkflowControlPlane {
    root: PathBuf,
    state: Mutex<WorkflowState>,
}

impl WorkflowControlPlane {
    /// Creates the control plane over the durable root directory `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            state: Mutex::new(WorkflowState::default()),
        }
    }

    /// The durable root directory the control plane mounts.
    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    /// Opens a teaching session in one of the three teaching modes.
    pub fn teach_start(
        &self,
        params: rpc::WorkflowTeachStartParams,
    ) -> Result<rpc::WorkflowTeachStartResponse, WorkflowControlPlaneError> {
        let mode = teach_mode(params.mode);
        let name = params
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKFLOW_NAME.to_string());
        WorkflowDefinitionId::parse(name.clone()).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!(
                "invalid workflow name `{name}`: {error}"
            ))
        })?;
        let session_id = format!("ws-{}", Uuid::new_v4().simple());
        let response = rpc::WorkflowTeachStartResponse {
            session_id: session_id.clone(),
            name: name.clone(),
            mode: params.mode,
            status: rpc::WorkflowTeachSessionStatus::Open,
            record_count: 0,
        };
        let mut state = self.lock();
        state.sessions.insert(
            session_id,
            TeachingSessionEntry {
                name,
                session: TeachingSession::new(mode),
            },
        );
        Ok(response)
    }

    /// Records one instruction statement into a teaching session.
    pub fn teach_instruct(
        &self,
        params: rpc::WorkflowTeachInstructParams,
    ) -> Result<rpc::WorkflowTeachRecordResponse, WorkflowControlPlaneError> {
        self.teach_record(
            &params.session_id,
            RecordOrigin::Instruction,
            TrajectoryEvent::Instruction { text: params.text },
            params.evidence,
        )
    }

    /// Records one demonstration event into a teaching session.
    pub fn teach_demonstrate(
        &self,
        params: rpc::WorkflowTeachDemonstrateParams,
    ) -> Result<rpc::WorkflowTeachRecordResponse, WorkflowControlPlaneError> {
        let event = match params.kind {
            rpc::WorkflowDemonstrationKind::Observation => {
                TrajectoryEvent::Observation { text: params.text }
            }
            rpc::WorkflowDemonstrationKind::Action => TrajectoryEvent::Action { text: params.text },
            rpc::WorkflowDemonstrationKind::Result => TrajectoryEvent::Result { text: params.text },
            rpc::WorkflowDemonstrationKind::Recovery => {
                TrajectoryEvent::Recovery { text: params.text }
            }
        };
        self.teach_record(
            &params.session_id,
            RecordOrigin::Demonstration,
            event,
            params.evidence,
        )
    }

    /// Records one trajectory event through the engine session.
    fn teach_record(
        &self,
        session_id: &str,
        origin: RecordOrigin,
        event: TrajectoryEvent,
        evidence: Vec<rpc::WorkflowTeachEvidenceInput>,
    ) -> Result<rpc::WorkflowTeachRecordResponse, WorkflowControlPlaneError> {
        let evidence: Vec<TeachingEvidence> = evidence
            .into_iter()
            .map(|input| {
                TeachingEvidence::new(input.label, input.locator, input.sha256).map_err(|error| {
                    WorkflowControlPlaneError::InvalidRequest(format!(
                        "invalid teaching evidence: {error}"
                    ))
                })
            })
            .collect::<Result<_, _>>()?;
        let mut state = self.lock();
        let entry = session_mut(&mut state, session_id)?;
        let mode = entry.session.mode();
        let sequence = entry
            .session
            .record(origin, event, evidence)
            .map_err(WorkflowControlPlaneError::Teaching)?;
        Ok(rpc::WorkflowTeachRecordResponse {
            session_id: session_id.to_string(),
            mode: teach_mode_protocol(mode),
            status: rpc::WorkflowTeachSessionStatus::Open,
            sequence,
            record_count: entry.session.len() as u64,
        })
    }

    /// Closes a teaching session and reports its reconciled trajectory.
    pub fn teach_reconcile(
        &self,
        params: rpc::WorkflowTeachReconcileParams,
    ) -> Result<rpc::WorkflowTeachReconcileResponse, WorkflowControlPlaneError> {
        let mut state = self.lock();
        let entry = session_mut(&mut state, &params.session_id)?;
        entry.session.close();
        let mode = entry.session.mode();
        let demonstration_records = entry
            .session
            .records()
            .iter()
            .filter(|record| record.origin() == RecordOrigin::Demonstration)
            .count() as u64;
        let instruction_records = entry
            .session
            .records()
            .iter()
            .filter(|record| record.origin() == RecordOrigin::Instruction)
            .count() as u64;
        Ok(rpc::WorkflowTeachReconcileResponse {
            session_id: params.session_id,
            mode: teach_mode_protocol(mode),
            status: rpc::WorkflowTeachSessionStatus::Closed,
            demonstration_records,
            instruction_records,
        })
    }

    /// Compiles a closed session into a validated candidate.
    ///
    /// Delegates to the teaching compiler, then runs the engine's
    /// validation and deterministic simulation passes so the candidate is
    /// review-ready (`Validated`).
    pub fn compile(
        &self,
        params: rpc::WorkflowCompileParams,
    ) -> Result<rpc::WorkflowCompileResponse, WorkflowControlPlaneError> {
        let mut state = self.lock();
        let entry = session_mut(&mut state, &params.session_id)?;
        if !entry.session.is_closed() {
            return Err(WorkflowControlPlaneError::InvalidRequest(format!(
                "teaching session `{}` must be reconciled (closed) before compiling",
                params.session_id
            )));
        }
        let name = entry.name.clone();
        // The reconcile merge policy applies before compilation: the
        // teaching compiler preserves the trajectory's recording order
        // exactly, so a hybrid session is compiled through its
        // reconciled view — every observed (demonstrated) step before
        // every instructed one, the VWO-010 Family A reconciled hybrid
        // origins ["observed", "instructed"]. Demonstrate and Instruct
        // sessions hold one record origin only, so their recording
        // order already is the reconciled order.
        let candidate_session = if entry.session.mode() == TeachingMode::Hybrid {
            reconciled_hybrid_session(&entry.session)?
        } else {
            entry.session.clone()
        };
        let mut candidate =
            compile(&candidate_session).map_err(WorkflowControlPlaneError::Teaching)?;
        let validation = candidate
            .validate()
            .map_err(WorkflowControlPlaneError::Teaching)?;
        let simulation = candidate
            .simulate(SimulationConfig::default())
            .map_err(WorkflowControlPlaneError::Teaching)?;
        let candidate_id = format!("cand-{}", Uuid::new_v4().simple());
        let response = rpc::WorkflowCompileResponse {
            candidate_id: candidate_id.clone(),
            status: candidate_status_protocol(candidate.status()),
            origin: candidate_origin_protocol(candidate.origin()),
            epoch: candidate.epoch(),
            step_count: candidate.ir().nodes.len() as u64,
            validation: validation_summary_protocol(&validation),
            simulation: simulation_summary_protocol(&simulation),
        };
        state
            .candidates
            .insert(candidate_id, CandidateEntry { name, candidate });
        Ok(response)
    }

    /// Reads the full review payload of a compiled candidate.
    pub fn review(
        &self,
        params: rpc::WorkflowReviewParams,
    ) -> Result<rpc::WorkflowReviewResponse, WorkflowControlPlaneError> {
        let state = self.lock();
        let entry = candidate_ref(&state, &params.candidate_id)?;
        review_response(&params.candidate_id, entry)
    }

    /// Records an approval decision on a candidate.
    ///
    /// An approval advances the candidate through the engine gate; a
    /// rejection records no engine approval, leaving the candidate
    /// unapproved and re-reviewable.
    pub fn approve(
        &self,
        params: rpc::WorkflowApproveParams,
    ) -> Result<rpc::WorkflowApproveResponse, WorkflowControlPlaneError> {
        let mut state = self.lock();
        let entry = candidate_mut(&mut state, &params.candidate_id)?;
        if params.decision == rpc::WorkflowApprovalDecision::Approved {
            let decision = ApprovalDecision::new(
                params.approver.clone(),
                params.reference.clone(),
                ApprovalDecisionKind::Approved,
            )
            .map_err(|error| {
                WorkflowControlPlaneError::InvalidRequest(format!(
                    "invalid approval decision: {error}"
                ))
            })?;
            entry
                .candidate
                .approve(decision)
                .map_err(WorkflowControlPlaneError::Teaching)?;
        }
        Ok(rpc::WorkflowApproveResponse {
            candidate_id: params.candidate_id,
            status: candidate_status_protocol(entry.candidate.status()),
            epoch: entry.candidate.epoch(),
            decision: params.decision,
            approver: params.approver,
            reference: params.reference,
        })
    }

    /// Publishes an approved candidate as an immutable workflow version.
    ///
    /// The workflow identity inputs (repository, commit sha, semantic
    /// version) are validated first, independently of the engine's
    /// publication state gate: an invalid input is reported as the
    /// input error, and the candidate is left `Approved` for a
    /// corrected retry — the state-consuming finalization never runs
    /// on inputs that would be rejected. The approved content is then
    /// finalized through the teaching compiler, sealed through the
    /// workflow-app publisher, and mirrored into the durable version
    /// store so instances can pin it.
    pub fn publish(
        &self,
        params: rpc::WorkflowPublishParams,
    ) -> Result<rpc::WorkflowPublishResponse, WorkflowControlPlaneError> {
        let stores = self.stores()?;
        let mut state = self.lock();
        let entry = candidate_mut(&mut state, &params.candidate_id)?;
        let repository = params
            .repository
            .filter(|repository| !repository.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_REPOSITORY.to_string());
        let repository = WorkflowRepositoryId::parse(repository).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!(
                "invalid repository identity: {error}"
            ))
        })?;
        let commit_sha = RevisionSha::parse(params.commit_sha).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!("invalid commit sha: {error}"))
        })?;
        let semantic_version = params
            .semantic_version
            .filter(|version| !version.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_SEMANTIC_VERSION.to_string());
        let semantic_version = semver::Version::parse(&semantic_version).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!(
                "invalid semantic version `{semantic_version}`: {error}"
            ))
        })?;
        let definition_id = WorkflowDefinitionId::parse(entry.name.clone()).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!(
                "invalid workflow name `{}`: {error}",
                entry.name
            ))
        })?;
        let approved = entry
            .candidate
            .finalize(definition_id)
            .map_err(WorkflowControlPlaneError::Teaching)?;
        let request = PublishRequest {
            approved,
            repository,
            source_revision: ImmutableSourceRevision::pin_commit(commit_sha),
            semantic_version,
            bindings: BTreeMap::new(),
            dependency_lock: DependencyLock::default(),
            provenance: None,
        };
        let artifact = publish(request).map_err(WorkflowControlPlaneError::Engine)?;
        let mut versions = stores.versions.clone();
        versions
            .publish(artifact.version.clone())
            .map_err(WorkflowControlPlaneError::Engine)?;
        let identity = &artifact.version.identity;
        Ok(rpc::WorkflowPublishResponse {
            workflow: identity.workflow.to_string(),
            version_id: artifact.version.version_id.to_string(),
            semantic_version: identity.semantic_version.to_string(),
            definition_digest: identity.definition_digest.to_string(),
            dependency_lock_digest: identity.dependency_lock_digest.to_string(),
            repository: identity.repository.to_string(),
            commit_sha: identity.source_revision.commit_sha.to_string(),
            binding_resolution: rpc::WorkflowBindingResolution {
                approved_digest: artifact.resolution.approved_digest,
                executable_digest: artifact.resolution.executable_digest,
            },
        })
    }

    /// Forks a published version into a new immutable release whose
    /// lineage pins the upstream (RWO-005).
    ///
    /// The upstream release is loaded from the durable version store and
    /// seeded into a fresh engine marketplace; the distribution port's
    /// existing `fork` (reused as-is, no new fork semantics) derives the
    /// new sealed release — the upstream's frozen definition and
    /// dependency lock re-sealed under the fork's repository, revision,
    /// and version, with provenance pointing back at the upstream. The
    /// mount then mirrors the derived sealed version into the durable
    /// store so it appears as a new immutable release instances can pin,
    /// refusing if that identity was already published: published
    /// releases are never silently overwritten.
    ///
    /// Defaults: the derived release keeps the upstream's semantic
    /// version and commit unless overridden, is owned by
    /// [`DEFAULT_FORK_OWNER`] unless named, carries
    /// [`DEFAULT_FORK_LICENSE`] unless declared, and pins its upgrade
    /// policy (a fork never auto-surfaces upgrade proposals). The
    /// carried attribution is passed through untouched — the engine
    /// refuses a fork whose carried attribution is empty.
    pub fn fork(
        &self,
        params: rpc::WorkflowForkParams,
    ) -> Result<rpc::WorkflowForkResponse, WorkflowControlPlaneError> {
        let stores = self.refresh_stores()?;
        let upstream_id = parse_version_id(&params.version_id)?;
        let upstream = stores.versions.load(&upstream_id)?.ok_or_else(|| {
            WorkflowControlPlaneError::NotFound(format!(
                "unknown published workflow version `{}`",
                params.version_id
            ))
        })?;
        let fork_repository =
            WorkflowRepositoryId::parse(params.fork_repository.clone()).map_err(|error| {
                WorkflowControlPlaneError::InvalidRequest(format!(
                    "invalid fork repository identity: {error}"
                ))
            })?;
        let semantic_version = match &params.semantic_version {
            Some(version) => semver::Version::parse(version).map_err(|error| {
                WorkflowControlPlaneError::InvalidRequest(format!(
                    "invalid semantic version `{version}`: {error}"
                ))
            })?,
            None => upstream.identity.semantic_version.clone(),
        };
        let commit_sha = match &params.commit_sha {
            Some(sha) => RevisionSha::parse(sha.clone()).map_err(|error| {
                WorkflowControlPlaneError::InvalidRequest(format!("invalid commit sha: {error}"))
            })?,
            None => upstream.identity.source_revision.commit_sha.clone(),
        };
        let owner = MarketplacePrincipal::parse(
            params
                .owner
                .clone()
                .unwrap_or_else(|| DEFAULT_FORK_OWNER.to_string()),
        )
        .map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!("invalid fork owner: {error}"))
        })?;
        let licensing = LicenseTerms {
            identifier: params
                .license
                .clone()
                .unwrap_or_else(|| DEFAULT_FORK_LICENSE.to_string()),
            url: None,
            custom_terms_digest: None,
        };
        let carried_attribution = params
            .attribution
            .iter()
            .map(|entry| Attribution {
                name: entry.name.clone(),
                contact: entry.contact.clone(),
            })
            .collect::<Vec<_>>();

        // Drive the engine's distribution port exactly as documented: the
        // upstream must be a marketplace entry, so a fresh in-memory
        // marketplace is seeded with the durable release (the RWO-001
        // bound: the mount's publication path seals versions without
        // distribution metadata), then `fork` derives the new release.
        let mut marketplace = InMemoryMarketplace::new(
            Box::new(InMemoryAccessPolicy::new()),
            Box::new(InMemoryEntitlements::new()),
        );
        marketplace
            .publish(PublishSubmission {
                metadata: seeded_release_metadata(&upstream)?,
                version: upstream.clone(),
                commercial: None,
                scope: PublicationScope::Unlisted,
            })
            .map_err(WorkflowControlPlaneError::Distribution)?;
        let listing = marketplace
            .fork(ForkRequest {
                upstream: PublishedVersionRef::of(&upstream),
                fork_repository,
                fork_revision: ImmutableSourceRevision::pin_commit(commit_sha),
                fork_version: semantic_version,
                owner,
                licensing,
                carried_attribution,
                upgrade_policy: UpgradePolicySetting::Pin,
            })
            .map_err(WorkflowControlPlaneError::Distribution)?;
        let derived_id = listing.metadata.release.version_id.clone();
        let derived = marketplace
            .fetch_version(&listing.metadata.release.identity.workflow, &derived_id)
            .map_err(WorkflowControlPlaneError::Distribution)?
            .ok_or_else(|| {
                WorkflowControlPlaneError::InvalidRequest(
                    "the forked release is missing from the engine marketplace".to_owned(),
                )
            })?;

        // The fork is a NEW immutable release: mirror the sealed version
        // into the durable store, refusing an identity that is already
        // published (the engine refuses the same collision inside one
        // marketplace; the durable store is the mount's record of truth
        // across processes, so the refusal survives restarts).
        if stores.versions.load(&derived_id)?.is_some() {
            return Err(WorkflowControlPlaneError::InvalidRequest(format!(
                "workflow version `{derived_id}` of workflow `{}` is already published",
                listing.metadata.release.identity.workflow,
            )));
        }
        let mut versions = stores.versions.clone();
        versions
            .publish(derived)
            .map_err(WorkflowControlPlaneError::Engine)?;

        let identity = &listing.metadata.release.identity;
        let lineage = listing
            .metadata
            .provenance
            .forked_from
            .as_ref()
            .ok_or_else(|| {
                WorkflowControlPlaneError::InvalidRequest(
                    "the forked release carries no lineage record".to_owned(),
                )
            })?;
        let upstream_identity = &lineage.identity;
        Ok(rpc::WorkflowForkResponse {
            workflow: identity.workflow.to_string(),
            version_id: derived_id.to_string(),
            semantic_version: identity.semantic_version.to_string(),
            definition_digest: identity.definition_digest.to_string(),
            dependency_lock_digest: identity.dependency_lock_digest.to_string(),
            repository: identity.repository.to_string(),
            commit_sha: identity.source_revision.commit_sha.to_string(),
            lineage: rpc::WorkflowForkLineage {
                workflow: upstream_identity.workflow.to_string(),
                version_id: lineage.version_id.to_string(),
                semantic_version: upstream_identity.semantic_version.to_string(),
                repository: upstream_identity.repository.to_string(),
                definition_digest: upstream_identity.definition_digest.to_string(),
                dependency_lock_digest: upstream_identity.dependency_lock_digest.to_string(),
                commit_sha: upstream_identity.source_revision.commit_sha.to_string(),
            },
            attribution: listing
                .metadata
                .provenance
                .carried_attribution
                .iter()
                .map(|entry| rpc::WorkflowForkAttribution {
                    name: entry.name.clone(),
                    contact: entry.contact.clone(),
                })
                .collect(),
        })
    }

    /// Runs one instance of a published version to a terminal state.
    ///
    /// Instantiation runs the engine gates (validate -> approve -> bind)
    /// over the durable records; the walk then settles the instance
    /// (`Succeeded`, `Paused`, or `Failed`) and persists it.
    pub async fn instance_run(
        &self,
        params: rpc::WorkflowInstanceRunParams,
    ) -> Result<rpc::WorkflowInstanceRunResponse, WorkflowControlPlaneError> {
        let stores = self.stores()?;
        self.reconcile_startup(&stores);
        let version_id = parse_version_id(&params.version_id)?;
        let mut lifecycle = self.lifecycle(&stores);
        lifecycle
            .select_version(&version_id)
            .map_err(WorkflowControlPlaneError::Engine)?;
        let instance = lifecycle
            .instantiate(Default::default())
            .await
            .map_err(WorkflowControlPlaneError::Engine)?;
        let outcome = lifecycle
            .run()
            .await
            .map_err(WorkflowControlPlaneError::Engine)?;
        let terminal = match &outcome.terminal {
            RunTerminal::Completed => rpc::WorkflowRunTerminal {
                kind: rpc::WorkflowRunTerminalKind::Completed,
                node: None,
                reason: None,
            },
            RunTerminal::Paused { node, reason } => rpc::WorkflowRunTerminal {
                kind: rpc::WorkflowRunTerminalKind::Paused,
                node: Some(node.to_string()),
                reason: Some(reason.clone()),
            },
            RunTerminal::Failed { reason } => rpc::WorkflowRunTerminal {
                kind: rpc::WorkflowRunTerminalKind::Failed,
                node: None,
                reason: Some(reason.clone()),
            },
        };
        Ok(rpc::WorkflowInstanceRunResponse {
            instance_id: instance.instance_id.to_string(),
            workflow: instance.workflow.to_string(),
            version_id: instance.version.to_string(),
            status: instance_status_protocol(outcome.instance.status),
            terminal,
            path: outcome.path.iter().map(ToString::to_string).collect(),
        })
    }

    /// Lists every durable instance with status and persisted position.
    ///
    /// The durable root is re-opened first (see [`Self::refresh_stores`])
    /// and the startup sweep runs over the refreshed view, so a store
    /// seeded after this plane was constructed — the post-crash restart
    /// state — lists an orphaned `Running` instance as `Paused` with its
    /// persisted position, never silently `Running` and without
    /// reconstructing the plane.
    pub fn instance_list(
        &self,
        _params: rpc::WorkflowInstanceListParams,
    ) -> Result<rpc::WorkflowInstanceListResponse, WorkflowControlPlaneError> {
        let stores = self.refresh_stores()?;
        self.reconcile_startup(&stores);
        let positions = stores.run_positions.clone();
        let instances = stores
            .instances
            .records()
            .into_iter()
            .map(|instance| instance_record(&instance, &positions))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rpc::WorkflowInstanceListResponse { instances })
    }

    /// Reads one durable instance with its evidence references.
    ///
    /// Like [`Self::instance_list`], the durable root is re-opened first
    /// and the startup sweep runs over the refreshed view, so a record
    /// written after this plane's handle was opened reads back with its
    /// current durable status — an orphaned `Running` instance reads
    /// `Paused`, never silently `Running`.
    pub fn instance_get(
        &self,
        params: rpc::WorkflowInstanceGetParams,
    ) -> Result<rpc::WorkflowInstanceGetResponse, WorkflowControlPlaneError> {
        let stores = self.refresh_stores()?;
        self.reconcile_startup(&stores);
        let instance_id = parse_instance_id(&params.instance_id)?;
        let instance = load_instance(&stores, &instance_id)?;
        let positions = stores.run_positions.clone();
        Ok(rpc::WorkflowInstanceGetResponse {
            instance: instance_record(&instance, &positions)?,
            evidence: instance
                .evidence
                .iter()
                .map(evidence_reference_protocol)
                .collect(),
        })
    }

    /// Resumes one paused instance and continues its run.
    ///
    /// The control-plane `Paused -> Running` transition is performed by
    /// the durable resume seam (an explicit operator action; nothing
    /// auto-resumes), then the continuation rehydrates from the persisted
    /// run position through the engine's resume path.
    pub async fn instance_resume(
        &self,
        params: rpc::WorkflowInstanceResumeParams,
    ) -> Result<rpc::WorkflowInstanceResumeResponse, WorkflowControlPlaneError> {
        let stores = self.stores()?;
        let instance_id = parse_instance_id(&params.instance_id)?;
        let node = resume_node(&stores, &instance_id)?;
        let directive = ResumeDirective {
            instance: instance_id,
            node,
            trigger: TriggerSource {
                trigger: TriggerClass::User,
                event_id: None,
            },
        };
        let mut control = stores.control.clone();
        control
            .resume(&directive)
            .map_err(WorkflowControlPlaneError::Trigger)?;
        let mut lifecycle = self.lifecycle(&stores);
        lifecycle
            .resume_run(&instance_id)
            .await
            .map_err(WorkflowControlPlaneError::Engine)?;
        let instance = load_instance(&stores, &instance_id)?;
        let positions = stores.run_positions.clone();
        Ok(rpc::WorkflowInstanceResumeResponse {
            instance: instance_record(&instance, &positions)?,
        })
    }

    /// Cancels one instance with an explicit operator reason.
    pub fn instance_cancel(
        &self,
        params: rpc::WorkflowInstanceCancelParams,
    ) -> Result<rpc::WorkflowInstanceCancelResponse, WorkflowControlPlaneError> {
        let stores = self.stores()?;
        let instance_id = parse_instance_id(&params.instance_id)?;
        let mut lifecycle = self.lifecycle(&stores);
        lifecycle
            .cancel(&instance_id, params.reason)
            .map_err(WorkflowControlPlaneError::Engine)?;
        let instance = load_instance(&stores, &instance_id)?;
        let positions = stores.run_positions.clone();
        Ok(rpc::WorkflowInstanceCancelResponse {
            instance: instance_record(&instance, &positions)?,
        })
    }

    /// Builds one ephemeral lifecycle over the shared durable stores.
    ///
    /// The lifecycle is the engine's application service: constructing it
    /// executes nothing. The approval and action seams are the mount's
    /// doubles (the Codex agent loop at the action seam is a later
    /// integration); taught workflows declare no capabilities, so their
    /// runs never consult the action source.
    fn lifecycle(&self, stores: &Arc<DurableStores>) -> WorkflowLifecycle {
        WorkflowLifecycle::with_positions(
            LifecycleDeps {
                versions: Box::new(stores.versions.clone()),
                instances: Box::new(stores.instances.clone()),
                evidence: Box::new(stores.evidence.clone()),
                approvals: Box::new(DurableBackedApprovalSource::new(
                    stores.evidence.clone(),
                    MOUNT_APPROVER,
                )),
                actions: Box::new(ScriptedActionSource::new(Vec::new())),
                events: Box::new(RecordingEventSink::new()),
                registry: codex_execution_contracts::CapabilityRegistry::new(),
            },
            Box::new(stores.run_positions.clone()),
        )
    }

    /// Opens (once) and returns the shared durable stores.
    fn stores(&self) -> Result<Arc<DurableStores>, WorkflowControlPlaneError> {
        let mut state = self.lock();
        if let Some(stores) = state.stores.as_ref() {
            return Ok(Arc::clone(stores));
        }
        let stores = Arc::new(DurableStores::open(&self.root)?);
        state.stores = Some(Arc::clone(&stores));
        Ok(stores)
    }

    /// Re-opens the durable root and swaps the cached handle.
    ///
    /// The frozen durable stores load their snapshot files exactly once,
    /// at open; a handle never observes records another process (or a
    /// direct seeding tool) writes afterwards. The read path therefore
    /// re-opens the root before reading: re-opening the same root
    /// reconstructs the full control-plane state (snapshots load
    /// atomically, journals replay in order), and every mutation this
    /// plane already made was flushed atomically at write time, so
    /// swapping the handle loses nothing. Mutating commands keep
    /// working through the (now refreshed) cached handle, preserving
    /// the single-writer discipline.
    fn refresh_stores(&self) -> Result<Arc<DurableStores>, WorkflowControlPlaneError> {
        let mut state = self.lock();
        let stores = Arc::new(DurableStores::open(&self.root)?);
        state.stores = Some(Arc::clone(&stores));
        Ok(stores)
    }

    /// Runs the one-shot startup reconciliation sweep over the stores.
    ///
    /// Post-crash `Running` records with no live run are transitioned to
    /// `Paused` with recovery evidence, awaiting an explicit resume —
    /// never silently `Running`, never auto-executing.
    fn reconcile_startup(&self, stores: &Arc<DurableStores>) {
        let mut state = self.lock();
        if state.reconciled {
            return;
        }
        state.reconciled = true;
        let mut lifecycle = self.lifecycle(stores);
        let records = stores.instances.records();
        match lifecycle.reconcile_startup(&records) {
            Ok(reconciled) => {
                if !reconciled.is_empty() {
                    tracing::info!(
                        count = reconciled.len(),
                        "workflow control plane reconciled orphaned running instances to paused"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(%error, "workflow startup reconciliation sweep failed");
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, WorkflowState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Builds the reconciled hybrid session view the control plane compiles.
///
/// The teaching compiler preserves the trajectory's recording order
/// exactly, so the reconcile step's merge policy is applied here, ahead
/// of compilation: every demonstration record (the observed steps)
/// precedes every instruction record (the instructed steps) — the
/// VWO-010 Family A reconciled hybrid origins `["observed",
/// "instructed"]` — with the recording order preserved inside each
/// group, so observation/action/result pairing survives the merge. The
/// view is replayed through the frozen session's public append-only
/// API; the original session is never rewritten.
fn reconciled_hybrid_session(
    session: &TeachingSession,
) -> Result<TeachingSession, WorkflowControlPlaneError> {
    let mut reconciled = TeachingSession::with_policy(session.mode(), session.policy().clone());
    for origin in [RecordOrigin::Demonstration, RecordOrigin::Instruction] {
        for record in session
            .records()
            .iter()
            .filter(|record| record.origin() == origin)
        {
            reconciled
                .record(
                    record.origin(),
                    record.event().clone(),
                    record.evidence().to_vec(),
                )
                .map_err(WorkflowControlPlaneError::Teaching)?;
        }
    }
    if session.is_closed() {
        reconciled.close();
    }
    Ok(reconciled)
}

/// Builds the marketplace entry metadata for one durable published
/// version, so the engine's fork port can be driven against it (the
/// port's documented precondition: the upstream must be a marketplace
/// entry).
///
/// The mount's publication path seals versions without distribution
/// metadata (the RWO-001 bound), so this derives the entry's facts
/// honestly from the sealed record itself: the compatibility spec covers
/// every capability the version's steps declare and every resource its
/// dependencies declare — exactly what the engine's
/// `validate_against_version` demands — and the attribution is the
/// sealed provenance's authorship when the version carries one.
fn seeded_release_metadata(
    version: &WorkflowVersion,
) -> Result<PublicationMetadata, WorkflowControlPlaneError> {
    let mut required_capabilities = Vec::new();
    for node in version.definition.ir.nodes.values() {
        if let WorkflowIrNode::Step(step) = node {
            for requirement in &step.capabilities {
                if !required_capabilities.contains(&requirement.capability) {
                    required_capabilities.push(requirement.capability.clone());
                }
            }
        }
    }
    let mut required_resources = Vec::new();
    for resource in &version.definition.dependencies.resources {
        if !required_resources.contains(&resource.resource_type) {
            required_resources.push(resource.resource_type.clone());
        }
    }
    let attribution = version
        .provenance
        .as_ref()
        .map(|provenance| provenance.authors.clone())
        .unwrap_or_default();
    PublicationMetadata::seal(
        PublishedVersionRef::of(version),
        attribution,
        MarketplacePrincipal::parse(MOUNT_APPROVER).map_err(|error| {
            WorkflowControlPlaneError::InvalidRequest(format!("invalid principal: {error}"))
        })?,
        LicenseTerms {
            identifier: DEFAULT_FORK_LICENSE.to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage::default(),
        CompatibilitySpec {
            minimum_runtime: semver::Version::new(0, 0, 0),
            required_capabilities,
            capability_classes: Vec::new(),
            required_resources,
        },
        UpgradePolicySetting::Pin,
    )
    .map_err(WorkflowControlPlaneError::Distribution)
}

/// Looks up one teaching session mutably.
fn session_mut<'a>(
    state: &'a mut WorkflowState,
    session_id: &str,
) -> Result<&'a mut TeachingSessionEntry, WorkflowControlPlaneError> {
    state.sessions.get_mut(session_id).ok_or_else(|| {
        WorkflowControlPlaneError::NotFound(format!("unknown teaching session `{session_id}`"))
    })
}

/// Looks up one candidate immutably.
fn candidate_ref<'a>(
    state: &'a WorkflowState,
    candidate_id: &str,
) -> Result<&'a CandidateEntry, WorkflowControlPlaneError> {
    state.candidates.get(candidate_id).ok_or_else(|| {
        WorkflowControlPlaneError::NotFound(format!("unknown candidate `{candidate_id}`"))
    })
}

/// Looks up one candidate mutably.
fn candidate_mut<'a>(
    state: &'a mut WorkflowState,
    candidate_id: &str,
) -> Result<&'a mut CandidateEntry, WorkflowControlPlaneError> {
    state.candidates.get_mut(candidate_id).ok_or_else(|| {
        WorkflowControlPlaneError::NotFound(format!("unknown candidate `{candidate_id}`"))
    })
}

/// Parses a `sha256:<hex>` version identity.
fn parse_version_id(version_id: &str) -> Result<WorkflowVersionId, WorkflowControlPlaneError> {
    WorkflowVersionId::try_from(version_id).map_err(|error| {
        WorkflowControlPlaneError::InvalidRequest(format!(
            "invalid workflow version id `{version_id}`: {error}"
        ))
    })
}

/// Parses a UUID instance identity.
fn parse_instance_id(instance_id: &str) -> Result<WorkflowInstanceId, WorkflowControlPlaneError> {
    let uuid = Uuid::parse_str(instance_id).map_err(|error| {
        WorkflowControlPlaneError::InvalidRequest(format!(
            "invalid workflow instance id `{instance_id}`: {error}"
        ))
    })?;
    Ok(WorkflowInstanceId::from(uuid))
}

/// Loads one durable instance record.
fn load_instance(
    stores: &Arc<DurableStores>,
    instance_id: &WorkflowInstanceId,
) -> Result<WorkflowInstance, WorkflowControlPlaneError> {
    let instances = stores.instances.clone();
    instances.load(instance_id)?.ok_or_else(|| {
        WorkflowControlPlaneError::NotFound(format!("unknown workflow instance `{instance_id}`"))
    })
}

/// The node a resume directive names: the persisted pending re-entry
/// point, or the last visited node when the wait was the final node.
fn resume_node(
    stores: &Arc<DurableStores>,
    instance_id: &WorkflowInstanceId,
) -> Result<codex_workflow_contracts::IrNodeId, WorkflowControlPlaneError> {
    let positions = stores.run_positions.clone();
    let position = positions.load(instance_id)?.ok_or_else(|| {
        WorkflowControlPlaneError::InvalidRequest(format!(
            "no run position is persisted for instance `{instance_id}`; only a paused \
             instance with a persisted position can be resumed"
        ))
    })?;
    position
        .walk
        .current
        .or_else(|| position.walk.path.last().cloned())
        .ok_or_else(|| {
            WorkflowControlPlaneError::InvalidRequest(format!(
                "the persisted position of instance `{instance_id}` names no node to resume at"
            ))
        })
}

/// Builds the review response from a candidate entry.
fn review_response(
    candidate_id: &str,
    entry: &CandidateEntry,
) -> Result<rpc::WorkflowReviewResponse, WorkflowControlPlaneError> {
    let candidate = &entry.candidate;
    let validation = candidate.last_validation().ok_or_else(|| {
        WorkflowControlPlaneError::InvalidRequest(format!(
            "candidate `{candidate_id}` has not been validated"
        ))
    })?;
    let simulation = candidate.last_simulation().ok_or_else(|| {
        WorkflowControlPlaneError::InvalidRequest(format!(
            "candidate `{candidate_id}` has not been simulated"
        ))
    })?;
    let steps = candidate
        .ir()
        .nodes
        .iter()
        .filter_map(|(node_id, node)| {
            let WorkflowIrNode::Step(step) = node else {
                return None;
            };
            Some(rpc::WorkflowStepReview {
                node_id: node_id.to_string(),
                origin: step_origin_protocol(candidate.node_origin(node_id)),
                description: step.description.clone(),
                evidence_count: candidate
                    .node_evidence(node_id)
                    .map(<[TeachingEvidence]>::len)
                    .unwrap_or_default() as u64,
            })
        })
        .collect();
    Ok(rpc::WorkflowReviewResponse {
        candidate_id: candidate_id.to_string(),
        status: candidate_status_protocol(candidate.status()),
        origin: candidate_origin_protocol(candidate.origin()),
        epoch: candidate.epoch(),
        description: candidate.description().map(str::to_string),
        steps,
        capability_inferences: candidate
            .capability_inferences()
            .iter()
            .map(|inference| rpc::WorkflowCapabilityInference {
                node_id: inference.node_id.to_string(),
                hint: inference.requirement_hint.clone(),
                rationale: inference.rationale.clone(),
            })
            .collect(),
        binding_proposals: candidate
            .binding_proposals()
            .iter()
            .map(|proposal| rpc::WorkflowBindingProposal {
                node_id: proposal.node_id.to_string(),
                kind: proposal.kind.as_str().to_string(),
                reference: proposal.proposed_reference.clone(),
                rationale: proposal.rationale.clone(),
                requires_approval: proposal.requires_approval,
            })
            .collect(),
        trigger_intents: candidate
            .trigger_intents()
            .iter()
            .map(|intent| rpc::WorkflowTriggerIntent {
                class_hint: intent.class_hint.clone(),
                description: intent.description.clone(),
            })
            .collect(),
        validation: validation_summary_protocol(validation),
        simulation: simulation_summary_protocol(simulation),
    })
}

/// Maps one durable instance to its protocol record.
fn instance_record(
    instance: &WorkflowInstance,
    positions: &DurableRunPositionStore,
) -> Result<rpc::WorkflowInstanceRecord, WorkflowControlPlaneError> {
    let position = positions.load(&instance.instance_id)?;
    Ok(rpc::WorkflowInstanceRecord {
        instance_id: instance.instance_id.to_string(),
        workflow: instance.workflow.to_string(),
        version_id: instance.version.to_string(),
        status: instance_status_protocol(instance.status),
        trigger: instance
            .trigger
            .as_ref()
            .map(|trigger| rpc::WorkflowTriggerSource {
                class: trigger_class_protocol(trigger.trigger),
                event_id: trigger.event_id.clone(),
            }),
        position: position.map(|position| rpc::WorkflowRunPositionSummary {
            current_node: position.walk.current.as_ref().map(ToString::to_string),
            steps_taken: position.walk.steps_taken,
            path: position.walk.path.iter().map(ToString::to_string).collect(),
        }),
    })
}

/// Maps one evidence reference to its protocol form.
fn evidence_reference_protocol(reference: &EvidenceReference) -> rpc::WorkflowEvidenceReference {
    rpc::WorkflowEvidenceReference {
        kind: evidence_kind_str(reference.kind).to_string(),
        locator: reference.locator.clone(),
        digest: reference.digest.to_string(),
    }
}

/// The lowercase label of one evidence kind.
fn evidence_kind_str(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Observation => "observation",
        EvidenceKind::Artifact => "artifact",
        EvidenceKind::Approval => "approval",
        EvidenceKind::Trace => "trace",
        EvidenceKind::TestResult => "test-result",
        EvidenceKind::Recovery => "recovery",
    }
}

/// Maps a protocol teaching mode to the engine mode.
fn teach_mode(mode: rpc::WorkflowTeachMode) -> TeachingMode {
    match mode {
        rpc::WorkflowTeachMode::Demonstrate => TeachingMode::Demonstrate,
        rpc::WorkflowTeachMode::Instruct => TeachingMode::Instruct,
        rpc::WorkflowTeachMode::Hybrid => TeachingMode::Hybrid,
    }
}

/// Maps an engine teaching mode to the protocol mode.
fn teach_mode_protocol(mode: TeachingMode) -> rpc::WorkflowTeachMode {
    match mode {
        TeachingMode::Demonstrate => rpc::WorkflowTeachMode::Demonstrate,
        TeachingMode::Instruct => rpc::WorkflowTeachMode::Instruct,
        TeachingMode::Hybrid => rpc::WorkflowTeachMode::Hybrid,
    }
}

/// Maps a candidate status to the protocol status.
fn candidate_status_protocol(status: CandidateStatus) -> rpc::WorkflowCandidateStatus {
    match status {
        CandidateStatus::Compiled => rpc::WorkflowCandidateStatus::Compiled,
        CandidateStatus::Validated => rpc::WorkflowCandidateStatus::Validated,
        CandidateStatus::Approved => rpc::WorkflowCandidateStatus::Approved,
        CandidateStatus::PublicationReady => rpc::WorkflowCandidateStatus::PublicationReady,
    }
}

/// Maps a candidate origin to the protocol teaching mode.
fn candidate_origin_protocol(origin: CandidateOrigin) -> rpc::WorkflowTeachMode {
    match origin {
        CandidateOrigin::Taught(mode) => teach_mode_protocol(mode),
        CandidateOrigin::AuthoredIr => rpc::WorkflowTeachMode::Instruct,
    }
}

/// Maps a step origin to the protocol origin.
fn step_origin_protocol(origin: Option<StepOrigin>) -> rpc::WorkflowStepOrigin {
    match origin {
        Some(StepOrigin::Observed) => rpc::WorkflowStepOrigin::Observed,
        Some(StepOrigin::Instructed) | None => rpc::WorkflowStepOrigin::Instructed,
    }
}

/// Maps an instance status to the protocol status.
fn instance_status_protocol(status: WorkflowInstanceStatus) -> rpc::WorkflowInstanceStatus {
    match status {
        WorkflowInstanceStatus::Pending => rpc::WorkflowInstanceStatus::Pending,
        WorkflowInstanceStatus::Running => rpc::WorkflowInstanceStatus::Running,
        WorkflowInstanceStatus::Paused => rpc::WorkflowInstanceStatus::Paused,
        WorkflowInstanceStatus::Succeeded => rpc::WorkflowInstanceStatus::Succeeded,
        WorkflowInstanceStatus::Failed => rpc::WorkflowInstanceStatus::Failed,
        WorkflowInstanceStatus::Cancelled => rpc::WorkflowInstanceStatus::Cancelled,
    }
}

/// Maps a trigger class to the protocol class.
fn trigger_class_protocol(class: TriggerClass) -> rpc::WorkflowTriggerClass {
    match class {
        TriggerClass::User => rpc::WorkflowTriggerClass::User,
        TriggerClass::Schedule => rpc::WorkflowTriggerClass::Schedule,
        TriggerClass::Webhook => rpc::WorkflowTriggerClass::Webhook,
        TriggerClass::ConnectorEvent => rpc::WorkflowTriggerClass::ConnectorEvent,
        TriggerClass::BrowserEvent => rpc::WorkflowTriggerClass::BrowserEvent,
        TriggerClass::ComputerEvent => rpc::WorkflowTriggerClass::ComputerEvent,
        TriggerClass::WorkflowEvent => rpc::WorkflowTriggerClass::WorkflowEvent,
        TriggerClass::HumanEvent => rpc::WorkflowTriggerClass::HumanEvent,
    }
}

/// Maps a validation summary to the protocol summary.
fn validation_summary_protocol(summary: &ValidationSummary) -> rpc::WorkflowValidationSummary {
    rpc::WorkflowValidationSummary {
        clean: summary.is_clean(),
        error_count: summary.error_count() as u64,
        warning_count: summary.warning_count() as u64,
        findings: summary
            .findings()
            .iter()
            .map(|finding| rpc::WorkflowValidationFinding {
                severity: match finding.severity {
                    Severity::Error => rpc::WorkflowValidationSeverity::Error,
                    Severity::Warning => rpc::WorkflowValidationSeverity::Warning,
                },
                code: format!("{:?}", finding.code),
                message: finding.message.clone(),
            })
            .collect(),
    }
}

/// Maps a simulation report to the protocol summary.
fn simulation_summary_protocol(report: &SimulationReport) -> rpc::WorkflowSimulationSummary {
    let (outcome, node, reason) = match report.outcome() {
        SimulationOutcome::Completed => (rpc::WorkflowSimulationOutcomeKind::Completed, None, None),
        SimulationOutcome::Paused { at } => (
            rpc::WorkflowSimulationOutcomeKind::Paused,
            Some(at.to_string()),
            None,
        ),
        SimulationOutcome::Indeterminate { at, reason } => (
            rpc::WorkflowSimulationOutcomeKind::Indeterminate,
            Some(at.to_string()),
            Some(reason.clone()),
        ),
        SimulationOutcome::Aborted { reason } => (
            rpc::WorkflowSimulationOutcomeKind::Aborted,
            None,
            Some(reason.clone()),
        ),
    };
    rpc::WorkflowSimulationSummary {
        outcome,
        node,
        reason,
        path: report.path().iter().map(ToString::to_string).collect(),
        steps_taken: report.steps_taken(),
        epoch: report.epoch(),
    }
}

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;
