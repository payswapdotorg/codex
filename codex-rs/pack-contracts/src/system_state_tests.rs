//! Tests for the pack system-state contracts.

use std::collections::BTreeSet;

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowVersionId;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

use super::*;
use crate::PackContractError;
use crate::PackId;
use crate::PackPolicyId;
use crate::PackProvenance;
use crate::PackRevisionId;
use crate::ProvenanceProducer;

/// Computes a real-shaped content digest for a named fixture.
fn content_digest(name: &str) -> ContentDigest {
    ContentDigest::of(&json!({ "fixture": name })).expect("digest computation succeeds")
}

/// Computes a real-shaped workflow version identity for a named workflow.
fn workflow_version_id(name: &str) -> WorkflowVersionId {
    WorkflowVersionId::from_digest(content_digest(&format!("workflow-{name}")))
}

/// Computes a real-shaped policy identity for a named policy.
fn policy_id(name: &str) -> PackPolicyId {
    PackPolicyId::from_digest(content_digest(&format!("policy-{name}")))
}

/// Computes a real-shaped pack revision identity for a named revision.
fn pack_revision_id(name: &str) -> PackRevisionId {
    PackRevisionId::from_digest(content_digest(&format!("revision-{name}")))
}

/// The onboarding workflow pin, with a descriptive role.
fn onboarding_ref() -> WorkflowVersionRef {
    WorkflowVersionRef::pin_with_role(workflow_version_id("onboarding"), "user onboarding")
        .expect("valid role")
}

/// The billing workflow pin, without a role.
fn billing_ref() -> WorkflowVersionRef {
    WorkflowVersionRef::pin(workflow_version_id("billing"))
}

/// The navigate_web capability requirement, with a constraint.
fn navigate_web_ref() -> CapabilityRef {
    CapabilityRef::constrained(
        CapabilityId::parse("navigate_web").expect("valid capability id"),
        "headless browser session",
    )
    .expect("valid constraint")
}

/// The governance policy reference.
fn governance_policy_ref() -> PolicyRef {
    PolicyRef::validated_against(
        policy_id("governance"),
        content_digest("governance-policy-content"),
    )
}

/// An evaluation reference for a named suite.
fn evaluation_ref(name: &str) -> EvaluationRef {
    EvaluationRef::from_digest(content_digest(&format!("evaluation-{name}")))
}

/// A rollback checkpoint with explicit components.
fn checkpoint_at(target: &str, state_digest: &str, reason: &str) -> RollbackCheckpoint {
    RollbackCheckpoint::new(
        pack_revision_id(target),
        content_digest(state_digest),
        reason,
    )
    .expect("valid checkpoint")
}

/// Assembles a system state, failing fast on invalid fixtures.
fn state(
    workflow_version_refs: Vec<WorkflowVersionRef>,
    capability_refs: Vec<CapabilityRef>,
    policy_refs: Vec<PolicyRef>,
    evaluation_refs: Vec<EvaluationRef>,
    rollback_checkpoint: Option<RollbackCheckpoint>,
) -> PackSystemState {
    PackSystemState::new(
        workflow_version_refs,
        capability_refs,
        policy_refs,
        evaluation_refs,
        rollback_checkpoint,
    )
    .expect("valid system state")
}

/// A single-reference state with a fully explicit JSON shape.
fn simple_state() -> PackSystemState {
    state(
        vec![onboarding_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    )
}

/// A two-workflow state used for ordering and mutation sweeps.
fn base_state() -> PackSystemState {
    state(
        vec![onboarding_ref(), billing_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    )
}

/// A child candidate of revision v17 with labeled lineage.
fn child_candidate() -> CandidatePackState {
    let parent = pack_revision_id("v17");
    CandidatePackState::propose(
        PackId::parse("atlas").expect("valid pack id"),
        ParentRevision::labeled(parent.clone(), "weekly baseline").expect("valid label"),
        simple_state(),
        PackProvenance::child_of(
            parent,
            ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
        ),
    )
    .expect("valid candidate")
}

#[test]
fn workflow_version_refs_pin_exact_identities_and_round_trip() {
    let pinned = workflow_version_id("onboarding");
    let bare = WorkflowVersionRef::pin(pinned.clone());
    let with_role =
        WorkflowVersionRef::pin_with_role(pinned.clone(), "user onboarding").expect("valid role");

    assert_eq!(bare.workflow_version_id, pinned);
    assert_eq!(bare.role, None);
    assert_eq!(
        with_role,
        WorkflowVersionRef {
            workflow_version_id: pinned.clone(),
            role: Some("user onboarding".to_owned()),
        },
    );

    let serialized = serde_json::to_value(&with_role).expect("serializable");
    assert_eq!(
        serialized,
        json!({
            "workflowVersionId": pinned.to_string(),
            "role": "user onboarding",
        }),
    );
    assert_eq!(
        serde_json::from_value::<WorkflowVersionRef>(serialized).expect("deserializable"),
        with_role,
    );
    assert_eq!(
        serde_json::to_value(&bare).expect("serializable"),
        json!({ "workflowVersionId": pinned.to_string() }),
    );
}

#[test]
fn workflow_version_refs_reject_blank_and_overlong_roles() {
    for invalid in ["", "  ", " onboarding", "onboarding "] {
        assert!(
            matches!(
                WorkflowVersionRef::pin_with_role(workflow_version_id("onboarding"), invalid),
                Err(PackContractError::InvalidIdentifier { .. })
            ),
            "role `{invalid}` must be rejected"
        );
    }
    let overlong = "x".repeat(513);
    assert!(
        matches!(
            WorkflowVersionRef::pin_with_role(workflow_version_id("onboarding"), overlong),
            Err(PackContractError::InvalidIdentifier { .. })
        ),
        "a 513-character role must be rejected"
    );
}

#[test]
fn workflow_version_ref_identity_is_bearing() {
    let onboarding = WorkflowVersionRef::pin(workflow_version_id("onboarding"));
    let billing = WorkflowVersionRef::pin(workflow_version_id("billing"));
    assert_ne!(onboarding, billing);
    assert_ne!(
        onboarding.workflow_version_id.to_string(),
        billing.workflow_version_id.to_string()
    );

    // The role is descriptive: it distinguishes records but never the pin.
    let labeled =
        WorkflowVersionRef::pin_with_role(workflow_version_id("onboarding"), "onboarding flow")
            .expect("valid role");
    assert_ne!(onboarding, labeled);
    assert_eq!(onboarding.workflow_version_id, labeled.workflow_version_id);
}

#[test]
fn capability_refs_reuse_universal_capability_identities() {
    let capability = CapabilityId::parse("navigate_web").expect("valid capability id");
    let required = CapabilityRef::requiring(capability.clone());
    let constrained = CapabilityRef::constrained(capability.clone(), "headless browser session")
        .expect("valid constraint");

    assert_eq!(
        required,
        CapabilityRef {
            capability: capability.clone(),
            constraint: None,
        },
    );
    assert_eq!(
        constrained,
        CapabilityRef {
            capability,
            constraint: Some("headless browser session".to_owned()),
        },
    );

    let serialized = serde_json::to_value(&constrained).expect("serializable");
    assert_eq!(
        serialized,
        json!({
            "capability": "navigate_web",
            "constraint": "headless browser session",
        }),
    );
    assert_eq!(
        serde_json::from_value::<CapabilityRef>(serialized).expect("deserializable"),
        constrained,
    );
    assert_eq!(
        serde_json::to_value(&required).expect("serializable"),
        json!({ "capability": "navigate_web" }),
    );
}

#[test]
fn capability_refs_reject_blank_constraints() {
    let capability = CapabilityId::parse("navigate_web").expect("valid capability id");
    for blank in ["", "  ", "headless "] {
        assert!(
            matches!(
                CapabilityRef::constrained(capability.clone(), blank),
                Err(PackContractError::InvalidIdentifier { .. })
            ),
            "constraint `{blank}` must be rejected"
        );
    }
}

#[test]
fn policy_refs_pin_policy_identity_and_validated_content() {
    let governance = policy_id("governance");
    let content = content_digest("governance-policy-content");
    let reference = PolicyRef::validated_against(governance.clone(), content.clone());

    assert_eq!(
        reference,
        PolicyRef {
            policy_id: governance.clone(),
            validated_content_digest: content.clone(),
        },
    );

    let serialized = serde_json::to_value(&reference).expect("serializable");
    assert_eq!(
        serialized,
        json!({
            "policyId": governance.to_string(),
            "validatedContentDigest": content.to_string(),
        }),
    );
    assert_eq!(
        serde_json::from_value::<PolicyRef>(serialized).expect("deserializable"),
        reference,
    );
}

#[test]
fn evaluation_refs_are_opaque_content_addresses() {
    let digest = content_digest("evaluation-suite-1");
    let reference = EvaluationRef::from_digest(digest.clone());
    assert_eq!(reference.digest(), &digest);
    assert_eq!(reference.to_string(), digest.to_string());
    assert_eq!(reference.as_ref(), digest.as_str());

    let serialized = serde_json::to_string(&reference).expect("serializable");
    assert_eq!(serialized, format!("\"{digest}\""));
    assert_eq!(
        serde_json::from_str::<EvaluationRef>(&serialized).expect("deserializable"),
        reference,
    );

    for malformed in ["", "sha256:", "deadbeef", "not-a-digest"] {
        assert!(
            matches!(
                EvaluationRef::try_from(malformed),
                Err(PackContractError::InvalidDigest { .. })
            ),
            "evaluation reference `{malformed}` must be rejected"
        );
    }
}

#[test]
fn rollback_checkpoints_round_trip_and_validate_reasons() {
    let target = pack_revision_id("v17");
    let state_digest = content_digest("state-v17");
    let checkpoint =
        RollbackCheckpoint::new(target.clone(), state_digest.clone(), "prior promoted state")
            .expect("valid reason");

    assert_eq!(
        checkpoint,
        RollbackCheckpoint {
            target_revision: target.clone(),
            state_digest: state_digest.clone(),
            reason: "prior promoted state".to_owned(),
        },
    );

    let serialized = serde_json::to_value(&checkpoint).expect("serializable");
    assert_eq!(
        serialized,
        json!({
            "targetRevision": target.to_string(),
            "stateDigest": state_digest.to_string(),
            "reason": "prior promoted state",
        }),
    );
    assert_eq!(
        serde_json::from_value::<RollbackCheckpoint>(serialized).expect("deserializable"),
        checkpoint,
    );

    for blank in ["", "   ", " rollback point"] {
        assert!(
            matches!(
                RollbackCheckpoint::new(target.clone(), state_digest.clone(), blank),
                Err(PackContractError::InvalidIdentifier { .. })
            ),
            "reason `{blank}` must be rejected"
        );
    }
}

#[test]
fn parent_revisions_round_trip_and_validate_labels() {
    let parent_id = pack_revision_id("v17");
    let unlabeled = ParentRevision::new(parent_id.clone());
    let labeled =
        ParentRevision::labeled(parent_id.clone(), "weekly baseline").expect("valid label");

    assert_eq!(
        unlabeled,
        ParentRevision {
            revision: parent_id.clone(),
            label: None,
        },
    );
    assert_eq!(
        labeled,
        ParentRevision {
            revision: parent_id.clone(),
            label: Some("weekly baseline".to_owned()),
        },
    );
    assert_eq!(
        serde_json::to_value(&labeled).expect("serializable"),
        json!({ "revision": parent_id.to_string(), "label": "weekly baseline" }),
    );
    assert_eq!(
        serde_json::to_value(&unlabeled).expect("serializable"),
        json!({ "revision": parent_id.to_string() }),
    );
    assert!(
        matches!(
            ParentRevision::labeled(parent_id, " "),
            Err(PackContractError::InvalidIdentifier { .. })
        ),
        "a blank label must be rejected"
    );
}

#[test]
fn identical_states_have_identical_digests() {
    let first = base_state();
    let second = base_state();
    assert_eq!(first, second);
    assert_eq!(
        first.content_digest().expect("digest computation succeeds"),
        second
            .content_digest()
            .expect("digest computation succeeds"),
    );
}

#[test]
fn empty_states_are_valid_and_deterministic() {
    let first = state(
        vec![],
        vec![],
        vec![],
        vec![],
        /*rollback_checkpoint*/ None,
    );
    let second = state(
        vec![],
        vec![],
        vec![],
        vec![],
        /*rollback_checkpoint*/ None,
    );
    assert_eq!(first, second);
    first.validate().expect("empty state is canonical");
    assert_eq!(
        first.content_digest().expect("digest computation succeeds"),
        second
            .content_digest()
            .expect("digest computation succeeds"),
    );
}

#[test]
fn state_digest_is_independent_of_reference_insertion_order() {
    let inserted = state(
        vec![onboarding_ref(), billing_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    let reversed = state(
        vec![billing_ref(), onboarding_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert_eq!(inserted, reversed);
    assert_eq!(
        inserted
            .content_digest()
            .expect("digest computation succeeds"),
        reversed
            .content_digest()
            .expect("digest computation succeeds"),
    );
}

#[test]
fn mutation_sweep_produces_distinct_state_digests() {
    let base = base_state();
    let base_digest = base.content_digest().expect("digest computation succeeds");
    let workflows = || vec![onboarding_ref(), billing_ref()];
    let capabilities = || vec![navigate_web_ref()];
    let policies = || vec![governance_policy_ref()];
    let evaluations = || vec![evaluation_ref("suite-1")];

    let mutations: Vec<(&str, PackSystemState)> = vec![
        (
            "workflow ref added",
            state(
                vec![
                    onboarding_ref(),
                    billing_ref(),
                    WorkflowVersionRef::pin(workflow_version_id("reporting")),
                ],
                capabilities(),
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "workflow pin changed",
            state(
                vec![
                    WorkflowVersionRef::pin_with_role(
                        workflow_version_id("onboarding-2"),
                        "user onboarding",
                    )
                    .expect("valid role"),
                    billing_ref(),
                ],
                capabilities(),
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "workflow ref removed",
            state(
                vec![onboarding_ref()],
                capabilities(),
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "workflow role changed",
            state(
                vec![
                    WorkflowVersionRef::pin_with_role(
                        workflow_version_id("onboarding"),
                        "tenant onboarding",
                    )
                    .expect("valid role"),
                    billing_ref(),
                ],
                capabilities(),
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "workflow role added",
            state(
                vec![
                    onboarding_ref(),
                    WorkflowVersionRef::pin_with_role(
                        workflow_version_id("billing"),
                        "billing flow",
                    )
                    .expect("valid role"),
                ],
                capabilities(),
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "capability added",
            state(
                workflows(),
                vec![
                    navigate_web_ref(),
                    CapabilityRef::requiring(
                        CapabilityId::parse("read_document").expect("valid capability id"),
                    ),
                ],
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "capability removed",
            state(
                workflows(),
                vec![],
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "capability constraint changed",
            state(
                workflows(),
                vec![
                    CapabilityRef::constrained(
                        CapabilityId::parse("navigate_web").expect("valid capability id"),
                        "desktop browser session",
                    )
                    .expect("valid constraint"),
                ],
                policies(),
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "policy added",
            state(
                workflows(),
                capabilities(),
                vec![
                    governance_policy_ref(),
                    PolicyRef::validated_against(
                        policy_id("privacy"),
                        content_digest("privacy-policy-content"),
                    ),
                ],
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "policy removed",
            state(
                workflows(),
                capabilities(),
                vec![],
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "policy validated digest changed",
            state(
                workflows(),
                capabilities(),
                vec![PolicyRef::validated_against(
                    policy_id("governance"),
                    content_digest("governance-policy-content-2"),
                )],
                evaluations(),
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "evaluation added",
            state(
                workflows(),
                capabilities(),
                policies(),
                vec![evaluation_ref("suite-1"), evaluation_ref("suite-2")],
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "evaluation changed",
            state(
                workflows(),
                capabilities(),
                policies(),
                vec![evaluation_ref("suite-2")],
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "evaluation removed",
            state(
                workflows(),
                capabilities(),
                policies(),
                vec![],
                /*rollback_checkpoint*/ None,
            ),
        ),
        (
            "rollback checkpoint added",
            state(
                workflows(),
                capabilities(),
                policies(),
                evaluations(),
                Some(checkpoint_at(
                    "v17",
                    "state-v17",
                    "prior promoted state retained for rollback",
                )),
            ),
        ),
        (
            "checkpoint target revision changed",
            state(
                workflows(),
                capabilities(),
                policies(),
                evaluations(),
                Some(checkpoint_at(
                    "v18",
                    "state-v17",
                    "prior promoted state retained for rollback",
                )),
            ),
        ),
        (
            "checkpoint state digest changed",
            state(
                workflows(),
                capabilities(),
                policies(),
                evaluations(),
                Some(checkpoint_at(
                    "v17",
                    "state-v16",
                    "prior promoted state retained for rollback",
                )),
            ),
        ),
        (
            "checkpoint reason changed",
            state(
                workflows(),
                capabilities(),
                policies(),
                evaluations(),
                Some(checkpoint_at(
                    "v17",
                    "state-v17",
                    "regulatory baseline retained for rollback",
                )),
            ),
        ),
    ];

    let mut digests = BTreeSet::from([base_digest]);
    for (name, mutated) in mutations {
        assert_ne!(mutated, base, "mutation `{name}` must change the state");
        let digest = mutated
            .content_digest()
            .unwrap_or_else(|error| panic!("mutation `{name}` must digest: {error}"));
        assert!(
            digests.insert(digest),
            "mutation `{name}` must produce a digest distinct from every other case"
        );
    }
    assert_eq!(
        digests.len(),
        19,
        "base state plus every mutation must yield distinct digests"
    );
}

#[test]
fn states_reject_duplicate_references() {
    let duplicate_workflows = PackSystemState::new(
        vec![
            onboarding_ref(),
            // Same pinned workflow version with a different descriptive shape.
            WorkflowVersionRef::pin(workflow_version_id("onboarding")),
        ],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert!(
        matches!(
            duplicate_workflows,
            Err(PackContractError::InvalidIdentifier {
                kind: "workflow version reference",
                ..
            })
        ),
        "a duplicate workflow version pin must be rejected"
    );

    let duplicate_capability = PackSystemState::new(
        vec![onboarding_ref()],
        vec![
            navigate_web_ref(),
            CapabilityRef::requiring(
                CapabilityId::parse("navigate_web").expect("valid capability id"),
            ),
        ],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert!(
        matches!(
            duplicate_capability,
            Err(PackContractError::InvalidIdentifier {
                kind: "capability reference",
                ..
            })
        ),
        "a duplicate capability reference must be rejected"
    );

    let duplicate_policy = PackSystemState::new(
        vec![onboarding_ref()],
        vec![navigate_web_ref()],
        vec![
            governance_policy_ref(),
            // Same policy identity validated against different content.
            PolicyRef::validated_against(policy_id("governance"), content_digest("other-content")),
        ],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert!(
        matches!(
            duplicate_policy,
            Err(PackContractError::InvalidIdentifier {
                kind: "policy reference",
                ..
            })
        ),
        "a duplicate policy reference must be rejected"
    );

    let duplicate_evaluation = PackSystemState::new(
        vec![onboarding_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1"), evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert!(
        matches!(
            duplicate_evaluation,
            Err(PackContractError::InvalidIdentifier {
                kind: "evaluation reference",
                ..
            })
        ),
        "a duplicate evaluation reference must be rejected"
    );
}

#[test]
fn validate_rejects_non_canonical_and_duplicate_deserialized_states() {
    // Constructors cannot produce these shapes, so they are deserialized from
    // crafted JSON the way a forged wire record would arrive.
    let onboarding = serde_json::to_value(onboarding_ref()).expect("serializable");

    let duplicate = json!({
        "workflowVersionRefs": [onboarding.clone(), onboarding],
        "capabilityRefs": [],
        "policyRefs": [],
        "evaluationRefs": [],
    });
    let duplicate_state: PackSystemState =
        serde_json::from_value(duplicate).expect("deserializable");
    assert!(
        matches!(
            duplicate_state.validate(),
            Err(PackContractError::InvalidIdentifier {
                kind: "workflow version reference",
                ..
            })
        ),
        "a deserialized duplicate reference must fail validation"
    );

    let canonical = state(
        vec![onboarding_ref(), billing_ref()],
        vec![],
        vec![],
        vec![],
        /*rollback_checkpoint*/ None,
    );
    canonical
        .validate()
        .expect("constructor output is canonical");
    let first = serde_json::to_value(&canonical.workflow_version_refs[0]).expect("serializable");
    let second = serde_json::to_value(&canonical.workflow_version_refs[1]).expect("serializable");
    let reversed = json!({
        "workflowVersionRefs": [second, first],
        "capabilityRefs": [],
        "policyRefs": [],
        "evaluationRefs": [],
    });
    let reversed_state: PackSystemState = serde_json::from_value(reversed).expect("deserializable");
    assert!(
        matches!(
            reversed_state.validate(),
            Err(PackContractError::RevisionIntegrity { .. })
        ),
        "a deserialized state in non-canonical order must fail validation"
    );
}

#[test]
fn state_serde_round_trips_with_and_without_checkpoint() {
    let bare = simple_state();
    let bare_json = serde_json::to_value(&bare).expect("serializable");
    assert_eq!(
        bare_json,
        json!({
            "workflowVersionRefs": [
                {
                    "workflowVersionId": workflow_version_id("onboarding").to_string(),
                    "role": "user onboarding",
                },
            ],
            "capabilityRefs": [
                {
                    "capability": "navigate_web",
                    "constraint": "headless browser session",
                },
            ],
            "policyRefs": [
                {
                    "policyId": policy_id("governance").to_string(),
                    "validatedContentDigest": content_digest("governance-policy-content")
                        .to_string(),
                },
            ],
            "evaluationRefs": [evaluation_ref("suite-1").to_string()],
        }),
    );
    assert_eq!(
        serde_json::from_value::<PackSystemState>(bare_json).expect("deserializable"),
        bare,
    );

    let with_checkpoint = state(
        vec![onboarding_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        Some(checkpoint_at("v17", "state-v17", "prior promoted state")),
    );
    let checkpoint_json = serde_json::to_value(&with_checkpoint).expect("serializable");
    assert_eq!(
        checkpoint_json["rollbackCheckpoint"],
        json!({
            "targetRevision": pack_revision_id("v17").to_string(),
            "stateDigest": content_digest("state-v17").to_string(),
            "reason": "prior promoted state",
        }),
    );
    assert_eq!(
        serde_json::from_value::<PackSystemState>(checkpoint_json).expect("deserializable"),
        with_checkpoint,
    );

    let injected = json!({
        "workflowVersionRefs": [],
        "capabilityRefs": [],
        "policyRefs": [],
        "evaluationRefs": [],
        "injected": 1,
    });
    assert!(
        serde_json::from_value::<PackSystemState>(injected).is_err(),
        "unknown fields must be rejected"
    );

    let missing = json!({
        "workflowVersionRefs": [],
        "policyRefs": [],
        "evaluationRefs": [],
    });
    assert!(
        serde_json::from_value::<PackSystemState>(missing).is_err(),
        "missing reference vectors must be rejected"
    );
}

#[test]
fn root_candidate_identity_is_deterministic_and_producer_independent() {
    let pack_id = PackId::parse("atlas").expect("valid pack id");
    let first = CandidatePackState::propose_root(
        pack_id.clone(),
        base_state(),
        PackProvenance::root(ProvenanceProducer::user("org:atlas").expect("valid subject")),
    )
    .expect("valid candidate");
    let second = CandidatePackState::propose_root(
        pack_id,
        base_state(),
        PackProvenance::root(
            ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
        ),
    )
    .expect("valid candidate");

    // Provenance differs, so the records differ...
    assert_ne!(first, second);
    // ...but identical proposed content yields the identical revision identity.
    assert_eq!(first.revision_id, second.revision_id);
    assert_eq!(first.system_state_digest, second.system_state_digest);
    assert_eq!(
        first.system_state_digest,
        first
            .system_state
            .content_digest()
            .expect("digest computation succeeds"),
    );
    first.verify_integrity().expect("integrity holds");
    second.verify_integrity().expect("integrity holds");
}

#[test]
fn child_candidate_preserves_parent_lineage() {
    let candidate = child_candidate();
    let parent_id = pack_revision_id("v17");
    assert_eq!(
        candidate.parent,
        Some(ParentRevision {
            revision: parent_id.clone(),
            label: Some("weekly baseline".to_owned()),
        }),
    );
    assert_eq!(candidate.provenance.parent_revision, Some(parent_id));
    candidate.verify_integrity().expect("integrity holds");
}

#[test]
fn candidates_reject_lineage_disagreement() {
    let pack_id = PackId::parse("atlas").expect("valid pack id");
    let parent_id = pack_revision_id("v17");
    let agent = || ProvenanceProducer::agent("pack-architect-worker").expect("valid agent");

    let missing_provenance_parent = CandidatePackState::propose(
        pack_id.clone(),
        ParentRevision::new(parent_id.clone()),
        base_state(),
        PackProvenance::root(agent()),
    );
    assert!(
        matches!(
            missing_provenance_parent,
            Err(PackContractError::RevisionIntegrity { .. })
        ),
        "a parent record without a matching provenance parent must be rejected"
    );

    let missing_parent_record = CandidatePackState::propose_root(
        pack_id,
        base_state(),
        PackProvenance::child_of(parent_id, agent()),
    );
    assert!(
        matches!(
            missing_parent_record,
            Err(PackContractError::RevisionIntegrity { .. })
        ),
        "a provenance parent without a matching parent record must be rejected"
    );
}

#[test]
fn candidate_verify_integrity_detects_tampering() {
    let candidate = child_candidate();
    candidate.verify_integrity().expect("integrity holds");

    let mut swapped_workflow_pin = candidate.clone();
    swapped_workflow_pin.system_state = state(
        vec![WorkflowVersionRef::pin(workflow_version_id(
            "onboarding-swapped",
        ))],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );

    let mut swapped_state_digest = candidate.clone();
    swapped_state_digest.system_state_digest = content_digest("forged-state-digest");

    let mut swapped_revision_id = candidate.clone();
    swapped_revision_id.revision_id = pack_revision_id("forged");

    let mut swapped_parent = candidate.clone();
    let forged_parent = pack_revision_id("v16");
    swapped_parent.parent = Some(ParentRevision::new(forged_parent.clone()));
    swapped_parent.provenance = PackProvenance::child_of(
        forged_parent,
        ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
    );

    let mut stripped_provenance_parent = candidate.clone();
    stripped_provenance_parent.provenance = PackProvenance::root(
        ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
    );

    let mut injected_checkpoint = candidate;
    injected_checkpoint.system_state = state(
        vec![onboarding_ref()],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        Some(checkpoint_at("v17", "state-v17", "injected checkpoint")),
    );

    let tampered_records = [
        ("swapped workflow pin", swapped_workflow_pin),
        ("swapped state digest", swapped_state_digest),
        ("swapped revision id", swapped_revision_id),
        ("swapped parent revision", swapped_parent),
        ("stripped provenance parent", stripped_provenance_parent),
        ("injected rollback checkpoint", injected_checkpoint),
    ];
    for (name, tampered) in tampered_records {
        assert!(
            tampered.verify_integrity().is_err(),
            "tampered candidate (`{name}`) must fail integrity verification"
        );
    }
}

#[test]
fn candidate_serde_round_trips() {
    let candidate = child_candidate();
    let serialized = serde_json::to_value(&candidate).expect("serializable");
    let parent_id = pack_revision_id("v17");
    assert_eq!(
        serialized,
        json!({
            "packId": "atlas",
            "parentRevision": {
                "revision": parent_id.to_string(),
                "label": "weekly baseline",
            },
            "systemState": {
                "workflowVersionRefs": [
                    {
                        "workflowVersionId": workflow_version_id("onboarding").to_string(),
                        "role": "user onboarding",
                    },
                ],
                "capabilityRefs": [
                    {
                        "capability": "navigate_web",
                        "constraint": "headless browser session",
                    },
                ],
                "policyRefs": [
                    {
                        "policyId": policy_id("governance").to_string(),
                        "validatedContentDigest": content_digest("governance-policy-content")
                            .to_string(),
                    },
                ],
                "evaluationRefs": [evaluation_ref("suite-1").to_string()],
            },
            "systemStateDigest": candidate.system_state_digest.to_string(),
            "provenance": {
                "parentRevision": parent_id.to_string(),
                "producer": { "kind": "agent", "agent": "pack-architect-worker" },
            },
            "revisionId": candidate.revision_id.to_string(),
        }),
    );
    assert_eq!(
        serde_json::from_value::<CandidatePackState>(serialized).expect("deserializable"),
        candidate,
    );

    let mut injected = serde_json::to_value(&candidate).expect("serializable");
    injected["injected"] = json!(1);
    assert!(
        serde_json::from_value::<CandidatePackState>(injected).is_err(),
        "unknown fields must be rejected"
    );
}

#[test]
fn promotion_creates_a_new_revision_and_preserves_the_candidate() {
    let candidate = child_candidate();
    let before_promotion = candidate.clone();
    let promoted = PromotedPackState::promote(&candidate).expect("promotion succeeds");

    // The candidate record is untouched by promotion.
    assert_eq!(candidate, before_promotion);

    // The promoted revision is a NEW identity, distinct from the candidate's.
    assert_ne!(promoted.revision_id, candidate.revision_id);
    assert_eq!(promoted.promoted_from, candidate.revision_id);

    // Content, lineage, and provenance carry over — provenance verbatim.
    assert_eq!(promoted.pack_id, candidate.pack_id);
    assert_eq!(promoted.parent, candidate.parent);
    assert_eq!(promoted.system_state, candidate.system_state);
    assert_eq!(promoted.system_state_digest, candidate.system_state_digest);
    assert_eq!(promoted.provenance, candidate.provenance);

    promoted.verify_integrity().expect("integrity holds");
    candidate
        .verify_integrity()
        .expect("candidate still verifies");
}

#[test]
fn promotion_of_identical_content_is_idempotent() {
    let candidate = child_candidate();
    let first = PromotedPackState::promote(&candidate).expect("promotion succeeds");
    let second = PromotedPackState::promote(&candidate).expect("promotion succeeds");
    assert_eq!(first, second);

    // An independently proposed, identical candidate promotes to the same
    // promoted revision: promoted identity is content-addressed.
    let twin = child_candidate();
    let twin_promoted = PromotedPackState::promote(&twin).expect("promotion succeeds");
    assert_eq!(twin_promoted, first);
}

#[test]
fn promote_rejects_a_tampered_candidate() {
    let mut tampered = child_candidate();
    tampered.revision_id = pack_revision_id("forged-candidate");
    let result = PromotedPackState::promote(&tampered);
    assert!(
        matches!(result, Err(PackContractError::RevisionIntegrity { .. })),
        "promoting a tampered candidate must fail with a revision integrity error"
    );
}

#[test]
fn promoted_verify_integrity_detects_tampering() {
    let promoted = PromotedPackState::promote(&child_candidate()).expect("promotion succeeds");
    promoted.verify_integrity().expect("integrity holds");

    let mut swapped_workflow_pin = promoted.clone();
    swapped_workflow_pin.system_state = state(
        vec![WorkflowVersionRef::pin(workflow_version_id(
            "onboarding-swapped",
        ))],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );

    let mut swapped_promoted_from = promoted.clone();
    swapped_promoted_from.promoted_from = pack_revision_id("forged-candidate");

    let mut swapped_revision_id = promoted.clone();
    swapped_revision_id.revision_id = pack_revision_id("forged-promoted");

    // Parent and provenance are swapped *consistently*, so only the
    // recomputed source-candidate identity can catch the forgery.
    let mut consistent_parent_swap = promoted.clone();
    let forged_parent = pack_revision_id("v16");
    consistent_parent_swap.parent = Some(ParentRevision::new(forged_parent.clone()));
    consistent_parent_swap.provenance = PackProvenance::child_of(
        forged_parent,
        ProvenanceProducer::agent("pack-architect-worker").expect("valid agent"),
    );

    // A forged duplicate reference injected on the wire must also fail.
    let mut injected = serde_json::to_value(&promoted).expect("serializable");
    let onboarding = serde_json::to_value(onboarding_ref()).expect("serializable");
    injected["systemState"]["workflowVersionRefs"]
        .as_array_mut()
        .expect("refs array")
        .push(onboarding);
    let with_duplicate: PromotedPackState =
        serde_json::from_value(injected).expect("deserializable");

    let tampered_records = [
        ("swapped workflow pin", swapped_workflow_pin),
        ("swapped promoted-from", swapped_promoted_from),
        ("swapped revision id", swapped_revision_id),
        ("consistent parent swap", consistent_parent_swap),
        ("injected duplicate reference", with_duplicate),
    ];
    for (name, tampered) in tampered_records {
        assert!(
            tampered.verify_integrity().is_err(),
            "tampered promoted revision (`{name}`) must fail integrity verification"
        );
    }
}

#[test]
fn promoted_serde_round_trips() {
    let promoted = PromotedPackState::promote(&child_candidate()).expect("promotion succeeds");
    let serialized = serde_json::to_value(&promoted).expect("serializable");
    let parent_id = pack_revision_id("v17");
    assert_eq!(
        serialized,
        json!({
            "packId": "atlas",
            "parentRevision": {
                "revision": parent_id.to_string(),
                "label": "weekly baseline",
            },
            "systemState": {
                "workflowVersionRefs": [
                    {
                        "workflowVersionId": workflow_version_id("onboarding").to_string(),
                        "role": "user onboarding",
                    },
                ],
                "capabilityRefs": [
                    {
                        "capability": "navigate_web",
                        "constraint": "headless browser session",
                    },
                ],
                "policyRefs": [
                    {
                        "policyId": policy_id("governance").to_string(),
                        "validatedContentDigest": content_digest("governance-policy-content")
                            .to_string(),
                    },
                ],
                "evaluationRefs": [evaluation_ref("suite-1").to_string()],
            },
            "systemStateDigest": promoted.system_state_digest.to_string(),
            "provenance": {
                "parentRevision": parent_id.to_string(),
                "producer": { "kind": "agent", "agent": "pack-architect-worker" },
            },
            "promotedFrom": promoted.promoted_from.to_string(),
            "revisionId": promoted.revision_id.to_string(),
        }),
    );
    assert_eq!(
        serde_json::from_value::<PromotedPackState>(serialized).expect("deserializable"),
        promoted,
    );

    let mut injected = serde_json::to_value(&promoted).expect("serializable");
    injected["injected"] = json!(1);
    assert!(
        serde_json::from_value::<PromotedPackState>(injected).is_err(),
        "unknown fields must be rejected"
    );
}

#[test]
fn pinned_workflow_versions_cannot_be_swapped_undetected() {
    // Authority statement under test: these contracts expose no API that
    // performs, redefines, or bypasses workflow transitions. The only
    // workflow authority carried by a pack is the pinned immutable
    // WorkflowVersionId, so swapping a pin is a content change that
    // integrity verification must detect in both record kinds.
    let mut tampered_candidate = child_candidate();
    tampered_candidate.system_state = state(
        vec![WorkflowVersionRef::pin(workflow_version_id(
            "billing-swapped",
        ))],
        vec![navigate_web_ref()],
        vec![governance_policy_ref()],
        vec![evaluation_ref("suite-1")],
        /*rollback_checkpoint*/ None,
    );
    assert!(
        tampered_candidate.verify_integrity().is_err(),
        "a swapped workflow pin must fail candidate integrity verification"
    );

    let promoted = PromotedPackState::promote(&child_candidate()).expect("promotion succeeds");
    let mut tampered_promoted = promoted;
    tampered_promoted.system_state = tampered_candidate.system_state;
    assert!(
        tampered_promoted.verify_integrity().is_err(),
        "a swapped workflow pin must fail promoted integrity verification"
    );
}
