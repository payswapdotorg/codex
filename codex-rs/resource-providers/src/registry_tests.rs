//! Unit tests for the host wiring registry and credential references.

use std::sync::Arc;

use pretty_assertions::assert_eq;

use crate::ResourceProviderError;
use crate::model::ProviderId;
use crate::registry::ProviderCredentials;
use crate::registry::ProviderRegistry;
use crate::sandbox_cloud::SandboxCloudFixture;

#[test]
fn registry_registers_looks_up_and_probes() {
    let mut registry = ProviderRegistry::new();
    assert!(registry.is_empty());
    let sandbox = Arc::new(SandboxCloudFixture::new(4).expect("fixture"));
    let id = registry.register(sandbox).expect("register");
    assert_eq!(id.as_ref(), crate::sandbox_cloud::SANDBOX_CLOUD_PROVIDER_ID);
    assert_eq!(registry.len(), 1);
    assert!(registry.provider(&id).is_some());
    let provider = registry.provider(&id).expect("registered provider");
    let health = provider.probe().expect("healthy probe");
    assert_eq!(
        health.status,
        crate::descriptor::ProviderHealthStatus::Healthy
    );

    let fleet = registry.probe_all();
    assert_eq!(fleet.len(), 1);
    let (probed_id, probed) = &fleet[0];
    assert_eq!(*probed_id, id);
    assert!(probed.is_ok());

    let ids = registry.provider_ids().cloned().collect::<Vec<_>>();
    assert_eq!(ids, vec![id]);
}

#[test]
fn registry_rejects_duplicates_and_malformed() {
    let mut registry = ProviderRegistry::new();
    let first = Arc::new(SandboxCloudFixture::new(2).expect("fixture"));
    registry.register(first.clone()).expect("register");
    let error = registry.register(first).expect_err("duplicate rejected");
    assert!(matches!(error, ResourceProviderError::InvalidSpec { .. }));

    let local = Arc::new(crate::local_containers::LocalContainersFixture::new().expect("fixture"));
    registry
        .register(local)
        .expect("distinct provider registers");
    assert_eq!(registry.len(), 2);
}

#[test]
fn registry_credentials_are_references_not_values() {
    let reference = ProviderCredentials::parse_env_key("SANDBOX_CLOUD_API_KEY").expect("env key");
    assert_eq!(reference.env_key_name(), "SANDBOX_CLOUD_API_KEY");

    for bad in [
        "".to_owned(),
        "HAS=EQUALS".to_owned(),
        "has space".to_owned(),
        "x".repeat(129),
    ] {
        assert!(
            ProviderCredentials::parse_env_key(bad).is_err(),
            "malformed env key must be rejected"
        );
    }

    let mut registry = ProviderRegistry::new();
    let sandbox = Arc::new(SandboxCloudFixture::new(2).expect("fixture"));
    let id = registry
        .register_with_credentials(sandbox, reference)
        .expect("register with credentials");
    let stored = registry.credentials(&id).expect("credential reference");
    assert_eq!(stored.env_key_name(), "SANDBOX_CLOUD_API_KEY");
    // The reference is a name only: there is no surface that could hold
    // a credential value.
    let rendered = format!("{stored:?}");
    assert!(rendered.contains("SANDBOX_CLOUD_API_KEY"));

    let unknown = ProviderId::parse("not-registered").expect("provider id");
    assert!(registry.credentials(&unknown).is_none());
    assert!(registry.provider(&unknown).is_none());
}
