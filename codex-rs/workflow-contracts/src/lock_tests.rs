use super::*;
use crate::ContentDigest;
use crate::DependencyKey;
use crate::ResolvedDependency;
use crate::ResolvedDependencyIdentity;
use crate::SkillName;
use crate::SubworkflowDependency;
use crate::SubworkflowDependencyId;
use crate::SubworkflowVersionPin;
use crate::WorkflowDependencies;
use crate::WorkflowVersionId;
use pretty_assertions::assert_eq;

fn subworkflow_dependency() -> SubworkflowDependency {
    SubworkflowDependency {
        dependency_id: SubworkflowDependencyId::parse("publish_notes").expect("valid id"),
        repository: crate::WorkflowRepositoryId::parse("https://github.com/acme/notes-bot")
            .expect("valid repo"),
        workflow: crate::WorkflowDefinitionId::parse("publish-notes").expect("valid workflow id"),
        pin: SubworkflowVersionPin::Exact("1.4.2".parse().expect("valid semver")),
    }
}

fn subworkflow_resolution(
    dependency: &SubworkflowDependency,
    version_id: WorkflowVersionId,
) -> ResolvedDependency {
    ResolvedDependency {
        key: DependencyKey::Subworkflow {
            dependency_id: dependency.dependency_id.clone(),
        },
        resolved: ResolvedDependencyIdentity::Subworkflow {
            repository: dependency.repository.clone(),
            workflow: dependency.workflow.clone(),
            version_id,
            semantic_version: "1.4.2".parse().expect("valid semver"),
        },
        provenance: None,
    }
}

fn version_id() -> WorkflowVersionId {
    WorkflowVersionId::from_digest(
        ContentDigest::of(&serde_json::json!({ "workflow": "publish-notes" }))
            .expect("digest succeeds"),
    )
}

#[test]
fn dependency_locks_reject_unresolved_subworkflow_dependencies() {
    let dependencies = WorkflowDependencies {
        subworkflows: vec![subworkflow_dependency()],
        ..Default::default()
    };
    let lock = DependencyLock::default();

    let error = lock
        .verify_covers(&dependencies)
        .expect_err("unresolved subworkflow must fail");
    assert!(
        error.to_string().contains("publish_notes"),
        "error must name the missing dependency, got: {error}"
    );
}

#[test]
fn dependency_locks_verify_complete_coverage() {
    let dependency = subworkflow_dependency();
    let dependencies = WorkflowDependencies {
        subworkflows: vec![dependency.clone()],
        ..Default::default()
    };
    let mut lock = DependencyLock::default();
    lock.insert(subworkflow_resolution(&dependency, version_id()));

    lock.verify_covers(&dependencies)
        .expect("complete lock verifies");
}

#[test]
fn dependency_locks_reject_subworkflow_resolved_to_wrong_target() {
    let dependency = subworkflow_dependency();
    let dependencies = WorkflowDependencies {
        subworkflows: vec![dependency.clone()],
        ..Default::default()
    };
    let mut wrong_target = subworkflow_resolution(&dependency, version_id());
    let ResolvedDependencyIdentity::Subworkflow {
        repository,
        workflow: _,
        version_id: _,
        semantic_version: _,
    } = &mut wrong_target.resolved
    else {
        panic!("resolution must be a subworkflow identity");
    };
    *repository =
        WorkflowRepositoryId::parse("https://github.com/acme/other-repo").expect("valid repo");

    let mut lock = DependencyLock::default();
    lock.insert(wrong_target);

    let error = lock
        .verify_covers(&dependencies)
        .expect_err("wrong target must fail");
    assert!(
        error.to_string().contains("different target"),
        "error must explain the mismatch, got: {error}"
    );
}

#[test]
fn dependency_locks_digest_stably_and_change_with_entries() {
    let dependency = subworkflow_dependency();
    let mut lock = DependencyLock::default();
    lock.insert(subworkflow_resolution(&dependency, version_id()));

    let digest = lock.digest().expect("lock digests");
    let reserialized: DependencyLock =
        serde_json::from_str(&serde_json::to_string(&lock).expect("lock serializes"))
            .expect("lock deserializes");
    assert_eq!(digest, reserialized.digest().expect("re-digest matches"));

    let mut other = lock.clone();
    other.insert(subworkflow_resolution(
        &dependency,
        WorkflowVersionId::from_digest(
            ContentDigest::of(&serde_json::json!({ "workflow": "other" })).expect("digest"),
        ),
    ));
    assert_ne!(
        digest,
        other.digest().expect("other lock digests"),
        "changing a resolved identity must change the lock digest"
    );
}

#[test]
fn resolved_identities_reject_branch_pins_by_construction() {
    // A subworkflow resolution can only name an immutable version id; there
    // is no field through which a branch or tag could enter the resolution.
    let resolution = subworkflow_resolution(&subworkflow_dependency(), version_id());
    let serialized = serde_json::to_string(&resolution.resolved).expect("resolution serializes");
    assert!(
        !serialized.contains("branch") && !serialized.contains("resolvedFrom"),
        "resolutions must not carry movable refs, got: {serialized}"
    );
}

#[test]
fn lock_entries_are_deterministically_ordered() {
    let dependency = subworkflow_dependency();
    let mut lock = DependencyLock::default();
    lock.insert(subworkflow_resolution(&dependency, version_id()));
    lock.insert(ResolvedDependency {
        key: DependencyKey::Skill {
            skill: SkillName::parse("browser-use").expect("valid skill"),
        },
        resolved: ResolvedDependencyIdentity::Skill {
            digest: ContentDigest::of(&serde_json::json!({ "skill": "browser-use" }))
                .expect("digest"),
            version: None,
        },
        provenance: None,
    });

    let first = serde_json::to_string(&lock).expect("lock serializes");
    let second = serde_json::to_string(&lock).expect("lock serializes again");
    assert_eq!(
        first, second,
        "BTreeMap-backed entries must serialize deterministically"
    );
}

#[test]
fn subworkflow_identity_is_integrity_digest_backed() {
    let version = version_id();
    let resolution = subworkflow_resolution(&subworkflow_dependency(), version.clone());

    let digest = resolution
        .resolved
        .integrity_digest()
        .expect("subworkflow digest");
    assert_eq!(digest, *version.digest());

    let skill_resolution = ResolvedDependency {
        key: DependencyKey::Skill {
            skill: SkillName::parse("browser-use").expect("valid skill"),
        },
        resolved: ResolvedDependencyIdentity::Skill {
            digest: ContentDigest::of(&serde_json::json!({ "skill": "browser-use" }))
                .expect("digest"),
            version: None,
        },
        provenance: None,
    };
    assert!(skill_resolution.resolved.integrity_digest().is_some());
}
