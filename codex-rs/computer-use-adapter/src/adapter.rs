//! The computer use workflow adapter facade.
//!
//! Binds workflow execution to the native Codex Computer Use capability:
//! lifecycle (`declared -> available -> ready -> authorized -> bound ->
//! executing`), explicit policy/authorization gates, session ownership with
//! takeover and recovery, lazy bridge provisioning with explicit readiness
//! diagnostics, evidence normalization, and recorded fallbacks to compatible
//! alternate desktop adapters. No second desktop-agent runtime and no second
//! workflow engine live here: actions are delegated to the native path
//! through `ComputerUseBridge`, and all records are plain data the execution
//! plane (WO-005) consumes over `codex-workflow-contracts` abstractions.

use std::sync::Arc;

use codex_workflow_contracts::EvidenceKind;
use codex_workflow_contracts::WorkflowInstanceStatus;
use serde::Deserialize;
use serde::Serialize;

use crate::action::ActionStatus;
use crate::action::NormalizedAction;
use crate::action::NormalizedActionResult;
use crate::bridge::BridgeCallFailure;
use crate::bridge::BridgeIdentity;
use crate::bridge::ComputerUseBridgeFactory;
use crate::capability::ADAPTER_IDENTITY;
use crate::capability::COMPUTER_USE_CAPABILITY_ID;
use crate::capability::CapabilityBinding;
use crate::capability::CapabilityLifecycle;
use crate::capability::CapabilityLifecycleState;
use crate::error::ComputerUseAdapterError;
use crate::evidence::AdapterEvidenceRecord;
use crate::evidence::EvidenceBuilder;
use crate::evidence::normalize_action_result;
use crate::evidence::normalize_authorization;
use crate::evidence::normalize_binding;
use crate::evidence::normalize_diagnostic;
use crate::evidence::normalize_observation;
use crate::evidence::normalize_recovery;
use crate::observation::ScreenObservation;
use crate::policy::AccessDecision;
use crate::policy::ApplicationIdentity;
use crate::policy::AuthorizationRecord;
use crate::policy::AuthorizationSource;
use crate::policy::ComputerUsePolicy;
use crate::policy::HostApproval;
use crate::session::ComputerUseSessionId;
use crate::session::OwnerToken;
use crate::session::RecoveryOutcome;
use crate::session::RecoveryRecord;
use crate::session::RecoveryStrategy;
use crate::session::RecoveryTrigger;
use crate::session::SessionOwnership;
use crate::session::TakeoverMode;
use crate::session::TakeoverReceipt;
use crate::session::UnixMsClock;
use crate::session::system_clock;
use crate::util::sha256_hex;

/// Which universal capability kind this adapter binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityKind {
    /// Desktop automation through the native Codex computer use path.
    ComputerUse,
}

/// Per-step recovery policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryPolicy {
    /// Stop at the first failed action.
    FailFast,
    /// Retry a failed action once before giving up on the step.
    RetryOnceThenFail,
    /// Execute every action, recording all failures.
    BestEffort,
}

/// One declared step plan.
///
/// `step_ref` is an opaque caller-supplied workflow node reference; the
/// adapter keeps it opaque and never interprets workflow semantics. Mixed
/// computer-use/browser-use graphs are sequenced by the universal execution
/// plane; this adapter only ever sees computer use steps.
#[derive(Clone, Debug)]
pub struct StepPlan {
    /// Opaque caller-supplied step/node reference.
    pub step_ref: String,
    /// Application targets the step will touch; each must be authorized.
    pub targets: Vec<ApplicationIdentity>,
    /// Actions to execute in order.
    pub actions: Vec<NormalizedAction>,
    /// Capture pre/post screen observations as evidence.
    pub capture_observations: bool,
    /// Recovery policy for the step.
    pub recovery: RecoveryPolicy,
}

/// Status of a settled step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum StepExecutionStatus {
    /// Every action succeeded.
    Completed,
    /// The step completed after at least one successful retry.
    CompletedWithRecovery,
    /// The step failed; see recorded results and recovery records.
    #[serde(rename_all = "camelCase")]
    Failed {
        /// Why the step failed.
        reason: String,
    },
}

impl StepExecutionStatus {
    /// Contract-level instance status implied by this step status; the
    /// execution plane (WO-005) applies it when updating a
    /// `codex_workflow_contracts::WorkflowInstance`.
    pub fn contract_status(&self) -> WorkflowInstanceStatus {
        match self {
            Self::Completed | Self::CompletedWithRecovery => WorkflowInstanceStatus::Succeeded,
            Self::Failed { .. } => WorkflowInstanceStatus::Failed,
        }
    }
}

/// Full record of one executed step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepExecutionRecord {
    /// Caller step reference.
    pub step_ref: String,
    /// Session that executed the step.
    pub session_id: ComputerUseSessionId,
    /// Status after settling.
    pub status: StepExecutionStatus,
    /// Pre-step observation when captured.
    pub pre_observation: Option<ScreenObservation>,
    /// Post-step observation when captured.
    pub post_observation: Option<ScreenObservation>,
    /// Per-action results in execution order.
    pub results: Vec<NormalizedActionResult>,
    /// Recovery records produced while settling the step.
    pub recoveries: Vec<RecoveryRecord>,
    /// Evidence records emitted for the step (append-only).
    pub evidence: Vec<AdapterEvidenceRecord>,
    /// Started at (adapter clock).
    pub started_at_unix_ms: u64,
    /// Settled at (adapter clock).
    pub finished_at_unix_ms: u64,
}

impl StepExecutionRecord {
    /// Contract-level instance status implied by this step's status.
    pub fn contract_status(&self) -> WorkflowInstanceStatus {
        self.status.contract_status()
    }
}

/// A compatible alternate desktop adapter a host may fall back to when the
/// native computer use path cannot reach `Ready`.
///
/// The adapter only records the fallback; execution of the alternate remains
/// host-owned, so no second runtime is created here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlternateDesktopAdapter {
    /// Alternate adapter identity.
    pub adapter_id: String,
    /// Bridge/runtime kind the alternate drives.
    pub bridge_kind: String,
    /// The alternate honors the same application policy tables.
    pub honors_app_policy: bool,
    /// The alternate honors Codex host approvals.
    pub honors_host_approvals: bool,
}

/// Why a fallback was recorded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum FallbackReason {
    /// Native readiness failed; carries the diagnostic.
    #[serde(rename_all = "camelCase")]
    NativeUnavailable {
        /// Readiness diagnostic.
        diagnostic: String,
    },
    /// Host directed the fallback.
    #[serde(rename_all = "camelCase")]
    HostDirected {
        /// Host-provided reason.
        reason: String,
    },
}

/// Recorded fallback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FallbackRecord {
    /// Alternate adapter.
    pub alternate: AlternateDesktopAdapter,
    /// Why the fallback applies.
    pub reason: FallbackReason,
    /// Digest of the policy the alternate must honor.
    pub policy_digest: String,
    /// Capability the fallback covers.
    pub capability_id: String,
    /// Unix milliseconds of the record.
    pub recorded_at_unix_ms: u64,
}

/// Workflow adapter binding workflow execution to the native Codex Computer
/// Use capability.
///
/// - Construction declares the capability (`Declared`); no bridge is touched.
/// - The bridge is provisioned lazily at `ensure_ready`; missing
///   `node_repl`/bridge/runtime provisioning is an explicit
///   [`ComputerUseAdapterError::Readiness`] with a remediation diagnostic.
/// - Authorization requires policy-allowed identities or host approvals;
///   only the opaque approval id is recorded (Codex approvals remain the
///   authority).
/// - Bindings record the exact capability/binding identity; evidence records
///   are capability-scoped so browser use steps (WO-006) can interleave in
///   the same workflow run without ambiguity and without a second engine.
pub struct ComputerUseWorkflowAdapter {
    policy: ComputerUsePolicy,
    policy_digest: String,
    factory: Arc<dyn ComputerUseBridgeFactory>,
    clock: UnixMsClock,
    lifecycle: CapabilityLifecycle,
    bridge: Option<Box<dyn crate::bridge::ComputerUseBridge>>,
    session: Option<SessionOwnership>,
    binding: Option<CapabilityBinding>,
    authorizations: Vec<AuthorizationRecord>,
    evidence: EvidenceBuilder,
    records: Vec<AdapterEvidenceRecord>,
    fallbacks: Vec<FallbackRecord>,
    last_readiness_diagnostic: Option<String>,
}

impl ComputerUseWorkflowAdapter {
    /// Creates the adapter in the `Declared` state. No bridge is provisioned
    /// here (lazy init).
    pub fn new(policy: ComputerUsePolicy, factory: Arc<dyn ComputerUseBridgeFactory>) -> Self {
        Self::with_clock(policy, factory, system_clock())
    }

    /// Creates the adapter with an injected clock (deterministic tests).
    pub fn with_clock(
        policy: ComputerUsePolicy,
        factory: Arc<dyn ComputerUseBridgeFactory>,
        clock: UnixMsClock,
    ) -> Self {
        let policy_digest = policy.policy_digest();
        Self {
            lifecycle: CapabilityLifecycle::new(),
            policy,
            policy_digest,
            factory,
            clock,
            bridge: None,
            session: None,
            binding: None,
            authorizations: Vec::new(),
            evidence: EvidenceBuilder::new(COMPUTER_USE_CAPABILITY_ID),
            records: Vec::new(),
            fallbacks: Vec::new(),
            last_readiness_diagnostic: None,
        }
    }

    /// Canonical capability id bound by this adapter.
    pub fn capability_id(&self) -> &'static str {
        COMPUTER_USE_CAPABILITY_ID
    }

    /// Capability kind (computer use; browser use is owned by WO-006).
    pub fn capability_kind(&self) -> CapabilityKind {
        CapabilityKind::ComputerUse
    }

    /// The policy in force.
    pub fn policy(&self) -> &ComputerUsePolicy {
        &self.policy
    }

    /// Current lifecycle state.
    pub fn lifecycle_state(&self) -> CapabilityLifecycleState {
        self.lifecycle.state()
    }

    /// Lifecycle transition audit trail.
    pub fn lifecycle_history(&self) -> &[crate::capability::LifecycleTransition] {
        self.lifecycle.history()
    }

    /// Evidence records emitted so far (append-only).
    pub fn evidence_records(&self) -> &[AdapterEvidenceRecord] {
        &self.records
    }

    /// Fallback records emitted so far.
    pub fn fallback_records(&self) -> &[FallbackRecord] {
        &self.fallbacks
    }

    /// Authorization records emitted so far.
    pub fn authorizations(&self) -> &[AuthorizationRecord] {
        &self.authorizations
    }

    /// Binding identity, present between bind and release/loss.
    pub fn binding(&self) -> Option<&CapabilityBinding> {
        self.binding.as_ref()
    }

    /// Last readiness diagnostic, when a readiness attempt failed.
    pub fn last_readiness_diagnostic(&self) -> Option<&str> {
        self.last_readiness_diagnostic.as_deref()
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn push_evidence(
        &mut self,
        kind: EvidenceKind,
        digest: String,
        session: Option<&ComputerUseSessionId>,
        recorded_at_unix_ms: u64,
    ) -> AdapterEvidenceRecord {
        let record = self
            .evidence
            .record(kind, digest, session, recorded_at_unix_ms);
        self.records.push(record.clone());
        record
    }

    /// Marks the native computer use surface as available.
    ///
    /// Native availability comes from the host (the Codex build ships the
    /// computer use feature; `codex-config` computer use settings are the
    /// policy authority). This never provisions the bridge (lazy init).
    pub fn mark_available(
        &mut self,
    ) -> Result<crate::capability::LifecycleTransition, ComputerUseAdapterError> {
        self.lifecycle.transition(
            CapabilityLifecycleState::Available,
            self.now(),
            Some("native Codex computer use surface present".to_string()),
        )
    }

    /// Ensures the bridge is provisioned and ready (lazy init boundary).
    ///
    /// On success moves `Available -> Ready` (or stays `Ready` after a
    /// bridge-loss regression re-provisions). On failure the lifecycle stays
    /// put, the diagnostic is recorded as trace evidence, and
    /// [`ComputerUseAdapterError::Readiness`] is returned — readiness
    /// problems are never silent.
    pub fn ensure_ready(&mut self) -> Result<BridgeIdentity, ComputerUseAdapterError> {
        if let Some(bridge) = &self.bridge {
            return Ok(bridge.identity());
        }
        let state = self.lifecycle_state();
        if !matches!(
            state,
            CapabilityLifecycleState::Available | CapabilityLifecycleState::Ready
        ) {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Available,
                actual: state,
            });
        }
        match self.factory.provision() {
            Ok(bridge) => match bridge.probe() {
                Ok(()) => {
                    let identity = bridge.identity();
                    self.bridge = Some(bridge);
                    if state == CapabilityLifecycleState::Available {
                        self.lifecycle.transition(
                            CapabilityLifecycleState::Ready,
                            self.now(),
                            Some(format!(
                                "bridge ready: {} on {}",
                                identity.bridge_kind, identity.runtime
                            )),
                        )?;
                    }
                    let detail = format!(
                        "{}|{}|{}",
                        identity.bridge_kind, identity.runtime, identity.version
                    );
                    let (kind, digest) = normalize_diagnostic("bridge-ready", &detail, self.now());
                    self.push_evidence(kind, digest, None, self.now());
                    Ok(identity)
                }
                Err(failure) => {
                    let diagnostic = failure.diagnostic();
                    self.last_readiness_diagnostic = Some(diagnostic.clone());
                    let (kind, digest) =
                        normalize_diagnostic("readiness-probe-failed", &diagnostic, self.now());
                    self.push_evidence(kind, digest, None, self.now());
                    Err(ComputerUseAdapterError::Readiness { diagnostic })
                }
            },
            Err(failure) => {
                let diagnostic = failure.diagnostic();
                self.last_readiness_diagnostic = Some(diagnostic.clone());
                let (kind, digest) =
                    normalize_diagnostic("readiness-provision-failed", &diagnostic, self.now());
                self.push_evidence(kind, digest, None, self.now());
                Err(ComputerUseAdapterError::Readiness { diagnostic })
            }
        }
    }

    /// Authorizes one application target for the capability scope.
    ///
    /// Policy-allowed applications pass without approval; policy-denied
    /// applications are rejected outright; unspecified applications require a
    /// host approval (Codex approvals remain the authority — only the opaque
    /// approval id is recorded, never credential material).
    pub fn authorize(
        &mut self,
        application: &ApplicationIdentity,
        approval: Option<HostApproval>,
    ) -> Result<AuthorizationRecord, ComputerUseAdapterError> {
        let state = self.lifecycle_state();
        if !matches!(
            state,
            CapabilityLifecycleState::Ready
                | CapabilityLifecycleState::Authorized
                | CapabilityLifecycleState::Bound
        ) {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Ready,
                actual: state,
            });
        }
        let subject = application.display_name.clone();
        let record = match self.policy.decide(application) {
            AccessDecision::Allowed => AuthorizationRecord {
                subject_digest: application.digest(),
                subject_display: subject,
                source: AuthorizationSource::PolicyAllow,
                recorded_at_unix_ms: self.now(),
            },
            AccessDecision::Denied => {
                return Err(ComputerUseAdapterError::PolicyDenied { subject });
            }
            AccessDecision::Unspecified => {
                let approval =
                    approval.ok_or(ComputerUseAdapterError::AuthorizationRequired { subject })?;
                AuthorizationRecord {
                    subject_digest: application.digest(),
                    subject_display: application.display_name.clone(),
                    source: AuthorizationSource::HostApproval(approval),
                    recorded_at_unix_ms: self.now(),
                }
            }
        };
        if state == CapabilityLifecycleState::Ready {
            self.lifecycle.transition(
                CapabilityLifecycleState::Authorized,
                self.now(),
                Some(format!("authorized {}", record.subject_display)),
            )?;
        }
        let (kind, digest) = normalize_authorization(&record);
        let session = self
            .session
            .as_ref()
            .map(|ownership| ownership.session_id.clone());
        self.push_evidence(kind, digest, session.as_ref(), record.recorded_at_unix_ms);
        self.authorizations.push(record.clone());
        Ok(record)
    }

    fn is_authorized(&self, target: &ApplicationIdentity) -> bool {
        let digest = target.digest();
        self.authorizations
            .iter()
            .any(|record| record.subject_digest == digest)
    }

    /// Binds a session to `owner`, recording the exact binding identity.
    pub fn bind(
        &mut self,
        owner: OwnerToken,
    ) -> Result<CapabilityBinding, ComputerUseAdapterError> {
        let state = self.lifecycle_state();
        if state != CapabilityLifecycleState::Authorized {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Authorized,
                actual: state,
            });
        }
        let bridge_identity = match &self.bridge {
            Some(bridge) => bridge.identity(),
            None => {
                return Err(ComputerUseAdapterError::Readiness {
                    diagnostic: "bridge is not provisioned; call ensure_ready before bind"
                        .to_string(),
                });
            }
        };
        let bound_at_unix_ms = self.now();
        let binding = CapabilityBinding {
            capability_id: COMPUTER_USE_CAPABILITY_ID.to_string(),
            adapter_identity: ADAPTER_IDENTITY.to_string(),
            bridge: bridge_identity,
            policy_digest: self.policy_digest.clone(),
            session_id: ComputerUseSessionId::generate(),
            owner: owner.clone(),
            bound_at_unix_ms,
        };
        self.lifecycle.transition(
            CapabilityLifecycleState::Bound,
            bound_at_unix_ms,
            Some(format!("bound session {}", binding.session_id)),
        )?;
        self.session = Some(SessionOwnership {
            session_id: binding.session_id.clone(),
            owner,
            acquired_at_unix_ms: bound_at_unix_ms,
        });
        let (kind, digest) = normalize_binding(&binding);
        let session = binding.session_id.clone();
        self.push_evidence(kind, digest, Some(&session), bound_at_unix_ms);
        self.binding = Some(binding.clone());
        Ok(binding)
    }

    fn execute_action_once(
        &mut self,
        action: &NormalizedAction,
        session_id: &ComputerUseSessionId,
    ) -> Result<NormalizedActionResult, BridgeCallFailure> {
        let call = match self.bridge.as_ref() {
            Some(bridge) => bridge.execute(action.clone()),
            None => Err(BridgeCallFailure::BridgeUnavailable {
                details: "bridge handle is not held; readiness regressed".to_string(),
            }),
        };
        match call {
            Ok(mut result) => {
                result.completed_at_unix_ms = self.now();
                let (kind, digest) = normalize_action_result(&result);
                let session = session_id.clone();
                self.push_evidence(kind, digest, Some(&session), result.completed_at_unix_ms);
                Ok(result)
            }
            Err(failure) => Err(failure),
        }
    }

    fn handle_bridge_loss(
        &mut self,
        failure: BridgeCallFailure,
        session_id: &ComputerUseSessionId,
        recoveries: &mut Vec<RecoveryRecord>,
        step_failed: &mut Option<String>,
    ) {
        let details = failure.diagnostic();
        *step_failed = Some(format!("bridge lost: {details}"));
        self.bridge = None;
        let record = RecoveryRecord {
            session_id: session_id.clone(),
            trigger: RecoveryTrigger::BridgeLost,
            strategy: RecoveryStrategy::ReleaseAndRebind,
            outcome: RecoveryOutcome::Escalated,
            at_unix_ms: self.now(),
        };
        let (kind, digest) = normalize_recovery(&record);
        let session = session_id.clone();
        self.push_evidence(kind, digest, Some(&session), record.at_unix_ms);
        recoveries.push(record);
    }

    /// Executes one desktop-automation step on the bound session.
    ///
    /// Policy boundary: every declared target must already be authorized, and
    /// the lifecycle must be `Bound` with a matching owner; rejected steps
    /// never enter the `Executing` state.
    pub fn execute_step(
        &mut self,
        owner: &OwnerToken,
        plan: StepPlan,
    ) -> Result<StepExecutionRecord, ComputerUseAdapterError> {
        if self.lifecycle_state() != CapabilityLifecycleState::Bound {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Bound,
                actual: self.lifecycle_state(),
            });
        }
        let ownership = match &self.session {
            Some(ownership) => ownership.clone(),
            None => {
                return Err(ComputerUseAdapterError::WrongLifecycleState {
                    expected: CapabilityLifecycleState::Bound,
                    actual: self.lifecycle_state(),
                });
            }
        };
        if &ownership.owner != owner {
            return Err(ComputerUseAdapterError::OwnerMismatch {
                session: ownership.session_id.to_string(),
            });
        }
        for target in &plan.targets {
            if !self.is_authorized(target) {
                return Err(ComputerUseAdapterError::TargetNotAuthorized {
                    subject: target.display_name.clone(),
                });
            }
        }

        let evidence_start = self.records.len();
        let started_at_unix_ms = self.now();
        self.lifecycle.transition(
            CapabilityLifecycleState::Executing,
            started_at_unix_ms,
            Some(format!("step {} executing", plan.step_ref)),
        )?;

        let mut pre_observation = None;
        if plan.capture_observations {
            pre_observation = self
                .bridge
                .as_ref()
                .and_then(|bridge| bridge.observe().ok());
            if let Some(observation) = pre_observation.as_mut() {
                observation.captured_at_unix_ms = started_at_unix_ms;
                let (kind, digest) = normalize_observation(observation);
                let session = ownership.session_id.clone();
                self.push_evidence(kind, digest, Some(&session), started_at_unix_ms);
            }
        }

        let mut results: Vec<NormalizedActionResult> = Vec::new();
        let mut recoveries: Vec<RecoveryRecord> = Vec::new();
        let mut permanent_failures: Vec<String> = Vec::new();
        let mut recovered = false;
        let mut step_failed: Option<String> = None;

        let mut index = 0;
        while index < plan.actions.len() {
            let action = &plan.actions[index];
            match self.execute_action_once(action, &ownership.session_id) {
                Ok(result) => {
                    let succeeded = matches!(result.status, ActionStatus::Succeeded);
                    if succeeded {
                        if matches!(action, NormalizedAction::Screenshot) {
                            let fingerprint = result.action_fingerprint.clone();
                            let artifact_digest =
                                sha256_hex(format!("screenshot|{fingerprint}").as_bytes());
                            let session = ownership.session_id.clone();
                            let at = self.now();
                            self.push_evidence(
                                EvidenceKind::Artifact,
                                artifact_digest,
                                Some(&session),
                                at,
                            );
                        }
                        results.push(result);
                        index += 1;
                        continue;
                    }
                    let reason = status_reason(&result.status);
                    match plan.recovery {
                        RecoveryPolicy::FailFast => {
                            step_failed =
                                Some(format!("action {} failed: {reason}", action.summary()));
                            results.push(result);
                            break;
                        }
                        RecoveryPolicy::BestEffort => {
                            permanent_failures.push(reason);
                            results.push(result);
                            index += 1;
                        }
                        RecoveryPolicy::RetryOnceThenFail => {
                            results.push(result);
                            match self.execute_action_once(action, &ownership.session_id) {
                                Ok(retry_result) => {
                                    let retry_succeeded =
                                        matches!(retry_result.status, ActionStatus::Succeeded);
                                    results.push(retry_result);
                                    if retry_succeeded {
                                        recovered = true;
                                        index += 1;
                                    } else {
                                        step_failed = Some(format!(
                                            "action {} failed after one retry",
                                            action.summary()
                                        ));
                                        break;
                                    }
                                }
                                Err(failure) => {
                                    self.handle_bridge_loss(
                                        failure,
                                        &ownership.session_id,
                                        &mut recoveries,
                                        &mut step_failed,
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(failure) => {
                    self.handle_bridge_loss(
                        failure,
                        &ownership.session_id,
                        &mut recoveries,
                        &mut step_failed,
                    );
                    break;
                }
            }
        }

        let mut post_observation = None;
        if plan.capture_observations {
            post_observation = self
                .bridge
                .as_ref()
                .and_then(|bridge| bridge.observe().ok());
            if let Some(observation) = post_observation.as_mut() {
                let stamp = self.now();
                observation.captured_at_unix_ms = stamp;
                let (kind, digest) = normalize_observation(observation);
                let session = ownership.session_id.clone();
                self.push_evidence(kind, digest, Some(&session), stamp);
            }
        }

        let finished_at_unix_ms = self.now();
        if step_failed.is_none() && !permanent_failures.is_empty() {
            step_failed = Some(format!(
                "best-effort step finished with {} failed action(s)",
                permanent_failures.len()
            ));
        }
        let status = match step_failed {
            Some(reason) => StepExecutionStatus::Failed { reason },
            None if recovered => StepExecutionStatus::CompletedWithRecovery,
            None => StepExecutionStatus::Completed,
        };
        self.lifecycle.transition(
            CapabilityLifecycleState::Bound,
            finished_at_unix_ms,
            Some(format!("step {} settled as {status:?}", plan.step_ref)),
        )?;
        if self.bridge.is_none() {
            self.lifecycle.transition(
                CapabilityLifecycleState::Ready,
                self.now(),
                Some(
                    "bridge lost; session released and readiness regressed for lazy re-provisioning"
                        .to_string(),
                ),
            )?;
            if let Some(ownership) = self.session.take() {
                let (kind, digest) = normalize_diagnostic(
                    "session-release",
                    ownership.session_id.as_str(),
                    self.now(),
                );
                self.push_evidence(kind, digest, None, self.now());
            }
            self.binding = None;
        }

        let evidence = self.records[evidence_start..].to_vec();
        Ok(StepExecutionRecord {
            step_ref: plan.step_ref.clone(),
            session_id: ownership.session_id,
            status,
            pre_observation,
            post_observation,
            results,
            recoveries,
            evidence,
            started_at_unix_ms,
            finished_at_unix_ms,
        })
    }

    /// Captures a normalized observation as evidence without executing a
    /// step. Requires the `Bound` state and the owning owner token.
    pub fn observe(
        &mut self,
        owner: &OwnerToken,
    ) -> Result<ScreenObservation, ComputerUseAdapterError> {
        if self.lifecycle_state() != CapabilityLifecycleState::Bound {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Bound,
                actual: self.lifecycle_state(),
            });
        }
        let ownership = match &self.session {
            Some(ownership) => ownership.clone(),
            None => {
                return Err(ComputerUseAdapterError::WrongLifecycleState {
                    expected: CapabilityLifecycleState::Bound,
                    actual: self.lifecycle_state(),
                });
            }
        };
        if &ownership.owner != owner {
            return Err(ComputerUseAdapterError::OwnerMismatch {
                session: ownership.session_id.to_string(),
            });
        }
        let mut observation = match self.bridge.as_ref() {
            Some(bridge) => bridge
                .observe()
                .map_err(|failure| ComputerUseAdapterError::BridgeCall(failure.diagnostic()))?,
            None => {
                return Err(ComputerUseAdapterError::Readiness {
                    diagnostic: "bridge is not provisioned; call ensure_ready".to_string(),
                });
            }
        };
        observation.captured_at_unix_ms = self.now();
        let (kind, digest) = normalize_observation(&observation);
        let at = observation.captured_at_unix_ms;
        self.push_evidence(kind, digest, Some(&ownership.session_id), at);
        Ok(observation)
    }

    /// Takes the session over per the requested mode.
    ///
    /// Same-owner re-claims are idempotent receipts. Cross-owner graceful
    /// takeover is allowed while the session is idle (`Bound`; steps execute
    /// synchronously). Cross-owner forced takeover additionally requires a
    /// recorded reason and a policy that allows takeover, and produces a
    /// recovery record.
    pub fn takeover_session(
        &mut self,
        claimant: OwnerToken,
        mode: TakeoverMode,
        reason: Option<String>,
    ) -> Result<TakeoverReceipt, ComputerUseAdapterError> {
        if self.lifecycle_state() != CapabilityLifecycleState::Bound {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Bound,
                actual: self.lifecycle_state(),
            });
        }
        let ownership = match &self.session {
            Some(ownership) => ownership.clone(),
            None => {
                return Err(ComputerUseAdapterError::WrongLifecycleState {
                    expected: CapabilityLifecycleState::Bound,
                    actual: self.lifecycle_state(),
                });
            }
        };
        if ownership.owner == claimant {
            return Ok(TakeoverReceipt {
                session_id: ownership.session_id,
                previous_owner: claimant.clone(),
                new_owner: claimant,
                mode,
                reason,
                at_unix_ms: self.now(),
            });
        }
        let trigger = match mode {
            TakeoverMode::Graceful => None,
            TakeoverMode::Forced => {
                let reason_text = match reason.clone() {
                    Some(reason_text) => reason_text,
                    None => return Err(ComputerUseAdapterError::TakeoverReasonRequired),
                };
                if !self.policy.takeover_allowed(true) {
                    return Err(ComputerUseAdapterError::TakeoverPolicyDenied);
                }
                Some(RecoveryTrigger::ForcedTakeover {
                    reason: reason_text,
                })
            }
        };
        let at_unix_ms = self.now();
        let receipt = TakeoverReceipt {
            session_id: ownership.session_id.clone(),
            previous_owner: ownership.owner.clone(),
            new_owner: claimant.clone(),
            mode,
            reason,
            at_unix_ms,
        };
        self.session = Some(SessionOwnership {
            session_id: ownership.session_id,
            owner: claimant,
            acquired_at_unix_ms: at_unix_ms,
        });
        let detail = format!(
            "{}|{}|{}",
            receipt.previous_owner.as_str(),
            receipt.new_owner.as_str(),
            receipt.session_id.as_str()
        );
        let (kind, digest) = normalize_diagnostic("session-takeover", &detail, at_unix_ms);
        let session = receipt.session_id.clone();
        self.push_evidence(kind, digest, Some(&session), at_unix_ms);
        if let Some(trigger) = trigger {
            let record = RecoveryRecord {
                session_id: receipt.session_id.clone(),
                trigger,
                strategy: RecoveryStrategy::Handover,
                outcome: RecoveryOutcome::Recovered,
                at_unix_ms,
            };
            let (kind, digest) = normalize_recovery(&record);
            self.push_evidence(kind, digest, Some(&receipt.session_id), at_unix_ms);
        }
        Ok(receipt)
    }

    /// Releases the bound session back to the `Authorized` state.
    pub fn release(
        &mut self,
        owner: &OwnerToken,
    ) -> Result<crate::capability::LifecycleTransition, ComputerUseAdapterError> {
        if self.lifecycle_state() != CapabilityLifecycleState::Bound {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Bound,
                actual: self.lifecycle_state(),
            });
        }
        let ownership = match &self.session {
            Some(ownership) => ownership.clone(),
            None => {
                return Err(ComputerUseAdapterError::WrongLifecycleState {
                    expected: CapabilityLifecycleState::Bound,
                    actual: self.lifecycle_state(),
                });
            }
        };
        if &ownership.owner != owner {
            return Err(ComputerUseAdapterError::OwnerMismatch {
                session: ownership.session_id.to_string(),
            });
        }
        let transition = self.lifecycle.transition(
            CapabilityLifecycleState::Authorized,
            self.now(),
            Some(format!("released session {}", ownership.session_id)),
        )?;
        let (kind, digest) = normalize_diagnostic(
            "session-release",
            ownership.session_id.as_str(),
            transition.at_unix_ms,
        );
        self.push_evidence(kind, digest, None, transition.at_unix_ms);
        self.session = None;
        self.binding = None;
        Ok(transition)
    }

    /// Records a fallback to a compatible alternate desktop adapter.
    ///
    /// Only alternates that honor the same application policy tables and
    /// Codex host approvals satisfy the semantic/policy requirements; the
    /// record is otherwise rejected. The adapter never executes the
    /// alternate: recording is the entire contract, so no second runtime is
    /// created here.
    pub fn record_fallback(
        &mut self,
        alternate: AlternateDesktopAdapter,
        reason: FallbackReason,
    ) -> Result<FallbackRecord, ComputerUseAdapterError> {
        if !alternate.honors_app_policy || !alternate.honors_host_approvals {
            return Err(ComputerUseAdapterError::FallbackRejected(format!(
                "alternate '{}' must honor the same application policy tables and Codex host approvals",
                alternate.adapter_id
            )));
        }
        let state = self.lifecycle_state();
        if !matches!(
            state,
            CapabilityLifecycleState::Declared
                | CapabilityLifecycleState::Available
                | CapabilityLifecycleState::Ready
        ) {
            return Err(ComputerUseAdapterError::WrongLifecycleState {
                expected: CapabilityLifecycleState::Available,
                actual: state,
            });
        }
        let record = FallbackRecord {
            alternate,
            reason,
            policy_digest: self.policy_digest.clone(),
            capability_id: COMPUTER_USE_CAPABILITY_ID.to_string(),
            recorded_at_unix_ms: self.now(),
        };
        let (kind, digest) = normalize_diagnostic(
            "desktop-adapter-fallback",
            &format!("{}|{}", record.alternate.adapter_id, record.policy_digest),
            record.recorded_at_unix_ms,
        );
        self.push_evidence(kind, digest, None, record.recorded_at_unix_ms);
        self.fallbacks.push(record.clone());
        Ok(record)
    }
}

fn status_reason(status: &ActionStatus) -> String {
    match status {
        ActionStatus::Succeeded => "succeeded".to_string(),
        ActionStatus::Failed { reason } => format!("failed ({reason:?})"),
        ActionStatus::Rejected { reason } => format!("rejected ({reason})"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use codex_workflow_contracts::EvidenceKind;
    use codex_workflow_contracts::WorkflowInstanceStatus;

    use super::AlternateDesktopAdapter;
    use super::CapabilityKind;
    use super::ComputerUseWorkflowAdapter;
    use super::FallbackReason;
    use super::RecoveryPolicy;
    use super::StepExecutionStatus;
    use super::StepPlan;
    use crate::action::NormalizedAction;
    use crate::bridge::BridgeReadinessFailure;
    use crate::capability::COMPUTER_USE_CAPABILITY_ID;
    use crate::capability::CapabilityLifecycleState;
    use crate::error::ComputerUseAdapterError;
    use crate::policy::AccessRequirement;
    use crate::policy::ApplicationIdentity;
    use crate::policy::ApplicationPlatformIdentity;
    use crate::policy::AuthorizationSource;
    use crate::policy::ComputerUsePolicy;
    use crate::policy::HostApproval;
    use crate::session::OwnerToken;
    use crate::session::RecoveryStrategy;
    use crate::session::TakeoverMode;
    use crate::test_support::ScriptedFactory;
    use crate::test_support::StubBridgeMode;
    use crate::test_support::UnprovisionedFactory;
    use crate::test_support::ticking_clock;

    fn macos_app(bundle_id: &str) -> ApplicationIdentity {
        ApplicationIdentity {
            display_name: format!("app:{bundle_id}"),
            platform: ApplicationPlatformIdentity::MacosBundle {
                bundle_id: bundle_id.to_string(),
            },
        }
    }

    fn allow_all_policy() -> ComputerUsePolicy {
        ComputerUsePolicy {
            default_app_access: Some(AccessRequirement::Allow),
            ..ComputerUsePolicy::default()
        }
    }

    fn simple_plan(step_ref: &str, app: &ApplicationIdentity) -> StepPlan {
        StepPlan {
            step_ref: step_ref.to_string(),
            targets: vec![app.clone()],
            actions: vec![
                NormalizedAction::LaunchApplication {
                    application: app.clone(),
                },
                NormalizedAction::Screenshot,
                NormalizedAction::Wait { millis: 10 },
            ],
            capture_observations: true,
            recovery: RecoveryPolicy::FailFast,
        }
    }

    #[test]
    fn full_lifecycle_bind_execute_records_binding_and_evidence() {
        let policy = ComputerUsePolicy {
            macos_bundle_ids: [("com.example.editor".to_string(), AccessRequirement::Allow)]
                .into_iter()
                .collect(),
            ..ComputerUsePolicy::default()
        };
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter = ComputerUseWorkflowAdapter::with_clock(
            policy.clone(),
            factory.clone(),
            ticking_clock(1_000),
        );
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Declared
        );
        assert_eq!(adapter.capability_id(), COMPUTER_USE_CAPABILITY_ID);
        assert_eq!(adapter.capability_kind(), CapabilityKind::ComputerUse);
        assert_eq!(
            factory.provision_count(),
            0,
            "lazy init: no provisioning before readiness"
        );

        adapter.mark_available().expect("available");
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Available
        );
        assert_eq!(
            factory.provision_count(),
            0,
            "lazy init: availability does not provision"
        );

        let identity = adapter.ensure_ready().expect("ready");
        assert_eq!(identity.bridge_kind, "stub-computer-use");
        assert_eq!(identity.runtime, "node_repl");
        assert_eq!(factory.provision_count(), 1);
        assert_eq!(adapter.lifecycle_state(), CapabilityLifecycleState::Ready);

        let app = macos_app("com.example.editor");
        let authorization = adapter.authorize(&app, None).expect("authorized by policy");
        assert_eq!(authorization.source, AuthorizationSource::PolicyAllow);
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Authorized
        );

        let owner = OwnerToken::new("instance-1/node-7");
        let binding = adapter.bind(owner.clone()).expect("bound");
        assert_eq!(binding.capability_id, COMPUTER_USE_CAPABILITY_ID);
        assert!(
            binding
                .adapter_identity
                .starts_with("codex-computer-use-adapter@")
        );
        assert_eq!(binding.bridge.runtime, "node_repl");
        assert_eq!(binding.policy_digest, policy.policy_digest());
        assert_eq!(binding.owner, owner);
        assert_eq!(adapter.lifecycle_state(), CapabilityLifecycleState::Bound);

        let record = adapter
            .execute_step(&owner, simple_plan("node-7", &app))
            .expect("step executed");
        assert_eq!(record.status, StepExecutionStatus::Completed);
        assert_eq!(record.contract_status(), WorkflowInstanceStatus::Succeeded);
        assert_eq!(record.results.len(), 3);
        assert!(record.pre_observation.is_some());
        assert!(record.post_observation.is_some());
        assert_eq!(adapter.lifecycle_state(), CapabilityLifecycleState::Bound);
        assert!(adapter.binding().is_some());

        let solo = adapter.observe(&owner).expect("standalone observation");
        assert_eq!(solo.screen.name.as_deref(), Some("stub-screen"));

        let kinds: Vec<String> = adapter
            .evidence_records()
            .iter()
            .map(|entry| serde_json::to_string(&entry.kind).expect("kind serde"))
            .collect();
        for expected in [
            "\"approval\"",
            "\"trace\"",
            "\"observation\"",
            "\"artifact\"",
        ] {
            assert!(
                kinds.iter().any(|kind| kind == expected),
                "missing {expected} in {kinds:?}"
            );
        }
        for evidence in adapter.evidence_records() {
            assert_eq!(evidence.capability_id, COMPUTER_USE_CAPABILITY_ID);
            assert!(evidence.locator.starts_with("computer-use/"));
        }
    }

    #[test]
    fn ensure_ready_is_lazy_and_idempotent() {
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter = ComputerUseWorkflowAdapter::with_clock(
            allow_all_policy(),
            factory.clone(),
            ticking_clock(0),
        );
        adapter.mark_available().expect("available");
        let first = adapter.ensure_ready().expect("ready");
        let second = adapter.ensure_ready().expect("ready again");
        assert_eq!(first, second);
        assert_eq!(factory.provision_count(), 1);
    }

    #[test]
    fn readiness_failure_is_explicit_and_fallback_is_recorded() {
        let factory = Arc::new(UnprovisionedFactory::new(
            BridgeReadinessFailure::NotProvisioned {
                expected_bridge_kind: "codex-native-computer-use".to_string(),
            },
        ));
        let mut adapter =
            ComputerUseWorkflowAdapter::with_clock(allow_all_policy(), factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        let error = adapter
            .ensure_ready()
            .expect_err("readiness must fail explicitly");
        match error {
            ComputerUseAdapterError::Readiness { diagnostic } => {
                assert!(
                    diagnostic.contains("not provisioned"),
                    "diagnostic: {diagnostic}"
                );
                assert!(
                    diagnostic.contains("node_repl"),
                    "diagnostic must name the runtime: {diagnostic}"
                );
            }
            other => panic!("expected readiness error, got {other:?}"),
        }
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Available
        );
        assert!(adapter.last_readiness_diagnostic().is_some());
        assert!(adapter.binding().is_none());

        let alternate = AlternateDesktopAdapter {
            adapter_id: "vendor.alt-desktop".to_string(),
            bridge_kind: "vendor-bridge".to_string(),
            honors_app_policy: true,
            honors_host_approvals: true,
        };
        let fallback = adapter
            .record_fallback(
                alternate,
                FallbackReason::NativeUnavailable {
                    diagnostic: adapter
                        .last_readiness_diagnostic()
                        .unwrap_or_default()
                        .to_string(),
                },
            )
            .expect("fallback recorded");
        assert_eq!(fallback.capability_id, COMPUTER_USE_CAPABILITY_ID);
        assert!(!fallback.policy_digest.is_empty());
        assert_eq!(adapter.fallback_records().len(), 1);

        let incompatible = AlternateDesktopAdapter {
            adapter_id: "vendor.unsafe-desktop".to_string(),
            bridge_kind: "unsafe-bridge".to_string(),
            honors_app_policy: false,
            honors_host_approvals: true,
        };
        let error = adapter
            .record_fallback(
                incompatible,
                FallbackReason::HostDirected {
                    reason: "host prefers".to_string(),
                },
            )
            .expect_err("incompatible alternates must be rejected");
        assert!(matches!(
            error,
            ComputerUseAdapterError::FallbackRejected(_)
        ));
        assert_eq!(adapter.fallback_records().len(), 1);
    }

    #[test]
    fn policy_gates_authorization_and_targets() {
        let policy = ComputerUsePolicy {
            macos_bundle_ids: [("com.example.blocked".to_string(), AccessRequirement::Deny)]
                .into_iter()
                .collect(),
            ..ComputerUsePolicy::default()
        };
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter = ComputerUseWorkflowAdapter::with_clock(policy, factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");

        let blocked = macos_app("com.example.blocked");
        let error = adapter
            .authorize(&blocked, None)
            .expect_err("denied by policy");
        assert!(matches!(
            error,
            ComputerUseAdapterError::PolicyDenied { .. }
        ));

        let unspecified = macos_app("com.example.unknown");
        let error = adapter
            .authorize(&unspecified, None)
            .expect_err("unspecified requires approval");
        assert!(matches!(
            error,
            ComputerUseAdapterError::AuthorizationRequired { .. }
        ));

        let record = adapter
            .authorize(
                &unspecified,
                Some(HostApproval {
                    approval_id: "approval-1".to_string(),
                }),
            )
            .expect("approved by host");
        match record.source {
            AuthorizationSource::HostApproval(approval) => {
                assert_eq!(approval.approval_id, "approval-1")
            }
            other => panic!("expected host approval source, got {other:?}"),
        }
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Authorized
        );

        let owner = OwnerToken::new("owner-a");
        adapter.bind(owner.clone()).expect("bound");
        let unauthorized = macos_app("com.example.never-authorized");
        let error = adapter
            .execute_step(&owner, simple_plan("node-9", &unauthorized))
            .expect_err("unauthorized target must block execution");
        assert!(matches!(
            error,
            ComputerUseAdapterError::TargetNotAuthorized { .. }
        ));
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Bound,
            "no lifecycle leak on rejected step"
        );
    }

    #[test]
    fn bridge_loss_records_recovery_releases_session_and_rebinds() {
        let factory = Arc::new(ScriptedFactory::scripted(vec![StubBridgeMode::AlwaysLoses]));
        let mut adapter = ComputerUseWorkflowAdapter::with_clock(
            allow_all_policy(),
            factory.clone(),
            ticking_clock(0),
        );
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");
        let app = macos_app("com.example.editor");
        adapter.authorize(&app, None).expect("authorized");
        let owner = OwnerToken::new("owner-a");
        adapter.bind(owner.clone()).expect("bound");
        assert_eq!(factory.provision_count(), 1);

        let record = adapter
            .execute_step(&owner, simple_plan("node-3", &app))
            .expect("step settled");
        match record.status {
            StepExecutionStatus::Failed { reason } => {
                assert!(reason.contains("bridge lost"), "reason: {reason}")
            }
            other => panic!("expected failed step, got {other:?}"),
        }
        assert_eq!(record.recoveries.len(), 1);
        assert_eq!(
            record.recoveries[0].strategy,
            RecoveryStrategy::ReleaseAndRebind
        );
        assert_eq!(
            adapter.lifecycle_state(),
            CapabilityLifecycleState::Ready,
            "readiness regressed after bridge loss"
        );
        assert!(
            adapter.binding().is_none(),
            "session released after bridge loss"
        );

        adapter.ensure_ready().expect("re-provisioned");
        assert_eq!(factory.provision_count(), 2);
        adapter
            .authorize(&app, None)
            .expect("re-authorized (recorded)");
        adapter.bind(owner).expect("re-bound");
        assert_eq!(adapter.lifecycle_state(), CapabilityLifecycleState::Bound);
    }

    #[test]
    fn retry_once_recovery_completes_the_step() {
        let factory = Arc::new(ScriptedFactory::scripted(vec![
            StubBridgeMode::FailsFirstAction,
        ]));
        let mut adapter =
            ComputerUseWorkflowAdapter::with_clock(allow_all_policy(), factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");
        let app = macos_app("com.example.editor");
        adapter.authorize(&app, None).expect("authorized");
        let owner = OwnerToken::new("owner-a");
        adapter.bind(owner.clone()).expect("bound");

        let mut plan = simple_plan("node-5", &app);
        plan.actions.truncate(1); // single action: fail + retry = 2 results
        plan.recovery = RecoveryPolicy::RetryOnceThenFail;
        plan.capture_observations = false;
        let record = adapter.execute_step(&owner, plan).expect("step executed");
        assert_eq!(record.status, StepExecutionStatus::CompletedWithRecovery);
        assert_eq!(record.contract_status(), WorkflowInstanceStatus::Succeeded);
        assert_eq!(record.results.len(), 2, "failed attempt + successful retry");
        assert!(record.recoveries.is_empty());
        assert_eq!(adapter.lifecycle_state(), CapabilityLifecycleState::Bound);
    }

    #[test]
    fn takeover_rules_are_explicit_and_recorded() {
        let policy = ComputerUsePolicy {
            allow_session_takeover: Some(true),
            ..allow_all_policy()
        };
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter = ComputerUseWorkflowAdapter::with_clock(policy, factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");
        let app = macos_app("com.example.editor");
        adapter.authorize(&app, None).expect("authorized");
        let owner_a = OwnerToken::new("owner-a");
        let owner_b = OwnerToken::new("owner-b");
        adapter.bind(owner_a.clone()).expect("bound");

        let error = adapter
            .takeover_session(owner_b.clone(), TakeoverMode::Forced, None)
            .expect_err("forced takeover requires a recorded reason");
        assert!(matches!(
            error,
            ComputerUseAdapterError::TakeoverReasonRequired
        ));

        let receipt = adapter
            .takeover_session(owner_b.clone(), TakeoverMode::Graceful, None)
            .expect("graceful takeover while idle");
        assert_eq!(receipt.previous_owner, owner_a);
        assert_eq!(receipt.new_owner, owner_b);

        let error = adapter
            .execute_step(&owner_a, simple_plan("node-1", &app))
            .expect_err("previous owner must be rejected");
        assert!(matches!(
            error,
            ComputerUseAdapterError::OwnerMismatch { .. }
        ));

        adapter
            .execute_step(&owner_b, simple_plan("node-2", &app))
            .expect("new owner executes");

        let receipt = adapter
            .takeover_session(
                owner_a.clone(),
                TakeoverMode::Forced,
                Some("operator intervention".to_string()),
            )
            .expect("forced takeover allowed by policy");
        assert_eq!(receipt.mode, TakeoverMode::Forced);
        let has_forced_recovery = adapter
            .evidence_records()
            .iter()
            .any(|record| record.kind == EvidenceKind::Recovery);
        assert!(has_forced_recovery);
    }

    #[test]
    fn forced_takeover_requires_policy_permission() {
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter =
            ComputerUseWorkflowAdapter::with_clock(allow_all_policy(), factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");
        let app = macos_app("com.example.editor");
        adapter.authorize(&app, None).expect("authorized");
        let owner_a = OwnerToken::new("owner-a");
        let owner_b = OwnerToken::new("owner-b");
        adapter.bind(owner_a).expect("bound");
        let error = adapter
            .takeover_session(owner_b, TakeoverMode::Forced, Some("because".to_string()))
            .expect_err("policy denies forced takeover by default");
        assert!(matches!(
            error,
            ComputerUseAdapterError::TakeoverPolicyDenied
        ));
    }

    #[test]
    fn mixed_composition_evidence_is_capability_scoped_and_browser_free() {
        let factory = Arc::new(ScriptedFactory::default());
        let mut adapter =
            ComputerUseWorkflowAdapter::with_clock(allow_all_policy(), factory, ticking_clock(0));
        adapter.mark_available().expect("available");
        adapter.ensure_ready().expect("ready");
        let app = macos_app("com.example.editor");
        adapter.authorize(&app, None).expect("authorized");
        let owner = OwnerToken::new("owner-a");
        adapter.bind(owner.clone()).expect("bound");
        adapter
            .execute_step(&owner, simple_plan("node-1", &app))
            .expect("step executed");

        assert!(!adapter.evidence_records().is_empty());
        for evidence in adapter.evidence_records() {
            let serialized = serde_json::to_string(evidence).expect("evidence serde");
            assert!(
                !serialized.contains("origin"),
                "browser origin semantics leaked: {serialized}"
            );
            assert!(
                !serialized.contains("\"url\""),
                "browser url semantics leaked: {serialized}"
            );
            assert_eq!(evidence.capability_id, "codex.computer-use");
        }
    }
}
