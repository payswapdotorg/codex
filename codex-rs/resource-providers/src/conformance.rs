//! The shared provider-neutral conformance harness (WO-016).
//!
//! [`assert_provider_conformance`] runs one identical operation script
//! and one identical assertion set against any
//! [`ExecutionResourceProvider`] reachable and healthy at call time:
//! probe, create, status, the full lifecycle matrix, undeclared-feature
//! rejection, invalid-spec rejection, unique id minting, release
//! semantics, foreign-handle rejection, binding minting, and
//! provider-scoped, credential-free evidence. Every expectation is
//! derived from the provider's **own** descriptor: an operation the
//! provider does not declare must surface as an explicit
//! [`ResourceProviderError::Unsupported`], never a panic, a silent
//! no-op, or a missing method.
//!
//! Running the same harness against two materially different
//! implementations ([`crate::sandbox_cloud::SandboxCloudFixture`] and
//! [`crate::local_containers::LocalContainersFixture`], and any real
//! host-supplied provider) is the acceptance evidence that the contract
//! is genuinely provider-neutral rather than shaped around a single
//! vendor: identical ops, identical assertions, different declared
//! capability matrices and failure modes.

use codex_workflow_contracts::ResourceTypeId;

use crate::ExecutionResourceProvider;
use crate::ResourceProviderError;
use crate::bind::bind;
use crate::descriptor::ProviderHealthStatus;
use crate::evidence::EVIDENCE_LOCATOR_PREFIX;
use crate::model::NetworkPolicy;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::model::ResourceHandle;
use crate::model::ResourceId;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::model::SPEC_LABELS_MAX;
use crate::provider::LifecycleOp;
use crate::provider::LifecycleOutcome;
use crate::provider::ResizeRequest;

/// Marker substrings that must never appear in a provider evidence
/// payload: credential material is absent by construction.
const CREDENTIAL_MARKERS: [&str; 7] = [
    "secret",
    "credential",
    "password",
    "bearer",
    "apikey",
    "api_key",
    "token",
];

/// Unwraps a provider result or fails the harness with context.
macro_rules! ok {
    ($result:expr, $context:literal) => {
        match $result {
            Ok(value) => value,
            Err(error) => panic!("conformance: {} failed: {error:?}", $context),
        }
    };
}

/// Asserts an explicit error variant or fails the harness with context.
macro_rules! err {
    ($result:expr, $variant:pat, $context:literal) => {
        match $result {
            Err($variant) => {}
            other => panic!("conformance: {} produced {other:?}", $context),
        }
    };
}

/// Runs the provider-neutral conformance suite against one provider.
///
/// The provider must be reachable and healthy when the harness runs.
/// Panics describe the violated expectation; a clean return is the
/// conformance pass.
pub fn assert_provider_conformance(provider: &dyn ExecutionResourceProvider) {
    let descriptor = provider.descriptor();
    ok!(descriptor.validate(), "descriptor validation");
    let provider_id = descriptor.provider.clone();
    assert!(
        !descriptor.capabilities.is_empty(),
        "conformance: providers declare at least one capability"
    );

    // Probe: a reachable provider answers live health.
    let health = ok!(provider.probe(), "probe");
    assert!(
        matches!(
            health.status,
            ProviderHealthStatus::Healthy | ProviderHealthStatus::Degraded
        ),
        "conformance: reachable providers report healthy or degraded"
    );

    // A class this provider does not serve, when one exists.
    let unserved = ResourceClass::ALL
        .into_iter()
        .find(|class| !descriptor.serves(*class));

    for capability in &descriptor.capabilities {
        let class = capability.class;
        let lifecycle = capability.lifecycle;

        // Create: a minimal spec provisions an active, owned handle.
        let handle = ok!(provider.create(minimal_spec(class)), "create");
        assert_eq!(handle.status, ResourceStatus::Active);
        assert_eq!(handle.class, class);
        assert_eq!(handle.provider, provider_id);

        // Status agrees with the handle.
        assert_eq!(
            ok!(provider.status(&handle), "status"),
            ResourceStatus::Active
        );

        // Unique minting: a second provisioning never reuses an id.
        let second = ok!(provider.create(minimal_spec(class)), "second create");
        assert_ne!(second.resource, handle.resource);
        ok!(provider.release(&second), "release second");

        // Binding minting: the canonical surface for the class.
        let binding = ok!(bind(&handle), "canonical bind");
        let expected_type = ok!(
            ResourceTypeId::parse(class.logical_resource_type()),
            "canonical resource type"
        );
        assert_eq!(binding.resource_type, expected_type);
        assert_eq!(binding.resource, handle.resource);
        assert_eq!(binding.holder, crate::bind::holder_environment(class));
        ok!(binding.validate(), "binding validation");

        // Pause/resume, per declaration.
        match provider.lifecycle(&handle, LifecycleOp::Pause) {
            Ok(LifecycleOutcome::Transitioned(paused)) => {
                assert!(lifecycle.pause_resume, "pause succeeded but was undeclared");
                assert_eq!(paused.status, ResourceStatus::Paused);
                assert_eq!(
                    ok!(provider.status(&handle), "status while paused"),
                    ResourceStatus::Paused
                );
                match provider.lifecycle(&handle, LifecycleOp::Resume) {
                    Ok(LifecycleOutcome::Transitioned(resumed)) => {
                        assert_eq!(resumed.status, ResourceStatus::Active);
                    }
                    other => panic!("conformance: resume produced {other:?}"),
                }
                assert_eq!(
                    ok!(provider.status(&handle), "status after resume"),
                    ResourceStatus::Active
                );
            }
            Err(ResourceProviderError::Unsupported {
                provider,
                operation,
            }) => {
                assert!(!lifecycle.pause_resume, "pause was declared but rejected");
                assert_eq!(provider, provider_id);
                assert_eq!(operation, LifecycleOp::Pause);
            }
            other => panic!("conformance: pause produced {other:?}"),
        }

        // Snapshot, per declaration.
        match provider.lifecycle(&handle, LifecycleOp::Snapshot) {
            Ok(LifecycleOutcome::Snapshotted(snapshot)) => {
                assert!(lifecycle.snapshot, "snapshot succeeded but was undeclared");
                assert_eq!(snapshot.resource, handle.resource);
                ok!(snapshot.validate(), "snapshot validation");
            }
            Err(ResourceProviderError::Unsupported {
                provider,
                operation,
            }) => {
                assert!(!lifecycle.snapshot, "snapshot was declared but rejected");
                assert_eq!(provider, provider_id);
                assert_eq!(operation, LifecycleOp::Snapshot);
            }
            other => panic!("conformance: snapshot produced {other:?}"),
        }

        // Fork, per declaration: a new live resource with a new id.
        match provider.lifecycle(&handle, LifecycleOp::Fork) {
            Ok(LifecycleOutcome::Transitioned(fork)) => {
                assert!(lifecycle.fork, "fork succeeded but was undeclared");
                assert_eq!(fork.status, ResourceStatus::Active);
                assert_eq!(fork.class, class);
                assert_ne!(fork.resource, handle.resource);
                ok!(provider.release(&fork), "release fork");
            }
            Err(ResourceProviderError::Unsupported {
                provider,
                operation,
            }) => {
                assert!(!lifecycle.fork, "fork was declared but rejected");
                assert_eq!(provider, provider_id);
                assert_eq!(operation, LifecycleOp::Fork);
            }
            other => panic!("conformance: fork produced {other:?}"),
        }

        // Resize, per declaration: the same resource, resized.
        let request = ResizeRequest {
            cpu_millis: Some(capability.capacity.max_cpu_millis),
            memory_mib: None,
            disk_mib: None,
        };
        match provider.lifecycle(&handle, LifecycleOp::Resize(request)) {
            Ok(LifecycleOutcome::Transitioned(resized)) => {
                assert!(lifecycle.resize, "resize succeeded but was undeclared");
                assert_eq!(resized.status, ResourceStatus::Active);
                assert_eq!(resized.resource, handle.resource);
            }
            Err(ResourceProviderError::Unsupported {
                provider,
                operation,
            }) => {
                assert!(!lifecycle.resize, "resize was declared but rejected");
                assert_eq!(provider, provider_id);
                assert_eq!(operation, LifecycleOp::Resize(request));
            }
            other => panic!("conformance: resize produced {other:?}"),
        }

        // Create as a lifecycle op, per declaration: a fresh resource
        // with the same shape and a new id.
        match provider.lifecycle(&handle, LifecycleOp::Create) {
            Ok(LifecycleOutcome::Transitioned(recreated)) => {
                assert!(
                    lifecycle.create,
                    "lifecycle create succeeded but was undeclared"
                );
                assert_eq!(recreated.status, ResourceStatus::Active);
                assert_eq!(recreated.class, class);
                assert_ne!(recreated.resource, handle.resource);
                ok!(provider.release(&recreated), "release recreation");
            }
            Err(ResourceProviderError::Unsupported {
                provider,
                operation,
            }) => {
                assert!(
                    !lifecycle.create,
                    "lifecycle create was declared but rejected"
                );
                assert_eq!(provider, provider_id);
                assert_eq!(operation, LifecycleOp::Create);
            }
            other => panic!("conformance: lifecycle create produced {other:?}"),
        }

        // Undeclared spec features are rejected explicitly, never
        // silently ignored.
        if !lifecycle.network_policy {
            let mut networked = minimal_spec(class);
            networked.network = Some(NetworkPolicy::Isolated);
            err!(
                provider.create(networked),
                ResourceProviderError::InvalidSpec { .. },
                "undeclared network policy rejection"
            );
        }
        if !lifecycle.gpu {
            let mut gpus = minimal_spec(class);
            gpus.gpu = Some(1);
            err!(
                provider.create(gpus),
                ResourceProviderError::InvalidSpec { .. },
                "undeclared GPU rejection"
            );
        }

        // Invalid specs are rejected.
        let mut labeled = minimal_spec(class);
        for index in 0..=(SPEC_LABELS_MAX + 1) {
            labeled
                .labels
                .insert(format!("label{index}"), "value".to_owned());
        }
        err!(
            provider.create(labeled),
            ResourceProviderError::InvalidSpec { .. },
            "oversized label rejection"
        );

        // Classes the provider does not serve are rejected.
        if let Some(unserved) = unserved {
            err!(
                provider.create(minimal_spec(unserved)),
                ResourceProviderError::InvalidSpec { .. },
                "unserved class rejection"
            );
        }

        // Release: the handle settles Released and stays queryable.
        let released = ok!(provider.release(&handle), "release");
        assert_eq!(released.status, ResourceStatus::Released);
        assert_eq!(
            ok!(provider.status(&released), "status after release"),
            ResourceStatus::Released
        );

        // Lifecycle ops on a released resource: `Create` (the recovery
        // path) still re-provisions; declared state ops surface the loss.
        match provider.lifecycle(&released, LifecycleOp::Create) {
            Ok(LifecycleOutcome::Transitioned(recreated)) => {
                assert_ne!(recreated.resource, released.resource);
                ok!(provider.release(&recreated), "release post-loss recreation");
            }
            Err(ResourceProviderError::Unsupported { .. }) => {
                assert!(!lifecycle.create, "create was declared but rejected");
            }
            other => panic!("conformance: post-release create produced {other:?}"),
        }
        if lifecycle.pause_resume {
            err!(
                provider.lifecycle(&released, LifecycleOp::Pause),
                ResourceProviderError::ResourceLost { .. },
                "pause on a released resource"
            );
        }

        // Bindings never mint from non-live handles.
        err!(
            bind(&released),
            ResourceProviderError::ResourceNotBindable { .. },
            "binding a released handle"
        );

        // Foreign handles are rejected as lost, never silently reused.
        let foreign = ResourceHandle {
            resource: ok!(
                ResourceId::parse("foreign-resource-1"),
                "foreign resource id"
            ),
            class,
            provider: ok!(
                ProviderId::parse("some-other-provider"),
                "foreign provider id"
            ),
            status: ResourceStatus::Active,
        };
        err!(
            provider.status(&foreign),
            ResourceProviderError::ResourceLost { .. },
            "foreign handle status"
        );
        err!(
            provider.lifecycle(&foreign, LifecycleOp::Create),
            ResourceProviderError::ResourceLost { .. },
            "foreign handle lifecycle"
        );
        err!(
            provider.release(&foreign),
            ResourceProviderError::ResourceLost { .. },
            "foreign handle release"
        );
    }

    // Evidence: provider-scoped locators, digest identity, credential
    // freedom, and workflow-semantic freedom.
    let records = provider.evidence_records();
    assert!(
        !records.is_empty(),
        "conformance: lifecycle operations produce evidence"
    );
    let prefix = format!("{EVIDENCE_LOCATOR_PREFIX}/{provider_id}/");
    for (index, record) in records.iter().enumerate() {
        assert!(
            record.locator.starts_with(&prefix),
            "conformance: locator `{}` is provider-scoped",
            record.locator
        );
        assert!(
            record.locator.ends_with(&format!("/{index}")),
            "conformance: locator `{}` is sequential",
            record.locator
        );
        assert_eq!(record.digest_hex.len(), 64);
        assert!(
            record
                .digest_hex
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
        );
        let serialized = ok!(
            serde_json::to_string(&record.payload),
            "evidence serialization"
        );
        let lowered = serialized.to_lowercase();
        for marker in CREDENTIAL_MARKERS {
            assert!(
                !lowered.contains(marker),
                "conformance: evidence payload contains credential marker `{marker}`"
            );
        }
        // Resource facts stay resource facts: no workflow identity leaks
        // into provider evidence payloads.
        assert!(
            !lowered.contains("workflowid") && !lowered.contains("versionid"),
            "conformance: provider evidence carries no workflow identity"
        );
    }
}

/// A minimal valid spec for `class`: no optional fields, so provider
/// defaults apply.
fn minimal_spec(class: ResourceClass) -> ResourceSpec {
    ResourceSpec {
        class,
        cpu_millis: None,
        memory_mib: None,
        disk_mib: None,
        gpu: None,
        os: None,
        region: None,
        labels: std::collections::BTreeMap::new(),
        network: None,
    }
}
