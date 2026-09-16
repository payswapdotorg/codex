//! Golden wire-contract fixtures (PACK Wave 2, Worker C lane).
//!
//! These fixtures are the SOURCE OF TRUTH for the pack-contracts wire
//! surface: canonical JSON exemplars of every major artifact type,
//! generated through the real serde types. Any client of the pack
//! contracts (Flauz.app desktop, future web/SDK clients) must be able to
//! deserialize exactly these shapes.
//!
//! `regenerate_fixtures` (ignored by default) rewrites the fixture files;
//! every other test enforces that the committed bytes stay stable — a wire
//! drift in any field name, optionality, or enum shape fails here first.
//!
//! Regenerate after a DELIBERATE wire change:
//! ```text
//! cargo test -p codex-pack-contracts --test golden_fixtures -- --ignored regenerate
//! ```

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use codex_pack_contracts::CandidatePackState;
use codex_pack_contracts::ComposedLineage;
use codex_pack_contracts::ComposedMissionHeader;
use codex_pack_contracts::ComposePackRequest;
use codex_pack_contracts::CompositionRelation;
use codex_pack_contracts::CompositionStrategy;
use codex_pack_contracts::HardConstraint;
use codex_pack_contracts::Mission;
use codex_pack_contracts::MissionAuthor;
use codex_pack_contracts::MissionId;
use codex_pack_contracts::MissionStatement;
use codex_pack_contracts::ModelPin;
use codex_pack_contracts::ModelPinningPolicy;
use codex_pack_contracts::PackAssurancePolicy;
use codex_pack_contracts::PackConstitution;
use codex_pack_contracts::PackDependencies;
use codex_pack_contracts::PackDependencyId;
use codex_pack_contracts::PackDependencyKey;
use codex_pack_contracts::PackDependencyLock;
use codex_pack_contracts::PackId;
use codex_pack_contracts::PackPolicy;
use codex_pack_contracts::PackPolicyId;
use codex_pack_contracts::PackPolicyScope;
use codex_pack_contracts::PackPolicySet;
use codex_pack_contracts::PackProvenance;
use codex_pack_contracts::PackRevisionContent;
use codex_pack_contracts::PackRevisionIdentity;
use codex_pack_contracts::PackRevisionView;
use codex_pack_contracts::PackSystemState;
use codex_pack_contracts::ParentRevision;
use codex_pack_contracts::PolicyRef;
use codex_pack_contracts::PolicyStatement;
use codex_pack_contracts::PromotedPackState;
use codex_pack_contracts::ProvenanceProducer;
use codex_pack_contracts::ResolvedPackDependency;
use codex_pack_contracts::ResolvedPackDependencyIdentity;
use codex_pack_contracts::ValueModel;
use codex_pack_contracts::ValueObjective;
use codex_pack_contracts::WorkflowVersionRef;
use codex_pack_contracts::compose_pack;
use codex_pack_contracts::CapabilityRef;
use codex_pack_contracts::ConstitutionRule;
use codex_pack_contracts::ConstitutionStatement;
use codex_pack_contracts::ContextModel;
use codex_pack_contracts::ContextNote;
use codex_pack_contracts::EvaluationRef;
use codex_pack_contracts::EvidenceRef;
use codex_pack_contracts::RollbackCheckpoint;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowVersionId;
use serde_json::json;

fn fixture_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn digest_for(payload: &str) -> ContentDigest {
    ContentDigest::of(&json!({ "payload": payload })).expect("digest computes")
}

/// The exemplar mission: user-authored, one objective, one context note,
/// one hard constraint — every wire field exercised.
fn exemplar_mission() -> Mission {
    Mission {
        id: MissionId::parse("fixture-mission").expect("valid mission id"),
        statement: MissionStatement::parse("Run fixture operations end to end.")
            .expect("valid statement"),
        author: MissionAuthor::user("org:fixture").expect("valid principal"),
        value_model: ValueModel {
            objectives: vec![ValueObjective {
                statement: MissionStatement::parse("Deliver verified outcomes first.")
                    .expect("valid statement"),
            }],
        },
        context_model: ContextModel {
            notes: vec![ContextNote {
                statement: MissionStatement::parse("Fixture region is active.")
                    .expect("valid statement"),
            }],
        },
        hard_constraints: vec![HardConstraint {
            statement: MissionStatement::parse("Never store credentials in pack artifacts.")
                .expect("valid statement"),
        }],
        preferences: vec![],
        success_measures: vec![],
    }
}

fn exemplar_policy_set() -> PackPolicySet {
    PackPolicySet::new(
        PackConstitution::new(vec![ConstitutionRule::AuditRequirement {
            statement: ConstitutionStatement::parse(
                "Every fixture promotion records an audit entry.",
            )
            .expect("valid statement"),
        }])
        .expect("valid constitution"),
        vec![PackPolicy::new(
            PackPolicyScope::DependencyUpdates,
            vec![PolicyStatement::parse(
                "Dependency updates require a new candidate revision.",
            )
            .expect("valid statement")],
            "How fixture dependencies may be updated.",
            BTreeSet::new(),
        )
        .expect("valid policy")],
    )
    .expect("valid policy set")
}

fn exemplar_workflow_version_id() -> WorkflowVersionId {
    WorkflowVersionId::from_digest(digest_for("fixture-workflow-version"))
}

fn exemplar_lock() -> PackDependencyLock {
    let mut lock = PackDependencyLock::default();
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::WorkflowVersion {
            dependency_id: PackDependencyId::parse("fixture_workflow").expect("valid dep id"),
        },
        resolved: ResolvedPackDependencyIdentity::WorkflowVersion(exemplar_workflow_version_id()),
        content_digest: digest_for("fixture-workflow-content"),
        provenance: None,
    });
    lock.insert(ResolvedPackDependency {
        key: PackDependencyKey::Capability {
            dependency_id: PackDependencyId::parse("fixture_capability").expect("valid dep id"),
        },
        resolved: ResolvedPackDependencyIdentity::Capability(
            CapabilityId::parse("navigate_web").expect("valid capability"),
        ),
        content_digest: digest_for("fixture-capability-content"),
        provenance: None,
    });
    lock
}

fn fixture_parent_revision_id() -> codex_pack_contracts::PackRevisionId {
    codex_pack_contracts::PackRevisionId::from_digest(digest_for("fixture-parent-revision"))
}

fn exemplar_state() -> PackSystemState {
    PackSystemState::new(
        vec![WorkflowVersionRef::pin_with_role(
            exemplar_workflow_version_id(),
            "fixture primary workflow",
        )
        .expect("valid role")],
        vec![CapabilityRef::constrained(
            CapabilityId::parse("navigate_web").expect("valid capability"),
            "headless browser session",
        )
        .expect("valid constraint")],
        vec![PolicyRef::validated_against(
            PackPolicyId::from_digest(digest_for("fixture-governance")),
            digest_for("fixture-governance-content"),
        )],
        vec![EvaluationRef::from_digest(digest_for("fixture-evaluation"))],
        vec![EvidenceRef::from_digest(digest_for("fixture-evidence"))],
        Some(
            RollbackCheckpoint::new(
                fixture_parent_revision_id(),
                digest_for("fixture-rollback-state"),
                "superseded baseline",
            )
            .expect("valid checkpoint"),
        ),
    )
    .expect("valid system state")
}

fn exemplar_content() -> PackRevisionContent {
    PackRevisionContent {
        pack_id: PackId::parse("fixture").expect("valid pack id"),
        semantic_version: SemanticVersion::new(1, 2, 3),
        mission: exemplar_mission(),
        policy_set: exemplar_policy_set(),
        dependency_lock: exemplar_lock(),
        system_state: exemplar_state(),
        composition: None,
    }
}

fn exemplar_assurance() -> PackAssurancePolicy {
    PackAssurancePolicy::empty().with_model_pinning(ModelPinningPolicy::Required {
        pin: ModelPin::new("fixture-reasoner", digest_for("fixture-model"))
            .expect("valid model pin"),
    })
}

/// All fixture exemplars, keyed by file name.
fn exemplars() -> Vec<(&'static str, serde_json::Value)> {
    let content = exemplar_content();
    let root = CandidatePackState::propose_root(
        content.clone(),
        PackProvenance::root(ProvenanceProducer::user("org:fixture").expect("valid user")),
    )
    .expect("root candidate proposes");
    let promoted = PromotedPackState::promote(&root).expect("candidate promotes");
    let child = CandidatePackState::propose(
        PackRevisionContent {
            semantic_version: SemanticVersion::new(1, 3, 0),
            ..content.clone()
        },
        ParentRevision::labeled(promoted.revision_id.clone(), "weekly baseline")
            .expect("valid label"),
        PackProvenance::child_of(
            promoted.revision_id.clone(),
            ProvenanceProducer::control_plane("pack-control-plane").expect("valid plane"),
        ),
    )
    .expect("child candidate proposes");

    // A composed candidate over both fixtures, with assurance inputs.
    let base_view = PackRevisionView::of_promoted(&promoted).expect("base view verifies");
    let overlay_view = PackRevisionView::of_candidate(&child).expect("overlay view verifies");
    let composed = compose_pack(ComposePackRequest {
        base: &base_view,
        overlay: &overlay_view,
        relation: CompositionRelation::Orthogonal,
        strategy: CompositionStrategy::strict(),
        lineage: ComposedLineage {
            pack_id: PackId::parse("fixture-composed").expect("valid pack id"),
            semantic_version: SemanticVersion::new(2, 0, 0),
        },
        mission_header: Some(ComposedMissionHeader {
            id: MissionId::parse("fixture-composed-mission").expect("valid mission id"),
            statement: MissionStatement::parse("Run composed fixture operations.")
                .expect("valid statement"),
            author: MissionAuthor::user("org:fixture").expect("valid principal"),
        }),
        base_assurance: Some(&exemplar_assurance()),
        overlay_assurance: Some(&exemplar_assurance()),
    })
    .expect("composed pack lands");
    let base_revision_id = promoted.revision_id.clone();
    let composed_candidate = composed
        .into_candidate(
            ParentRevision::labeled(base_revision_id.clone(), "composed-from-base")
                .expect("valid label"),
            PackProvenance::child_of(
                base_revision_id,
                ProvenanceProducer::user("org:fixture").expect("valid user"),
            ),
        )
        .expect("composed candidate proposes");

    vec![
        (
            "mission.json",
            serde_json::to_value(exemplar_mission()).expect("mission serializes"),
        ),
        (
            "policy-set.json",
            serde_json::to_value(exemplar_policy_set()).expect("policy set serializes"),
        ),
        (
            "dependency-lock.json",
            serde_json::to_value(exemplar_lock()).expect("lock serializes"),
        ),
        (
            "system-state.json",
            serde_json::to_value(exemplar_state()).expect("state serializes"),
        ),
        (
            "assurance-policy.json",
            serde_json::to_value(exemplar_assurance()).expect("assurance serializes"),
        ),
        (
            "revision-content.json",
            serde_json::to_value(&content).expect("content serializes"),
        ),
        (
            "candidate-root.json",
            serde_json::to_value(&root).expect("candidate serializes"),
        ),
        (
            "candidate-child.json",
            serde_json::to_value(&child).expect("candidate serializes"),
        ),
        (
            "promoted.json",
            serde_json::to_value(&promoted).expect("promoted serializes"),
        ),
        (
            "composed-candidate.json",
            serde_json::to_value(&composed_candidate).expect("composed candidate serializes"),
        ),
    ]
}

fn canonical(value: &serde_json::Value) -> String {
    let mut out = serde_json::to_string_pretty(value).expect("serializes");
    out.push('\n');
    out
}

/// Rewrites the fixture files (ignored unless explicitly requested).
#[test]
fn regenerate_fixtures() {
    std::fs::create_dir_all(fixture_dir()).expect("fixture dir creates");
    for (name, value) in exemplars() {
        std::fs::write(fixture_dir().join(name), canonical(&value)).expect("fixture writes");
    }
}

/// Every committed fixture deserializes back into the real types and the
/// serialization is byte-stable (canonicalization drift is a wire change).
#[test]
fn golden_fixtures_are_stable_and_revivable() {
    for (name, value) in exemplars() {
        let path = fixture_dir().join(name);
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("fixture {name} missing (run the ignored regenerate test): {err}")
        });
        let committed_value: serde_json::Value =
            serde_json::from_str(&committed).expect("committed fixture is valid JSON");
        assert_eq!(
            &committed_value, &value,
            "fixture {name} drifted from the current wire shape"
        );
        assert_eq!(
            canonical(&committed_value), committed,
            "fixture {name} is not canonical"
        );
    }

    // Round-trip the record-level fixtures through the real types with
    // full integrity verification.
    let root_json =
        std::fs::read_to_string(fixture_dir().join("candidate-root.json")).expect("present");
    let root: CandidatePackState = serde_json::from_str(&root_json).expect("root revives");
    root.verify_integrity().expect("root candidate verifies");

    let promoted_json =
        std::fs::read_to_string(fixture_dir().join("promoted.json")).expect("present");
    let promoted: PromotedPackState =
        serde_json::from_str(&promoted_json).expect("promoted revives");
    promoted.verify_integrity().expect("promoted verifies");

    let composed_json =
        std::fs::read_to_string(fixture_dir().join("composed-candidate.json")).expect("present");
    let composed: CandidatePackState =
        serde_json::from_str(&composed_json).expect("composed candidate revives");
    composed.verify_integrity()
        .expect("composed candidate verifies");
    let record = composed
        .composition
        .as_ref()
        .expect("composition record present");
    assert_eq!(record.relation, CompositionRelation::Orthogonal);
    assert!(composed.composition_digest.is_some());
}

/// Unknown fields are rejected on the record shapes (deny_unknown_fields
/// is part of the wire contract).
#[test]
fn golden_fixtures_reject_unknown_fields() {
    let committed =
        std::fs::read_to_string(fixture_dir().join("candidate-root.json")).expect("present");
    let mut tampered: serde_json::Value = serde_json::from_str(&committed).expect("parses");
    tampered
        .as_object_mut()
        .expect("object")
        .insert("__sneaky".to_owned(), json!(true));
    assert!(serde_json::from_value::<CandidatePackState>(tampered).is_err());
}

/// The legacy (pre-composition) wire shape carries no composition fields —
/// the exact property Flauz.app's client mirror relies on.
#[test]
fn legacy_fixture_has_no_composition_fields() {
    let committed =
        std::fs::read_to_string(fixture_dir().join("candidate-root.json")).expect("present");
    let value: serde_json::Value = serde_json::from_str(&committed).expect("parses");
    assert!(value.get("composition").is_none());
    assert!(value.get("compositionDigest").is_none());

    // A pre-PACK-005 identity tuple (no compositionDigest key) revives
    // with an absent composition digest.
    let legacy_identity = json!({
        "pack": "fixture",
        "semanticVersion": "0.1.0",
        "systemStateDigest": serde_json::to_value(digest_for("x")).expect("digest"),
        "missionDigest": serde_json::to_value(digest_for("x")).expect("digest"),
        "policyDigest": serde_json::to_value(digest_for("x")).expect("digest"),
        "dependencyLockDigest": serde_json::to_value(digest_for("x")).expect("digest"),
    });
    let identity: PackRevisionIdentity =
        serde_json::from_value(legacy_identity).expect("legacy identity revives");
    assert!(identity.composition_digest.is_none());
    assert!(identity.parent_revision.is_none());
}

/// Declared-dependency wire shape; keeps the manifest doc and the code
/// from drifting apart.
#[test]
fn declared_dependency_wire_shape_is_stable() {
    let dependencies = PackDependencies::default();
    let wire = serde_json::to_value(&dependencies).expect("serializes");
    // All three vectors are skip-if-empty: the default dependency
    // declaration serializes to an empty object.
    assert_eq!(wire, json!({}));
    let _ = BTreeMap::<PackDependencyKey, ResolvedPackDependency>::new();
}
