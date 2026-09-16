//! PACK-005 composition tests.
//!
//! The critical property under test is the one the architecture states
//! verbatim: `Pack A + Pack B → Candidate Pack State` — a new governed
//! candidate — never a mutation of either parent. Beyond the golden path,
//! every conflict class must surface as a DISTINCT typed error, composition
//! must be deterministic, assurance must delegate to PACK-003 composition,
//! and legacy (pre-composition) artifacts must deserialize, verify, and
//! digest exactly as before.

use std::collections::BTreeMap;

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::DevelopmentRef;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::RoleId;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowDependencies;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowRole;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

use crate::ActivationMergeMode;
use crate::ActivationScope;
use crate::ActivationTarget;
use crate::CandidatePackState;
use crate::CapabilityRef;
use crate::ComposePackRequest;
use crate::ComposedLineage;
use crate::ComposedMissionHeader;
use crate::CompositionConflictDetail;
use crate::CompositionRecord;
use crate::CompositionRelation;
use crate::CompositionStrategy;
use crate::ConstitutionRule;
use crate::ConstitutionStatement;
use crate::ContextModel;
use crate::ContextNote;
use crate::ContextualActivation;
use crate::EvaluationRef;
use crate::EvidenceRef;
use crate::HardConstraint;
use crate::Mission;
use crate::MissionAuthor;
use crate::MissionId;
use crate::MissionStatement;
use crate::ModelPin;
use crate::ModelPinningPolicy;
use crate::PackAssurancePolicy;
use crate::PackConstitution;
use crate::PackContractError;
use crate::PackDependencyId;
use crate::PackDependencyKey;
use crate::PackDependencyLock;
use crate::PackId;
use crate::PackPolicy;
use crate::PackPolicyId;
use crate::PackPolicyScope;
use crate::PackPolicySet;
use crate::PackProvenance;
use crate::PackRevisionContent;
use crate::PackRevisionView;
use crate::PackSystemState;
use crate::ParentRevision;
use crate::PolicyRef;
use crate::PolicyStatement;
use crate::PromotedPackState;
use crate::ProvenanceProducer;
use crate::ResolvedPackDependency;
use crate::ResolvedPackDependencyIdentity;
use crate::ValueModel;
use crate::ValueObjective;
use crate::WorkflowVersionRef;
use crate::compose_pack;
use codex_workflow_contracts::SemanticVersion;

const SEALED_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn digest_for(payload: &str) -> ContentDigest {
    ContentDigest::of(&json!({ "payload": payload })).expect("digest computes")
}

/// A minimal real workflow version sealed through the genuine API.
fn real_workflow_version(workflow_name: &str) -> WorkflowVersion {
    let entry = IrNodeId::parse("check").expect("valid node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![],
            roles: vec![RoleId::parse("verifier").expect("valid role id")],
            description: Some("Check the deliverable".to_owned()),
            next: None,
        }),
    );
    let mut roles = BTreeMap::new();
    roles.insert(
        RoleId::parse("verifier").expect("valid role id"),
        WorkflowRole {
            role: RoleId::parse("verifier").expect("valid role id"),
            name: "Verifier".to_owned(),
            objective: "Verify readiness.".to_owned(),
            allowed_capabilities: vec![],
            approval: codex_workflow_contracts::ApprovalRequirement::NotRequired,
        },
    );
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse(workflow_name).expect("valid workflow id"),
        description: Some("Compose-test workflow".to_owned()),
        ir: WorkflowIr {
            ir_format: codex_workflow_contracts::IR_FORMAT_VERSION,
            entry,
            nodes,
            conditions: BTreeMap::new(),
        },
        roles,
        triggers: vec![],
        dependencies: WorkflowDependencies::default(),
    };
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse("https://github.com/acme/pack-fixture").expect("valid repo"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(SEALED_COMMIT).expect("valid sha"),
            DevelopmentRef::branch("main").ok(),
        ),
        "1.0.0".parse().expect("valid semver"),
        DependencyLock::default(),
        /*provenance*/ None,
    )
    .expect("version seals")
}

fn mission_for(pack: &str, statement: &str, context_notes: &[&str]) -> Mission {
    Mission {
        id: MissionId::parse(format!("{pack}-mission")).expect("valid mission id"),
        statement: MissionStatement::parse(statement).expect("valid statement"),
        author: MissionAuthor::user(format!("org:{pack}")).expect("valid principal"),
        value_model: ValueModel {
            objectives: vec![ValueObjective {
                statement: MissionStatement::parse(format!("{pack} primary objective."))
                    .expect("valid statement"),
            }],
        },
        context_model: ContextModel {
            notes: context_notes
                .iter()
                .map(|note| ContextNote {
                    statement: MissionStatement::parse(*note).expect("valid statement"),
                })
                .collect(),
        },
        hard_constraints: vec![HardConstraint {
            statement: MissionStatement::parse("Never store credentials in pack artifacts.")
                .expect("valid statement"),
        }],
        preferences: vec![],
        success_measures: vec![],
    }
}

fn policy_set_for(pack: &str) -> PackPolicySet {
    PackPolicySet::new(
        PackConstitution::new(vec![ConstitutionRule::AuditRequirement {
            statement: ConstitutionStatement::parse(format!(
                "Every {pack} promotion records an audit entry."
            ))
            .expect("valid statement"),
        }])
        .expect("valid constitution"),
        vec![
            PackPolicy::new(
                PackPolicyScope::DependencyUpdates,
                vec![
                    PolicyStatement::parse("Dependency updates require a new candidate revision.")
                        .expect("valid statement"),
                ],
                format!("How {pack} dependencies may be updated."),
                Default::default(),
            )
            .expect("valid policy"),
        ],
    )
    .expect("valid policy set")
}

/// Each pack declares its OWN dependency ids (snake_case, prefixed by the
/// pack name): two packs pinning different workflow versions under their
/// own keys is a union, not a re-pin.
fn lock_pinning(
    pack: &str,
    workflow_version: &WorkflowVersionId,
    capability: &str,
) -> PackDependencyLock {
    let mut lock = PackDependencyLock::default();
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::WorkflowVersion {
            dependency_id: PackDependencyId::parse(format!("{pack}_workflow"))
                .expect("valid dep id"),
        },
        resolved: ResolvedPackDependencyIdentity::WorkflowVersion(workflow_version.clone()),
        content_digest: digest_for(&format!("workflow-definition-{pack}")),
        provenance: None,
    });
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::Capability {
            dependency_id: PackDependencyId::parse(format!("{pack}_capability"))
                .expect("valid dep id"),
        },
        resolved: ResolvedPackDependencyIdentity::Capability(
            CapabilityId::parse(capability).expect("valid capability"),
        ),
        content_digest: digest_for(&format!("capability-implementation-{pack}")),
        provenance: None,
    });
    lock
}

fn state_referencing(workflow_version: &WorkflowVersionId, capability: &str) -> PackSystemState {
    PackSystemState::new(
        vec![WorkflowVersionRef::pin(workflow_version.clone())],
        vec![CapabilityRef::requiring(
            CapabilityId::parse(capability).expect("valid capability"),
        )],
        vec![PolicyRef::validated_against(
            PackPolicyId::from_digest(digest_for(&format!("governance-{capability}"))),
            digest_for("governance-content"),
        )],
        vec![EvaluationRef::from_digest(digest_for("evaluation-1"))],
        vec![EvidenceRef::from_digest(digest_for("evidence-1"))],
        /*rollback_checkpoint*/ None,
    )
    .expect("valid system state")
}

/// A complete root candidate for one pack lineage over a real sealed
/// workflow version.
fn root_candidate(pack: &str, workflow_name: &str, capability: &str) -> CandidatePackState {
    let version = real_workflow_version(workflow_name);
    let content = PackRevisionContent {
        pack_id: PackId::parse(pack).expect("valid pack id"),
        semantic_version: SemanticVersion::new(0, 1, 0),
        mission: mission_for(pack, &format!("Run {pack} operations end to end."), &[]),
        policy_set: policy_set_for(pack),
        dependency_lock: lock_pinning(pack, &version.version_id, capability),
        system_state: state_referencing(&version.version_id, capability),
        composition: None,
    };
    CandidatePackState::propose_root(
        content,
        PackProvenance::root(ProvenanceProducer::user(format!("org:{pack}")).expect("valid")),
    )
    .expect("root candidate proposes")
}

fn promoted_view(candidate: &CandidatePackState) -> PackRevisionView {
    let promoted = PromotedPackState::promote(candidate).expect("candidate promotes");
    PackRevisionView::of_promoted(&promoted).expect("promoted view verifies")
}

fn composed_header() -> ComposedMissionHeader {
    ComposedMissionHeader {
        id: MissionId::parse("atlas-meridian-mission").expect("valid mission id"),
        statement: MissionStatement::parse("Run combined billing and reporting operations.")
            .expect("valid statement"),
        author: MissionAuthor::user("org:atlas").expect("valid principal"),
    }
}

fn orthogonal_request<'a>(
    base: &'a PackRevisionView,
    overlay: &'a PackRevisionView,
) -> ComposePackRequest<'a> {
    ComposePackRequest {
        base,
        overlay,
        relation: CompositionRelation::Orthogonal,
        strategy: CompositionStrategy::strict(),
        lineage: ComposedLineage {
            pack_id: PackId::parse("atlas-meridian").expect("valid pack id"),
            semantic_version: SemanticVersion::new(1, 0, 0),
        },
        mission_header: Some(composed_header()),
        base_assurance: None,
        overlay_assurance: None,
    }
}

fn base_provenance(base_revision: &crate::PackRevisionId) -> PackProvenance {
    PackProvenance::child_of(
        base_revision.clone(),
        ProvenanceProducer::user("org:atlas").expect("valid user"),
    )
}

/// The composed candidate's parent lineage edge for the golden path.
fn composed_parent(base: &PackRevisionView) -> ParentRevision {
    ParentRevision::labeled(
        base.identity
            .revision_id()
            .expect("base revision id derives"),
        "composed-from-base",
    )
    .expect("valid label")
}

#[test]
fn compose_two_packs_yields_a_new_governed_candidate() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let composed = compose_pack(orthogonal_request(&base, &overlay)).expect("composition lands");
    assert_eq!(composed.report.relation, CompositionRelation::Orthogonal);
    assert_eq!(composed.report.workflow_versions, 2, "both workflows union");
    assert_eq!(composed.report.capabilities, 2, "both capabilities union");
    assert_eq!(composed.report.constitution_rules, 2, "both rules union");
    assert_eq!(composed.report.dependency_entries, 4);

    let base_revision_id = base.identity.revision_id().expect("base revision id");
    let candidate = composed
        .into_candidate(composed_parent(&base), base_provenance(&base_revision_id))
        .expect("composed candidate proposes");
    candidate
        .verify_integrity()
        .expect("composed candidate verifies");

    // The full governed lifecycle still applies to composed candidates.
    let promoted = PromotedPackState::promote(&candidate).expect("composed candidate promotes");
    promoted
        .verify_integrity()
        .expect("promoted composed verifies");
    assert_ne!(promoted.revision_id, candidate.revision_id);
}

#[test]
fn composition_never_mutates_its_parents() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let base_digest_before = base.content.system_state.content_digest().expect("digest");
    let overlay_digest_before = overlay
        .content
        .system_state
        .content_digest()
        .expect("digest");
    let base_identity_before = base.identity.revision_id().expect("id");
    let overlay_identity_before = overlay.identity.revision_id().expect("id");

    let _composed = compose_pack(orthogonal_request(&base, &overlay)).expect("composition lands");

    assert_eq!(
        base.content.system_state.content_digest().expect("digest"),
        base_digest_before
    );
    assert_eq!(
        overlay
            .content
            .system_state
            .content_digest()
            .expect("digest"),
        overlay_digest_before
    );
    assert_eq!(
        base.identity.revision_id().expect("id"),
        base_identity_before
    );
    assert_eq!(
        overlay.identity.revision_id().expect("id"),
        overlay_identity_before
    );
}

#[test]
fn identical_inputs_and_strategy_yield_identical_composed_identities() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let first = compose_pack(orthogonal_request(&base, &overlay))
        .expect("first composition lands")
        .into_candidate(
            composed_parent(&base),
            base_provenance(&base.identity.revision_id().expect("base revision id")),
        )
        .expect("first candidate proposes");
    let second = compose_pack(orthogonal_request(&base, &overlay))
        .expect("second composition lands")
        .into_candidate(
            composed_parent(&base),
            base_provenance(&base.identity.revision_id().expect("base revision id")),
        )
        .expect("second candidate proposes");

    assert_eq!(first.revision_id, second.revision_id);

    // A different strategy is a different governed result.
    let mut request = orthogonal_request(&base, &overlay);
    request.strategy = CompositionStrategy {
        activation_merge: ActivationMergeMode::Union,
    };
    let unioned = compose_pack(request)
        .expect("union composition lands")
        .into_candidate(
            composed_parent(&base),
            base_provenance(&base.identity.revision_id().expect("base revision id")),
        )
        .expect("union candidate proposes");
    assert_ne!(first.revision_id, unioned.revision_id);
}

#[test]
fn dependency_repin_is_an_explicit_typed_conflict() {
    // Same declared dependency key, different pinned immutable versions.
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_content = {
        // Build the overlay content pinning a DIFFERENT workflow version
        // under the SAME declared dependency key as the base pack
        // ("atlas_workflow"): a genuine re-pin.
        let version = real_workflow_version("billing-v2");
        PackRevisionContent {
            pack_id: PackId::parse("meridian").expect("valid pack id"),
            semantic_version: SemanticVersion::new(0, 1, 0),
            mission: mission_for("meridian", "Run meridian operations.", &[]),
            policy_set: policy_set_for("meridian"),
            dependency_lock: lock_pinning("atlas", &version.version_id, "navigate_web"),
            system_state: state_referencing(&version.version_id, "navigate_web"),
            composition: None,
        }
        .tap_propose_root()
    };
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_content);

    let error = compose_pack(orthogonal_request(&base, &overlay)).expect_err("re-pin conflicts");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::DependencyRePin { .. }
            ));
        }
        other => panic!("expected a composition conflict, got {other:?}"),
    }
}

#[test]
fn policy_validation_mismatch_is_an_explicit_typed_conflict() {
    // The same policy identity validated against different content in the
    // two parents' states.
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");

    let overlay_candidate = {
        let version = real_workflow_version("reporting");
        let mut state = state_referencing(&version.version_id, "generate_report");
        // Force the same governance policy id as the base, but validated
        // against different content.
        // Same policy identity as the base's governance ref, validated
        // against DIFFERENT content: an explicit composition conflict.
        state.policy_refs[0] = PolicyRef::validated_against(
            PackPolicyId::from_digest(digest_for("governance-navigate_web")),
            digest_for("different-governance-content"),
        );
        let content = PackRevisionContent {
            pack_id: PackId::parse("meridian").expect("valid pack id"),
            semantic_version: SemanticVersion::new(0, 1, 0),
            mission: mission_for("meridian", "Run meridian operations.", &[]),
            policy_set: policy_set_for("meridian"),
            dependency_lock: lock_pinning("meridian", &version.version_id, "generate_report"),
            system_state: state,
            composition: None,
        };
        CandidatePackState::propose_root(
            content,
            PackProvenance::root(ProvenanceProducer::user("org:meridian").expect("valid user")),
        )
        .expect("overlay proposes")
    };

    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);
    let error = compose_pack(orthogonal_request(&base, &overlay))
        .expect_err("validated-content mismatch conflicts");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::PolicyContentMismatch { .. }
            ));
        }
        other => panic!("expected a composition conflict, got {other:?}"),
    }
}

/// A parent candidate whose composition record carries explicit activation
/// rules (the contract shape a previously-composed pack carries).
fn candidate_with_activation_rules(
    pack: &str,
    workflow_name: &str,
    capability: &str,
    notes: &[&str],
    rules: Vec<ContextualActivation>,
) -> CandidatePackState {
    let version = real_workflow_version(workflow_name);
    let record = CompositionRecord {
        base: synthetic_identity("synthetic-base"),
        overlay: synthetic_identity("synthetic-overlay"),
        relation: CompositionRelation::Orthogonal,
        strategy: CompositionStrategy::strict(),
        activations: rules,
    };
    let content = PackRevisionContent {
        pack_id: PackId::parse(pack).expect("valid pack id"),
        semantic_version: SemanticVersion::new(0, 2, 0),
        mission: mission_for(pack, &format!("Run {pack} operations."), notes),
        policy_set: policy_set_for(pack),
        dependency_lock: lock_pinning(pack, &version.version_id, capability),
        system_state: state_referencing(&version.version_id, capability),
        composition: Some(record),
    };
    CandidatePackState::propose_root(
        content,
        PackProvenance::root(ProvenanceProducer::user(format!("org:{pack}")).expect("valid")),
    )
    .expect("candidate with activation rules proposes")
}

fn synthetic_identity(name: &str) -> crate::PackRevisionIdentity {
    crate::PackRevisionIdentity {
        pack: PackId::parse(name).expect("valid pack id"),
        semantic_version: SemanticVersion::new(0, 1, 0),
        system_state_digest: digest_for(name),
        mission_digest: digest_for(name),
        policy_digest: digest_for(name),
        dependency_lock_digest: digest_for(name),
        parent_revision: None,
        composition_digest: None,
    }
}

#[test]
fn activation_conflict_errors_under_strict_and_merges_under_union() {
    let version = real_workflow_version("billing");
    let conditional_note =
        MissionStatement::parse("EU region is active.").expect("valid statement");
    // Both parents pin the SAME workflow (equal locks and states) but
    // disagree on activation: base always (no rule + reference present),
    // overlay conditional.
    let base_candidate = candidate_with_activation_rules(
        "atlas",
        "billing",
        "navigate_web",
        &["EU region is active."],
        vec![], // no rule + reference => always
    );
    let overlay_candidate = candidate_with_activation_rules(
        "meridian",
        "billing",
        "navigate_web",
        &["EU region is active."],
        vec![ContextualActivation {
            target: ActivationTarget::WorkflowVersion(version.version_id),
            scope: ActivationScope::WhenContextNote(conditional_note),
        }],
    );

    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    // Strict: explicit error.
    let error = compose_pack(orthogonal_request(&base, &overlay))
        .expect_err("always vs conditional conflicts under strict");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::ActivationIncompatible { .. }
            ));
        }
        other => panic!("expected an activation conflict, got {other:?}"),
    }

    // Union: always subsumes the conditional rule.
    let mut request = orthogonal_request(&base, &overlay);
    request.strategy = CompositionStrategy {
        activation_merge: ActivationMergeMode::Union,
    };
    let composed = compose_pack(request).expect("union composition lands");
    assert!(
        composed.report.activations.is_empty(),
        "always subsumes the conditional rule under Union"
    );
}

#[test]
fn two_conditions_coexist_under_union_and_conflict_under_strict() {
    let version = real_workflow_version("billing");
    let eu_note = MissionStatement::parse("EU region is active.").expect("valid statement");
    let us_note = MissionStatement::parse("US region is active.").expect("valid statement");

    let base_candidate = candidate_with_activation_rules(
        "atlas",
        "billing",
        "navigate_web",
        &["EU region is active."],
        vec![ContextualActivation {
            target: ActivationTarget::WorkflowVersion(version.version_id.clone()),
            scope: ActivationScope::WhenContextNote(eu_note),
        }],
    );
    let overlay_candidate = candidate_with_activation_rules(
        "meridian",
        "billing",
        "navigate_web",
        &["US region is active."],
        vec![ContextualActivation {
            target: ActivationTarget::WorkflowVersion(version.version_id),
            scope: ActivationScope::WhenContextNote(us_note),
        }],
    );
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let error = compose_pack(orthogonal_request(&base, &overlay))
        .expect_err("two conditions conflict under strict");
    assert!(matches!(
        error,
        PackContractError::CompositionConflict {
            detail: CompositionConflictDetail::ActivationIncompatible { .. }
        }
    ));

    let mut request = orthogonal_request(&base, &overlay);
    request.strategy = CompositionStrategy {
        activation_merge: ActivationMergeMode::Union,
    };
    let composed = compose_pack(request).expect("union composition lands");
    assert_eq!(
        composed.report.activations.len(),
        2,
        "both conditions coexist (OR semantics) under Union"
    );
}

#[test]
fn assurance_composition_delegates_to_pack003_semantics() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    // Equal pins compose.
    let pin = ModelPin::new("atlas-reasoner", digest_for("model-a")).expect("valid pin");
    let base_assurance = PackAssurancePolicy::empty()
        .with_model_pinning(ModelPinningPolicy::Required { pin: pin.clone() });
    let overlay_assurance =
        PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required { pin });

    let mut request = orthogonal_request(&base, &overlay);
    request.base_assurance = Some(&base_assurance);
    request.overlay_assurance = Some(&overlay_assurance);
    let composed = compose_pack(request).expect("equal pins compose");
    let composed_assurance = composed
        .composed_assurance
        .as_ref()
        .expect("assurance composed");
    assert_eq!(
        composed_assurance.model_pinning.pin().map(ModelPin::model),
        Some("atlas-reasoner")
    );
    // The composed assurance policy is pinned by the composed state.
    let composed_id = composed_assurance.policy_id().expect("policy id derives");
    assert!(
        composed
            .content
            .system_state
            .policy_refs
            .iter()
            .any(|r| r.policy_id == composed_id)
    );

    // Different pins conflict.
    let base_pin = ModelPin::new("atlas-reasoner", digest_for("model-a")).expect("valid pin");
    let overlay_pin = ModelPin::new("bertha-coder", digest_for("model-b")).expect("valid pin");
    let base_assurance = PackAssurancePolicy::empty()
        .with_model_pinning(ModelPinningPolicy::Required { pin: base_pin });
    let overlay_assurance = PackAssurancePolicy::empty()
        .with_model_pinning(ModelPinningPolicy::Required { pin: overlay_pin });
    let mut request = orthogonal_request(&base, &overlay);
    request.base_assurance = Some(&base_assurance);
    request.overlay_assurance = Some(&overlay_assurance);
    let error = compose_pack(request).expect_err("different model pins conflict");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::AssuranceIncompatible { .. }
            ));
        }
        other => panic!("expected an assurance conflict, got {other:?}"),
    }
}

#[test]
fn specialization_takes_the_overlay_mission_verbatim() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "billing", "navigate_web");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let mut request = orthogonal_request(&base, &overlay);
    request.relation = CompositionRelation::Specialization;
    request.mission_header = None;
    let composed = compose_pack(request).expect("specialization lands");
    assert_eq!(composed.content.mission, overlay.content.mission);
    assert_eq!(
        composed.report.relation,
        CompositionRelation::Specialization
    );

    // A header under specialization is refused (the overlay governs).
    let mut request = orthogonal_request(&base, &overlay);
    request.relation = CompositionRelation::Specialization;
    request.mission_header = Some(composed_header());
    assert!(matches!(
        compose_pack(request),
        Err(PackContractError::CompositionConflict {
            detail: CompositionConflictDetail::MissionHeaderUnexpected
        })
    ));
}

#[test]
fn orthogonal_mission_merges_by_union_under_a_caller_header() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let mut request = orthogonal_request(&base, &overlay);
    request.mission_header = None;
    assert!(matches!(
        compose_pack(request),
        Err(PackContractError::CompositionConflict {
            detail: CompositionConflictDetail::MissionHeaderRequired
        })
    ));

    let composed =
        compose_pack(orthogonal_request(&base, &overlay)).expect("orthogonal merge lands");
    let header = composed_header();
    assert_eq!(composed.content.mission.id, header.id);
    assert_eq!(composed.content.mission.author, header.author);
    // Both parents' objectives survive the union (deduplicated by text).
    assert_eq!(composed.content.mission.value_model.objectives.len(), 2);
    assert!(
        composed
            .content
            .mission
            .hard_constraints
            .iter()
            // Both parents share the same constraint text: deduplicated.
            .any(|c| c.statement.as_str().contains("credentials"))
    );
}

#[test]
fn legacy_content_without_composition_digests_exactly_as_before() {
    let candidate = root_candidate("atlas", "billing", "navigate_web");
    assert!(candidate.composition.is_none());
    assert!(candidate.composition_digest.is_none());

    // The wire form carries no composition fields at all.
    let wire = serde_json::to_value(&candidate).expect("candidate serializes");
    assert!(wire.get("composition").is_none());
    assert!(wire.get("compositionDigest").is_none());

    // A hand-written legacy candidate JSON (pre-PACK-005 shape, no
    // composition keys) round-trips and verifies.
    let legacy = json!({
        "packId": "atlas",
        "semanticVersion": candidate.semantic_version.to_string(),
        "mission": serde_json::to_value(&candidate.mission).expect("mission"),
        "missionDigest": serde_json::to_value(&candidate.mission_digest).expect("digest"),
        "policySet": serde_json::to_value(&candidate.policy_set).expect("policy"),
        "policyDigest": serde_json::to_value(&candidate.policy_digest).expect("digest"),
        "dependencyLock": serde_json::to_value(&candidate.dependency_lock).expect("lock"),
        "dependencyLockDigest": serde_json::to_value(&candidate.dependency_lock_digest).expect("digest"),
        "systemState": serde_json::to_value(&candidate.system_state).expect("state"),
        "systemStateDigest": serde_json::to_value(&candidate.system_state_digest).expect("digest"),
        "provenance": serde_json::to_value(&candidate.provenance).expect("provenance"),
        "revisionId": serde_json::to_value(&candidate.revision_id).expect("revision"),
    });
    let revived: CandidatePackState =
        serde_json::from_value(legacy).expect("legacy candidate revives");
    revived
        .verify_integrity()
        .expect("legacy candidate verifies");
    assert_eq!(revived.revision_id, candidate.revision_id);
}

#[test]
fn composed_candidate_round_trips_serde() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let composed = compose_pack(orthogonal_request(&base, &overlay)).expect("composition lands");
    let candidate = composed
        .into_candidate(
            composed_parent(&base),
            base_provenance(&base.identity.revision_id().expect("base revision id")),
        )
        .expect("candidate proposes");

    let wire = serde_json::to_string(&candidate).expect("candidate serializes");
    let revived: CandidatePackState = serde_json::from_str(&wire).expect("candidate revives");
    assert_eq!(revived, candidate);
    revived.verify_integrity().expect("revived verifies");

    // Unknown fields are rejected.
    let mut tampered = serde_json::to_value(&candidate).expect("value");
    tampered
        .as_object_mut()
        .expect("object")
        .insert("sneakyField".to_owned(), json!(true));
    assert!(serde_json::from_value::<CandidatePackState>(tampered).is_err());
}

#[test]
fn composition_record_tampering_is_detected() {
    let base_candidate = root_candidate("atlas", "billing", "navigate_web");
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let composed = compose_pack(orthogonal_request(&base, &overlay)).expect("composition lands");
    let mut candidate = composed
        .into_candidate(
            composed_parent(&base),
            base_provenance(&base.identity.revision_id().expect("base revision id")),
        )
        .expect("candidate proposes");

    // Tamper with the composition provenance: claim a different relation.
    if let Some(record) = candidate.composition.as_mut() {
        record.relation = CompositionRelation::Specialization;
    }
    assert!(candidate.verify_integrity().is_err());
}

#[test]
fn dangling_activation_context_is_an_explicit_error() {
    // The base's rule references a note the composed mission does not
    // carry: under Specialization the mission is the overlay's verbatim,
    // which lacks the base's context note.
    let version = real_workflow_version("billing");
    let eu_rule = || ContextualActivation {
        target: ActivationTarget::WorkflowVersion(version.version_id.clone()),
        scope: ActivationScope::WhenContextNote(
            MissionStatement::parse("EU region is active.").expect("valid statement"),
        ),
    };
    let base_candidate = candidate_with_activation_rules(
        "atlas",
        "billing",
        "navigate_web",
        &["EU region is active."],
        vec![eu_rule()],
    );
    // The overlay carries the SAME conditional rule (so the merge is
    // equal-scoped, not incompatible), but its mission — which governs
    // verbatim under Specialization — has no such note.
    let overlay_candidate = candidate_with_activation_rules(
        "meridian",
        "billing",
        "navigate_web",
        &[],
        vec![eu_rule()],
    );
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let mut request = orthogonal_request(&base, &overlay);
    request.relation = CompositionRelation::Specialization;
    request.mission_header = None;
    let error = compose_pack(request).expect_err("dangling activation context conflicts");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::DanglingActivationContext { .. }
            ));
        }
        other => panic!("expected a dangling-context conflict, got {other:?}"),
    }
}

#[test]
fn activation_rule_for_an_absent_target_is_an_explicit_error() {
    // A rule for a workflow the merged state does not reference.
    let absent_version = real_workflow_version("absent-workflow");
    let base_candidate = candidate_with_activation_rules(
        "atlas",
        "billing",
        "navigate_web",
        &["EU region is active."],
        vec![ContextualActivation {
            target: ActivationTarget::WorkflowVersion(absent_version.version_id),
            scope: ActivationScope::WhenContextNote(
                MissionStatement::parse("EU region is active.").expect("valid statement"),
            ),
        }],
    );
    let overlay_candidate = root_candidate("meridian", "reporting", "generate_report");
    let base = promoted_view(&base_candidate);
    let overlay = promoted_view(&overlay_candidate);

    let error = compose_pack(orthogonal_request(&base, &overlay))
        .expect_err("absent activation target conflicts");
    match error {
        PackContractError::CompositionConflict { detail } => {
            assert!(matches!(
                detail,
                CompositionConflictDetail::ActivationTargetMissing { .. }
            ));
        }
        other => panic!("expected a target-missing conflict, got {other:?}"),
    }
}

/// Test-only helper: propose a root candidate from content.
trait ProposeRoot {
    fn tap_propose_root(self) -> CandidatePackState;
}

impl ProposeRoot for PackRevisionContent {
    fn tap_propose_root(self) -> CandidatePackState {
        CandidatePackState::propose_root(
            self,
            PackProvenance::root(ProvenanceProducer::user("org:meridian").expect("valid user")),
        )
        .expect("candidate proposes")
    }
}
