//! Shared fixtures for the crate's unit tests.

use std::collections::BTreeMap;

use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;

/// The unit-test probe workflow identity.
pub(crate) const PROBE_WORKFLOW: &str = "unit-probe";

/// The unit-test probe repository identity.
pub(crate) const PROBE_REPOSITORY: &str = "github.com/unit/probe-workflows";

/// Builds a two-step probe definition shaped like the evaluation fixture.
pub(crate) fn probe_definition() -> WorkflowDefinition {
    let capability = CapabilityId::parse("inspect_environment").expect("capability");
    let step_one = IrNodeId::parse("step-001").expect("node");
    let step_two = IrNodeId::parse("step-002").expect("node");
    WorkflowDefinition {
        id: WorkflowDefinitionId::parse(PROBE_WORKFLOW).expect("workflow id"),
        description: None,
        ir: WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: step_one.clone(),
            nodes: BTreeMap::from([
                (
                    step_one,
                    WorkflowIrNode::Step(StepNode {
                        capabilities: vec![CapabilityRequirement {
                            capability: capability.clone(),
                            purpose: Some("inspect the issues list".to_string()),
                        }],
                        roles: Vec::new(),
                        description: None,
                        next: Some(step_two.clone()),
                    }),
                ),
                (
                    step_two,
                    WorkflowIrNode::Step(StepNode {
                        capabilities: vec![CapabilityRequirement {
                            capability,
                            purpose: Some("record the summary".to_string()),
                        }],
                        roles: Vec::new(),
                        description: None,
                        next: None,
                    }),
                ),
            ]),
            conditions: BTreeMap::new(),
        },
        roles: BTreeMap::new(),
        triggers: Vec::new(),
        dependencies: codex_workflow_contracts::WorkflowDependencies::default(),
    }
}

/// Seals a probe version at `(1, 0, 0)` with an empty lock.
pub(crate) fn probe_version() -> WorkflowVersion {
    seal_probe(probe_definition(), "ab", DependencyLock::default())
}

/// Seals a probe definition at a deterministic revision and lock.
pub(crate) fn seal_probe(
    definition: WorkflowDefinition,
    sha_prefix: &str,
    lock: DependencyLock,
) -> WorkflowVersion {
    let sha = format!("{sha_prefix}{}", "0".repeat(40 - sha_prefix.len()));
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse(PROBE_REPOSITORY).expect("repository"),
        ImmutableSourceRevision::pin_commit(RevisionSha::parse(sha).expect("sha")),
        SemanticVersion::new(1, 0, 0),
        lock,
        None,
    )
    .expect("sealed version")
}

/// Parses a probe node id.
pub(crate) fn node(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node")
}
