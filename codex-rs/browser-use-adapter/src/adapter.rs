//! The workflow-facing adapter binding execution to Codex Browser Use.
//!
//! [`BrowserUseAdapter`] is the single entry point. It is deliberately
//! lazy and side-effect free at construction: readiness is resolved only
//! when [`BrowserUseAdapter::prepare`] runs, so unrelated workflows and
//! chat startup are never blocked. The adapter drives the capability
//! lifecycle (`declared -> available -> ready -> authorized -> bound ->
//! executing` plus interruption/recovery/take-over), prefers the native
//! Codex Browser Use path, permits a compatible fallback only when the
//! workflow's semantic/policy requirements remain satisfied, and always
//! records a selected fallback as evidence. It never launches browsers,
//! never mutates workflow semantics, and never bypasses Codex approvals
//! or sandboxing.

use crate::authorization::{AuthorizationOutcome, SessionGovernance, check_authorization};
use crate::binding::{
    BROWSER_USE_CAPABILITY_ID, BindingIdentity, BrowserAdapterKind, BrowserSessionIdentity,
    FallbackAdapter, FallbackRecord, evaluate_fallback, fallback_incompatible_diagnostic,
};
use crate::bridge::EvidenceRecord;
use crate::config_snapshot::BrowserUseConfigSnapshot;
use crate::diagnostics::BridgeProbe;
use crate::error::AdapterError;
use crate::lifecycle::{CapabilityLifecycle, CapabilityLifecycleState, InterruptionKind};
use crate::requirements::BrowserUseRequirement;
use crate::session::BrowserExecutionSession;
use codex_workflow_contracts::EvidenceKind;
use serde::{Deserialize, Serialize};

/// The result of a successful [`BrowserUseAdapter::prepare`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareReport {
    /// Lifecycle state after preparation (`Authorized` on success).
    pub state: CapabilityLifecycleState,
    /// `true` when the native Codex Browser Use path reported healthy.
    pub native_available: bool,
    /// The recorded fallback selection, when a fallback was selected.
    pub fallback: Option<FallbackRecord>,
    /// Session-level governance items derived from the requirement; the
    /// native runtime (or a declared-compatible fallback) enforces these.
    pub session_governed: Vec<SessionGovernance>,
}

/// Actionable, credential-free remediation for one interruption kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRemediation {
    /// The interruption being remediated.
    pub interruption: InterruptionKind,
    /// The lifecycle state the matching `recover_to_*` call targets.
    pub recovery_state: CapabilityLifecycleState,
    /// Ordered remediation steps.
    pub steps: Vec<String>,
}

impl RecoveryRemediation {
    /// Builds the remediation for an interruption kind.
    pub fn for_interruption(kind: InterruptionKind) -> Self {
        match kind {
            InterruptionKind::RuntimeLost | InterruptionKind::BridgeMissing => Self {
                interruption: kind,
                recovery_state: CapabilityLifecycleState::Available,
                steps: vec![
                    "re-probe the native Browser Use bridge/runtime (BridgeProbe)".to_string(),
                    "re-run BrowserUseAdapter::prepare to restore readiness and authorization"
                        .to_string(),
                    "re-bind with BrowserUseAdapter::bind and resume execution".to_string(),
"or supply a compatible fallback adapter; a selected fallback is always  recorded as evidence"
                        .to_string(),
                ],
            },
            InterruptionKind::AuthorizationExpired => Self {
                interruption: kind,
                recovery_state: CapabilityLifecycleState::Ready,
                steps: vec![
                    "re-run the authorization pre-flight via BrowserUseAdapter::prepare"
                        .to_string(),
                    "re-bind if the browser session also changed, then resume execution"
                        .to_string(),
                ],
            },
            InterruptionKind::BindingLost => Self {
                interruption: kind,
                recovery_state: CapabilityLifecycleState::Authorized,
                steps: vec![
                    "re-acquire or replace the browser session (profile/tabs) out of band"
                        .to_string(),
"re-bind with BrowserUseAdapter::bind; use take_over when adopting an  existing session"
                        .to_string(),
                    "resume execution with BrowserUseAdapter::begin_execution".to_string(),
                ],
            },
        }
    }
}

/// Binds workflow execution to the Codex Browser Use capability.
pub struct BrowserUseAdapter {
    requirement: BrowserUseRequirement,
    probe: BridgeProbe,
    config: BrowserUseConfigSnapshot,
    fallback: Option<FallbackAdapter>,
    fallback_recorded: bool,
    lifecycle: CapabilityLifecycle,
    authorization: Option<AuthorizationOutcome>,
    binding: Option<BindingIdentity>,
    records: Vec<EvidenceRecord>,
}

impl BrowserUseAdapter {
    /// Creates the adapter without any side effect: nothing is probed,
    /// resolved, or initialized until [`Self::prepare`] is called (lazy
    /// initialization).
    pub fn new(
        requirement: BrowserUseRequirement,
        probe: BridgeProbe,
        config: BrowserUseConfigSnapshot,
    ) -> Self {
        Self {
            requirement,
            probe,
            config,
            fallback: None,
            fallback_recorded: false,
            lifecycle: CapabilityLifecycle::declared(),
            authorization: None,
            binding: None,
            records: Vec::new(),
        }
    }

    /// Registers a candidate fallback browser adapter. It is evaluated
    /// only if the native path reports a gap, and only a compatible
    /// fallback is ever selected — and then always recorded.
    pub fn with_fallback(mut self, fallback: FallbackAdapter) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Current lifecycle state.
    pub fn state(&self) -> CapabilityLifecycleState {
        self.lifecycle.state()
    }

    /// The recorded lifecycle transition history.
    pub fn lifecycle_history(&self) -> &[crate::lifecycle::LifecycleTransition] {
        self.lifecycle.history()
    }

    /// The current binding, once bound.
    pub fn binding(&self) -> Option<&BindingIdentity> {
        self.binding.as_ref()
    }

    /// The last authorization outcome, once prepared.
    pub fn authorization(&self) -> Option<&AuthorizationOutcome> {
        self.authorization.as_ref()
    }

    /// Adapter-level evidence records (currently: fallback selection).
    pub fn adapter_records(&self) -> &[EvidenceRecord] {
        &self.records
    }

    /// Exports the lifecycle history as evidence records.
    pub fn lifecycle_evidence(&self, scope: &str) -> Vec<EvidenceRecord> {
        self.lifecycle.evidence_records(scope)
    }

    fn record_fallback(&mut self, fallback: &FallbackAdapter, note: &str) {
        if self.fallback_recorded {
            return;
        }
        let payload = FallbackRecord {
            adapter_name: fallback.name.clone(),
            note: note.to_string(),
        };
        let locator = format!("codex-browser-use/fallback/{}", fallback.name);
        self.records.push(EvidenceRecord::from_payload(
            EvidenceKind::Recovery,
            locator,
            &payload,
        ));
        self.fallback_recorded = true;
    }

    /// Resolves readiness: probes the native bridge/runtime, selects a
    /// compatible fallback if (and only if) the native path has a gap,
    /// runs the requirement-vs-policy authorization pre-flight, and
    /// advances the lifecycle to `Authorized`.
    ///
    /// Callable from `Declared`, `Available` (after recovery), or `Ready`
    /// (after authorization expiry recovery). On failure the lifecycle
    /// never advances past what already succeeded, and the error carries
    /// actionable diagnostics.
    pub fn prepare(&mut self) -> Result<PrepareReport, AdapterError> {
        match self.lifecycle.state() {
            CapabilityLifecycleState::Declared
            | CapabilityLifecycleState::Available
            | CapabilityLifecycleState::Ready => {}
            _ => return Err(AdapterError::NotPrepared),
        }
        let native_available = self.probe.gap().is_none();
        let mut fallback_record = None;
        if let Some((code, mut diagnostics)) = self.probe.gap() {
            let fallback = match &self.fallback {
                Some(fallback) => fallback.clone(),
                None => {
                    return Err(AdapterError::Unavailable { code, diagnostics });
                }
            };
            let evaluation = evaluate_fallback(
                self.requirement.fallback_needs(),
                &fallback.declared_capabilities,
            );
            if !evaluation.compatible {
                diagnostics.push(fallback_incompatible_diagnostic(&evaluation.unmet));
                return Err(AdapterError::Unavailable { code, diagnostics });
            }
            let note = format!("selected fallback because native path reported {code:?}");
            if self.lifecycle.state() == CapabilityLifecycleState::Declared {
                self.lifecycle.mark_available()?;
            }
            self.record_fallback(&fallback, &note);
            fallback_record = Some(FallbackRecord {
                adapter_name: fallback.name.clone(),
                note,
            });
        } else if self.lifecycle.state() == CapabilityLifecycleState::Declared {
            self.lifecycle.mark_available()?;
        }
        let outcome = check_authorization(&self.requirement, &self.config);
        if !outcome.authorized {
            return Err(AdapterError::Unauthorized {
                count: outcome.violations.len(),
                diagnostics: outcome.diagnostics(),
            });
        }
        let session_governed = outcome.session_governed.clone();
        if self.lifecycle.state() == CapabilityLifecycleState::Available {
            self.lifecycle.mark_ready()?;
        }
        self.lifecycle.authorize()?;
        self.authorization = Some(outcome);
        Ok(PrepareReport {
            state: self.lifecycle.state(),
            native_available,
            fallback: fallback_record,
            session_governed,
        })
    }

    /// `Authorized -> Bound` against a concrete browser session identity.
    /// Prefers the native adapter; uses the recorded fallback only when the
    /// native path has a gap (that selection was recorded during prepare).
    pub fn bind(
        &mut self,
        session: BrowserSessionIdentity,
        origin_scope: Vec<String>,
    ) -> Result<BindingIdentity, AdapterError> {
        if self.lifecycle.state() != CapabilityLifecycleState::Authorized {
            return Err(AdapterError::NotPrepared);
        }
        let adapter = if self.probe.gap().is_none() {
            BrowserAdapterKind::Native
        } else {
            let fallback = self.fallback.clone().ok_or(AdapterError::NotPrepared)?;
            BrowserAdapterKind::Fallback(fallback)
        };
        let binding = BindingIdentity {
            capability_id: BROWSER_USE_CAPABILITY_ID.to_string(),
            adapter,
            session,
            origin_scope,
        };
        self.lifecycle.bind()?;
        self.binding = Some(binding.clone());
        Ok(binding)
    }

    /// `Bound -> Executing`; returns the live execution session that
    /// normalizes browser artifacts into evidence.
    pub fn begin_execution(&mut self) -> Result<BrowserExecutionSession, AdapterError> {
        if self.lifecycle.state() != CapabilityLifecycleState::Bound {
            return Err(AdapterError::NotBound);
        }
        let binding = self.binding.clone().ok_or(AdapterError::NotBound)?;
        self.lifecycle.begin_execution()?;
        Ok(BrowserExecutionSession::new(binding))
    }

    /// Records an interruption on the lifecycle and the live session and
    /// returns actionable remediation. The caller performs remediation out
    /// of band (never by mutating workflow semantics), then drives
    /// `recover_to_*` -> `prepare`/`bind` -> `begin_execution`.
    pub fn interrupt(
        &mut self,
        session: &mut BrowserExecutionSession,
        kind: InterruptionKind,
    ) -> Result<RecoveryRemediation, AdapterError> {
        self.lifecycle.interrupt(kind)?;
        session.record_interruption(kind);
        Ok(RecoveryRemediation::for_interruption(kind))
    }

    /// `Interrupted -> Available` (requires `RuntimeLost`/`BridgeMissing`).
    pub fn recover_to_available(&mut self) -> Result<(), AdapterError> {
        self.lifecycle.recover_to_available()
    }

    /// `Interrupted -> Ready` (requires `AuthorizationExpired`).
    pub fn recover_to_ready(&mut self) -> Result<(), AdapterError> {
        self.lifecycle.recover_to_ready()
    }

    /// `Interrupted -> Authorized` (requires `BindingLost`).
    pub fn recover_to_authorized(&mut self) -> Result<(), AdapterError> {
        self.lifecycle.recover_to_authorized()
    }

    /// Takes over `next_session` while executing: swaps the binding
    /// (recording `takeover_of` lineage), records the take-over as recovery
    /// evidence on the live session, and returns to `Executing`. Workflow
    /// semantics are untouched.
    pub fn take_over(
        &mut self,
        session: &mut BrowserExecutionSession,
        next_session: BrowserSessionIdentity,
    ) -> Result<(), AdapterError> {
        if self.lifecycle.state() != CapabilityLifecycleState::Executing {
            return Err(AdapterError::NotBound);
        }
        let previous = self.binding.clone().ok_or(AdapterError::NotBound)?;
        self.lifecycle.take_over()?;
        self.lifecycle.begin_execution()?;
        let mut next = next_session;
        next.takeover_of = Some(previous.session.session_id.clone());
        let next_binding = BindingIdentity {
            capability_id: BROWSER_USE_CAPABILITY_ID.to_string(),
            adapter: previous.adapter.clone(),
            session: next,
            origin_scope: previous.origin_scope.clone(),
        };
        // The displaced binding's session loss is itself a recovery event:
        // recording it makes takeover evidence self-contained even without
        // the previous owner's execution log.
        session.record_interruption(InterruptionKind::BindingLost);
        session.rebind(&previous, next_binding.clone(), true);
        self.binding = Some(next_binding);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_snapshot::OriginPolicySnapshot;
    use crate::diagnostics::DiagnosticCode;
    use crate::requirements::{AllowDeny, OriginPolicyRequirement};
    use crate::session::{
        BrowserAction, BrowserActionKind, BrowserActionOutcome, BrowserActionResult,
        BrowserObservation, SessionEnd,
    };
    use codex_workflow_contracts::WorkflowInstanceStatus;
    use std::collections::BTreeMap;

    fn healthy_probe() -> BridgeProbe {
        BridgeProbe {
            native_bridge_present: true,
            native_runtime_provisioned: true,
            detail: None,
        }
    }

    fn missing_bridge_probe() -> BridgeProbe {
        BridgeProbe {
            native_bridge_present: false,
            native_runtime_provisioned: false,
            detail: Some("no bridge registered".to_string()),
        }
    }

    fn requirement() -> BrowserUseRequirement {
        BrowserUseRequirement {
            origins: BTreeMap::from([(
                "https://example.com".to_string(),
                OriginPolicyRequirement {
                    access: Some(AllowDeny::Allow),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }
    }

    fn config() -> BrowserUseConfigSnapshot {
        BrowserUseConfigSnapshot {
            origins: BTreeMap::from([(
                "https://example.com".to_string(),
                OriginPolicySnapshot {
                    access: Some(AllowDeny::Allow),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }
    }

    fn strong_fallback() -> FallbackAdapter {
        FallbackAdapter {
            name: "org-fallback".to_string(),
            declared_capabilities: crate::binding::FallbackCapabilities {
                origin_scoping: true,
                history_governance: true,
                webmcp_governance: true,
                review_controls: true,
                persistent_approval_controls: true,
                evidence_capture: true,
                takeover: true,
                profiles: true,
                tabs: true,
            },
        }
    }

    fn session_identity(id: &str) -> BrowserSessionIdentity {
        BrowserSessionIdentity {
            profile: None,
            session_id: id.to_string(),
            tabs: vec!["tab-1".to_string()],
            takeover_of: None,
        }
    }

    #[test]
    fn construction_is_lazy_and_side_effect_free() {
        let adapter = BrowserUseAdapter::new(requirement(), missing_bridge_probe(), config());
        assert_eq!(adapter.state(), CapabilityLifecycleState::Declared);
        assert!(adapter.binding().is_none());
        assert!(adapter.adapter_records().is_empty());
        assert!(adapter.lifecycle_history().is_empty());
    }

    #[test]
    fn missing_bridge_without_fallback_is_explicit_and_actionable() {
        let mut adapter = BrowserUseAdapter::new(requirement(), missing_bridge_probe(), config());
        let error = adapter.prepare().expect_err("must fail explicitly");
        match error {
            AdapterError::Unavailable { code, diagnostics } => {
                assert_eq!(code, DiagnosticCode::NativeBridgeMissing);
                assert!(
                    diagnostics[0]
                        .message
                        .contains("refusing to start a second browser agent")
                );
                assert!(!diagnostics[0].remediation.is_empty());
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(adapter.state(), CapabilityLifecycleState::Declared);
    }

    #[test]
    fn happy_native_path_lifecycle_and_execution() {
        let mut adapter = BrowserUseAdapter::new(requirement(), healthy_probe(), config());
        let report = adapter.prepare().expect("prepare");
        assert!(report.native_available);
        assert!(report.fallback.is_none());
        assert!(report.session_governed.is_empty());
        assert_eq!(report.state, CapabilityLifecycleState::Authorized);
        let binding = adapter
            .bind(
                session_identity("native-1"),
                vec!["https://example.com".to_string()],
            )
            .expect("bind");
        assert_eq!(binding.capability_id, BROWSER_USE_CAPABILITY_ID);
        assert!(matches!(binding.adapter, BrowserAdapterKind::Native));
        assert_eq!(adapter.state(), CapabilityLifecycleState::Bound);
        let mut execution = adapter.begin_execution().expect("executing");
        assert_eq!(adapter.state(), CapabilityLifecycleState::Executing);
        execution.record_observation(&BrowserObservation {
            origin: "https://example.com".to_string(),
            url: Some("https://example.com/a".to_string()),
            title: None,
            tab_ids: vec!["tab-1".to_string()],
            redacted: true,
        });
        execution.record_action(
            &BrowserAction {
                kind: BrowserActionKind::Navigate,
                origin: Some("https://example.com".to_string()),
                target: Some("https://example.com/a".to_string()),
                detail: None,
            },
            &BrowserActionResult {
                outcome: BrowserActionOutcome::Succeeded,
                detail: None,
            },
        );
        let outcome = execution.finish(SessionEnd::Completed);
        assert_eq!(
            outcome.suggested_instance_status,
            WorkflowInstanceStatus::Succeeded
        );
        assert_eq!(outcome.records.len(), 2);
        assert_eq!(outcome.final_binding.session.session_id, "native-1");
        assert_eq!(outcome.recovery_count, 0);
    }

    #[test]
    fn unauthorized_requirement_blocks_readiness_without_advancing() {
        let mut adapter = BrowserUseAdapter::new(
            requirement(),
            healthy_probe(),
            BrowserUseConfigSnapshot::default(),
        );
        let error = adapter.prepare().expect_err("policy conflict");
        assert!(matches!(error, AdapterError::Unauthorized { count: 1, .. }));
        assert_eq!(adapter.state(), CapabilityLifecycleState::Available);
    }

    #[test]
    fn fallback_selection_is_recorded_once_with_evidence() {
        let mut adapter = BrowserUseAdapter::new(requirement(), missing_bridge_probe(), config())
            .with_fallback(strong_fallback());
        let report = adapter.prepare().expect("prepare via fallback");
        assert!(!report.native_available);
        let record = report.fallback.as_ref().expect("fallback recorded");
        assert_eq!(record.adapter_name, "org-fallback");
        assert!(record.note.contains("NativeBridgeMissing"));
        assert_eq!(adapter.adapter_records().len(), 1);
        assert_eq!(adapter.adapter_records()[0].kind, EvidenceKind::Recovery);
        assert!(adapter.adapter_records()[0].locator.contains("fallback"));
        let binding = adapter
            .bind(
                session_identity("fb-1"),
                vec!["https://example.com".to_string()],
            )
            .expect("bind");
        assert!(matches!(binding.adapter, BrowserAdapterKind::Fallback(_)));
    }

    #[test]
    fn incompatible_fallback_is_rejected_with_actionable_diagnostic() {
        let weak = FallbackAdapter {
            name: "weak".to_string(),
            declared_capabilities: crate::binding::FallbackCapabilities::default(),
        };
        let mut adapter = BrowserUseAdapter::new(requirement(), missing_bridge_probe(), config())
            .with_fallback(weak);
        let error = adapter.prepare().expect_err("incompatible fallback");
        match error {
            AdapterError::Unavailable { diagnostics, .. } => {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagnosticCode::FallbackIncompatible)
                );
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagnosticCode::NativeBridgeMissing)
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(adapter.state(), CapabilityLifecycleState::Declared);
        assert!(adapter.adapter_records().is_empty());
    }

    #[test]
    fn interruption_recovery_and_takeover_round_trip() {
        let mut adapter = BrowserUseAdapter::new(requirement(), healthy_probe(), config());
        adapter.prepare().expect("prepare");
        adapter
            .bind(
                session_identity("s1"),
                vec!["https://example.com".to_string()],
            )
            .expect("bind");
        let mut execution = adapter.begin_execution().expect("executing");
        execution.record_action(
            &BrowserAction {
                kind: BrowserActionKind::Click,
                origin: Some("https://example.com".to_string()),
                target: None,
                detail: None,
            },
            &BrowserActionResult {
                outcome: BrowserActionOutcome::Succeeded,
                detail: None,
            },
        );

        let remediation = adapter
            .interrupt(&mut execution, InterruptionKind::RuntimeLost)
            .expect("interrupt");
        assert_eq!(
            remediation.recovery_state,
            CapabilityLifecycleState::Available
        );
        assert!(!remediation.steps.is_empty());
        assert_eq!(adapter.state(), CapabilityLifecycleState::Interrupted);
        assert_eq!(
            execution
                .records()
                .last()
                .expect("interruption record")
                .kind,
            EvidenceKind::Recovery
        );
        adapter.recover_to_available().expect("recovered");
        adapter.prepare().expect("re-prepare");
        assert_eq!(adapter.state(), CapabilityLifecycleState::Authorized);
        adapter
            .bind(
                session_identity("s2"),
                vec!["https://example.com".to_string()],
            )
            .expect("re-bind");
        let mut execution = adapter.begin_execution().expect("executing again");
        assert!(execution.records().is_empty());

        adapter
            .take_over(&mut execution, session_identity("s3"))
            .expect("takeover");
        assert_eq!(adapter.state(), CapabilityLifecycleState::Executing);
        let binding = adapter.binding().expect("binding").clone();
        assert_eq!(binding.session.session_id, "s3");
        assert_eq!(binding.session.takeover_of.as_deref(), Some("s2"));
        assert_eq!(
            binding.origin_scope,
            vec!["https://example.com".to_string()]
        );
        let takeover_record = execution.records().last().expect("takeover record");
        assert_eq!(takeover_record.kind, EvidenceKind::Recovery);
        assert!(takeover_record.locator.contains("takeover"));
        assert!(
            adapter
                .lifecycle_history()
                .iter()
                .any(|transition| matches!(
                    transition.kind,
                    crate::lifecycle::TransitionKind::TakeOver
                ))
        );

        let outcome = execution.finish(SessionEnd::Failed);
        assert_eq!(
            outcome.suggested_instance_status,
            WorkflowInstanceStatus::Failed
        );
        assert_eq!(outcome.final_binding.session.session_id, "s3");
        assert!(outcome.recovery_count >= 2);
    }
}
