//! The environment adapter boundary.
//!
//! [`EnvironmentAdapter`] is the single execution abstraction that routes
//! workflow step execution into concrete environments: Codex-native
//! capabilities (Browser Use, Computer Use, terminal exec, tools, MCP,
//! human surfaces) and compatible external bridges. Later Work Orders
//! (WO-006, WO-007) implement this trait over those runtimes; this crate
//! defines only the contract.
//!
//! ## Boundary rules
//!
//! - Adapters are **executors, never authorities**. They declare what they
//!   provide, report probe outcomes, prepare sessions, and execute one
//!   action at a time. Binding decisions, policy, and readiness authority
//!   stay with [`crate::CapabilityRegistry`] and the control plane.
//! - Adapters receive already-selected work: the registry chose the
//!   binding, policy was checked, resources are attached. An adapter
//!   cannot change workflow semantics, escalate its own permissions, or
//!   select work.
//! - Credentials stay inside adapters: resource bindings carry opaque
//!   instance identities, and adapters resolve them to secret material
//!   without ever surfacing it.
//! - All adapter outputs (probe reports, observations, results, failures)
//!   are untrusted input by default.
//!
//! ## Async convention
//!
//! The trait follows the repository's object-safe boxed-future convention
//! (the same shape as `codex-tools`' `ToolExecutor`): each method returns
//! a named `Pin<Box<dyn Future + Send>>` alias, so the registry can hold
//! `Arc<dyn EnvironmentAdapter>` and await it from any runtime. No
//! `#[async_trait]` is used.

use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;
use serde::Serialize;

use crate::Action;
use crate::ActionResult;
use crate::AdapterId;
use crate::BindingClass;
use crate::BindingDiagnostic;
use crate::CapabilityBindingId;
use crate::DiagnosticCode;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ExecutionFailure;
use crate::ResourceBinding;
use crate::SessionHandle;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ResourceTypeId;

/// Future returned by [`EnvironmentAdapter::probe`].
pub type AdapterProbeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProbeReport, ExecutionFailure>> + Send + 'a>>;

/// Future returned by [`EnvironmentAdapter::prepare`].
pub type AdapterPrepareFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PreparedSession, ExecutionFailure>> + Send + 'a>>;

/// Future returned by [`EnvironmentAdapter::execute`].
pub type AdapterExecuteFuture<'a> = Pin<Box<dyn Future<Output = ActionResult> + Send + 'a>>;

/// One capability an adapter provides, as declared in its descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvidedCapability {
    /// The semantic capability being provided.
    pub capability: CapabilityId,
    /// Resource types that must be bound before this capability executes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourceTypeId>,
}

/// What an adapter's probe found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus {
    /// The capability is installed and provisioned: the binding can be
    /// used.
    Ready,
    /// The capability is installed and discoverable but not yet
    /// provisioned (for example an MCP server is listed but no session is
    /// established).
    Available,
    /// The capability's implementation is not present.
    NotInstalled,
}

/// The outcome of one adapter probe.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeReport {
    /// What the probe found.
    pub status: ProbeStatus,
    /// Diagnostic detail, required when the capability is not installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<BindingDiagnostic>,
}

impl ProbeReport {
    /// A successful `READY` probe report.
    pub fn ready() -> Self {
        Self {
            status: ProbeStatus::Ready,
            diagnostic: None,
        }
    }

    /// Validates the report's consistency: a `NotInstalled` report must
    /// carry a diagnostic.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.status == ProbeStatus::NotInstalled && self.diagnostic.is_none() {
            return Err(ExecutionContractError::InvalidRecord {
                reason: "not-installed probe reports require a diagnostic".to_owned(),
            });
        }
        Ok(())
    }
}

/// The immutable registration descriptor of one adapter.
///
/// The descriptor is the adapter's *declaration* of what it provides;
/// the registry owns whether and how those declarations become bindings.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterDescriptor {
    /// The adapter's identity.
    pub adapter: AdapterId,
    /// The environment class the adapter serves.
    pub environment: ExecutionEnvironment,
    /// The adapter's position in the fallback chain.
    pub class: BindingClass,
    /// The capabilities the adapter provides.
    pub provides: Vec<ProvidedCapability>,
}

impl AdapterDescriptor {
    /// Validates the descriptor's structural rules.
    ///
    /// - the environment must be a peer (not reserved);
    /// - at least one capability is provided;
    /// - capabilities are unique;
    /// - per-capability resource lists are duplicate-free.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.environment.is_reserved() {
            return Err(ExecutionContractError::ReservedEnvironment {
                environment: self.environment,
                reason: "reserved environments cannot have adapters yet",
            });
        }
        if self.provides.is_empty() {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!("adapter `{}` provides no capabilities", self.adapter),
            });
        }
        let mut capabilities: Vec<&CapabilityId> =
            self.provides.iter().map(|item| &item.capability).collect();
        capabilities.sort();
        let distinct = capabilities.len();
        capabilities.dedup();
        if capabilities.len() != distinct {
            return Err(ExecutionContractError::InvalidRecord {
                reason: format!(
                    "adapter `{}` provides the same capability twice",
                    self.adapter
                ),
            });
        }
        for provided in &self.provides {
            let mut resources = provided.resources.clone();
            resources.sort();
            let distinct = resources.len();
            resources.dedup();
            if resources.len() != distinct {
                return Err(ExecutionContractError::InvalidRecord {
                    reason: format!(
                        "adapter `{}` lists duplicate resources for capability `{}`",
                        self.adapter,
                        provided.capability.as_ref()
                    ),
                });
            }
        }
        Ok(())
    }
}

/// The request to prepare a binding's execution session.
///
/// Resources carry the opaque instance identities bound by the registry;
/// the adapter resolves them to its own handles (and any credentials)
/// internally.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareRequest {
    /// The binding being prepared.
    pub binding: CapabilityBindingId,
    /// The capability being prepared.
    pub capability: CapabilityId,
    /// The resource bindings attached to the execution.
    pub resources: Vec<ResourceBinding>,
}

/// A prepared execution session for one binding.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedSession {
    /// The binding the session serves.
    pub binding: CapabilityBindingId,
    /// The opaque session handle the adapter allocated.
    pub session: SessionHandle,
}

/// The execution boundary implemented by every environment adapter.
///
/// Implementations are expected to:
///
/// - keep [`EnvironmentAdapter::descriptor`] cheap and synchronous: it is
///   the registration surface;
/// - treat [`EnvironmentAdapter::probe`] as a health check, not a
///   long-running operation: it drives `AVAILABLE`/`READY` readiness;
/// - establish adapter-internal session state in
///   [`EnvironmentAdapter::prepare`], resolving resource identities to
///   concrete handles without surfacing credentials;
/// - execute exactly one action per
///   [`EnvironmentAdapter::execute`] call and return a normalized
///   [`ActionResult`], never panicking across the boundary.
pub trait EnvironmentAdapter: Send + Sync {
    /// The adapter's registration declaration.
    fn descriptor(&self) -> &AdapterDescriptor;

    /// Probes whether the adapter's capabilities are installed and
    /// provisioned.
    fn probe(&self) -> AdapterProbeFuture<'_>;

    /// Prepares an execution session for one binding with its attached
    /// resources.
    fn prepare(&self, request: PrepareRequest) -> AdapterPrepareFuture<'_>;

    /// Executes one action in a prepared session.
    fn execute<'a>(
        &'a self,
        session: &'a SessionHandle,
        action: Action,
    ) -> AdapterExecuteFuture<'a>;
}

/// Builds a diagnostic for a preparation failure from a normalized
/// execution failure.
pub(crate) fn preparation_diagnostic(
    environment: ExecutionEnvironment,
    failure: &ExecutionFailure,
) -> BindingDiagnostic {
    BindingDiagnostic::new(DiagnosticCode::ExecutionFailure, failure.message.clone())
        .in_environment(environment)
}

/// Builds a diagnostic for a probe error from a normalized execution
/// failure.
pub(crate) fn probe_error_diagnostic(
    environment: ExecutionEnvironment,
    failure: &ExecutionFailure,
) -> BindingDiagnostic {
    BindingDiagnostic::new(DiagnosticCode::ProbeFailed, failure.message.clone())
        .in_environment(environment)
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
