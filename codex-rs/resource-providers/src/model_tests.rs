//! Unit tests for the provider-neutral resource model.

use codex_execution_contracts::ResourceId;
use pretty_assertions::assert_eq;

use crate::ResourceProviderError;
use crate::model::CPU_MILLIS_MAX;
use crate::model::DISK_MIB_MAX;
use crate::model::GPU_MAX;
use crate::model::LABEL_KEY_MAX_BYTES;
use crate::model::LABEL_VALUE_MAX_BYTES;
use crate::model::MEMORY_MIB_MAX;
use crate::model::NETWORK_HOST_MAX_BYTES;
use crate::model::NETWORK_HOSTS_MAX;
use crate::model::NetworkPolicy;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::model::ResourceHandle;
use crate::model::ResourceSnapshot;
use crate::model::ResourceSpec;
use crate::model::ResourceStatus;
use crate::model::SNAPSHOT_ARTIFACT_MAX_BYTES;
use crate::model::SPEC_LABELS_MAX;
use crate::model::SPEC_TEXT_MAX_BYTES;

/// A minimal spec for `class` with every optional field unset.
fn spec(class: ResourceClass) -> ResourceSpec {
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

#[test]
fn provider_id_parses_and_roundtrips() {
    let id = ProviderId::parse("fixture-sandbox-cloud").expect("valid provider id");
    assert_eq!(id.as_ref(), "fixture-sandbox-cloud");
    assert_eq!(id.to_string(), "fixture-sandbox-cloud");
    let json = serde_json::to_value(id.clone()).expect("serialize");
    assert_eq!(json, serde_json::json!("fixture-sandbox-cloud"));
    let parsed: ProviderId = serde_json::from_value(json).expect("deserialize");
    assert_eq!(parsed, id);
}

#[test]
fn provider_id_rejects_malformed_tokens() {
    for bad in ["", ".", "..", ".hidden", "a..b", "a/b", "a b", "a:b"] {
        let error = ProviderId::parse(bad).expect_err("malformed provider id");
        assert!(
            matches!(error, ResourceProviderError::InvalidIdentifier { .. }),
            "{bad} must fail identifier validation, got {error:?}"
        );
    }
}

#[test]
fn resource_class_and_status_wire_names_roundtrip() {
    for class in ResourceClass::ALL {
        assert_eq!(
            class,
            ResourceClass::parse(class.as_str()).expect("parse wire name")
        );
        let json = serde_json::to_value(class).expect("serialize class");
        assert_eq!(json, serde_json::json!(class.as_str()));
        let parsed: ResourceClass = serde_json::from_value(json).expect("deserialize class");
        assert_eq!(parsed, class);
    }
    assert!(ResourceClass::parse("bogus").is_err());
    for status in ResourceStatus::ALL {
        let json = serde_json::to_value(status).expect("serialize status");
        assert_eq!(json, serde_json::json!(status.as_str()));
        let parsed: ResourceStatus = serde_json::from_value(json).expect("deserialize status");
        assert_eq!(parsed, status);
        assert_eq!(
            status.is_bindable(),
            matches!(status, ResourceStatus::Active | ResourceStatus::Paused)
        );
    }
}

#[test]
fn resource_spec_minimal_validates_and_roundtrips() {
    let minimal = spec(ResourceClass::Sandbox);
    minimal.validate().expect("minimal spec validates");
    let json = serde_json::to_value(&minimal).expect("serialize spec");
    assert_eq!(json, serde_json::json!({"class": "sandbox"}));
    let parsed: ResourceSpec = serde_json::from_value(json).expect("deserialize spec");
    assert_eq!(parsed, minimal);
    let denied: Result<ResourceSpec, _> =
        serde_json::from_value(serde_json::json!({"class": "sandbox", "vendorField": 1}));
    assert!(denied.is_err());
}

#[test]
fn resource_spec_rejects_oversized_labels() {
    let mut counted = spec(ResourceClass::PersistentWorkspace);
    for index in 0..=SPEC_LABELS_MAX {
        counted
            .labels
            .insert(format!("key{index}"), "value".to_owned());
    }
    let error = counted.validate().expect_err("too many labels");
    assert!(matches!(error, ResourceProviderError::InvalidSpec { .. }));

    let mut keyed = spec(ResourceClass::Sandbox);
    keyed
        .labels
        .insert("x".repeat(LABEL_KEY_MAX_BYTES + 1), "value".to_owned());
    assert!(keyed.validate().is_err());

    let mut valued = spec(ResourceClass::Sandbox);
    valued
        .labels
        .insert("key".to_owned(), "x".repeat(LABEL_VALUE_MAX_BYTES + 1));
    assert!(valued.validate().is_err());
}

#[test]
fn resource_spec_rejects_out_of_bounds_capacity() {
    /// (cpu, memory, disk, gpu) as a named case to keep the table's type
    /// within clippy's complexity budget.
    type CapacityCase = (Option<u32>, Option<u32>, Option<u64>, Option<u32>);
    let cases: [CapacityCase; 8] = [
        (Some(0), None, None, None),
        (Some(CPU_MILLIS_MAX + 1), None, None, None),
        (None, Some(0), None, None),
        (None, Some(MEMORY_MIB_MAX + 1), None, None),
        (None, None, Some(0), None),
        (None, None, Some(DISK_MIB_MAX + 1), None),
        (None, None, None, Some(0)),
        (None, None, None, Some(GPU_MAX + 1)),
    ];
    for (cpu, memory, disk, gpu) in cases {
        let mut bounded = spec(ResourceClass::GpuCompute);
        bounded.cpu_millis = cpu;
        bounded.memory_mib = memory;
        bounded.disk_mib = disk;
        bounded.gpu = gpu;
        let error = bounded.validate().expect_err("out-of-bounds capacity");
        assert!(matches!(error, ResourceProviderError::InvalidSpec { .. }));
    }
    let mut maximal = spec(ResourceClass::GpuCompute);
    maximal.cpu_millis = Some(CPU_MILLIS_MAX);
    maximal.memory_mib = Some(MEMORY_MIB_MAX);
    maximal.disk_mib = Some(DISK_MIB_MAX);
    maximal.gpu = Some(GPU_MAX);
    maximal.validate().expect("boundary values validate");
}

#[test]
fn resource_spec_rejects_bad_text_and_network_policy() {
    let mut empty_os = spec(ResourceClass::Sandbox);
    empty_os.os = Some(String::new());
    assert!(empty_os.validate().is_err());

    let mut long_region = spec(ResourceClass::Sandbox);
    long_region.region = Some("x".repeat(SPEC_TEXT_MAX_BYTES + 1));
    assert!(long_region.validate().is_err());

    let mut spaced_os = spec(ResourceClass::Sandbox);
    spaced_os.os = Some("lin ux".to_owned());
    assert!(spaced_os.validate().is_err());

    let mut too_many_hosts = spec(ResourceClass::Sandbox);
    too_many_hosts.network = Some(NetworkPolicy::Restricted {
        allowed_hosts: (0..=NETWORK_HOSTS_MAX)
            .map(|index| format!("host{index}.example.com"))
            .collect(),
    });
    assert!(too_many_hosts.validate().is_err());

    let mut long_host = spec(ResourceClass::Sandbox);
    long_host.network = Some(NetworkPolicy::Restricted {
        allowed_hosts: vec!["x".repeat(NETWORK_HOST_MAX_BYTES + 1)],
    });
    assert!(long_host.validate().is_err());

    let mut duplicated = spec(ResourceClass::Sandbox);
    duplicated.network = Some(NetworkPolicy::Restricted {
        allowed_hosts: vec!["api.example.com".to_owned(), "api.example.com".to_owned()],
    });
    assert!(duplicated.validate().is_err());

    let mut isolated = spec(ResourceClass::Sandbox);
    isolated.network = Some(NetworkPolicy::Isolated);
    isolated
        .validate()
        .expect("isolated network policy validates");
}

#[test]
fn resource_handle_and_snapshot_roundtrip_credential_free() {
    let handle = ResourceHandle {
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
        class: ResourceClass::RemoteDesktop,
        provider: ProviderId::parse("fixture-sandbox-cloud").expect("provider id"),
        status: ResourceStatus::Active,
    };
    let json = serde_json::to_value(&handle).expect("serialize handle");
    assert_eq!(
        json,
        serde_json::json!({
            "resource": "sbx-0001",
            "class": "remoteDesktop",
            "provider": "fixture-sandbox-cloud",
            "status": "active",
        })
    );
    // The handle is exactly four typed identities: no credential field
    // exists structurally.
    assert_eq!(json.as_object().expect("object").len(), 4);
    let parsed: ResourceHandle = serde_json::from_value(json).expect("deserialize handle");
    assert_eq!(parsed, handle);

    let snapshot = ResourceSnapshot {
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
        artifact: "snap-0001".to_owned(),
    };
    snapshot.validate().expect("snapshot validates");
    let oversize = ResourceSnapshot {
        resource: ResourceId::parse("sbx-0001").expect("resource id"),
        artifact: "x".repeat(SNAPSHOT_ARTIFACT_MAX_BYTES + 1),
    };
    assert!(oversize.validate().is_err());
}
