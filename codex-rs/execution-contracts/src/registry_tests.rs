//! Registry conformance tests.
//!
//! These tests are the WO-005 acceptance evidence: a scripted adapter
//! implementing [`EnvironmentAdapter`] drives the registry through the
//! full binding lifecycle, mixed-environment planning, fallback with
//! evidence, explicit unavailability diagnostics, and single-action
//! dispatch.

use super::*;
use crate::Action;
use crate::ActionOutcome;
use crate::ActionResult;
use crate::ActionTarget;
use crate::AdapterDescriptor;
use crate::AdapterExecuteFuture;
use crate::AdapterId;
use crate::AdapterPrepareFuture;
use crate::AdapterProbeFuture;
use crate::AuthorizationGrant;
use crate::BindingClass;
use crate::BindingDiagnostic;
use crate::BindingPolicy;
use crate::CapabilityBindingId;
use crate::DiagnosticCode;
use crate::EnvironmentAdapter;
use crate::EnvironmentScope;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ExecutionFailure;
use crate::FailureKind;
use crate::FallbackReason;
use crate::HumanFallbackPolicy;
use crate::Observation;
use crate::OperationId;
use crate::PrepareRequest;
use crate::PreparedSession;
use crate::ProbeReport;
use crate::ProbeStatus;
use crate::ProvidedCapability;
use crate::ReadinessState;
use crate::ResourceBinding;
use crate::ResourceId;
use crate::SessionHandle;
use crate::TransitionCause;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::sync::Arc;

/// What the scripted adapter's probe reports.
#[derive(Clone)]
enum ProbeBehavior {
    Ready,
    AvailableOnly,
    NotInstalled,
    Error,
}

/// A deterministic adapter for conformance testing.
struct ScriptedAdapter {
    descriptor: AdapterDescriptor,
    probe: ProbeBehavior,
    prepare_fails: Option<ExecutionFailure>,
    execute_fails: Option<ExecutionFailure>,
}

impl ScriptedAdapter {
    fn new(descriptor: AdapterDescriptor) -> Self {
        Self {
            descriptor,
            probe: ProbeBehavior::Ready,
            prepare_fails: None,
            execute_fails: None,
        }
    }

    fn probing(mut self, probe: ProbeBehavior) -> Self {
        self.probe = probe;
        self
    }

    fn failing_prepare(mut self, failure: ExecutionFailure) -> Self {
        self.prepare_fails = Some(failure);
        self
    }

    fn failing_execute(mut self, failure: ExecutionFailure) -> Self {
        self.execute_fails = Some(failure);
        self
    }

    fn arc(self) -> Arc<dyn EnvironmentAdapter> {
        Arc::new(self)
    }
}

impl EnvironmentAdapter for ScriptedAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn probe(&self) -> AdapterProbeFuture<'_> {
        let behavior = self.probe.clone();
        let environment = self.descriptor.environment;
        Box::pin(async move {
            match behavior {
                ProbeBehavior::Ready => Ok(ProbeReport::ready()),
                ProbeBehavior::AvailableOnly => Ok(ProbeReport {
                    status: ProbeStatus::Available,
                    diagnostic: None,
                }),
                ProbeBehavior::NotInstalled => Ok(ProbeReport {
                    status: ProbeStatus::NotInstalled,
                    diagnostic: Some(
                        BindingDiagnostic::new(DiagnosticCode::NotInstalled, "runtime missing")
                            .in_environment(environment),
                    ),
                }),
                ProbeBehavior::Error => Err(ExecutionFailure::new(
                    FailureKind::Unavailable,
                    "probe transport failed",
                )
                .expect("valid failure")),
            }
        })
    }

    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_> {
        let failure = self.prepare_fails.clone();
        Box::pin(async move {
            if let Some(failure) = failure {
                return Err(failure);
            }
            let session = SessionHandle::parse(format!("{}|session-1", request.binding))
                .expect("valid session handle");
            Ok(PreparedSession {
                binding: request.binding,
                session,
            })
        })
    }

    fn execute<'a>(
        &'a self,
        session: &'a SessionHandle,
        action: Action,
    ) -> AdapterExecuteFuture<'a> {
        let failure = self.execute_fails.clone();
        let environment = self.descriptor.environment;
        let adapter = self.descriptor.adapter.clone();
        let operation = action.operation;
        let binding = CapabilityBindingId::parse(
            session
                .as_ref()
                .split_once('|')
                .expect("scripted sessions encode their binding")
                .0
                .to_owned(),
        )
        .expect("valid binding id");
        Box::pin(async move {
            if let Some(failure) = failure {
                return ActionResult::failed(binding, failure);
            }
            let observation = Observation::new(
                environment,
                adapter,
                serde_json::json!({ "executed": operation }),
            );
            ActionResult::succeeded(binding).with_observations(vec![observation])
        })
    }
}

fn descriptor(
    adapter: &str,
    environment: ExecutionEnvironment,
    class: BindingClass,
    capabilities: &[(&str, &[&str])],
) -> AdapterDescriptor {
    AdapterDescriptor {
        adapter: AdapterId::parse(adapter).expect("valid adapter"),
        environment,
        class,
        provides: capabilities
            .iter()
            .map(|(capability, resources)| ProvidedCapability {
                capability: CapabilityId::parse(*capability).expect("valid capability"),
                resources: resources
                    .iter()
                    .map(|resource| ResourceTypeId::parse(*resource).expect("valid resource"))
                    .collect(),
            })
            .collect(),
    }
}

fn requirement(capability: &str) -> CapabilityRequirement {
    CapabilityRequirement {
        capability: CapabilityId::parse(capability).expect("valid capability"),
        purpose: None,
    }
}

fn approval_evidence(locator: &str) -> EvidenceReference {
    EvidenceReference {
        kind: EvidenceKind::Approval,
        locator: locator.to_owned(),
        digest: ContentDigest::of(&serde_json::json!({ "decision": "approved" }))
            .expect("digestable approval"),
    }
}

fn browser_profile() -> ResourceBinding {
    ResourceBinding {
        resource_type: ResourceTypeId::parse("browser_profile").expect("valid resource type"),
        resource: ResourceId::parse("profile-default").expect("valid resource id"),
        holder: ExecutionEnvironment::Browser,
    }
}

fn browser_binding() -> CapabilityBindingId {
    CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding id")
}

fn linear_ir(step_capabilities: &[(&str, &[&str])]) -> WorkflowIr {
    let mut nodes = BTreeMap::new();
    let names: Vec<IrNodeId> = step_capabilities
        .iter()
        .map(|(node, _)| IrNodeId::parse(*node).expect("valid node id"))
        .collect();
    for (index, (node, capabilities)) in step_capabilities.iter().enumerate() {
        let next = names.get(index + 1).cloned();
        nodes.insert(
            IrNodeId::parse(*node).expect("valid node id"),
            WorkflowIrNode::Step(StepNode {
                capabilities: capabilities
                    .iter()
                    .map(|capability| requirement(capability))
                    .collect(),
                roles: Vec::new(),
                description: None,
                next,
            }),
        );
    }
    let entry = names.first().cloned().expect("at least one step");
    WorkflowIr {
        ir_format: IR_FORMAT_VERSION,
        entry,
        nodes,
        conditions: BTreeMap::new(),
    }
}

#[tokio::test]
async fn registration_creates_declared_bindings() {
    let mut registry = CapabilityRegistry::new();
    let created = registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &["browser_profile"])],
            ))
            .arc(),
        )
        .expect("adapter registers");
    assert_eq!(
        created,
        vec![
            CapabilityBindingId::parse("codex-browser-use:navigate_web").expect("valid binding id")
        ]
    );
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Declared);
    assert!(registered.authorization.is_none());
    assert!(registered.attached.is_empty());
}

#[tokio::test]
async fn duplicate_adapters_are_rejected() {
    let mut registry = CapabilityRegistry::new();
    let adapter = ScriptedAdapter::new(descriptor(
        "codex-browser-use",
        ExecutionEnvironment::Browser,
        BindingClass::CodexNative,
        &[("navigate_web", &[])],
    ))
    .arc();
    registry
        .register_adapter(adapter.clone())
        .expect("first registration");
    let error = registry
        .register_adapter(adapter)
        .expect_err("duplicate adapters are rejected");
    assert!(matches!(
        error,
        ExecutionContractError::DuplicateAdapter { .. }
    ));
}

#[tokio::test]
async fn reserved_environments_cannot_register_adapters() {
    let mut registry = CapabilityRegistry::new();
    let error = registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "mobile-bridge",
                ExecutionEnvironment::Mobile,
                BindingClass::Compatible,
                &[("tap_element", &[])],
            ))
            .arc(),
        )
        .expect_err("reserved environments are rejected");
    assert!(matches!(
        error,
        ExecutionContractError::ReservedEnvironment { .. }
    ));
}

#[tokio::test]
async fn probing_ready_walks_two_transitions_to_ready() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .refresh_readiness()
        .await
        .expect("refresh succeeds");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Ready);
    assert_eq!(registered.readiness.history().len(), 2);
    assert_eq!(registered.readiness.diagnostic(), None);
}

#[tokio::test]
async fn probing_available_stops_at_available() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .probing(ProbeBehavior::AvailableOnly)
            .arc(),
        )
        .expect("registered");
    registry
        .refresh_readiness()
        .await
        .expect("refresh succeeds");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Available);
}

#[tokio::test]
async fn missing_runtimes_become_unavailable_with_diagnostics() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .probing(ProbeBehavior::NotInstalled)
            .arc(),
        )
        .expect("registered");
    registry
        .refresh_readiness()
        .await
        .expect("refresh succeeds");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Unavailable);
    let diagnostic = registered
        .readiness
        .diagnostic()
        .expect("unavailable bindings carry diagnostics");
    assert_eq!(diagnostic.code, DiagnosticCode::NotInstalled);
}

#[tokio::test]
async fn probe_errors_become_failed_with_diagnostics() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .probing(ProbeBehavior::Error)
            .arc(),
        )
        .expect("registered");
    registry
        .refresh_readiness()
        .await
        .expect("refresh succeeds");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Failed);
    assert_eq!(
        registered.readiness.diagnostic().map(|d| d.code),
        Some(DiagnosticCode::ProbeFailed)
    );
}

#[tokio::test]
async fn authorization_requires_a_matching_grant_and_ready_state() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    // A grant issued for another binding is rejected.
    let foreign = AuthorizationGrant::new(
        CapabilityBindingId::parse("terminal.local:run_terminal_command").expect("other binding"),
        approval_evidence("approvals/1"),
    )
    .expect("valid grant");
    let error = registry
        .advance(&browser_binding(), TransitionCause::Authorized(foreign))
        .expect_err("grants must match their subject binding");
    assert!(matches!(
        error,
        ExecutionContractError::AuthorizationSubjectMismatch { .. }
    ));

    // Authorizing before READY is illegal.
    let mut fresh = CapabilityRegistry::new();
    fresh
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    let grant = AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/1"))
        .expect("valid grant");
    let error = fresh
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(grant.clone()),
        )
        .expect_err("authorization requires READY");
    assert!(matches!(
        error,
        ExecutionContractError::IllegalTransition { .. }
    ));

    // The valid path stores the grant.
    registry
        .advance(&browser_binding(), TransitionCause::Authorized(grant))
        .expect("authorized");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Authorized);
    assert!(registered.authorization.is_some());
}

#[tokio::test]
async fn binding_requires_every_required_resource() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &["browser_profile"])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(
                AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/1"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");

    let error = registry
        .bind(&browser_binding())
        .expect_err("unbound resources fail binding explicitly");
    assert!(matches!(
        error,
        ExecutionContractError::ResourceRequirementMissing { ref missing, .. }
            if missing.len() == 1
    ));

    registry
        .bind_resource(browser_profile())
        .expect("resource binds");
    let attached = registry.bind(&browser_binding()).expect("bound");
    assert_eq!(attached, vec![browser_profile()]);
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Bound);
    assert_eq!(registered.attached, vec![browser_profile()]);
}

#[tokio::test]
async fn unavailable_required_capabilities_fail_resolution_explicitly() {
    let mut registry = CapabilityRegistry::new();
    let diagnostic = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect_err("no adapter is registered");
    assert_eq!(diagnostic.code, DiagnosticCode::NotRegistered);
    assert!(diagnostic.message.contains("navigate_web"));

    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .probing(ProbeBehavior::NotInstalled)
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    let diagnostic = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect_err("unavailable capability is not resolvable");
    assert_eq!(diagnostic.code, DiagnosticCode::NotInstalled);
    assert!(
        !registry
            .diagnose(&CapabilityId::parse("navigate_web").expect("capability"))
            .is_empty()
    );
}

#[tokio::test]
async fn resolution_prefers_codex_native_bindings() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "external-browser",
                ExecutionEnvironment::Browser,
                BindingClass::Compatible,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    let decision = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect("resolvable");
    assert_eq!(decision.selected.class, BindingClass::CodexNative);
    assert_eq!(
        decision.selected.adapter,
        AdapterId::parse("codex-browser-use").expect("adapter")
    );
    assert!(decision.fallback.is_none());
}

#[tokio::test]
async fn the_same_step_semantics_bind_to_different_environments_under_policy() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "acme-api",
                ExecutionEnvironment::Api,
                BindingClass::Compatible,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    // Unscoped policy prefers the native browser binding.
    let native = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect("resolvable");
    assert_eq!(native.selected.environment, ExecutionEnvironment::Browser);

    // A scoped policy routes the same semantic requirement to the API
    // binding and records why the preferred binding was skipped.
    let api_only = BindingPolicy {
        environments: EnvironmentScope::Only(vec![ExecutionEnvironment::Api]),
        human_fallback: HumanFallbackPolicy::Forbidden,
    };
    let routed = registry
        .resolve(&requirement("navigate_web"), &api_only)
        .expect("resolvable under scope");
    assert_eq!(routed.selected.environment, ExecutionEnvironment::Api);
    let fallback = routed
        .fallback
        .as_ref()
        .expect("policy fallback is recorded");
    assert_eq!(fallback.skipped.environment, ExecutionEnvironment::Browser);
    assert!(matches!(
        fallback.reason,
        FallbackReason::PreferredOutsideEnvironmentScope
    ));
    let reference = routed
        .to_evidence_reference("bindings/1")
        .expect("fallback decisions are evidence");
    assert_eq!(reference.kind, EvidenceKind::Recovery);
    assert_eq!(reference.locator, "bindings/1");
}

#[tokio::test]
async fn fallback_from_an_unready_native_binding_is_evidenced() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .probing(ProbeBehavior::NotInstalled)
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "external-browser",
                ExecutionEnvironment::Browser,
                BindingClass::Compatible,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    let decision = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect("compatible binding resolves");
    assert_eq!(decision.selected.class, BindingClass::Compatible);
    let fallback = decision.fallback.as_ref().expect("fallback recorded");
    assert_eq!(fallback.skipped.class, BindingClass::CodexNative);
    match &fallback.reason {
        FallbackReason::PreferredNotReady { state, diagnostic } => {
            assert_eq!(*state, ReadinessState::Unavailable);
            assert!(diagnostic.is_some());
        }
        other => panic!("expected a not-ready fallback reason, got {other:?}"),
    }
    let reference = decision
        .to_evidence_reference("bindings/2")
        .expect("fallback decisions are evidence");
    assert_eq!(
        reference.digest,
        ContentDigest::of(&decision).expect("digestable")
    );
}

#[tokio::test]
async fn human_fallback_requires_explicit_permission() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "human-ops",
                ExecutionEnvironment::Human,
                BindingClass::HumanFallback,
                &[("approve_payment", &["human_approver"])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .bind_resource(ResourceBinding {
            resource_type: ResourceTypeId::parse("human_approver").expect("valid type"),
            resource: ResourceId::parse("approver-ada").expect("valid resource"),
            holder: ExecutionEnvironment::Human,
        })
        .expect("resource binds");
    registry.refresh_readiness().await.expect("refreshed");

    // The default policy forbids silently routing work to a human.
    let diagnostic = registry
        .resolve(&requirement("approve_payment"), &BindingPolicy::default())
        .expect_err("human fallback is not the default");
    assert_eq!(diagnostic.code, DiagnosticCode::PolicyDenied);

    let permitted = BindingPolicy {
        environments: EnvironmentScope::All,
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    let decision = registry
        .resolve(&requirement("approve_payment"), &permitted)
        .expect("explicit permission selects the human binding");
    assert_eq!(decision.selected.class, BindingClass::HumanFallback);
    assert_eq!(decision.selected.environment, ExecutionEnvironment::Human);
}

#[tokio::test]
async fn bindings_with_unbound_resources_are_not_selected() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &["browser_profile"])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    let diagnostic = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect_err("unbound resources block selection");
    assert_eq!(diagnostic.code, DiagnosticCode::ResourceMissing);

    registry
        .bind_resource(browser_profile())
        .expect("resource binds");
    registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect("resolvable once the resource is bound");
}

#[tokio::test]
async fn one_plan_routes_multiple_environments_in_one_run() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "terminal.local",
                ExecutionEnvironment::Terminal,
                BindingClass::CodexNative,
                &[("run_terminal_command", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "human-ops",
                ExecutionEnvironment::Human,
                BindingClass::HumanFallback,
                &[("approve_payment", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    let ir = linear_ir(&[
        ("step-1", &["navigate_web"]),
        ("step-2", &["run_terminal_command"]),
        ("step-3", &["approve_payment"]),
    ]);
    ir.validate().expect("well-formed IR");

    let default_plan = registry.plan(&ir, &BindingPolicy::default());
    assert!(
        default_plan.is_err(),
        "human fallback is not permitted by default"
    );

    let policy = BindingPolicy {
        environments: EnvironmentScope::All,
        human_fallback: HumanFallbackPolicy::Allowed,
    };
    let plan = registry.plan(&ir, &policy).expect("mixed plan");
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(
        plan.environments(),
        vec![
            ExecutionEnvironment::Browser,
            ExecutionEnvironment::Terminal,
            ExecutionEnvironment::Human,
        ]
    );
    for step in &plan.steps {
        assert_eq!(step.decisions.len(), 1, "every requirement binds");
    }
}

#[tokio::test]
async fn plans_fail_explicitly_on_unavailable_capabilities() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    let ir = linear_ir(&[("step-1", &["navigate_web"]), ("step-2", &["send_email"])]);
    let diagnostic = registry
        .plan(&ir, &BindingPolicy::default())
        .expect_err("send_email has no binding");
    assert_eq!(diagnostic.code, DiagnosticCode::NotRegistered);
    assert!(diagnostic.message.contains("send_email"));
}

#[tokio::test]
async fn diagnosis_explains_every_obstacle() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &["browser_profile"])],
            ))
            .arc(),
        )
        .expect("registered");

    // Before probing: the binding is declared but not ready.
    let diagnostics = registry.diagnose(&CapabilityId::parse("navigate_web").expect("cap"));
    assert_eq!(diagnostics.len(), 2, "not-installed + missing resource");

    registry.refresh_readiness().await.expect("refreshed");
    let diagnostics = registry.diagnose(&CapabilityId::parse("navigate_web").expect("cap"));
    assert_eq!(diagnostics.len(), 1, "only the missing resource remains");
    assert_eq!(diagnostics[0].code, DiagnosticCode::ResourceMissing);

    let unknown = registry.diagnose(&CapabilityId::parse("send_email").expect("cap"));
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].code, DiagnosticCode::NotRegistered);
}

#[tokio::test]
async fn dispatch_executes_one_action_through_a_prepared_session() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &["browser_profile"])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(
                AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/9"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry
        .bind_resource(browser_profile())
        .expect("resource binds");
    registry.bind(&browser_binding()).expect("bound");

    let mut action = Action::new(OperationId::parse("navigate").expect("valid operation"));
    action.target = Some(ActionTarget::parse("https://example.com").expect("valid target"));
    let result = registry
        .dispatch(&browser_binding(), action)
        .await
        .expect("dispatch succeeds");
    assert_eq!(result.outcome, ActionOutcome::Succeeded);
    assert_eq!(result.binding, browser_binding());
    assert_eq!(result.observations.len(), 1);
    let reference = result.observations[0]
        .to_evidence_reference("observations/9")
        .expect("observations are evidence");
    assert_eq!(reference.kind, EvidenceKind::Observation);

    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Bound);
    assert_eq!(registered.readiness.history().len(), 6);
}

#[tokio::test]
async fn dispatching_unbound_bindings_is_a_contract_violation() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    let action = Action::new(OperationId::parse("navigate").expect("valid operation"));
    let error = registry
        .dispatch(&browser_binding(), action)
        .await
        .expect_err("unbound bindings cannot dispatch");
    assert!(matches!(
        error,
        ExecutionContractError::BindingNotReady {
            state: ReadinessState::Ready,
            ..
        }
    ));
}

#[tokio::test]
async fn failed_actions_fail_the_binding_with_diagnostics() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .failing_execute(
                ExecutionFailure::new(FailureKind::Permanent, "element not found")
                    .expect("valid failure"),
            )
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(
                AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/2"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry.bind(&browser_binding()).expect("bound");

    let action = Action::new(OperationId::parse("click").expect("valid operation"));
    let result = registry
        .dispatch(&browser_binding(), action)
        .await
        .expect("dispatch itself succeeds");
    assert_eq!(result.outcome, ActionOutcome::Failed);
    let failure = result.failure.expect("failed results carry failures");
    assert_eq!(failure.kind, FailureKind::Permanent);

    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Failed);
    assert_eq!(
        registered.readiness.diagnostic().map(|d| d.code),
        Some(DiagnosticCode::ExecutionFailure)
    );

    // Recovery re-establishes readiness for a retry.
    registry
        .advance(&browser_binding(), TransitionCause::Recovered)
        .expect("recovery returns the binding to READY");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Ready);
}

#[tokio::test]
async fn preparation_failures_return_failed_results() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .failing_prepare(
                ExecutionFailure::new(FailureKind::Transient, "session setup timed out")
                    .expect("valid failure"),
            )
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(
                AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/3"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry.bind(&browser_binding()).expect("bound");

    let action = Action::new(OperationId::parse("navigate").expect("valid operation"));
    let result = registry
        .dispatch(&browser_binding(), action)
        .await
        .expect("dispatch completes with a normalized failure");
    assert_eq!(result.outcome, ActionOutcome::Failed);
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Failed);
}

#[tokio::test]
async fn unknown_bindings_and_invalid_actions_fail_explicitly() {
    let mut registry = CapabilityRegistry::new();
    let action = Action::new(OperationId::parse("navigate").expect("valid operation"));
    let error = registry
        .dispatch(&browser_binding(), action)
        .await
        .expect_err("unknown bindings fail");
    assert!(matches!(
        error,
        ExecutionContractError::UnknownBinding { .. }
    ));

    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry
        .advance(
            &browser_binding(),
            TransitionCause::Authorized(
                AuthorizationGrant::new(browser_binding(), approval_evidence("approvals/4"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry.bind(&browser_binding()).expect("bound");

    let mut oversized = Action::new(OperationId::parse("type_text").expect("valid operation"));
    oversized.inputs.insert(
        "text".to_owned(),
        serde_json::Value::String("x".repeat(70_000)),
    );
    let error = registry
        .dispatch(&browser_binding(), oversized)
        .await
        .expect_err("invalid actions fail before reaching the adapter");
    assert!(matches!(
        error,
        ExecutionContractError::PayloadTooLarge { .. }
    ));
}

#[tokio::test]
async fn parallel_steps_can_dispatch_twice_on_one_binding() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "terminal.local",
                ExecutionEnvironment::Terminal,
                BindingClass::CodexNative,
                &[("run_terminal_command", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    let binding =
        CapabilityBindingId::parse("terminal.local:run_terminal_command").expect("binding");
    registry
        .advance(
            &binding,
            TransitionCause::Authorized(
                AuthorizationGrant::new(binding.clone(), approval_evidence("approvals/5"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry.bind(&binding).expect("bound");

    let first = registry
        .dispatch(
            &binding,
            Action::new(OperationId::parse("exec").expect("op")),
        )
        .await
        .expect("first dispatch");
    assert_eq!(first.outcome, ActionOutcome::Succeeded);
    let second = registry
        .dispatch(
            &binding,
            Action::new(OperationId::parse("exec").expect("op")),
        )
        .await
        .expect("second dispatch while bound again");
    assert_eq!(second.outcome, ActionOutcome::Succeeded);
}

#[tokio::test]
async fn ready_bindings_are_not_reprobed_by_refresh() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");
    registry.refresh_readiness().await.expect("second refresh");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.history().len(), 2);
}

#[tokio::test]
async fn environment_loss_and_recovery_flow_through_the_generic_gate() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "codex-browser-use",
                ExecutionEnvironment::Browser,
                BindingClass::CodexNative,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    registry
        .advance(
            &browser_binding(),
            TransitionCause::EnvironmentLost(BindingDiagnostic::new(
                DiagnosticCode::EnvironmentLost,
                "browser closed",
            )),
        )
        .expect("live bindings can lose their environment");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Unavailable);

    registry
        .advance(&browser_binding(), TransitionCause::Rediscovered)
        .expect("the capability can come back");
    let registered = registry
        .binding(&browser_binding())
        .expect("binding exists");
    assert_eq!(registered.readiness.state(), ReadinessState::Available);
}

#[tokio::test]
async fn resolution_prefers_bindings_that_already_passed_the_gates() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "browser-a",
                ExecutionEnvironment::Browser,
                BindingClass::Compatible,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry
        .register_adapter(
            ScriptedAdapter::new(descriptor(
                "browser-b",
                ExecutionEnvironment::Browser,
                BindingClass::Compatible,
                &[("navigate_web", &[])],
            ))
            .arc(),
        )
        .expect("registered");
    registry.refresh_readiness().await.expect("refreshed");

    // browser-b is authorized and bound; browser-a is only ready. Both are
    // the same class, so readiness decides.
    let binding_b = CapabilityBindingId::parse("browser-b:navigate_web").expect("valid binding");
    registry
        .advance(
            &binding_b,
            TransitionCause::Authorized(
                AuthorizationGrant::new(binding_b.clone(), approval_evidence("approvals/6"))
                    .expect("valid grant"),
            ),
        )
        .expect("authorized");
    registry.bind(&binding_b).expect("bound");

    let decision = registry
        .resolve(&requirement("navigate_web"), &BindingPolicy::default())
        .expect("resolvable");
    assert_eq!(decision.selected.binding, binding_b);
}
