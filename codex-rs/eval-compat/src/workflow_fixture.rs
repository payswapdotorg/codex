//! Deterministic workflow fixtures for evaluation runs (WO-013).
//!
//! The fixture publishes one immutable workflow version through the real
//! frozen pipeline — teaching session, compilation, approval, forge
//! release, install — exactly the way the WO-010 end-to-end tests do, but
//! bound to the scripted evaluation capability
//! ([`EVAL_CAPABILITY`](crate::env_adapter::EVAL_CAPABILITY)) so runs
//! execute against [`crate::env_adapter::ScriptedEnvironmentAdapter`].
//!
//! Everything is deterministic: fixed repository identity, fixed source
//! revision (resolved through the in-memory forge), fixed semantic version,
//! and a fixed binding map. Two harnesses that publish
//! [`published_fixture`] land on the **same** [`WorkflowVersionId`], which
//! is what makes provider-differential comparison meaningful: the only
//! variable is the model provider serving the action-proposal seam.

use std::collections::BTreeMap;

use codex_teaching_compiler::ApprovalDecision;
use codex_teaching_compiler::ApprovalDecisionKind;
use codex_teaching_compiler::SimulationConfig;
use codex_teaching_compiler::TeachingEvidence;
use codex_teaching_compiler::TeachingMode;
use codex_teaching_compiler::TeachingSession;
use codex_teaching_compiler::TrajectoryEvent;
use codex_teaching_compiler::compile;
use codex_workflow_app::PublishRequest;
use codex_workflow_app::PublishedArtifact;
use codex_workflow_app::WorkflowAppError;
use codex_workflow_app::install_version;
use codex_workflow_app::publish;
use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ForgeKind;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::MANIFEST_FORMAT_VERSION;
use codex_workflow_contracts::RepositoryRelativePath;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_forge::InMemoryForge;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::publish_release;

use crate::env_adapter::EVAL_CAPABILITY;

/// The fixture workflow identity.
pub const EVAL_WORKFLOW: &str = "eval-compat-probe";

/// The fixture repository identity.
pub const EVAL_REPOSITORY: &str = "https://github.com/eval-compat/fixture-workflows";

/// The fixture semantic version.
pub const EVAL_SEMANTIC_VERSION: (u64, u64, u64) = (1, 0, 0);

/// The fixture release tag.
pub const EVAL_RELEASE_TAG: &str = "eval-v1.0.0";

/// Teaches the two-step probe workflow and drives the compiler to an
/// approved, digest-pinned candidate.
///
/// Step 001 inspects the issue tracker; step 002 records the summary. The
/// demonstration mirrors the canonical WO-010 fixture so evaluation runs
/// exercise the same teaching → compilation → publication path as the
/// application's own tests. A clean validation and a publication-allowing
/// simulation are fixture invariants guarded with assertions: a failing
/// assert means the fixture itself is broken, not the system under
/// evaluation.
pub fn taught_fixture() -> Result<codex_teaching_compiler::ApprovedWorkflow, WorkflowAppError> {
    let mut session = TeachingSession::new(TeachingMode::Demonstrate);
    session.record(
        codex_teaching_compiler::RecordOrigin::Demonstration,
        TrajectoryEvent::Observation {
            text: "the issue tracker is open".to_string(),
        },
        Vec::new(),
    )?;
    session.record(
        codex_teaching_compiler::RecordOrigin::Demonstration,
        TrajectoryEvent::Action {
            text: "Inspect the issues list.".to_string(),
        },
        vec![TeachingEvidence::new(
            "inspect-issues",
            "rollout://eval/0001",
            "ab".repeat(32),
        )?],
    )?;
    session.record(
        codex_teaching_compiler::RecordOrigin::Demonstration,
        TrajectoryEvent::Result {
            text: "the list shows inspected issues".to_string(),
        },
        Vec::new(),
    )?;
    session.record(
        codex_teaching_compiler::RecordOrigin::Demonstration,
        TrajectoryEvent::Action {
            text: "Record the summary in the tracker app.".to_string(),
        },
        vec![TeachingEvidence::new(
            "record-summary",
            "rollout://eval/0002",
            "cd".repeat(32),
        )?],
    )?;
    session.close();

    let mut candidate = compile(&session)?;
    let summary = candidate.validate()?;
    assert!(
        summary.is_clean(),
        "the evaluation fixture must validate cleanly"
    );
    let simulation = candidate.simulate(SimulationConfig::default())?;
    assert!(
        simulation.allows_publication(),
        "the evaluation fixture must simulate to a publishable outcome"
    );
    candidate.approve(ApprovalDecision::new(
        "eval-compat-approver",
        "eval-review-1",
        ApprovalDecisionKind::Approved,
    )?)?;
    let approved = candidate.finalize(definition_id()?)?;
    Ok(approved)
}

/// The reviewed step bindings: both probe steps declare the scripted
/// evaluation capability.
pub fn fixture_bindings() -> Result<BTreeMap<IrNodeId, Vec<CapabilityRequirement>>, WorkflowAppError>
{
    let capability = CapabilityId::parse(EVAL_CAPABILITY).map_err(WorkflowAppError::from)?;
    Ok(BTreeMap::from([
        (
            node_id("step-001")?,
            vec![CapabilityRequirement {
                capability: capability.clone(),
                purpose: Some("inspect the issues list".to_string()),
            }],
        ),
        (
            node_id("step-002")?,
            vec![CapabilityRequirement {
                capability,
                purpose: Some("record the summary in the tracker app".to_string()),
            }],
        ),
    ]))
}

/// Publishes and installs the fixture workflow version.
///
/// The version is sealed through the git lifecycle (forge repository,
/// development commit, ref resolution, seal, release, install) and its
/// integrity is verified before returning. Repeated calls produce
/// byte-identical version records.
pub fn published_fixture() -> Result<(PublishedArtifact, InstallRegistry), WorkflowAppError> {
    let repository = repository_id()?;
    let mut forge = InMemoryForge::new(ForgeKind::parse("github")?);
    let manifest = WorkflowManifest {
        manifest_version: MANIFEST_FORMAT_VERSION,
        workflow: definition_id()?,
        display_name: Some(EVAL_WORKFLOW.to_string()),
        description: None,
        repository: repository.clone(),
        definition_path: RepositoryRelativePath::parse("workflows/eval-compat-probe.json")?,
        provenance: None,
    };
    let branch = DevelopmentRef::branch("main")?;
    forge.seed_repository(
        repository.clone(),
        branch.clone(),
        vec![Attribution {
            name: "eval-compat-bot".to_string(),
            contact: None,
        }],
        vec![manifest],
    )?;
    forge.commit(
        &repository,
        &branch,
        format!("publish {EVAL_RELEASE_TAG}"),
        "eval-compat-bot",
    )?;
    let revision = forge.resolve_ref_blocking(&repository, &branch)?;
    let artifact = publish(PublishRequest {
        approved: taught_fixture()?,
        repository: repository.clone(),
        source_revision: revision.clone(),
        semantic_version: SemanticVersion::new(
            EVAL_SEMANTIC_VERSION.0,
            EVAL_SEMANTIC_VERSION.1,
            EVAL_SEMANTIC_VERSION.2,
        ),
        bindings: fixture_bindings()?,
        dependency_lock: DependencyLock::default(),
        provenance: Some(WorkflowProvenance {
            authors: vec![Attribution {
                name: "eval-compat-bot".to_string(),
                contact: None,
            }],
            forked_from: None,
            license: None,
            upgrade_policy: None,
        }),
    })?;
    artifact.version.verify_integrity()?;
    let release = publish_release(
        &repository,
        &[],
        EVAL_RELEASE_TAG,
        &revision,
        std::slice::from_ref(&artifact.reference),
    )?;
    assert_eq!(
        release.versions,
        vec![artifact.version.version_id.clone()],
        "the fixture release must pin exactly the published version"
    );
    let mut installs = InstallRegistry::new();
    install_version(&mut installs, &artifact)?;
    Ok((artifact, installs))
}

/// The fixture workflow identity.
pub fn definition_id() -> Result<WorkflowDefinitionId, WorkflowAppError> {
    WorkflowDefinitionId::parse(EVAL_WORKFLOW).map_err(WorkflowAppError::from)
}

/// The fixture repository identity.
pub fn repository_id() -> Result<WorkflowRepositoryId, WorkflowAppError> {
    WorkflowRepositoryId::parse(EVAL_REPOSITORY).map_err(WorkflowAppError::from)
}

/// Parses a fixture node id.
pub fn node_id(value: &str) -> Result<IrNodeId, WorkflowAppError> {
    IrNodeId::parse(value).map_err(WorkflowAppError::from)
}
