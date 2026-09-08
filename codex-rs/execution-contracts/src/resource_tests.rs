use super::*;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ResourceId;
use codex_workflow_contracts::ResourceTypeId;
use pretty_assertions::assert_eq;

fn browser_profile_binding() -> ResourceBinding {
    ResourceBinding {
        resource_type: ResourceTypeId::parse("browser_profile").expect("valid resource type"),
        resource: ResourceId::parse("profile-default").expect("valid resource id"),
        holder: ExecutionEnvironment::Browser,
    }
}

#[test]
fn resource_bindings_carry_no_credential_material() {
    let binding = browser_profile_binding();
    // The record is typed identities only: an opaque instance id plus the
    // owning environment. Nothing in the type can carry a secret.
    let serialized = serde_json::to_string(&binding).expect("serializable");
    assert_eq!(
        serialized,
        "{\"resourceType\":\"browser_profile\",\"resource\":\"profile-default\",\"holder\":\"browser\"}"
    );
}

#[test]
fn resource_bindings_reject_reserved_holders() {
    let binding = ResourceBinding {
        resource_type: ResourceTypeId::parse("mobile_device").expect("valid resource type"),
        resource: ResourceId::parse("pixel-8").expect("valid resource id"),
        holder: ExecutionEnvironment::Mobile,
    };
    let error = binding
        .validate()
        .expect_err("reserved environments cannot hold resources");
    assert!(matches!(
        error,
        ExecutionContractError::ReservedEnvironment { .. }
    ));
}

#[test]
fn resource_bindings_round_trip_through_serde() {
    let binding = browser_profile_binding();
    let serialized = serde_json::to_string(&binding).expect("serializable");
    let round_tripped: ResourceBinding = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, binding);
}
