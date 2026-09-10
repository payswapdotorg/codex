//! Upstream-Codex compatibility snapshots (WO-013).
//!
//! The compatibility surface under evaluation is the ordinary-Codex
//! behavior contract: **with no workflow selected, nothing executes**.
//! [`UpstreamCompatSnapshot`] pins that contract observably — the lifecycle
//! reports no active state, instantiation and running refuse with
//! `NoActiveWorkflow`, no events are recorded, no evidence is stored, no
//! instances exist, the registry state is untouched, the environment
//! adapter executes nothing, and the model provider is never invoked.
//!
//! Snapshots are content-addressed
//! ([`UpstreamCompatSnapshot::digest`]); a compatibility test pins the
//! digest so any future change that breaks the no-workflow contract fails
//! loudly instead of silently regressing ordinary Codex behavior.

use std::sync::Arc;

use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowContractError;

use crate::env_adapter::ScriptedEnvTurn;
use crate::harness::WorkflowEvalHarness;
use crate::model_fixture::ScriptedModelProvider;

/// The revision of the compatibility fixture.
///
/// Bump when the pinned snapshot deliberately changes (for example when a
/// Work Order extends the observable contract); the pinned digest in the
/// compatibility tests changes with it.
pub const COMPAT_FIXTURE_VERSION: u32 = 1;

/// One captured ordinary-Codex compatibility snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamCompatSnapshot {
    /// The fixture revision the snapshot was captured with.
    pub fixture_version: u32,
    /// Whether the lifecycle held any workflow state before any selection.
    pub lifecycle_inactive: bool,
    /// The refusal instantiated without a selected version produced.
    pub instantiate_refusal: Option<String>,
    /// The refusal running without an active instantiation produced.
    pub run_refusal: Option<String>,
    /// Events recorded by the event sink.
    pub events_recorded: usize,
    /// Evidence payloads stored.
    pub evidence_payloads: usize,
    /// Instance records created.
    pub instances_created: usize,
    /// Versions stored.
    pub versions_stored: usize,
    /// Actions executed by the environment adapter.
    pub adapter_executions: usize,
    /// Model invocations served.
    pub model_requests: usize,
    /// Bindings registered in the capability registry (unchanged by the
    /// refused operations).
    pub registered_bindings: usize,
}

impl UpstreamCompatSnapshot {
    /// Captures the no-workflow behavior of a freshly built harness.
    ///
    /// The harness is constructed with a scripted provider and environment
    /// script — doubles that would record any execution — but no version is
    /// installed or selected, so the ordinary-Codex no-op path is the only
    /// possible behavior. Any recorded activity is a compatibility
    /// violation the projection exposes.
    pub async fn capture(
        provider: Arc<ScriptedModelProvider>,
        env_script: Vec<ScriptedEnvTurn>,
    ) -> Result<Self, WorkflowAppError> {
        let mut harness = WorkflowEvalHarness::new(provider, env_script)?;
        let lifecycle_inactive = !harness.is_active();
        let instantiate_refusal = match harness.instantiate_without_workflow().await {
            Ok(instance) => Some(format!(
                "instantiation unexpectedly succeeded as {:?}",
                instance.status
            )),
            Err(error) => Some(error.to_string()),
        };
        let run_refusal = match harness.run_without_workflow().await {
            Ok(outcome) => Some(format!(
                "run unexpectedly succeeded with terminal {:?}",
                outcome.terminal
            )),
            Err(error) => Some(error.to_string()),
        };
        Ok(Self {
            fixture_version: COMPAT_FIXTURE_VERSION,
            lifecycle_inactive,
            instantiate_refusal,
            run_refusal,
            events_recorded: harness.observed_events().len(),
            evidence_payloads: harness.evidence_count(),
            instances_created: harness.instance_count(),
            versions_stored: harness.version_count(),
            adapter_executions: harness.adapter_executions().len(),
            model_requests: harness.model_request_count(),
            registered_bindings: harness.registered_bindings(),
        })
    }

    /// The canonical projection of the snapshot.
    pub fn projection(&self) -> serde_json::Value {
        serde_json::json!({
            "fixtureVersion": self.fixture_version,
            "lifecycleInactive": self.lifecycle_inactive,
            "instantiateRefusal": &self.instantiate_refusal,
            "runRefusal": &self.run_refusal,
            "eventsRecorded": self.events_recorded,
            "evidencePayloads": self.evidence_payloads,
            "instancesCreated": self.instances_created,
            "versionsStored": self.versions_stored,
            "adapterExecutions": self.adapter_executions,
            "modelRequests": self.model_requests,
            "registeredBindings": self.registered_bindings,
        })
    }

    /// The content-addressed digest of the projection.
    ///
    /// Pinning this digest in tests makes any change to the no-workflow
    /// observable behavior a loud compatibility failure.
    pub fn digest(&self) -> Result<ContentDigest, WorkflowContractError> {
        ContentDigest::of(&self.projection())
    }
}
