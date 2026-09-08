use super::*;
use crate::AdapterId;
use crate::BindingClass;
use crate::CapabilityBindingId;
use crate::ExecutionEnvironment;
use crate::ReadinessState;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceTypeId;
use pretty_assertions::assert_eq;

fn requirement(capability: &str) -> CapabilityRequirement {
    CapabilityRequirement {
        capability: CapabilityId::parse(capability).expect("valid capability"),
        purpose: None,
    }
}

fn selection(binding: &str, environment: ExecutionEnvironment) -> SelectedBinding {
    let binding = CapabilityBindingId::parse(binding).expect("valid binding");
    SelectedBinding {
        adapter: AdapterId::parse(binding.as_ref().split(':').next().expect("adapter part"))
            .expect("valid adapter"),
        binding,
        environment,
        class: BindingClass::Compatible,
    }
}

#[test]
fn plans_report_their_distinct_environments() {
    let plan = BindingPlan {
        steps: vec![
            StepBinding {
                node: IrNodeId::parse("browse").expect("valid node"),
                decisions: vec![BindingDecision {
                    requirement: requirement("navigate_web"),
                    selected: selection(
                        "external-browser:navigate_web",
                        ExecutionEnvironment::Browser,
                    ),
                    fallback: None,
                }],
            },
            StepBinding {
                node: IrNodeId::parse("run").expect("valid node"),
                decisions: vec![BindingDecision {
                    requirement: requirement("run_terminal_command"),
                    selected: selection(
                        "terminal.local:run_terminal_command",
                        ExecutionEnvironment::Terminal,
                    ),
                    fallback: None,
                }],
            },
        ],
    };
    assert_eq!(
        plan.environments(),
        vec![
            ExecutionEnvironment::Browser,
            ExecutionEnvironment::Terminal
        ]
    );
    let single = BindingPlan {
        steps: vec![plan.steps[0].clone()],
    };
    assert_eq!(single.environments(), vec![ExecutionEnvironment::Browser]);
}

#[test]
fn plans_keep_steps_without_requirements_explicit() {
    let plan = BindingPlan {
        steps: vec![StepBinding {
            node: IrNodeId::parse("manual-review").expect("valid node"),
            decisions: Vec::new(),
        }],
    };
    assert!(plan.environments().is_empty());
    assert_eq!(plan.steps.len(), 1);
}

#[test]
fn decisions_round_trip_through_serde_with_fallback_records() {
    let decision = BindingDecision {
        requirement: requirement("navigate_web"),
        selected: selection(
            "external-browser:navigate_web",
            ExecutionEnvironment::Browser,
        ),
        fallback: Some(FallbackRecord {
            skipped: selection(
                "codex-browser-use:navigate_web",
                ExecutionEnvironment::Browser,
            ),
            reason: FallbackReason::PreferredOutsideEnvironmentScope,
        }),
    };
    let serialized = serde_json::to_string(&decision).expect("serializable");
    let round_tripped: BindingDecision = serde_json::from_str(&serialized).expect("deserializable");
    assert_eq!(round_tripped, decision);
}

#[test]
fn every_fallback_reason_round_trips_through_serde() {
    let reasons = vec![
        FallbackReason::PreferredNotReady {
            state: ReadinessState::Failed,
            diagnostic: None,
        },
        FallbackReason::PreferredOutsideEnvironmentScope,
        FallbackReason::PreferredHumanFallbackForbidden,
        FallbackReason::PreferredMissingResources {
            missing: vec![ResourceTypeId::parse("browser_profile").expect("valid type")],
        },
    ];
    for reason in reasons {
        let serialized = serde_json::to_string(&reason).expect("serializable");
        let round_tripped: FallbackReason =
            serde_json::from_str(&serialized).expect("deserializable");
        assert_eq!(round_tripped, reason);
    }
}
