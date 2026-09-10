//! Candidate contract tests.

use std::collections::BTreeMap;

use codex_execution_contracts::BindingPolicy;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowIrNode;
use pretty_assertions::assert_eq;

use super::CandidateId;
use super::ProposedChange;
use super::VersionBump;
use crate::WorkflowEvolutionError;
use crate::test_support::node;
use crate::test_support::probe_definition;
use crate::test_support::probe_version;
use crate::test_support::seal_probe;

#[test]
fn candidate_ids_validate_their_shape() {
    assert!(CandidateId::parse("evolution-definition-delta-0123456789abcdef").is_ok());
    assert!(CandidateId::parse("").is_err());
    assert!(CandidateId::parse("Evolution-Caps").is_err());
    assert!(CandidateId::parse("evolution spaces").is_err());
    assert!(CandidateId::parse("e".repeat(97)).is_err());
}

#[test]
fn version_bumps_apply_plain_successors() {
    let incumbent = SemanticVersion::new(1, 4, 7);
    assert_eq!(
        VersionBump::Patch.apply(&incumbent),
        SemanticVersion::new(1, 4, 8)
    );
    assert_eq!(
        VersionBump::Minor.apply(&incumbent),
        SemanticVersion::new(1, 5, 0)
    );
    assert_eq!(
        VersionBump::Major.apply(&incumbent),
        SemanticVersion::new(2, 0, 0)
    );
    // Pre-release and build metadata clear: successors are plain releases.
    let tagged = SemanticVersion::new(3, 1, 4);
    assert_eq!(
        VersionBump::Patch.apply(&tagged),
        SemanticVersion::new(3, 1, 5)
    );
}

#[test]
fn binding_changes_apply_to_step_nodes_only() {
    let incumbent = probe_version();
    let capability = CapabilityId::parse("inspect_environment").expect("capability");
    let change = ProposedChange::Bindings {
        bindings: BTreeMap::from([(
            node("step-001"),
            vec![CapabilityRequirement {
                capability,
                purpose: Some("revised purpose".to_string()),
            }],
        )]),
    };
    let (definition, lock) = change
        .applied_over(&CandidateId::parse("unit-candidate").unwrap(), &incumbent)
        .expect("applied");
    assert_eq!(lock, incumbent.dependency_lock);
    let WorkflowIrNode::Step(step) = &definition.ir.nodes[&node("step-001")] else {
        panic!("step node");
    };
    assert_eq!(
        step.capabilities[0].purpose.as_deref(),
        Some("revised purpose")
    );
    // The incumbent record is untouched: applying is read-only over it.
    assert_ne!(definition, incumbent.definition);
    assert_eq!(incumbent, probe_version());
}

#[test]
fn binding_changes_refuse_non_step_and_missing_nodes() {
    let incumbent = probe_version();
    let owner = CandidateId::parse("unit-candidate").unwrap();
    let missing = node("step-999");
    let requirement = CapabilityRequirement {
        capability: CapabilityId::parse("inspect_environment").expect("capability"),
        purpose: None,
    };
    let targeting_missing = ProposedChange::Bindings {
        bindings: BTreeMap::from([(missing, vec![requirement.clone()])]),
    };
    assert!(matches!(
        targeting_missing.applied_over(&owner, &incumbent),
        Err(WorkflowEvolutionError::InvalidCandidate { .. })
    ));
    // Turn the second step into a wait node so a binding change has a
    // non-step target to refuse.
    let mut mutated = incumbent.definition;
    mutated.ir.nodes.remove(&node("step-002"));
    mutated.ir.nodes.insert(
        node("step-002"),
        WorkflowIrNode::Wait(codex_workflow_contracts::WaitNode {
            wait_for: codex_workflow_contracts::WaitFor::Condition(
                codex_workflow_contracts::ConditionId::parse("ready").expect("condition"),
            ),
            next: None,
        }),
    );
    mutated.ir.conditions.insert(
        codex_workflow_contracts::ConditionId::parse("ready").expect("condition"),
        codex_workflow_contracts::WorkflowCondition {
            condition: codex_workflow_contracts::ConditionId::parse("ready").expect("condition"),
            description: "probe ready".to_string(),
        },
    );
    let owner_two = CandidateId::parse("unit-candidate-2").unwrap();
    let incumbent_two = seal_probe(mutated, "cd", DependencyLock::default());
    let targeting_wait = ProposedChange::Bindings {
        bindings: BTreeMap::from([(node("step-002"), vec![requirement])]),
    };
    assert!(matches!(
        targeting_wait.applied_over(&owner_two, &incumbent_two),
        Err(WorkflowEvolutionError::InvalidCandidate { .. })
    ));
}

#[test]
fn definition_deltas_must_keep_workflow_identity() {
    let incumbent = probe_version();
    let mut definition = probe_definition();
    definition.id = codex_workflow_contracts::WorkflowDefinitionId::parse("other-workflow")
        .expect("workflow id");
    let change = ProposedChange::Definition { definition };
    let owner = CandidateId::parse("unit-candidate").unwrap();
    assert!(matches!(
        change.applied_over(&owner, &incumbent),
        Err(WorkflowEvolutionError::InvalidCandidate { .. })
    ));
}

#[test]
fn recovery_and_schedule_changes_keep_version_content() {
    let incumbent = probe_version();
    let recovery = ProposedChange::Recovery {
        policy: BindingPolicy::default(),
    };
    let owner = CandidateId::parse("unit-candidate").unwrap();
    let (definition, lock) = recovery.applied_over(&owner, &incumbent).expect("applied");
    assert_eq!(definition, incumbent.definition);
    assert_eq!(lock, incumbent.dependency_lock);
    let schedule = ProposedChange::Schedule {
        spec: codex_workflow_triggers::ScheduleSpec::Every { period_ms: 60_000 },
    };
    let (definition, lock) = schedule.applied_over(&owner, &incumbent).expect("applied");
    assert_eq!(definition, incumbent.definition);
    assert_eq!(lock, incumbent.dependency_lock);
}
