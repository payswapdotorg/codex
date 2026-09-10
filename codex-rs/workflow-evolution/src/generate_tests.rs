//! Generator rule tests: every change category fires from its evidence
//! stream, deterministically, and only from evidence.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_eval_compat::WorkflowRunRecord;
use codex_execution_contracts::ExecutionEnvironment;
use codex_workflow_app::RunTerminal;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyKey;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::ResolvedDependency;
use codex_workflow_contracts::ResolvedDependencyIdentity;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::SkillName;
use codex_workflow_contracts::WorkflowInstanceStatus;
use codex_workflow_triggers::ScheduleSpec;
use pretty_assertions::assert_eq;

use crate::CandidateGenerator;
use crate::ChangeKind;
use crate::EvidenceCorpus;
use crate::test_support::node;
use crate::test_support::probe_definition;
use crate::test_support::probe_version;

/// One synthetic evidence reference of `kind`.
fn reference(kind: EvidenceKind, sequence: usize) -> EvidenceReference {
    EvidenceReference {
        kind,
        locator: format!("unit/{sequence}"),
        digest: ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32))).expect("digest"),
    }
}

/// One synthetic run record of the incumbent.
fn run(
    version: &codex_workflow_contracts::WorkflowVersion,
    escalations: usize,
) -> WorkflowRunRecord {
    WorkflowRunRecord {
        provider_id: "unit-provider".to_string(),
        workflow: version.definition.id.clone(),
        version: version.version_id.clone(),
        terminal: RunTerminal::Completed,
        status: WorkflowInstanceStatus::Succeeded,
        path: Vec::new(),
        actions: Vec::new(),
        evidence_histogram: BTreeMap::new(),
        recovery_histogram: BTreeMap::new(),
        escalations,
        model_requests: 0,
    }
}

/// A corpus over the probe incumbent with the given evidence shape.
fn corpus(
    version: &codex_workflow_contracts::WorkflowVersion,
    references: Vec<EvidenceReference>,
    escalations: usize,
    schedule: Option<ScheduleSpec>,
) -> EvidenceCorpus {
    EvidenceCorpus::from_observations(
        version,
        &[run(version, escalations)],
        references,
        BTreeSet::from([ExecutionEnvironment::Api]),
        schedule,
    )
    .expect("corpus")
}

#[test]
fn an_empty_evidence_stream_generates_nothing() {
    let candidates = CandidateGenerator::new()
        .generate(&corpus(&probe_version(), Vec::new(), 0, None))
        .expect("generated");
    assert_eq!(candidates, Vec::new());
}

#[test]
fn observation_evidence_generates_a_binding_candidate() {
    let incumbent = probe_version();
    let candidates = CandidateGenerator::new()
        .generate(&corpus(
            &incumbent,
            vec![
                reference(EvidenceKind::Observation, 1),
                reference(EvidenceKind::Observation, 2),
            ],
            0,
            None,
        ))
        .expect("generated");
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.change_kind(), ChangeKind::CapabilityBinding);
    assert_eq!(*candidate.workflow(), incumbent.definition.id);
    // Purposes cite the observation evidence; capabilities never change.
    let (definition, _) = candidate
        .change
        .applied_over(&candidate.id, &incumbent)
        .expect("applied");
    let codex_workflow_contracts::WorkflowIrNode::Step(step) =
        &definition.ir.nodes[&node("step-001")]
    else {
        panic!("step node");
    };
    assert_eq!(
        step.capabilities[0].purpose.as_deref(),
        Some("inspect the issues list (evidence: 2 observations)")
    );
    let codex_workflow_contracts::WorkflowIrNode::Step(incumbent_step) =
        &incumbent.definition.ir.nodes[&node("step-001")]
    else {
        panic!("incumbent step node");
    };
    assert_eq!(
        step.capabilities[0].capability,
        incumbent_step.capabilities[0].capability
    );
    // Provenance carries exactly the corpus references and run identity.
    assert_eq!(candidate.provenance.evidence.len(), 2);
    assert_eq!(candidate.provenance.runs.len(), 1);
    assert_eq!(
        candidate.provenance.runs[0].fingerprint,
        run(&incumbent, 0)
            .fingerprint_digest()
            .expect("fingerprint")
    );
}

#[test]
fn trace_evidence_generates_a_definition_candidate() {
    let incumbent = probe_version();
    let candidates = CandidateGenerator::new()
        .generate(&corpus(
            &incumbent,
            vec![reference(EvidenceKind::Trace, 1)],
            0,
            None,
        ))
        .expect("generated");
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.change_kind(), ChangeKind::DefinitionDelta);
    let (definition, _) = candidate
        .change
        .applied_over(&candidate.id, &incumbent)
        .expect("applied");
    assert_eq!(
        definition.description.as_deref(),
        Some("governed evolution from 1 recorded trace(s) and 1 run(s)")
    );
    // The graph itself is untouched: only the description changed.
    assert_eq!(definition.ir, incumbent.definition.ir);
}

#[test]
fn recovery_evidence_generates_a_recovery_candidate() {
    let incumbent = probe_version();
    let candidates = CandidateGenerator::new()
        .generate(&corpus(
            &incumbent,
            vec![
                reference(EvidenceKind::Recovery, 1),
                reference(EvidenceKind::Observation, 2),
            ],
            1,
            None,
        ))
        .expect("generated");
    let recovery = candidates
        .iter()
        .find(|candidate| candidate.change_kind() == ChangeKind::RecoveryPolicy)
        .expect("recovery candidate");
    let crate::ProposedChange::Recovery { policy } = &recovery.change else {
        panic!("recovery change");
    };
    // Scoped to the observed environments; human fallback stays forbidden.
    assert_eq!(
        policy.environments,
        codex_execution_contracts::EnvironmentScope::Only(vec![ExecutionEnvironment::Api])
    );
    assert_eq!(
        policy.human_fallback,
        codex_execution_contracts::HumanFallbackPolicy::Forbidden
    );
}

#[test]
fn schedule_observation_and_healthy_runs_generate_a_schedule_candidate() {
    let incumbent = probe_version();
    let candidates = CandidateGenerator::new()
        .generate(&corpus(
            &incumbent,
            Vec::new(),
            0,
            Some(ScheduleSpec::Every {
                period_ms: 3_600_000,
            }),
        ))
        .expect("generated");
    assert_eq!(candidates.len(), 1);
    let crate::ProposedChange::Schedule { spec } = &candidates[0].change else {
        panic!("schedule change");
    };
    assert_eq!(
        *spec,
        ScheduleSpec::Every {
            period_ms: 1_800_000
        }
    );

    // Escalated or failed runs mean the schedule is not tuned; the
    // escalation instead feeds the recovery rule.
    let unhealthy = {
        let mut corpus = corpus(
            &incumbent,
            Vec::new(),
            1,
            Some(ScheduleSpec::Every {
                period_ms: 3_600_000,
            }),
        );
        corpus.runs[0].status = WorkflowInstanceStatus::Failed;
        corpus
    };
    let generated = CandidateGenerator::new()
        .generate(&unhealthy)
        .expect("generated");
    assert!(
        generated
            .iter()
            .all(|candidate| candidate.change_kind() != ChangeKind::ScheduleTuning),
        "unhealthy runs never generate schedule tuning"
    );
    assert!(
        generated
            .iter()
            .any(|candidate| candidate.change_kind() == ChangeKind::RecoveryPolicy),
        "the escalation feeds the recovery rule instead"
    );
}

#[test]
fn excess_lock_entries_generate_a_dependency_candidate() {
    let incumbent = {
        let mut lock = DependencyLock::default();
        lock.insert(ResolvedDependency {
            key: DependencyKey::Skill {
                skill: SkillName::parse("extra-skill").expect("skill"),
            },
            resolved: ResolvedDependencyIdentity::Skill {
                digest: ContentDigest::try_from(format!("sha256:{}", "ab".repeat(32)))
                    .expect("digest"),
                version: None,
            },
            provenance: None,
        });
        codex_workflow_contracts::WorkflowVersion::seal(
            probe_definition(),
            codex_workflow_contracts::WorkflowRepositoryId::parse(
                "github.com/unit/probe-workflows",
            )
            .expect("repository"),
            codex_workflow_contracts::ImmutableSourceRevision::pin_commit(
                codex_workflow_contracts::RevisionSha::parse("ef".repeat(20)).expect("sha"),
            ),
            SemanticVersion::new(1, 0, 0),
            lock,
            None,
        )
        .expect("sealed")
    };
    let candidates = CandidateGenerator::new()
        .generate(&corpus(&incumbent, Vec::new(), 0, None))
        .expect("generated");
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.change_kind(), ChangeKind::DependencyChoice);
    let (_, lock) = candidate
        .change
        .applied_over(&candidate.id, &incumbent)
        .expect("applied");
    assert!(
        lock.entries.is_empty(),
        "the minimized lock drops the excess entry"
    );
}

#[test]
fn generation_is_deterministic_and_content_addressed() {
    let incumbent = probe_version();
    let evidence = corpus(
        &incumbent,
        vec![reference(EvidenceKind::Observation, 1)],
        0,
        None,
    );
    let first = CandidateGenerator::new()
        .generate(&evidence)
        .expect("generated");
    let second = CandidateGenerator::new()
        .generate(&evidence)
        .expect("generated");
    assert_eq!(first, second);
    // Different evidence streams derive different candidate ids.
    let other = CandidateGenerator::new()
        .generate(&corpus(
            &incumbent,
            vec![
                reference(EvidenceKind::Observation, 1),
                reference(EvidenceKind::Observation, 2),
            ],
            0,
            None,
        ))
        .expect("generated");
    assert_ne!(first[0].id, other[0].id);
}

#[test]
fn corpora_refuse_run_records_of_other_versions() {
    let incumbent = probe_version();
    let other_version = codex_workflow_contracts::WorkflowVersion::seal(
        probe_definition(),
        codex_workflow_contracts::WorkflowRepositoryId::parse("github.com/unit/probe-workflows")
            .expect("repository"),
        codex_workflow_contracts::ImmutableSourceRevision::pin_commit(
            codex_workflow_contracts::RevisionSha::parse("ee".repeat(20)).expect("sha"),
        ),
        SemanticVersion::new(2, 0, 0),
        DependencyLock::default(),
        None,
    )
    .expect("sealed");
    let result = EvidenceCorpus::from_observations(
        &incumbent,
        &[run(&other_version, 0)],
        Vec::new(),
        BTreeSet::from([ExecutionEnvironment::Api]),
        None,
    );
    assert!(matches!(
        result,
        Err(crate::WorkflowEvolutionError::InvalidCandidate { .. })
    ));
}
