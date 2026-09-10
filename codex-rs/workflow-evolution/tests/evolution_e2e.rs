//! End-to-end governed-evolution tests (WO-014).
//!
//! Every scenario is static and end to end: the incumbent workflow is the
//! real evaluation fixture (published through the teaching → compile →
//! approve → publish → install pipeline), execution evidence comes from
//! real runs through eval-compat's harness, candidates are generated from
//! that evidence, and validation replays through the harness with scripted
//! model and environment doubles. The scenarios cover the Work Order's
//! acceptance surface:
//!
//! - evidence → candidate → provenance traceability;
//! - replay-differential validation, with a divergence failing the gate;
//! - explicit approval required (no auto-promotion, rejection honored);
//! - the immutable predecessor (mutation rejected, record unchanged);
//! - lineage and rollback transitions (recorded, never silent);
//! - retention bounds;
//! - the no-credential invariant;
//! - runtime adjustments (recovery policy, schedule) riding lineage.

// The workspace clippy.toml allows `expect`/`unwrap` in test code; the
// same intent is spelled out at file level here (matching the WO-010 and
// WO-013 end-to-end test precedent) for plain helper functions.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::sync::Arc;

use codex_eval_compat::EVAL_ENVIRONMENT;
use codex_eval_compat::ScriptedEnvTurn;
use codex_eval_compat::ScriptedModelProvider;
use codex_eval_compat::ScriptedTurn;
use codex_eval_compat::WorkflowEvalHarness;
use codex_eval_compat::equivalent_capabilities;
use codex_eval_compat::published_fixture;
use codex_execution_contracts::ExecutionEnvironment;
use codex_workflow_app::RunTerminal;
use codex_workflow_app::WalkConfig;
use codex_workflow_app::WorkflowEvent;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyKey;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DependencyProvenance;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::ResolvedDependency;
use codex_workflow_contracts::ResolvedDependencyIdentity;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::SkillName;
use codex_workflow_contracts::WorkflowInstanceId;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::UpdateDecision;
use pretty_assertions::assert_eq;

use codex_workflow_evolution::AppliedAdjustment;
use codex_workflow_evolution::ApprovalDecision;
use codex_workflow_evolution::CandidateGenerator;
use codex_workflow_evolution::CandidateId;
use codex_workflow_evolution::ChangeKind;
use codex_workflow_evolution::EvalReplayPort;
use codex_workflow_evolution::EvidenceCorpus;
use codex_workflow_evolution::EvolutionGovernor;
use codex_workflow_evolution::EvolutionPolicy;
use codex_workflow_evolution::InMemoryApprovalPort;
use codex_workflow_evolution::PolicyGate;
use codex_workflow_evolution::ProposedChange;
use codex_workflow_evolution::RetentionPolicy;
use codex_workflow_evolution::StageName;
use codex_workflow_evolution::VersionBump;
use codex_workflow_evolution::WorkflowEvolutionError;

const PROVIDER: &str = "evidence-provider";

/// The action proposal the fixture model answers for step 001.
fn inspect_action_text() -> String {
    r#"{"operation":"inspect","target":"issues.example.com","inputs":{"depth":"full"}}"#.to_string()
}

/// The action proposal the fixture model answers for step 002.
fn record_action_text() -> String {
    r#"{"operation":"record","target":"summary","inputs":{"format":"markdown"}}"#.to_string()
}

/// The two fixture action proposals, in walk order.
fn action_script() -> Vec<ScriptedTurn> {
    vec![
        ScriptedTurn::message(inspect_action_text()),
        ScriptedTurn::message(record_action_text()),
    ]
}

/// Two succeeding environment turns.
fn env_script() -> Vec<ScriptedEnvTurn> {
    vec![
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "inspected"})),
        ScriptedEnvTurn::succeed(serde_json::json!({"state": "recorded"})),
    ]
}

/// An escalating environment script: the first action fails permanently.
fn escalating_env_script() -> Vec<ScriptedEnvTurn> {
    vec![
        ScriptedEnvTurn::fail(
            codex_execution_contracts::FailureKind::Permanent,
            "tracker rejected the action",
        )
        .expect("scripted failure"),
    ]
}

/// A provider equipped with the equivalent capability fixture.
fn provider(provider_id: &str, script: Vec<ScriptedTurn>) -> Arc<ScriptedModelProvider> {
    Arc::new(
        ScriptedModelProvider::new(provider_id, script).with_capability(
            codex_eval_compat::EVAL_MODEL_SLUG,
            equivalent_capabilities(provider_id),
        ),
    )
}

/// Extracts the single instance id a run created.
fn only_instance_id(harness: &WorkflowEvalHarness) -> WorkflowInstanceId {
    harness
        .observed_events()
        .iter()
        .find_map(|event| match event {
            WorkflowEvent::InstanceCreated { instance, .. } => Some(*instance),
            _ => None,
        })
        .expect("instance created event")
}

/// One evidence-producing run of the incumbent through a real harness.
async fn evidence_run(
    artifact: &codex_workflow_app::PublishedArtifact,
    env_script: Vec<ScriptedEnvTurn>,
) -> (
    codex_eval_compat::WorkflowRunRecord,
    Vec<codex_workflow_contracts::EvidenceReference>,
) {
    let mut harness = WorkflowEvalHarness::new(provider(PROVIDER, action_script()), env_script)
        .expect("evidence harness");
    let version = harness.install(artifact).expect("install incumbent");
    let record = harness
        .run(&version, WalkConfig::default())
        .await
        .expect("evidence run");
    let instance = only_instance_id(&harness);
    let verified = harness.verify(&instance).expect("verify");
    (record, verified.evidence)
}

/// Builds a governor over an installed incumbent with clean replay
/// scripts.
fn governor(
    incumbent: &WorkflowVersion,
    installs: &InstallRegistry,
    retention: RetentionPolicy,
) -> EvolutionGovernor<EvalReplayPort, PolicyGate> {
    EvolutionGovernor::over_incumbent(
        incumbent.clone(),
        installs.clone(),
        EvalReplayPort::new(action_script(), env_script()),
        PolicyGate::new(EvolutionPolicy::conservative()),
        Box::new(InMemoryApprovalPort::new()),
        retention,
    )
    .expect("governor")
}

/// The observed environment classes of the scripted evaluation adapter.
fn observed_environments() -> BTreeSet<ExecutionEnvironment> {
    BTreeSet::from([EVAL_ENVIRONMENT])
}

/// The minor successor version of the fixture incumbent.
fn minor_successor(incumbent: &WorkflowVersion) -> SemanticVersion {
    VersionBump::Minor.apply(&incumbent.identity.semantic_version)
}

/// A minimal host-authored candidate.
fn host_candidate(
    id: &str,
    incumbent: &codex_workflow_forge::PublishedVersionRef,
    change: ProposedChange,
) -> codex_workflow_evolution::ImprovementCandidate {
    codex_workflow_evolution::ImprovementCandidate {
        id: CandidateId::parse(id).expect("candidate id"),
        incumbent: incumbent.clone(),
        change,
        rationale: "host-authored candidate for the end-to-end scenarios".to_string(),
        provenance: codex_workflow_evolution::CandidateProvenance::default(),
    }
}

/// Runs the incumbent for evidence, generates candidates, and returns the
/// generator output plus the run's evidence references.
async fn generated_from_evidence(
    artifact: &codex_workflow_app::PublishedArtifact,
    env_script: Vec<ScriptedEnvTurn>,
) -> (
    Vec<codex_workflow_evolution::ImprovementCandidate>,
    Vec<codex_workflow_contracts::EvidenceReference>,
    codex_eval_compat::WorkflowRunRecord,
) {
    let (record, evidence) = evidence_run(artifact, env_script).await;
    let corpus = EvidenceCorpus::from_observations(
        &artifact.version,
        std::slice::from_ref(&record),
        evidence.clone(),
        observed_environments(),
        None,
    )
    .expect("corpus");
    let candidates = CandidateGenerator::new()
        .generate(&corpus)
        .expect("generated");
    (candidates, evidence, record)
}

#[tokio::test]
async fn evidence_to_candidate_provenance_is_traceable() {
    // Evidence produced by a real run of the incumbent flows into generated
    // candidates with full provenance: the exact evidence references and
    // the run's content-addressed identity.
    let (artifact, installs) = published_fixture().expect("fixture");
    let (candidates, evidence, record) = generated_from_evidence(&artifact, env_script()).await;

    // The clean run recorded observations and traces: exactly the streams
    // that feed the binding and definition rules.
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.change_kind() == ChangeKind::CapabilityBinding)
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.change_kind() == ChangeKind::DefinitionDelta)
    );
    let binding = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::CapabilityBinding)
        .expect("binding candidate");
    assert_eq!(binding.provenance.evidence, evidence);
    assert_eq!(binding.provenance.runs.len(), 1);
    assert_eq!(
        binding.provenance.runs[0].fingerprint,
        record.fingerprint_digest().expect("fingerprint")
    );
    assert_eq!(
        binding.provenance.runs[0].status,
        WorkflowInstanceStatus::Succeeded
    );
    assert_eq!(binding.incumbent.version_id, artifact.version.version_id);
    assert_eq!(*binding.workflow(), artifact.version.definition.id);

    // Provenance survives into the published lineage unchanged (asserted
    // by the golden-path test below through the same generator output).
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    for candidate in &candidates {
        evolution
            .record_candidate(candidate.clone())
            .expect("recorded");
    }
    let report = evolution
        .validate_candidate(&binding.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    assert!(
        report.passed(),
        "the binding candidate must validate: {report:?}"
    );
    evolution
        .decide_approval(
            &binding.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: Some("evidence-cited purposes look right".to_string()),
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&binding.id, "eval-v1.1.0")
        .expect("published");
    assert_eq!(publication.lineage.provenance, binding.provenance);
    assert_eq!(publication.lineage.candidate, binding.id);
}

#[tokio::test]
async fn replay_divergence_fails_the_gate() {
    // A host-authored definition delta that drops the second step changes
    // observable behavior: the differential gate must enumerate the
    // divergences and fail, and publication must be refused.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());

    let mut definition = artifact.version.definition.clone();
    definition
        .ir
        .nodes
        .remove(&codex_eval_compat::node_id("step-002").expect("node"));
    if let Some(WorkflowIrNode::Step(step)) = definition
        .ir
        .nodes
        .get_mut(&codex_eval_compat::node_id("step-001").expect("node"))
    {
        step.next = None;
    }
    let candidate = host_candidate(
        "host-divergent-drop-step",
        &artifact.reference,
        ProposedChange::Definition { definition },
    );
    evolution.record_candidate(candidate).expect("recorded");

    let report = evolution
        .validate_candidate(
            &CandidateId::parse("host-divergent-drop-step").expect("id"),
            &minor_successor(&artifact.version),
        )
        .await
        .expect("validation runs and reports");
    assert!(!report.passed(), "a behavioral change must fail the gate");
    assert_eq!(report.failed_stages(), vec!["differential"]);
    let divergences = report
        .stages
        .iter()
        .find_map(|stage| match stage {
            codex_workflow_evolution::StageEvidence::Differential { divergences, .. } => {
                Some(divergences)
            }
            _ => None,
        })
        .expect("differential stage");
    // The walk visits fewer nodes, dispatches fewer actions, records less
    // evidence, and invokes the model less: every difference enumerated.
    assert!(!divergences.is_empty());
    assert!(divergences.iter().any(|divergence| matches!(
        divergence,
        codex_eval_compat::WorkflowDivergence::Path { .. }
    )));
    assert!(divergences.iter().any(|divergence| matches!(
        divergence,
        codex_eval_compat::WorkflowDivergence::ActionCount { .. }
    )));

    // Publication is refused without even reaching the approval plane.
    let refusal = evolution.publish_successor(
        &CandidateId::parse("host-divergent-drop-step").expect("id"),
        "eval-divergent",
    );
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::ValidationFailed { .. })
    ));
    // Nothing was published: no releases, no lineage, no new versions.
    assert!(evolution.releases().is_empty());
    assert!(evolution.lineages().is_empty());
}

#[tokio::test]
async fn approval_is_required_and_rejection_is_honored() {
    // No auto-promotion: a passed validation publishes nothing without an
    // explicit approval; a rejection is honored; only an explicit approval
    // publishes. Upgrade is refused until promotion happened.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    let (candidates, ..) = generated_from_evidence(&artifact, env_script()).await;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::DefinitionDelta)
        .expect("definition candidate");
    evolution
        .record_candidate(candidate.clone())
        .expect("recorded");

    // Upgrade before anything happened is refused.
    let upgrade_refusal = evolution.upgrade_installation(
        &candidate.id,
        UpdateDecision::Apply,
        Some("premature".to_string()),
    );
    assert!(matches!(
        upgrade_refusal,
        Err(WorkflowEvolutionError::CandidateNotPromoted { .. })
    ));

    let report = evolution
        .validate_candidate(&candidate.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    assert!(report.passed());

    // No approval yet: publication is refused.
    let missing = evolution.publish_successor(&candidate.id, "eval-unapproved");
    assert!(matches!(
        missing,
        Err(WorkflowEvolutionError::ApprovalRequired { .. })
    ));

    // An explicit rejection is honored and recorded.
    let rejection = evolution
        .decide_approval(
            &candidate.id,
            ApprovalDecision::Reject {
                approver: "bob".to_string(),
                reason: "the description change is premature".to_string(),
            },
        )
        .expect("rejected");
    assert!(!rejection.approves());
    let refused = evolution.publish_successor(&candidate.id, "eval-rejected");
    assert!(matches!(
        refused,
        Err(WorkflowEvolutionError::ApprovalRejected { .. })
    ));
    assert!(evolution.releases().is_empty());

    // Only the explicit approval publishes.
    evolution
        .decide_approval(
            &candidate.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: None,
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&candidate.id, "eval-v1.1.0")
        .expect("published after approval");
    assert_eq!(publication.lineage.approval.approves(), true);
}

#[tokio::test]
async fn the_predecessor_is_immutable() {
    // The published predecessor never changes across a full governed
    // evolution, and mutation attempts are rejected: a tampered incumbent
    // record cannot even construct a governor, and a candidate that would
    // reproduce the incumbent content exactly is refused as a no-op.
    let (artifact, installs) = published_fixture().expect("fixture");
    let predecessor_before = artifact.version.clone();
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    let (candidates, ..) = generated_from_evidence(&artifact, env_script()).await;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::CapabilityBinding)
        .expect("binding candidate");
    evolution
        .record_candidate(candidate.clone())
        .expect("recorded");
    evolution
        .validate_candidate(&candidate.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    evolution
        .decide_approval(
            &candidate.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: None,
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&candidate.id, "eval-v1.1.0")
        .expect("published");
    let successor_id = publication.successor.version_id.clone();

    // The predecessor record is byte-identical and still verifies.
    let predecessor_after = evolution
        .version_record(&artifact.version.version_id)
        .expect("lookup")
        .expect("predecessor record");
    assert_eq!(predecessor_after, &predecessor_before);
    predecessor_after.verify_integrity().expect("integrity");
    // The successor is a different immutable identity.
    assert_ne!(successor_id, artifact.version.version_id);

    // A tampered incumbent record cannot construct a governor at all.
    let mut tampered = artifact.version.clone();
    tampered.definition.description = Some("tampered by the integrity probe".to_string());
    let refusal = EvolutionGovernor::over_incumbent(
        tampered,
        installs.clone(),
        EvalReplayPort::new(action_script(), env_script()),
        PolicyGate::new(EvolutionPolicy::conservative()),
        Box::new(InMemoryApprovalPort::new()),
        RetentionPolicy::default(),
    );
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::Contract(
            codex_workflow_contracts::WorkflowContractError::VersionIntegrity { .. }
        ))
    ));

    // A candidate that reproduces the incumbent content exactly is a
    // no-op, not an evolution.
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    let noop = host_candidate(
        "host-noop",
        &artifact.reference,
        ProposedChange::Definition {
            definition: artifact.version.definition.clone(),
        },
    );
    evolution.record_candidate(noop).expect("recorded");
    let noop_refusal = evolution
        .validate_candidate(
            &CandidateId::parse("host-noop").expect("id"),
            &minor_successor(&artifact.version),
        )
        .await;
    assert!(matches!(
        noop_refusal,
        Err(WorkflowEvolutionError::NoopEvolution { .. })
    ));
}

#[tokio::test]
async fn lineage_and_rollback_transitions_are_recorded() {
    // The full governed succession: publish, upgrade (apply), rollback
    // (reject, then apply) — every transition recorded in the install
    // registry's history, and the lineage pinning predecessor to
    // successor.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    let (candidates, ..) = generated_from_evidence(&artifact, env_script()).await;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::DefinitionDelta)
        .expect("definition candidate");
    evolution
        .record_candidate(candidate.clone())
        .expect("recorded");
    let report = evolution
        .validate_candidate(&candidate.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    assert!(report.passed());
    evolution
        .decide_approval(
            &candidate.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: None,
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&candidate.id, "eval-v1.1.0")
        .expect("published");
    let successor_ref = publication.successor.clone();

    // Lineage: predecessor to successor with the full decision trail.
    assert_eq!(
        publication.lineage.predecessor.version_id,
        artifact.version.version_id
    );
    assert_eq!(
        publication.lineage.successor.version_id,
        successor_ref.version_id
    );
    assert_eq!(publication.lineage.candidate, candidate.id);
    assert_eq!(publication.lineage.release_tag, "eval-v1.1.0");
    assert_eq!(
        publication
            .lineage
            .validation_stages
            .iter()
            .map(|summary| summary.stage)
            .collect::<Vec<_>>(),
        vec![
            StageName::Replay,
            StageName::Differential,
            StageName::Policy
        ]
    );
    assert!(
        publication
            .lineage
            .validation_stages
            .iter()
            .all(|summary| summary.passed)
    );
    assert!(publication.lineage.approval.approves());
    assert_eq!(publication.lineage.adjustment, None);
    assert_eq!(
        publication.release.versions,
        vec![successor_ref.version_id.clone()]
    );
    assert_eq!(publication.release.tag, "eval-v1.1.0");

    // The installation still pins the predecessor until the explicit
    // upgrade decision.
    assert_eq!(
        evolution
            .install_registry()
            .installed(&artifact.version.definition.id)
            .expect("installed")
            .installed
            .version_id,
        artifact.version.version_id
    );
    let upgrade = evolution
        .upgrade_installation(
            &candidate.id,
            UpdateDecision::Apply,
            Some("apply the evidence-backed description refresh".to_string()),
        )
        .expect("upgrade");
    assert!(upgrade.applied);
    assert_eq!(
        evolution
            .install_registry()
            .installed(&artifact.version.definition.id)
            .expect("installed")
            .installed
            .version_id,
        successor_ref.version_id
    );

    // A rejected rollback keeps the installation and is recorded.
    let rejected = evolution
        .rollback_to_predecessor(UpdateDecision::Reject, Some("not yet".to_string()))
        .expect("rejected rollback is recorded");
    assert!(!rejected.applied);
    assert_eq!(
        evolution
            .install_registry()
            .installed(&artifact.version.definition.id)
            .expect("installed")
            .installed
            .version_id,
        successor_ref.version_id
    );

    // The applied rollback moves back to the lineage's predecessor.
    let rollback = evolution
        .rollback_to_predecessor(UpdateDecision::Apply, Some("roll back".to_string()))
        .expect("rollback");
    assert!(rollback.applied);
    assert_eq!(rollback.from, successor_ref.version_id);
    assert_eq!(rollback.to, artifact.version.version_id);
    assert_eq!(
        evolution
            .install_registry()
            .installed(&artifact.version.definition.id)
            .expect("installed")
            .installed
            .version_id,
        artifact.version.version_id
    );

    // The full history is explicit: upgrade applied, rollback rejected,
    // rollback applied.
    let history = evolution
        .install_registry()
        .history_for(&artifact.version.definition.id);
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].applied, true);
    assert_eq!(history[1].applied, false);
    assert_eq!(history[2].applied, true);
    assert_eq!(history[2].to, artifact.version.version_id);

    // No lineage to roll back to anymore: the installed predecessor has
    // none (its succession went forward only once, and that lineage's
    // successor is the version we just left).
    let exhausted = evolution.rollback_to_predecessor(UpdateDecision::Apply, None);
    assert!(matches!(
        exhausted,
        Err(WorkflowEvolutionError::NoPredecessorToRollback { .. })
    ));
}

#[tokio::test]
async fn recovery_adjustments_ride_the_lineage() {
    // Escalation evidence generates a recovery-policy candidate whose
    // governed successor carries the adjustment: publishing and upgrading
    // makes it effective, rolling back removes it — all recorded.
    let (artifact, installs) = published_fixture().expect("fixture");
    let (candidates, evidence, record) =
        generated_from_evidence(&artifact, escalating_env_script()).await;
    let recovery = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::RecoveryPolicy)
        .expect("recovery candidate from escalation evidence");
    assert!(matches!(record.terminal, RunTerminal::Failed { .. }));
    assert_eq!(record.status, WorkflowInstanceStatus::Failed);
    assert_eq!(record.escalations, 1);
    assert_eq!(recovery.provenance.evidence, evidence);

    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());
    evolution
        .record_candidate(recovery.clone())
        .expect("recorded");
    let report = evolution
        .validate_candidate(&recovery.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    // The adjustment does not change version content, so the replay is
    // equivalent and every gate passes.
    assert!(
        report.passed(),
        "the recovery candidate must validate: {report:?}"
    );
    evolution
        .decide_approval(
            &recovery.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: Some("scope resolution to the observed environment".to_string()),
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&recovery.id, "eval-v1.1.0-recovery")
        .expect("published");
    assert_eq!(
        publication.lineage.adjustment,
        Some(AppliedAdjustment::Recovery {
            policy: codex_execution_contracts::BindingPolicy {
                environments: codex_execution_contracts::EnvironmentScope::Only(vec![
                    EVAL_ENVIRONMENT
                ]),
                human_fallback: codex_execution_contracts::HumanFallbackPolicy::Forbidden,
            },
        })
    );

    // The adjustment is effective only once the successor is installed.
    assert_eq!(evolution.effective_adjustment().expect("effective"), None);
    evolution
        .upgrade_installation(&recovery.id, UpdateDecision::Apply, None)
        .expect("upgrade");
    assert_eq!(
        evolution.effective_adjustment().expect("effective"),
        Some(AppliedAdjustment::Recovery {
            policy: codex_execution_contracts::BindingPolicy {
                environments: codex_execution_contracts::EnvironmentScope::Only(vec![
                    EVAL_ENVIRONMENT
                ]),
                human_fallback: codex_execution_contracts::HumanFallbackPolicy::Forbidden,
            },
        })
    );

    // Rolling back removes the adjustment: it rides the version lineage.
    evolution
        .rollback_to_predecessor(UpdateDecision::Apply, Some("restore".to_string()))
        .expect("rollback");
    assert_eq!(evolution.effective_adjustment().expect("effective"), None);
}

#[tokio::test]
async fn dependency_choices_minimize_excess_locks() {
    // An incumbent with resolved lock entries beyond its declared
    // dependencies generates a dependency-choice candidate; the governed
    // successor publishes the minimized lock as a new immutable version.
    let (artifact, _fixture_installs) = published_fixture().expect("fixture");
    let mut excess = DependencyLock::default();
    excess.insert(ResolvedDependency {
        key: DependencyKey::Skill {
            skill: SkillName::parse("unused-skill").expect("skill"),
        },
        resolved: ResolvedDependencyIdentity::Skill {
            digest: ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32))).expect("digest"),
            version: None,
        },
        provenance: Some(DependencyProvenance {
            source: Some("https://github.com/acme/skills".to_string()),
            license: None,
        }),
    });
    let incumbent = WorkflowVersion::seal(
        artifact.version.definition.clone(),
        artifact.version.identity.repository.clone(),
        ImmutableSourceRevision::pin_commit(RevisionSha::parse("ff".repeat(20)).expect("sha")),
        SemanticVersion::new(1, 0, 0),
        excess,
        artifact.version.provenance.clone(),
    )
    .expect("sealed incumbent");
    let mut installs = InstallRegistry::new();
    installs
        .install(&codex_workflow_forge::PublishedVersionRef::of(&incumbent))
        .expect("installed");

    let corpus = EvidenceCorpus::from_observations(
        &incumbent,
        &[],
        Vec::new(),
        observed_environments(),
        None,
    )
    .expect("corpus");
    let candidates = CandidateGenerator::new()
        .generate(&corpus)
        .expect("generated");
    let dependency = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::DependencyChoice)
        .expect("dependency candidate");

    let mut evolution = governor(&incumbent, &installs, RetentionPolicy::default());
    evolution
        .record_candidate(dependency.clone())
        .expect("recorded");
    evolution
        .validate_candidate(&dependency.id, &minor_successor(&incumbent))
        .await
        .expect("validated");
    evolution
        .decide_approval(
            &dependency.id,
            ApprovalDecision::Approve {
                approver: "alice".to_string(),
                note: None,
            },
        )
        .expect("approved");
    let publication = evolution
        .publish_successor(&dependency.id, "eval-v1.1.0-locked")
        .expect("published");
    let successor = evolution
        .version_record(&publication.successor.version_id)
        .expect("lookup")
        .expect("successor record");
    assert!(successor.dependency_lock.entries.is_empty());
    assert_eq!(successor.definition, incumbent.definition);
    assert_ne!(successor.version_id, incumbent.version_id);
    // The incumbent's lock is untouched.
    assert_eq!(
        evolution
            .version_record(&incumbent.version_id)
            .expect("lookup")
            .expect("incumbent")
            .dependency_lock
            .entries
            .len(),
        1
    );
}

#[tokio::test]
async fn retention_bounds_the_ledger() {
    // The candidate ledger is bounded: recording beyond the policy cap
    // evicts the oldest entries first, and the bound is enforced on every
    // insert.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(
        &artifact.version,
        &installs,
        RetentionPolicy {
            max_candidates: 2,
            max_evidence_per_candidate: 8,
        },
    );
    for sequence in 0..4 {
        let candidate = host_candidate(
            &format!("host-retention-{sequence}"),
            &artifact.reference,
            ProposedChange::Schedule {
                spec: codex_workflow_triggers::ScheduleSpec::Every { period_ms: 60_000 },
            },
        );
        evolution.record_candidate(candidate).expect("recorded");
        assert!(evolution.ledger().entries().len() <= 2);
    }
    let ids: Vec<String> = evolution
        .ledger()
        .entries()
        .iter()
        .map(|entry| entry.candidate.id.to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "host-retention-2".to_string(),
            "host-retention-3".to_string(),
        ]
    );
}

#[tokio::test]
async fn credentials_never_enter_candidates_or_approvals() {
    // The scrubbing/checking step refuses credential-shaped content at
    // both entry points: candidate recording and approval decisions.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());

    // A credential-shaped URL in a dependency provenance source is
    // refused at candidate recording.
    let mut contaminated_lock = DependencyLock::default();
    contaminated_lock.insert(ResolvedDependency {
        key: DependencyKey::Skill {
            skill: SkillName::parse("leaky-skill").expect("skill"),
        },
        resolved: ResolvedDependencyIdentity::Skill {
            digest: ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32))).expect("digest"),
            version: None,
        },
        provenance: Some(DependencyProvenance {
            source: Some("https://alice:ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ12@github.com".to_string()),
            license: None,
        }),
    });
    let contaminated = host_candidate(
        "host-leaky-lock",
        &artifact.reference,
        ProposedChange::Dependency {
            lock: contaminated_lock,
        },
    );
    let refusal = evolution.record_candidate(contaminated);
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::CredentialContamination { .. })
    ));

    // A credential-shaped rationale is refused too.
    let mut leaky_rationale = host_candidate(
        "host-leaky-rationale",
        &artifact.reference,
        ProposedChange::Schedule {
            spec: codex_workflow_triggers::ScheduleSpec::Every { period_ms: 60_000 },
        },
    );
    leaky_rationale.rationale = "rotate the key sk-proj-abcdefghijklmnopqrstuvwx soon".to_string();
    let rationale_refusal = evolution.record_candidate(leaky_rationale);
    assert!(matches!(
        rationale_refusal,
        Err(WorkflowEvolutionError::CredentialContamination { .. })
    ));

    // A credential-shaped approval note is refused at the approval port,
    // and the approval still binds to a passed validation.
    let (candidates, ..) = generated_from_evidence(&artifact, env_script()).await;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::DefinitionDelta)
        .expect("definition candidate");
    evolution
        .record_candidate(candidate.clone())
        .expect("recorded");
    evolution
        .validate_candidate(&candidate.id, &minor_successor(&artifact.version))
        .await
        .expect("validated");
    let note_refusal = evolution.decide_approval(
        &candidate.id,
        ApprovalDecision::Approve {
            approver: "alice".to_string(),
            note: Some("approved; secret follow-up -----BEGIN RSA PRIVATE KEY-----".to_string()),
        },
    );
    assert!(matches!(
        note_refusal,
        Err(WorkflowEvolutionError::CredentialContamination { .. })
    ));
    // The candidate still has no approval: publication stays refused.
    let publish_refusal = evolution.publish_successor(&candidate.id, "eval-leaky");
    assert!(matches!(
        publish_refusal,
        Err(WorkflowEvolutionError::ApprovalRequired { .. })
    ));
}

#[tokio::test]
async fn policy_gates_refuse_unauthorized_and_incompatible_changes() {
    // The policy stage fails candidates that introduce undeclared
    // capabilities or propose an incompatible successor version, and
    // failed stages refuse publication.
    let (artifact, installs) = published_fixture().expect("fixture");
    let mut evolution = governor(&artifact.version, &installs, RetentionPolicy::default());

    // A binding change introducing a capability the incumbent never
    // declared: the resource check fails (and the replay cannot resolve
    // the new capability either).
    let node = codex_eval_compat::node_id("step-001").expect("node");
    let escalated = host_candidate(
        "host-escalated-capability",
        &artifact.reference,
        ProposedChange::Bindings {
            bindings: std::collections::BTreeMap::from([(
                node,
                vec![codex_workflow_contracts::CapabilityRequirement {
                    capability: codex_workflow_contracts::CapabilityId::parse(
                        "brand_new_capability",
                    )
                    .expect("capability"),
                    purpose: None,
                }],
            )]),
        },
    );
    evolution.record_candidate(escalated).expect("recorded");
    let report = evolution
        .validate_candidate(
            &CandidateId::parse("host-escalated-capability").expect("id"),
            &minor_successor(&artifact.version),
        )
        .await
        .expect("validation reports");
    assert!(!report.passed());
    let failed = report.failed_stages();
    assert!(
        failed.contains(&"policy"),
        "the capability escalation must fail the policy gate: {failed:?}"
    );

    // A major-bump successor is outside the conservative policy's
    // compatible bumps.
    let (candidates, ..) = generated_from_evidence(&artifact, env_script()).await;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::DefinitionDelta)
        .expect("definition candidate");
    evolution
        .record_candidate(candidate.clone())
        .expect("recorded");
    let major = VersionBump::Major.apply(&artifact.version.identity.semantic_version);
    let report = evolution
        .validate_candidate(&candidate.id, &major)
        .await
        .expect("validation reports");
    assert!(!report.passed());
    assert_eq!(report.failed_stages(), vec!["policy"]);
    let refusal = evolution.publish_successor(&candidate.id, "eval-major");
    assert!(matches!(
        refusal,
        Err(WorkflowEvolutionError::ValidationFailed { .. })
    ));
}
