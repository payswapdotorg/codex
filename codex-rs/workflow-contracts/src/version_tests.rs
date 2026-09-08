use super::*;
use crate::CapabilityId;
use crate::CapabilityRequirement;
use crate::DependencyKey;
use crate::DevelopmentRef;
use crate::IrNodeId;
use crate::ResolvedDependency;
use crate::ResolvedDependencyIdentity;
use crate::RoleId;
use crate::StepNode;
use crate::SubworkflowDependency;
use crate::SubworkflowDependencyId;
use crate::SubworkflowVersionPin;
use crate::WorkflowDependencies;
use crate::WorkflowIr;
use crate::WorkflowIrNode;
use crate::WorkflowRole;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use std::collections::BTreeMap;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER_COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn definition(workflow_name: &str) -> WorkflowDefinition {
    let entry = IrNodeId::parse("check").expect("valid node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Step(StepNode {
            capabilities: vec![CapabilityRequirement {
                capability: CapabilityId::parse("navigate_web").expect("valid capability"),
                purpose: None,
            }],
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
            approval: crate::ApprovalRequirement::NotRequired,
        },
    );
    WorkflowDefinition {
        id: WorkflowDefinitionId::parse(workflow_name).expect("valid workflow id"),
        description: Some("Draft and publish release notes".to_owned()),
        ir: WorkflowIr {
            ir_format: crate::IR_FORMAT_VERSION,
            entry,
            nodes,
            conditions: BTreeMap::new(),
        },
        roles,
        triggers: vec![],
        dependencies: WorkflowDependencies::default(),
    }
}

fn subworkflow_version_id(workflow: &str) -> WorkflowVersionId {
    WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "workflow": workflow })).expect("digest"),
    )
}

fn composed_definition() -> WorkflowDefinition {
    let dependency = SubworkflowDependency {
        dependency_id: SubworkflowDependencyId::parse("publish_notes").expect("valid id"),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/notes-bot")
            .expect("valid repo"),
        workflow: WorkflowDefinitionId::parse("publish-notes").expect("valid workflow id"),
        pin: SubworkflowVersionPin::Exact("1.4.2".parse().expect("valid semver")),
    };
    let entry = IrNodeId::parse("delegate").expect("valid node id");
    let mut nodes = BTreeMap::new();
    nodes.insert(
        entry.clone(),
        WorkflowIrNode::Subworkflow(crate::SubworkflowNode {
            dependency: dependency.dependency_id.clone(),
            description: None,
            next: None,
        }),
    );
    WorkflowDefinition {
        id: WorkflowDefinitionId::parse("release-notes").expect("valid workflow id"),
        description: None,
        ir: WorkflowIr {
            ir_format: crate::IR_FORMAT_VERSION,
            entry,
            nodes,
            conditions: BTreeMap::new(),
        },
        roles: BTreeMap::new(),
        triggers: vec![],
        dependencies: WorkflowDependencies {
            subworkflows: vec![dependency],
            ..Default::default()
        },
    }
}

fn composed_lock(definition: &WorkflowDefinition) -> DependencyLock {
    let dependency = definition.dependencies.subworkflows.first().cloned();
    let Some(dependency) = dependency else {
        panic!("composed definition must declare a subworkflow");
    };
    let mut lock = DependencyLock::default();
    lock.insert(ResolvedDependency {
        key: DependencyKey::Subworkflow {
            dependency_id: dependency.dependency_id,
        },
        resolved: ResolvedDependencyIdentity::Subworkflow {
            repository: dependency.repository,
            workflow: dependency.workflow,
            version_id: subworkflow_version_id("publish-notes"),
            semantic_version: "1.4.2".parse().expect("valid semver"),
        },
        provenance: None,
    });
    lock
}

fn sealed_version(definition: WorkflowDefinition, lock: DependencyLock) -> WorkflowVersion {
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(COMMIT).expect("valid sha"),
            DevelopmentRef::branch("main").ok(),
        ),
        "1.4.2".parse().expect("valid semver"),
        lock,
        None,
    )
    .expect("version seals")
}

#[test]
fn sealed_versions_verify_their_own_integrity() {
    let version = sealed_version(definition("release-notes"), DependencyLock::default());

    version.verify_integrity().expect("sealed version verifies");
}

#[test]
fn sealed_versions_round_trip_through_serde() {
    let version = sealed_version(definition("release-notes"), DependencyLock::default());

    let serialized = serde_json::to_string(&version).expect("version serializes");
    let parsed: WorkflowVersion = serde_json::from_str(&serialized).expect("version parses");

    assert_eq!(version, parsed);
    parsed
        .verify_integrity()
        .expect("round-tripped version verifies");
}

#[test]
fn mutation_of_identity_covered_content_changes_version_identity() {
    let base = sealed_version(definition("release-notes"), DependencyLock::default());

    // Any identity-covered mutation produces a different version id.
    let different_semver = {
        let mut version = base.clone();
        version.identity.semantic_version = "1.4.3".parse().expect("valid semver");
        version
    };
    assert_ne!(
        base.version_id,
        different_semver.identity.version_id().expect("digest"),
        "semantic version is part of execution identity"
    );

    let different_commit = {
        let mut version = base.clone();
        version.identity.source_revision = ImmutableSourceRevision::pin_commit(
            RevisionSha::parse(OTHER_COMMIT).expect("valid sha"),
        );
        version
    };
    assert_ne!(
        base.version_id,
        different_commit.identity.version_id().expect("digest"),
        "source revision is part of execution identity"
    );

    let different_definition =
        sealed_version(definition("release-notes"), DependencyLock::default());
    let mutated_definition = {
        let mut version = different_definition.clone();
        version.definition.description = Some("Tampered".to_owned());
        version
    };
    assert_ne!(
        different_definition.version_id,
        {
            let mut repaired = mutated_definition;
            repaired.identity.definition_digest =
                repaired.definition.digest().expect("recomputed digest");
            repaired.identity.version_id().expect("digest")
        },
        "definition digest is part of execution identity"
    );
}

#[test]
fn tampered_records_fail_integrity_verification() {
    let version = sealed_version(definition("release-notes"), DependencyLock::default());

    // Tamper with definition content while keeping the recorded digests.
    let mut tampered_definition = version.clone();
    tampered_definition.definition.description = Some("Tampered".to_owned());
    let error = tampered_definition
        .verify_integrity()
        .expect_err("tampered definition must fail");
    assert!(
        error.to_string().contains("definition content"),
        "error must explain the definition mismatch, got: {error}"
    );

    // Tamper with the version id while keeping content.
    let mut tampered_id = version;
    tampered_id.version_id = WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "tampered": true })).expect("digest"),
    );
    let error = tampered_id
        .verify_integrity()
        .expect_err("mismatched version id must fail");
    assert!(
        error.to_string().contains("version id"),
        "error must explain the id mismatch, got: {error}"
    );
}

#[test]
fn moving_development_refs_does_not_change_identity_or_validity() {
    // Published from `main` at COMMIT.
    let from_main = sealed_version(definition("release-notes"), DependencyLock::default());

    // Later, the same commit is reachable from a different branch (or the
    // original ref moved). Re-sealing with new provenance yields the same
    // version identity and still verifies.
    let from_release_branch = WorkflowVersion::seal(
        definition("release-notes"),
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin(
            RevisionSha::parse(COMMIT).expect("valid sha"),
            DevelopmentRef::branch("release/1.4").ok(),
        ),
        "1.4.2".parse().expect("valid semver"),
        DependencyLock::default(),
        None,
    )
    .expect("re-sealed version verifies");

    assert_eq!(
        from_main.version_id, from_release_branch.version_id,
        "branch provenance must not influence execution identity"
    );
    from_release_branch
        .verify_integrity()
        .expect("re-sealed version still verifies");
}

#[test]
fn provenance_only_changes_preserve_identity() {
    let plain = sealed_version(definition("release-notes"), DependencyLock::default());
    let attributed = WorkflowVersion::seal(
        definition("release-notes"),
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin_commit(RevisionSha::parse(COMMIT).expect("valid sha")),
        "1.4.2".parse().expect("valid semver"),
        DependencyLock::default(),
        Some(crate::WorkflowProvenance {
            authors: vec![crate::Attribution {
                name: "Ada Lovelace".to_owned(),
                contact: None,
            }],
            forked_from: None,
            license: None,
            upgrade_policy: None,
        }),
    )
    .expect("attributed version seals");

    assert_eq!(
        plain.version_id, attributed.version_id,
        "publication provenance must not influence execution identity"
    );
    attributed
        .verify_integrity()
        .expect("attributed version verifies");
}

#[test]
fn sealing_requires_complete_dependency_locks() {
    let error = WorkflowVersion::seal(
        composed_definition(),
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin_commit(RevisionSha::parse(COMMIT).expect("valid sha")),
        "1.4.2".parse().expect("valid semver"),
        DependencyLock::default(),
        None,
    )
    .expect_err("unresolved dependencies must block sealing");

    assert!(
        error.to_string().contains("publish_notes"),
        "error must name the unresolved dependency, got: {error}"
    );
}

#[test]
fn dependency_upgrades_create_new_version_identities() {
    let definition = composed_definition();
    let lock = composed_lock(&definition);
    let before = sealed_version(definition.clone(), lock.clone());

    // Upgrade the pinned subworkflow to a different immutable version.
    let mut upgraded_lock = lock;
    for entry in upgraded_lock.entries.values_mut() {
        if let ResolvedDependencyIdentity::Subworkflow {
            version_id,
            semantic_version,
            ..
        } = &mut entry.resolved
        {
            *version_id = subworkflow_version_id("publish-notes-2");
            *semantic_version = "1.5.0".parse().expect("valid semver");
        }
    }

    let after = WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse("https://github.com/acme/release-bot").expect("valid repo"),
        ImmutableSourceRevision::pin_commit(RevisionSha::parse(COMMIT).expect("valid sha")),
        "1.4.2".parse().expect("valid semver"),
        upgraded_lock,
        None,
    )
    .expect("upgraded version seals");

    assert_ne!(
        before.version_id, after.version_id,
        "dependency upgrades must create a new version identity, never silently mutate"
    );
    after.verify_integrity().expect("upgraded version verifies");
}

#[test]
fn execution_identity_tuple_is_the_frozen_architecture_tuple() {
    let version = sealed_version(definition("release-notes"), DependencyLock::default());
    let identity = &version.identity;

    assert_eq!(identity.workflow.as_ref(), "release-notes");
    assert_eq!(identity.semantic_version.to_string(), "1.4.2");
    assert_eq!(identity.repository.as_str(), "github.com/acme/release-bot");
    assert_eq!(identity.source_revision.commit_sha.as_str(), COMMIT);
    assert_eq!(
        identity.definition_digest,
        version.definition.digest().expect("definition digest")
    );
    assert_eq!(
        identity.dependency_lock_digest,
        version.dependency_lock.digest().expect("lock digest")
    );
}

#[test]
fn abbreviated_commits_cannot_anchor_versions() {
    assert!(
        RevisionSha::parse("0123456").is_err(),
        "abbreviated SHAs must be rejected as revision anchors"
    );

    // The version record can only carry an ImmutableSourceRevision whose
    // commit anchor is a RevisionSha, which cannot be constructed from an
    // abbreviated SHA in the first place.
    let serialized = serde_json::to_string(&ImmutableSourceRevision::pin_commit(
        RevisionSha::parse(COMMIT).expect("valid sha"),
    ))
    .expect("revision serializes");
    let parsed: ImmutableSourceRevision =
        serde_json::from_str(&serialized).expect("revision parses");
    assert_eq!(parsed.commit_sha.as_str(), COMMIT);
}
