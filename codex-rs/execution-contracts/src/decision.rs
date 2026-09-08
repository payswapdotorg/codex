//! Binding decisions, fallback evidence, and binding plans.
//!
//! A [`BindingDecision`] is the record of routing one semantic capability
//! requirement to one selected binding. When the preferred candidate was
//! skipped, the decision carries a [`FallbackRecord`] naming the skipped
//! candidate and the structured reason. Per the frozen architecture, the
//! selected binding and the fallback reason must be recorded as evidence;
//! [`BindingDecision::to_evidence_reference`] produces that reference,
//! using the `Recovery` evidence kind because a fallback selection is a
//! rebind-class recovery decision (the architecture's failure pipeline
//! lists `rebind` among recover/retry/takeover/rebind/replan).
//!
//! A [`BindingPlan`] resolves every step of a workflow IR: it is how one
//! workflow execution comes to contain multiple environments — each step
//! requirement is bound independently, so a single plan can route
//! different steps (or even different capabilities of one step) to
//! different environment classes. The plan is a decision record, not an
//! execution schedule: walking the graph remains control-plane authority.

use serde::Deserialize;
use serde::Serialize;

use crate::AdapterId;
use crate::BindingClass;
use crate::BindingDiagnostic;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ReadinessState;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::EvidenceReference;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceTypeId;

/// The binding selected by resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedBinding {
    /// The selected binding's identity.
    pub binding: CapabilityBindingId,
    /// The adapter providing the binding.
    pub adapter: AdapterId,
    /// The environment class work is routed to.
    pub environment: ExecutionEnvironment,
    /// The binding's class in the fallback chain.
    pub class: BindingClass,
}

/// Why a preferred candidate was not selected.
///
/// Fallback is only permitted when the semantic requirement and policy
/// remain satisfied by the alternate binding; each variant names the
/// specific reason the preferred candidate was skipped, so the decision
/// is diagnosable and auditable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum FallbackReason {
    /// The preferred binding exists but is not selectable right now.
    PreferredNotReady {
        /// The readiness state the preferred binding is in.
        state: ReadinessState,
        /// The preferred binding's diagnostic, when it is failed or
        /// unavailable.
        diagnostic: Option<BindingDiagnostic>,
    },
    /// The preferred binding's environment is outside the policy scope.
    PreferredOutsideEnvironmentScope,
    /// The preferred binding is human fallback, which policy forbids.
    PreferredHumanFallbackForbidden,
    /// The preferred binding's required resources are not bound.
    PreferredMissingResources {
        /// The resource types with no bound resource.
        missing: Vec<ResourceTypeId>,
    },
}

/// The record of a preferred candidate that was skipped in favor of the
/// selected binding.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FallbackRecord {
    /// The preferred candidate that was not selected.
    pub skipped: SelectedBinding,
    /// Why it was skipped.
    pub reason: FallbackReason,
}

/// The routing decision for one capability requirement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingDecision {
    /// The requirement that was resolved.
    pub requirement: CapabilityRequirement,
    /// The binding selected for the requirement.
    pub selected: SelectedBinding,
    /// The preferred candidate that was skipped, when the decision is a
    /// fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FallbackRecord>,
}

impl BindingDecision {
    /// Converts the decision into an evidence reference for the evidence
    /// plane.
    ///
    /// The digest covers the canonical serialization of the whole
    /// decision — the requirement, the selected binding, and the fallback
    /// record with its reason — so the routing decision is auditable and
    /// tamper-evident. Control-plane code must record this evidence for
    /// every fallback selection.
    pub fn to_evidence_reference(
        &self,
        locator: impl Into<String>,
    ) -> Result<EvidenceReference, ExecutionContractError> {
        let digest = ContentDigest::of(self)?;
        Ok(EvidenceReference {
            kind: EvidenceKind::Recovery,
            locator: crate::validate_locator(locator)?,
            digest,
        })
    }
}

/// The binding decisions for one workflow step.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepBinding {
    /// The IR node the decisions belong to.
    pub node: IrNodeId,
    /// One decision per capability requirement declared by the step, in
    /// declaration order.
    pub decisions: Vec<BindingDecision>,
}

/// The binding plan for one workflow IR.
///
/// A plan contains one [`StepBinding`] per step node of the graph, in node
/// id order. Steps with no capability requirements appear with empty
/// decision lists so the plan is explicit about every executable node.
///
/// Mixed-environment execution is a property of this record: nothing
/// constrains the selected bindings of different steps (or of different
/// capabilities within one step) to the same environment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingPlan {
    /// The step bindings, ordered by node id.
    pub steps: Vec<StepBinding>,
}

impl BindingPlan {
    /// The distinct environments the plan routes work to.
    ///
    /// A plan that routes to more than one environment is a
    /// mixed-environment execution.
    pub fn environments(&self) -> Vec<ExecutionEnvironment> {
        let mut environments: Vec<ExecutionEnvironment> = self
            .steps
            .iter()
            .flat_map(|step| step.decisions.iter())
            .map(|decision| decision.selected.environment)
            .collect();
        environments.sort();
        environments.dedup();
        environments
    }
}

#[cfg(test)]
#[path = "decision_tests.rs"]
mod tests;
