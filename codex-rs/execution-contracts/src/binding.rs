//! Capability bindings.
//!
//! A [`CapabilityBinding`] is the registry record pairing one semantic
//! capability (for example `navigate_web`) with the concrete adapter that
//! provides it. Bindings are *runtime routing records*: they are created by
//! adapter registration, evaluated against policy during resolution, and
//! never enter workflow semantics. The same semantic capability can have
//! many bindings across different environments — that multiplicity is what
//! lets one workflow step bind to different compatible environments.

use serde::Deserialize;
use serde::Serialize;

use crate::AdapterId;
use crate::CapabilityBindingId;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ProvidedCapability;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ResourceTypeId;

/// The class of a capability binding, used for fallback ordering.
///
/// The frozen architecture's fallback chain for a semantic capability is:
///
/// ```text
/// preferred Codex-native binding
///     -> compatible alternate binding(s)
///     -> human
/// ```
///
/// `BindingClass` encodes position in that chain. Ordering is
/// `CodexNative < Compatible < HumanFallback`, and resolution only
/// considers a lower class after recording why every higher class was
/// skipped (see [`crate::FallbackReason`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BindingClass {
    /// The preferred Codex-native implementation (for example the Codex
    /// Browser Use or Computer Use runtime, a Codex tool, or a skill-backed
    /// MCP binding).
    CodexNative,
    /// A compatible alternate implementation (for example an external
    /// browser adapter or a semantically equivalent API connector).
    Compatible,
    /// Human execution as the last resort.
    HumanFallback,
}

impl BindingClass {
    /// Every binding class, in fallback order.
    pub const ALL: [Self; 3] = [Self::CodexNative, Self::Compatible, Self::HumanFallback];
}

/// One registered capability binding.
///
/// Binding records are derived from an adapter's
/// [`crate::AdapterDescriptor`] at registration time and are immutable
/// afterwards; readiness state is tracked separately by the registry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityBinding {
    /// The binding's deterministic identity.
    pub binding: CapabilityBindingId,
    /// The semantic capability being provided.
    pub capability: CapabilityId,
    /// The environment class the binding routes work to.
    pub environment: ExecutionEnvironment,
    /// The adapter providing the capability.
    pub adapter: AdapterId,
    /// The binding's position in the fallback chain.
    pub class: BindingClass,
    /// Resource types that must be bound before this binding executes.
    pub resources: Vec<ResourceTypeId>,
}

impl CapabilityBinding {
    /// Derives a binding record from an adapter descriptor's provided
    /// capability.
    pub fn from_parts(
        adapter: &AdapterId,
        environment: ExecutionEnvironment,
        class: BindingClass,
        provided: &ProvidedCapability,
    ) -> Self {
        let mut resources = provided.resources.clone();
        resources.sort();
        resources.dedup();
        Self {
            binding: CapabilityBindingId::derive(adapter, &provided.capability),
            capability: provided.capability.clone(),
            environment,
            adapter: adapter.clone(),
            class,
            resources,
        }
    }

    /// Validates the binding record's invariants.
    ///
    /// - the environment must not be reserved;
    /// - the required resource types must be unique.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.environment.is_reserved() {
            return Err(ExecutionContractError::ReservedEnvironment {
                environment: self.environment,
                reason: "reserved environments cannot carry bindings yet",
            });
        }
        let mut sorted = self.resources.clone();
        sorted.sort();
        let distinct = sorted.len();
        sorted.dedup();
        if sorted.len() != distinct {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!("binding `{}` lists duplicate resource types", self.binding),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
