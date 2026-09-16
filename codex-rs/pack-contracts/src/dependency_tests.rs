//! Tests for the pack dependency contracts.

use super::*;
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;
use serde_json::json;

fn dependency_id(value: &str) -> PackDependencyId {
    PackDependencyId::parse(value).expect("valid dependency id")
}

fn content_digest(payload: &str) -> ContentDigest {
    ContentDigest::of(&json!({ "payload": payload })).expect("digest computes")
}

fn workflow_version_id(payload: &str) -> WorkflowVersionId {
    WorkflowVersionId::from_digest(content_digest(payload))
}

fn pack_revision_id(payload: &str) -> PackRevisionId {
    PackRevisionId::from_digest(content_digest(payload))
}

fn base_dependencies() -> PackDependencies {
    PackDependencies {
        workflows: vec![WorkflowVersionDependency {
            dependency_id: dependency_id("billing"),
            version: workflow_version_id("billing@4.0.3"),
        }],
        capabilities: vec![CapabilityDependency {
            dependency_id: dependency_id("browser"),
            capability: CapabilityId::parse("navigate_web").expect("valid capability"),
        }],
        packs: vec![PackRevisionDependency {
            dependency_id: dependency_id("atlas"),
            pack: pack_revision_id("atlas@1"),
        }],
    }
}

fn resolved_lock(dependencies: &PackDependencies) -> PackDependencyLock {
    let mut lock = PackDependencyLock::default();
    for dependency in &dependencies.workflows {
        lock.insert(ResolvedPackDependency {
            key: PackDependencyKey::WorkflowVersion {
                dependency_id: dependency.dependency_id.clone(),
            },
            resolved: ResolvedPackDependencyIdentity::WorkflowVersion(dependency.version.clone()),
            content_digest: content_digest("billing-definition"),
            provenance: None,
        });
    }
    for dependency in &dependencies.capabilities {
        lock.insert(ResolvedPackDependency {
            key: PackDependencyKey::Capability {
                dependency_id: dependency.dependency_id.clone(),
            },
            resolved: ResolvedPackDependencyIdentity::Capability(dependency.capability.clone()),
            content_digest: content_digest("browser-implementation"),
            provenance: None,
        });
    }
    for dependency in &dependencies.packs {
        lock.insert(ResolvedPackDependency {
            key: PackDependencyKey::PackRevision {
                dependency_id: dependency.dependency_id.clone(),
            },
            resolved: ResolvedPackDependencyIdentity::PackRevision(dependency.pack.clone()),
            content_digest: content_digest("atlas-system-state"),
            provenance: None,
        });
    }
    lock
}

#[test]
fn dependency_ids_accept_and_reject_expected_forms() {
    for valid in ["billing", "browser", "b2", "navigate_web"] {
        assert!(
            PackDependencyId::parse(valid).is_ok(),
            "`{valid}` must be accepted"
        );
    }
    let invalid = [
        "".to_owned(),
        "Billing".to_owned(),
        "billing flow".to_owned(),
        "_billing".to_owned(),
        "billing_".to_owned(),
        "billing-flow".to_owned(),
    ];
    for invalid in invalid {
        assert!(
            PackDependencyId::parse(invalid.clone()).is_err(),
            "`{invalid}` must be rejected"
        );
    }
    assert!(serde_json::from_str::<PackDependencyId>("\"_x\"").is_err());
}

#[test]
fn complete_locks_cover_every_declared_dependency() {
    let dependencies = base_dependencies();
    let lock = resolved_lock(&dependencies);
    lock.verify_covers(&dependencies)
        .expect("complete lock covers");
}

#[test]
fn coverage_failures_name_the_missing_dependency() {
    let dependencies = base_dependencies();

    let missing_workflow = PackDependencyLock::default();
    let error = missing_workflow
        .verify_covers(&dependencies)
        .expect_err("missing workflow entry must fail");
    assert!(
        error.to_string().contains("`billing`"),
        "error must name the dependency, got: {error}"
    );

    let mut missing_capability = resolved_lock(&dependencies);
    missing_capability
        .entries
        .remove(&PackDependencyKey::Capability {
            dependency_id: dependency_id("browser"),
        });
    let error = missing_capability
        .verify_covers(&dependencies)
        .expect_err("missing capability entry must fail");
    assert!(
        error.to_string().contains("`browser`"),
        "error must name the dependency, got: {error}"
    );

    let mut missing_pack = resolved_lock(&dependencies);
    missing_pack
        .entries
        .remove(&PackDependencyKey::PackRevision {
            dependency_id: dependency_id("atlas"),
        });
    let error = missing_pack
        .verify_covers(&dependencies)
        .expect_err("missing pack entry must fail");
    assert!(
        error.to_string().contains("`atlas`"),
        "error must name the dependency, got: {error}"
    );
}

#[test]
fn resolutions_must_match_the_declared_pins_exactly() {
    let dependencies = base_dependencies();

    // The lock re-pins a different workflow version than the declaration.
    let mut re_pinned = resolved_lock(&dependencies);
    let entry = re_pinned
        .entries
        .get_mut(&PackDependencyKey::WorkflowVersion {
            dependency_id: dependency_id("billing"),
        })
        .expect("entry exists");
    entry.resolved =
        ResolvedPackDependencyIdentity::WorkflowVersion(workflow_version_id("billing@4.0.4"));
    let error = re_pinned
        .verify_covers(&dependencies)
        .expect_err("re-pinned resolution must fail");
    assert!(
        error.to_string().contains("different version"),
        "error must explain the pin mismatch, got: {error}"
    );

    // The entry resolved to the wrong kind of identity entirely.
    let mut wrong_kind = resolved_lock(&dependencies);
    let entry = wrong_kind
        .entries
        .get_mut(&PackDependencyKey::WorkflowVersion {
            dependency_id: dependency_id("billing"),
        })
        .expect("entry exists");
    entry.resolved = ResolvedPackDependencyIdentity::Capability(
        CapabilityId::parse("navigate_web").expect("valid capability"),
    );
    let error = wrong_kind
        .verify_covers(&dependencies)
        .expect_err("kind-mismatched resolution must fail");
    assert!(
        error.to_string().contains("non-workflow identity"),
        "error must explain the kind mismatch, got: {error}"
    );

    // A wire-tampered lock cannot hide an entry behind another key.
    let mut hidden = resolved_lock(&dependencies);
    let entry = hidden
        .entries
        .get_mut(&PackDependencyKey::Capability {
            dependency_id: dependency_id("browser"),
        })
        .expect("entry exists");
    entry.key = PackDependencyKey::WorkflowVersion {
        dependency_id: dependency_id("browser"),
    };
    let error = hidden
        .verify_covers(&dependencies)
        .expect_err("mismatched entry key must fail");
    assert!(
        error.to_string().contains("mismatched key"),
        "error must explain the key mismatch, got: {error}"
    );
}

#[test]
fn duplicate_declared_dependency_ids_are_rejected() {
    let duplicated = PackDependencies {
        workflows: vec![WorkflowVersionDependency {
            dependency_id: dependency_id("billing"),
            version: workflow_version_id("billing@4.0.3"),
        }],
        capabilities: vec![CapabilityDependency {
            dependency_id: dependency_id("billing"),
            capability: CapabilityId::parse("navigate_web").expect("valid capability"),
        }],
        packs: vec![],
    };
    let lock = resolved_lock(&duplicated);
    let error = lock
        .verify_covers(&duplicated)
        .expect_err("duplicate ids must fail");
    assert!(
        error.to_string().contains("unique across the declared set"),
        "error must explain the uniqueness rule, got: {error}"
    );
}

#[test]
fn identical_locks_produce_identical_digests() {
    let dependencies = base_dependencies();
    let lock = resolved_lock(&dependencies);
    assert_eq!(
        lock.digest().expect("digest computes"),
        resolved_lock(&dependencies)
            .digest()
            .expect("digest computes")
    );
}

#[test]
fn every_lock_content_mutation_changes_the_digest() {
    let dependencies = base_dependencies();
    let base = resolved_lock(&dependencies);
    let base_digest = base.digest().expect("digest computes");

    let mut re_pinned_version = base.clone();
    for entry in re_pinned_version.entries.values_mut() {
        if let ResolvedPackDependencyIdentity::WorkflowVersion(version) = &mut entry.resolved {
            *version = workflow_version_id("billing@5.0.0");
        }
    }
    let mut changed_content_digest = base.clone();
    for entry in changed_content_digest.entries.values_mut() {
        if matches!(
            entry.resolved,
            ResolvedPackDependencyIdentity::Capability(_)
        ) {
            entry.content_digest = content_digest("browser-implementation-v2");
        }
    }
    let mut attributed = base.clone();
    for entry in attributed.entries.values_mut() {
        if matches!(
            entry.resolved,
            ResolvedPackDependencyIdentity::PackRevision(_)
        ) {
            entry.provenance = Some(DependencyProvenance {
                source: Some("internal registry".to_owned()),
                license: None,
            });
        }
    }
    let mut extra_entry = base;
    extra_entry.insert(ResolvedPackDependency {
        key: PackDependencyKey::WorkflowVersion {
            dependency_id: dependency_id("reporting"),
        },
        resolved: ResolvedPackDependencyIdentity::WorkflowVersion(workflow_version_id(
            "reporting@1.8.1",
        )),
        content_digest: content_digest("reporting-definition"),
        provenance: None,
    });

    let mutations = [
        ("re-pinned workflow version", re_pinned_version),
        ("changed capability content digest", changed_content_digest),
        ("added resolution provenance", attributed),
        ("added lock entry", extra_entry),
    ];
    for (covered_change, mutation) in mutations {
        assert_ne!(
            base_digest,
            mutation.digest().expect("digest computes"),
            "{covered_change} must change the lock digest"
        );
    }
}

#[test]
fn locks_and_dependencies_round_trip_through_serde() {
    let dependencies = base_dependencies();
    let lock = resolved_lock(&dependencies);

    let serialized = serde_json::to_string(&dependencies).expect("serializable");
    let parsed: PackDependencies = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, dependencies);

    let serialized = serde_json::to_string(&lock).expect("serializable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).expect("valid json"),
        json!({
            "entries": {
                "capability:browser": {
                    "key": "capability:browser",
                    "resolved": { "capability": "navigate_web" },
                    "contentDigest": content_digest("browser-implementation").to_string(),
                },
                "pack:atlas": {
                    "key": "pack:atlas",
                    "resolved": { "packRevision": pack_revision_id("atlas@1").to_string() },
                    "contentDigest": content_digest("atlas-system-state").to_string(),
                },
                "workflow:billing": {
                    "key": "workflow:billing",
                    "resolved": { "workflowVersion": workflow_version_id("billing@4.0.3").to_string() },
                    "contentDigest": content_digest("billing-definition").to_string(),
                },
            }
        })
    );
    let parsed: PackDependencyLock = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(parsed, lock);
    parsed
        .verify_covers(&dependencies)
        .expect("round-tripped lock still covers");

    let tampered = serialized.replace('{', "{\"injected\":1,");
    assert!(serde_json::from_str::<PackDependencyLock>(&tampered).is_err());
}

#[test]
fn dependency_keys_round_trip_as_compact_strings() {
    let keys = [
        PackDependencyKey::WorkflowVersion {
            dependency_id: dependency_id("billing"),
        },
        PackDependencyKey::Capability {
            dependency_id: dependency_id("browser"),
        },
        PackDependencyKey::PackRevision {
            dependency_id: dependency_id("atlas"),
        },
    ];
    for key in keys {
        let serialized = serde_json::to_string(&key).expect("serializable");
        assert_eq!(
            serde_json::from_str::<PackDependencyKey>(&serialized).expect("deserializable"),
            key
        );
    }

    let invalid = [
        "\"billing\"",
        "\"tool:billing\"",
        "\"workflow:Billing\"",
        "\"workflow:_billing\"",
        "\"workflow:\"",
        "\"workflow\"",
    ];
    for invalid in invalid {
        assert!(
            serde_json::from_str::<PackDependencyKey>(invalid).is_err(),
            "`{invalid}` must be rejected as a dependency key"
        );
    }
}

#[test]
fn empty_locks_cover_empty_dependencies() {
    let lock = PackDependencyLock::default();
    lock.verify_covers(&PackDependencies::default())
        .expect("an empty lock covers an empty declaration set");
    let serialized = serde_json::to_string(&lock).expect("serializable");
    assert_eq!(serialized, "{}");
}
