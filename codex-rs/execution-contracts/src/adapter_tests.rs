use super::*;
use crate::BindingClass;
use crate::BindingDiagnostic;
use crate::DiagnosticCode;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ResourceTypeId;
use pretty_assertions::assert_eq;

fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor {
        adapter: AdapterId::parse("codex-browser-use").expect("valid adapter"),
        environment: ExecutionEnvironment::Browser,
        class: BindingClass::CodexNative,
        provides: vec![ProvidedCapability {
            capability: CapabilityId::parse("navigate_web").expect("valid capability"),
            resources: vec![ResourceTypeId::parse("browser_profile").expect("valid type")],
        }],
    }
}

#[test]
fn descriptors_validate_the_codex_native_browser_shape() {
    descriptor().validate().expect("valid descriptor");
}

#[test]
fn descriptors_reject_reserved_environments() {
    let descriptor = AdapterDescriptor {
        environment: ExecutionEnvironment::Mobile,
        ..descriptor()
    };
    let error = descriptor
        .validate()
        .expect_err("reserved environments cannot have adapters");
    assert!(matches!(
        error,
        ExecutionContractError::ReservedEnvironment { .. }
    ));
}

#[test]
fn descriptors_require_capabilities() {
    let descriptor = AdapterDescriptor {
        provides: Vec::new(),
        ..descriptor()
    };
    let error = descriptor
        .validate()
        .expect_err("empty adapters are rejected");
    assert!(matches!(
        error,
        ExecutionContractError::InvalidRecord { .. }
    ));
}

#[test]
fn descriptors_reject_duplicate_capabilities() {
    let descriptor = AdapterDescriptor {
        provides: vec![
            ProvidedCapability {
                capability: CapabilityId::parse("navigate_web").expect("valid capability"),
                resources: Vec::new(),
            },
            ProvidedCapability {
                capability: CapabilityId::parse("navigate_web").expect("valid capability"),
                resources: Vec::new(),
            },
        ],
        ..descriptor()
    };
    assert!(descriptor.validate().is_err());
}

#[test]
fn descriptors_reject_duplicate_resource_requirements() {
    let descriptor = AdapterDescriptor {
        provides: vec![ProvidedCapability {
            capability: CapabilityId::parse("navigate_web").expect("valid capability"),
            resources: vec![
                ResourceTypeId::parse("browser_profile").expect("valid type"),
                ResourceTypeId::parse("browser_profile").expect("duplicate"),
            ],
        }],
        ..descriptor()
    };
    assert!(descriptor.validate().is_err());
}

#[test]
fn ready_probe_reports_need_no_diagnostic() {
    let report = ProbeReport::ready();
    report.validate().expect("ready reports are valid");
    assert_eq!(report.status, ProbeStatus::Ready);
    assert_eq!(report.diagnostic, None);
}

#[test]
fn not_installed_reports_require_diagnostics() {
    let missing = ProbeReport {
        status: ProbeStatus::NotInstalled,
        diagnostic: None,
    };
    assert!(missing.validate().is_err());
    let present = ProbeReport {
        status: ProbeStatus::NotInstalled,
        diagnostic: Some(BindingDiagnostic::new(
            DiagnosticCode::NotInstalled,
            "browser runtime missing",
        )),
    };
    present.validate().expect("diagnostic present");
}

#[test]
fn probe_statuses_round_trip_through_serde() {
    for status in [
        ProbeStatus::Ready,
        ProbeStatus::Available,
        ProbeStatus::NotInstalled,
    ] {
        let serialized = serde_json::to_string(&status).expect("serializable");
        let round_tripped: ProbeStatus = serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, status);
    }
}
