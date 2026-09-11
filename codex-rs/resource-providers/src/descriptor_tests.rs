//! Unit tests for provider capability declarations.

use pretty_assertions::assert_eq;

use crate::ResourceProviderError;
use crate::descriptor::CostMetadata;
use crate::descriptor::ProviderDescriptor;
use crate::descriptor::ProviderHealth;
use crate::descriptor::ProviderHealthStatus;
use crate::descriptor::ResourceCapability;
use crate::descriptor::ResourceLifecycleSupport;
use crate::descriptor::ResourceQuota;
use crate::model::CPU_MILLIS_MAX;
use crate::model::ProviderId;
use crate::model::ResourceClass;
use crate::provider::LifecycleOp;
use crate::provider::ResizeRequest;

fn quota() -> ResourceQuota {
    ResourceQuota {
        max_cpu_millis: 2_000,
        max_memory_mib: 4_096,
        max_disk_mib: 20_480,
    }
}

fn descriptor() -> ProviderDescriptor {
    ProviderDescriptor {
        provider: ProviderId::parse("fixture-declared").expect("provider id"),
        capabilities: vec![ResourceCapability {
            class: ResourceClass::Sandbox,
            lifecycle: ResourceLifecycleSupport::none()
                .with_create()
                .with_snapshot(),
            capacity: quota(),
        }],
        health: ProviderHealth::healthy(),
        regions: vec!["local".to_owned()],
        cost: None,
    }
}

#[test]
fn lifecycle_support_builders_and_op_matrix() {
    let full = ResourceLifecycleSupport::none()
        .with_create()
        .with_pause_resume()
        .with_snapshot()
        .with_fork()
        .with_resize()
        .with_network_policy()
        .with_gpu();
    assert!(full.create);
    assert!(full.pause_resume);
    assert!(full.snapshot);
    assert!(full.fork);
    assert!(full.resize);
    assert!(full.network_policy);
    assert!(full.gpu);
    let resize = ResizeRequest {
        cpu_millis: Some(1),
        memory_mib: None,
        disk_mib: None,
    };
    assert!(full.supports(&LifecycleOp::Create));
    assert!(full.supports(&LifecycleOp::Pause));
    assert!(full.supports(&LifecycleOp::Resume));
    assert!(full.supports(&LifecycleOp::Snapshot));
    assert!(full.supports(&LifecycleOp::Fork));
    assert!(full.supports(&LifecycleOp::Resize(resize)));

    let minimal = ResourceLifecycleSupport::none().with_create();
    assert!(minimal.supports(&LifecycleOp::Create));
    assert!(!minimal.supports(&LifecycleOp::Pause));
    assert!(!minimal.supports(&LifecycleOp::Snapshot));
    assert!(!minimal.supports(&LifecycleOp::Fork));
    assert!(!minimal.supports(&LifecycleOp::Resize(resize)));

    let empty = ResourceLifecycleSupport::none();
    assert!(!empty.supports(&LifecycleOp::Create));
}

#[test]
fn quota_validation_bounds() {
    quota().validate().expect("quota validates");
    let zero_cpu = ResourceQuota {
        max_cpu_millis: 0,
        ..quota()
    };
    assert!(zero_cpu.validate().is_err());
    let over_cpu = ResourceQuota {
        max_cpu_millis: CPU_MILLIS_MAX + 1,
        ..quota()
    };
    assert!(over_cpu.validate().is_err());
    let zero_disk = ResourceQuota {
        max_disk_mib: 0,
        ..quota()
    };
    assert!(zero_disk.validate().is_err());
}

#[test]
fn descriptor_validation_accepts_and_serves() {
    let declared = descriptor();
    declared.validate().expect("descriptor validates");
    assert!(declared.serves(ResourceClass::Sandbox));
    assert!(!declared.serves(ResourceClass::RemoteDesktop));
    let capability = declared
        .capability_for(ResourceClass::Sandbox)
        .expect("sandbox capability");
    assert_eq!(capability.class, ResourceClass::Sandbox);
    assert!(capability.lifecycle.create);
    assert!(!capability.lifecycle.fork);
}

#[test]
fn descriptor_validation_rejects_malformed() {
    let empty = ProviderDescriptor {
        capabilities: Vec::new(),
        ..descriptor()
    };
    let error = empty.validate().expect_err("empty capabilities");
    assert!(matches!(error, ResourceProviderError::InvalidSpec { .. }));

    let duplicated = ProviderDescriptor {
        capabilities: vec![
            ResourceCapability {
                class: ResourceClass::Sandbox,
                lifecycle: ResourceLifecycleSupport::none().with_create(),
                capacity: quota(),
            },
            ResourceCapability {
                class: ResourceClass::Sandbox,
                lifecycle: ResourceLifecycleSupport::none().with_create(),
                capacity: quota(),
            },
        ],
        ..descriptor()
    };
    assert!(duplicated.validate().is_err());

    let regionless = ProviderDescriptor {
        regions: Vec::new(),
        ..descriptor()
    };
    assert!(regionless.validate().is_err());

    let repeated_region = ProviderDescriptor {
        regions: vec!["local".to_owned(), "local".to_owned()],
        ..descriptor()
    };
    assert!(repeated_region.validate().is_err());

    let verbose_health = ProviderDescriptor {
        health: ProviderHealth {
            status: ProviderHealthStatus::Degraded,
            detail: Some("x".repeat(257)),
            latency_ms: Some(1),
        },
        ..descriptor()
    };
    assert!(verbose_health.validate().is_err());

    let upper_currency = ProviderDescriptor {
        cost: Some(CostMetadata {
            currency: "USD".to_owned(),
            per_hour_micros: 1,
        }),
        ..descriptor()
    };
    assert!(upper_currency.validate().is_err());

    let valid_cost = ProviderDescriptor {
        cost: Some(CostMetadata {
            currency: "usd".to_owned(),
            per_hour_micros: 3_600_000,
        }),
        ..descriptor()
    };
    valid_cost.validate().expect("lowercase currency validates");
}

#[test]
fn provider_health_status_roundtrips() {
    for status in ProviderHealthStatus::ALL {
        assert_eq!(
            status,
            ProviderHealthStatus::parse(status.as_str()).expect("parse health status")
        );
        let json = serde_json::to_value(status).expect("serialize health status");
        assert_eq!(json, serde_json::json!(status.as_str()));
        let parsed: ProviderHealthStatus =
            serde_json::from_value(json).expect("deserialize health status");
        assert_eq!(parsed, status);
    }
    assert!(ProviderHealthStatus::parse("bogus").is_err());
    ProviderHealth::healthy()
        .validate()
        .expect("healthy validates");
}
