use super::*;
use crate::AdapterId;
use crate::BindingClass;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ResourceTypeId;
use pretty_assertions::assert_eq;

fn provided() -> ProvidedCapability {
    ProvidedCapability {
        capability: CapabilityId::parse("navigate_web").expect("valid capability"),
        resources: vec![
            ResourceTypeId::parse("browser_profile").expect("valid resource type"),
            ResourceTypeId::parse("terminal_workspace").expect("valid resource type"),
        ],
    }
}

#[test]
fn bindings_derive_identity_from_adapter_and_capability() {
    let adapter = AdapterId::parse("codex-browser-use").expect("valid adapter");
    let binding = CapabilityBinding::from_parts(
        &adapter,
        ExecutionEnvironment::Browser,
        BindingClass::CodexNative,
        &provided(),
    );
    assert_eq!(
        binding.binding,
        CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding id")
    );
    assert_eq!(binding.environment, ExecutionEnvironment::Browser);
    assert_eq!(binding.class, BindingClass::CodexNative);
}

#[test]
fn bindings_sort_and_deduplicate_required_resources() {
    let adapter = AdapterId::parse("codex-browser-use").expect("valid adapter");
    let mut duplicated = provided();
    duplicated
        .resources
        .push(ResourceTypeId::parse("browser_profile").expect("dup"));
    let binding = CapabilityBinding::from_parts(
        &adapter,
        ExecutionEnvironment::Browser,
        BindingClass::CodexNative,
        &duplicated,
    );
    assert_eq!(
        binding.resources,
        vec![
            ResourceTypeId::parse("browser_profile").expect("valid resource type"),
            ResourceTypeId::parse("terminal_workspace").expect("valid resource type"),
        ]
    );
    binding.validate().expect("deduplicated record is valid");
}

#[test]
fn bindings_reject_reserved_environments() {
    let adapter = AdapterId::parse("mobile-bridge").expect("valid adapter");
    let binding = CapabilityBinding::from_parts(
        &adapter,
        ExecutionEnvironment::Mobile,
        BindingClass::Compatible,
        &provided(),
    );
    let error = binding
        .validate()
        .expect_err("reserved environments cannot carry bindings");
    assert!(matches!(
        error,
        ExecutionContractError::ReservedEnvironment { .. }
    ));
}

#[test]
fn binding_classes_order_the_fallback_chain() {
    let mut classes = vec![
        BindingClass::HumanFallback,
        BindingClass::Compatible,
        BindingClass::CodexNative,
    ];
    classes.sort();
    assert_eq!(
        classes,
        vec![
            BindingClass::CodexNative,
            BindingClass::Compatible,
            BindingClass::HumanFallback
        ]
    );
}

#[test]
fn bindings_round_trip_through_serde() {
    let adapter = AdapterId::parse("terminal.local").expect("valid adapter");
    let binding = CapabilityBinding::from_parts(
        &adapter,
        ExecutionEnvironment::Terminal,
        BindingClass::CodexNative,
        &provided(),
    );
    let serialized = serde_json::to_string(&binding).expect("serializable");
    let round_tripped: CapabilityBinding =
        serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, binding);
}
