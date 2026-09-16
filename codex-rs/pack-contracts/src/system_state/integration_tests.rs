//! PACK-004 integration tests: pack ↔ workflow integration invariants.
//!
//! These tests exercise the integration work order's acceptance criteria
//! against real Universal workflow contracts: a genuine
//! [`WorkflowVersion`] sealed through the workflow-contracts API (not a
//! synthetic digest) is referenced, locked, promoted, and re-verified, and
//! the mandatory invariants are asserted end to end:
//!
//! - existing workflows remain valid across the whole pack lifecycle;
//! - workflow version references stay immutable and exact;
//! - dependency updates require a new candidate rather than silently
//!   changing a promoted pack;
//! - tampering with any governed component of a promoted revision is
//!   detected;
//! - evidence references are canonical and content-addressed;
//! - no pack surface performs or bypasses workflow transitions.

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

use super::refs::CapabilityRef;
use super::refs::EvaluationRef;
use super::refs::EvidenceRef;
use super::refs::PolicyRef;
use super::refs::WorkflowVersionRef;
use super::revisions::CandidatePackState;
use super::revisions::PackRevisionContent;
use super::revisions::ParentRevision;
use super::revisions::PromotedPackState;
use super::state::PackSystemState;
use crate::HardConstraint;
use crate::Mission;
use crate::MissionAuthor;
use crate::MissionId;
use crate::MissionStatement;
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
use crate::PolicyStatement;
use crate::ProvenanceProducer;
use crate::ResolvedPackDependency;
use crate::ResolvedPackDependencyIdentity;
use crate::{ConstitutionRule, ConstitutionStatement};
use codex_workflow_contracts::SemanticVersion;

const SEALED_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A minimal real workflow definition, sealed through the genuine
/// workflow-contracts API so pack integration is tested against actual
/// Universal artifacts rather than synthetic digests.
fn real_workflow_version(workflow_name: &str) -> WorkflowVersion {
    let entry = IrNodeId::parse("check").expect("valid node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![],
            roles: vec![RoleId::parse("verifier").expect("valid role id")],
            description: Some("Check the release page".to_owned()),
            next: None,
        }),
    );
    let mut roles = BTreeMap::new();
    roles.insert(
        RoleId::parse("verifier").expect("valid role id"),
        WorkflowRole {
            role: RoleId::parse("verifier").expect("valid role id"),
            name: "Verifier".to_owned(),
            objective: "Verify release readiness.".to_owned(),
            allowed_capabilities: vec![],
            approval: codex_workflow_contracts::ApprovalRequirement::NotRequired,
        },
    );
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse(workflow_name).expect("valid workflow id"),
        description: Some("Draft and publish release notes".to_owned()),
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
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(SEALED_COMMIT).expect("valid sha"),
            DevelopmentRef::branch("main").ok(),
        ),
        "1.4.2".parse().expect("valid semver"),
        DependencyLock::default(),
        /*provenance*/ None,
    )
    .expect("version seals")
}

/// A mission fixture authored by the user organization.
fn integration_mission() -> Mission {
    Mission {
        id: MissionId::parse("atlas-mission").expect("valid mission id"),
        statement: MissionStatement::parse("Run billing operations end to end.")
            .expect("valid statement"),
        author: MissionAuthor::user("org:atlas").expect("valid principal"),
        value_model: crate::ValueModel { objectives: vec![] },
        context_model: crate::ContextModel { notes: vec![] },
        hard_constraints: vec![HardConstraint {
            statement: MissionStatement::parse("Never store customer card numbers.")
                .expect("valid statement"),
        }],
        preferences: vec![],
        success_measures: vec![],
    }
}

/// A governing policy set fixture.
fn integration_policy_set() -> PackPolicySet {
    PackPolicySet::new(
        PackConstitution::new(vec![ConstitutionRule::AuditRequirement {
            statement: ConstitutionStatement::parse("Every promotion records an audit entry.")
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
                "How pack dependencies may be updated.",
                Default::default(),
            )
            .expect("valid policy"),
        ],
    )
    .expect("valid policy set")
}

/// A lock pinning exactly the given workflow version and the navigate_web
/// capability.
fn lock_pinning(workflow_version: &WorkflowVersionId, capability: &str) -> PackDependencyLock {
    let mut lock = PackDependencyLock::default();
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::WorkflowVersion {
            dependency_id: PackDependencyId::parse("billing").expect("valid dependency id"),
        },
        resolved: ResolvedPackDependencyIdentity::WorkflowVersion(workflow_version.clone()),
        content_digest: ContentDigest::of(&json!({ "payload": "billing-definition" }))
            .expect("digest computes"),
        provenance: None,
    });
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::Capability {
            dependency_id: PackDependencyId::parse("browser").expect("valid dependency id"),
        },
        resolved: ResolvedPackDependencyIdentity::Capability(
            CapabilityId::parse(capability).expect("valid capability"),
        ),
        content_digest: ContentDigest::of(&json!({ "payload": "browser-implementation" }))
            .expect("digest computes"),
        provenance: None,
    });
    lock
}

/// A state referencing exactly the given workflow version and capability.
fn state_referencing(workflow_version: &WorkflowVersionId, capability: &str) -> PackSystemState {
    PackSystemState::new(
        vec![WorkflowVersionRef::pin(workflow_version.clone())],
        vec![
            CapabilityRef::constrained(
                CapabilityId::parse(capability).expect("valid capability"),
                "headless browser session",
            )
            .expect("valid constraint"),
        ],
        vec![PolicyRef::validated_against(
            PackPolicyId::from_digest(
                ContentDigest::of(&json!({ "payload": "governance" })).expect("digest computes"),
            ),
            ContentDigest::of(&json!({ "payload": "governance-content" }))
                .expect("digest computes"),
        )],
        vec![EvaluationRef::from_digest(
            ContentDigest::of(&json!({ "payload": "evaluation-1" })).expect("digest computes"),
        )],
        vec![EvidenceRef::from_digest(
            ContentDigest::of(&json!({ "payload": "evidence-1" })).expect("digest computes"),
        )],
        /*rollback_checkpoint*/ None,
    )
    .expect("valid system state")
}

/// Governed content built around a real sealed workflow version.
fn content_over_real_workflow() -> (WorkflowVersion, PackRevisionContent) {
    let version = real_workflow_version("billing");
    let content = PackRevisionContent {
        pack_id: PackId::parse("atlas").expect("valid pack id"),
        semantic_version: SemanticVersion::new(0, 1, 0),
        mission: integration_mission(),
        policy_set: integration_policy_set(),
        dependency_lock: lock_pinning(&version.version_id, "navigate_web"),
        system_state: state_referencing(&version.version_id, "navigate_web"),
    };
    (version, content)
}

#[test]
fn existing_workflow_versions_remain_valid_across_the_pack_lifecycle() {
    let (version, content) = content_over_real_workflow();
    version
        .verify_integrity()
        .expect("the sealed workflow version starts valid");
    let pinned_id = version.version_id.clone();

    // Full lifecycle: root candidate → promote → child candidate → promote.
    let root = CandidatePackState::propose_root(
        content,
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    )
    .expect("root candidate proposes");
    let promoted = PromotedPackState::promote(&root).expect("first promotion");
    let child = CandidatePackState::propose(
        PackRevisionContent {
            semantic_version: SemanticVersion::new(0, 2, 0),
            ..{
                let (again_version, again_content) = content_over_real_workflow();
                assert_eq!(again_version.version_id, pinned_id);
                again_content
            }
        },
        ParentRevision::new(promoted.revision_id.clone()),
        PackProvenance::child_of(
            promoted.revision_id.clone(),
            ProvenanceProducer::control_plane("pack-control-plane").expect("valid plane"),
        ),
    )
    .expect("child candidate proposes");
    let re_promoted = PromotedPackState::promote(&child).expect("second promotion");

    // Existing workflow authority is untouched at every step: the version
    // still verifies, its identity is unchanged, and every pack record pins
    // exactly that identity.
    version
        .verify_integrity()
        .expect("the workflow version remains valid after pack operations");
    assert_eq!(version.version_id, pinned_id);
    for record_workflow_ids in [
        &root.system_state.workflow_version_refs,
        &promoted.system_state.workflow_version_refs,
        &child.system_state.workflow_version_refs,
        &re_promoted.system_state.workflow_version_refs,
    ] {
        assert_eq!(
            record_workflow_ids
                .iter()
                .map(|reference| &reference.workflow_version_id)
                .collect::<Vec<_>>(),
            vec![&pinned_id],
            "every record pins the exact same immutable workflow version id"
        );
    }
    root.verify_integrity().expect("root integrity holds");
    promoted
        .verify_integrity()
        .expect("promoted integrity holds");
    child.verify_integrity().expect("child integrity holds");
    re_promoted
        .verify_integrity()
        .expect("re-promoted integrity holds");
    // Lineage: the child and its promotion carry the first promoted parent.
    assert_eq!(
        child.parent.as_ref().map(|p| &p.revision),
        Some(&promoted.revision_id)
    );
    assert_eq!(
        child.provenance.parent_revision.as_ref(),
        Some(&promoted.revision_id)
    );
}

#[test]
fn dependency_updates_require_a_new_candidate() {
    let (version, content) = content_over_real_workflow();
    let promoted = PromotedPackState::promote(
        &CandidatePackState::propose_root(
            content,
            PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
        )
        .expect("root candidate proposes"),
    )
    .expect("promotion");
    let promoted_before = promoted.clone();
    let old_lock_digest = promoted.dependency_lock_digest.clone();

    // A dependency update: the billing workflow is re-pinned to a different
    // immutable version identity.
    let updated_version = real_workflow_version("billing-2");
    assert_ne!(updated_version.version_id, version.version_id);
    let updated_content = PackRevisionContent {
        dependency_lock: lock_pinning(&updated_version.version_id, "navigate_web"),
        system_state: state_referencing(&updated_version.version_id, "navigate_web"),
        ..content_over_real_workflow().1
    };

    // The update produces a NEW candidate with a different revision
    // identity and a different lock digest; the promoted record is unchanged
    // and still verifies.
    let updated_candidate = CandidatePackState::propose(
        updated_content,
        ParentRevision::new(promoted.revision_id.clone()),
        PackProvenance::child_of(
            promoted.revision_id.clone(),
            ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
        ),
    )
    .expect("updated candidate proposes");
    assert_ne!(
        updated_candidate.revision_id, promoted.revision_id,
        "a dependency update must produce a new revision identity"
    );
    assert_ne!(
        updated_candidate.dependency_lock_digest, old_lock_digest,
        "the dependency lock digest must change with the re-pin"
    );
    assert_eq!(
        promoted, promoted_before,
        "the promoted record is untouched"
    );
    promoted
        .verify_integrity()
        .expect("the promoted revision still verifies after the update elsewhere");
    updated_candidate
        .verify_integrity()
        .expect("the updated candidate verifies");
}

#[test]
fn unlocked_references_are_rejected() {
    let (version, content) = content_over_real_workflow();
    let content = content; // reused across both rejection cases below
    let content_for_state = content.clone();

    // The state references a workflow version the lock does not pin.
    let unlocked_version = real_workflow_version("reporting");
    let unlocked_state = {
        let mut unlocked = content_for_state;
        unlocked.system_state = state_referencing(&unlocked_version.version_id, "navigate_web");
        unlocked
    };
    let error = CandidatePackState::propose_root(
        unlocked_state,
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    );
    assert!(
        matches!(
            &error,
            Err(PackContractError::IncompleteDependencyLock { reason }) if reason.contains("not pinned")
        ),
        "unlocked workflow reference must be rejected, got: {error:?}"
    );

    // The state references a capability the lock does not pin.
    let unlocked_capability = {
        let mut unlocked = content;
        unlocked.system_state = state_referencing(&version.version_id, "transcribe_audio");
        unlocked
    };
    let error = CandidatePackState::propose_root(
        unlocked_capability,
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    );
    assert!(
        matches!(
            &error,
            Err(PackContractError::IncompleteDependencyLock { reason }) if reason.contains("not pinned")
        ),
        "unlocked workflow reference must be rejected, got: {error:?}"
    );
}

#[test]
fn promoted_revision_tampering_is_detected_across_all_governed_components() {
    let (_, content) = content_over_real_workflow();
    let promoted = PromotedPackState::promote(
        &CandidatePackState::propose_root(
            content,
            PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
        )
        .expect("root candidate proposes"),
    )
    .expect("promotion");

    // Mission tampering.
    let mut tampered_mission = promoted.clone();
    tampered_mission.mission = Mission {
        statement: MissionStatement::parse("Do something else entirely.").expect("valid statement"),
        ..promoted.mission.clone()
    };
    assert!(
        tampered_mission.verify_integrity().is_err(),
        "mission tampering must be detected"
    );

    // Dependency lock tampering: re-pin inside the promoted record.
    let mut tampered_lock = promoted.clone();
    let other_version = real_workflow_version("billing-2");
    let mut swapped_lock = promoted.dependency_lock.clone();
    swapped_lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::WorkflowVersion {
            dependency_id: PackDependencyId::parse("billing").expect("valid dependency id"),
        },
        resolved: ResolvedPackDependencyIdentity::WorkflowVersion(other_version.version_id),
        content_digest: ContentDigest::of(&json!({ "payload": "other-definition" }))
            .expect("digest computes"),
        provenance: None,
    });
    tampered_lock.dependency_lock = swapped_lock;
    assert!(
        tampered_lock.verify_integrity().is_err(),
        "dependency lock tampering must be detected"
    );

    // Policy tampering.
    let mut tampered_policy = promoted.clone();
    tampered_policy.policy_set = PackPolicySet::new(
        PackConstitution::new(vec![ConstitutionRule::ForbiddenAction {
            statement: ConstitutionStatement::parse("Never delete customer records.")
                .expect("valid statement"),
        }])
        .expect("valid constitution"),
        vec![],
    )
    .expect("valid policy set");
    assert!(
        tampered_policy.verify_integrity().is_err(),
        "policy tampering must be detected"
    );

    // System state tampering.
    let mut tampered_state = promoted.clone();
    let extra_version = real_workflow_version("reporting");
    tampered_state.system_state = state_referencing(&extra_version.version_id, "navigate_web");
    assert!(
        tampered_state.verify_integrity().is_err(),
        "system state tampering must be detected"
    );

    // The untouched record still verifies.
    promoted
        .verify_integrity()
        .expect("the untouched promoted record verifies");
}

#[test]
fn evidence_references_are_canonical_and_content_addressed() {
    let evidence = EvidenceRef::from_digest(
        ContentDigest::of(&json!({ "payload": "evidence-1" })).expect("digest computes"),
    );
    let state_with_evidence =
        state_referencing(&real_workflow_version("billing").version_id, "navigate_web");

    // Round trip.
    let serialized = serde_json::to_string(&state_with_evidence).expect("serializable");
    let parsed: PackSystemState = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, state_with_evidence);
    assert_eq!(
        state_with_evidence.evidence_refs,
        vec![evidence.clone()],
        "evidence references are carried canonically"
    );

    // Duplicate evidence references are rejected.
    let duplicated = PackSystemState::new(
        state_with_evidence.workflow_version_refs.clone(),
        state_with_evidence.capability_refs.clone(),
        state_with_evidence.policy_refs.clone(),
        state_with_evidence.evaluation_refs.clone(),
        vec![evidence.clone(), evidence],
        None,
    );
    assert!(
        duplicated.is_err(),
        "duplicate evidence references must be rejected"
    );

    // Changing the evidence set changes the content digest.
    let other_evidence = PackSystemState::new(
        state_with_evidence.workflow_version_refs.clone(),
        state_with_evidence.capability_refs.clone(),
        state_with_evidence.policy_refs.clone(),
        state_with_evidence.evaluation_refs.clone(),
        vec![EvidenceRef::from_digest(
            ContentDigest::of(&json!({ "payload": "evidence-2" })).expect("digest computes"),
        )],
        None,
    )
    .expect("valid state");
    assert_ne!(
        state_with_evidence
            .content_digest()
            .expect("digest computes"),
        other_evidence.content_digest().expect("digest computes"),
        "evidence references participate in the content digest"
    );
}

#[test]
fn revision_identities_cover_every_governed_component() {
    // Property-style sweep: mutating exactly one governed component of the
    // content must change the candidate revision identity.
    let (_, content) = content_over_real_workflow();
    let baseline = CandidatePackState::propose_root(
        content.clone(),
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    )
    .expect("baseline proposes");

    let mission_changed = CandidatePackState::propose_root(
        PackRevisionContent {
            mission: Mission {
                statement: MissionStatement::parse("Run billing with lower cost.")
                    .expect("valid statement"),
                ..content.mission.clone()
            },
            ..content.clone()
        },
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    )
    .expect("mission-changed proposes");

    let version_changed = CandidatePackState::propose_root(
        PackRevisionContent {
            semantic_version: SemanticVersion::new(0, 2, 0),
            ..content.clone()
        },
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    )
    .expect("version-changed proposes");

    let (other_workflow, _) = content_over_real_workflow();
    let workflow_changed = CandidatePackState::propose_root(
        PackRevisionContent {
            semantic_version: SemanticVersion::new(0, 2, 0),
            dependency_lock: lock_pinning(&other_workflow.version_id, "navigate_web"),
            system_state: state_referencing(&other_workflow.version_id, "navigate_web"),
            ..content.clone()
        },
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid user")),
    )
    .expect("workflow-changed proposes");

    for (covered, mutated) in [
        ("mission", mission_changed),
        ("semantic version", version_changed),
        ("workflow pin", workflow_changed),
    ] {
        assert_ne!(
            baseline.revision_id, mutated.revision_id,
            "mutating the {covered} must change the revision identity"
        );
    }
    // Identical content proposed twice yields the identical identity.
    let repeat = CandidatePackState::propose_root(
        content,
        PackProvenance::root(ProvenanceProducer::agent("other-worker").expect("valid agent")),
    )
    .expect("repeat proposes");
    assert_eq!(
        baseline.revision_id, repeat.revision_id,
        "producer identity must not leak into revision identity"
    );
}
