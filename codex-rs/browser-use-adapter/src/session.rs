//! Browser execution sessions and artifact normalization.
//!
//! A session opens when the adapter moves `Bound -> Executing`. Everything
//! the native Browser Use capability observes or does enters the session as
//! a normalized, digest-bearing evidence record mapped onto
//! `codex_workflow_contracts::EvidenceKind`:
//!
//! - observations -> `EvidenceKind::Observation`
//! - actions/results -> `EvidenceKind::Trace`
//! - artifacts -> `EvidenceKind::Artifact`
//! - approvals -> `EvidenceKind::Approval`
//! - verifications -> `EvidenceKind::TestResult`
//! - interruptions, recovery, and take-overs -> `EvidenceKind::Recovery`
//!
//! Browser/UI state is evidence only: it never becomes workflow authority.
//! Payloads must be pre-redacted by the host; credentials never enter
//! records.

use crate::binding::BindingIdentity;
use crate::bridge::EvidenceRecord;
use crate::lifecycle::InterruptionKind;
use codex_workflow_contracts::{EvidenceKind, WorkflowInstanceStatus};
use serde::{Deserialize, Serialize};

/// A normalized browser observation (evidence only, never authority).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserObservation {
    /// Origin the observation was taken on.
    pub origin: String,
    /// Observed URL, when captured.
    pub url: Option<String>,
    /// Observed page title, when captured.
    pub title: Option<String>,
    /// Observed tab identifiers.
    pub tab_ids: Vec<String>,
    /// Asserts the host pre-redacted sensitive content from this record.
    pub redacted: bool,
}

/// The kind of a browser action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserActionKind {
    /// Navigate to a URL.
    Navigate,
    /// Click an element.
    Click,
    /// Type into an element.
    Input,
    /// Scroll the page.
    Scroll,
    /// Press a key.
    KeyPress,
    /// Wait for a condition.
    Wait,
    /// Capture a snapshot.
    Snapshot,
    /// Upload content.
    Upload,
    /// Download content.
    Download,
    /// An adapter-specific action, described by `detail`.
    Custom,
}

/// A normalized browser action request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAction {
    /// Action kind.
    pub kind: BrowserActionKind,
    /// Origin the action targets, when known.
    pub origin: Option<String>,
    /// Action target (selector/URL), when applicable.
    pub target: Option<String>,
    /// Additional action detail; must be credential-free.
    pub detail: Option<String>,
}

/// Outcome of a browser action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserActionOutcome {
    /// The action succeeded.
    Succeeded,
    /// The action failed.
    Failed {
        /// Failure reason class; must be credential-free.
        reason: String,
    },
    /// The native capability or policy layer blocked the action.
    BlockedByPolicy {
        /// The policy that blocked the action.
        policy: String,
    },
    /// An approval flow denied the action.
    DeniedByApproval,
}

/// The result of a browser action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionResult {
    /// Outcome of the action.
    pub outcome: BrowserActionOutcome,
    /// Additional result detail; must be credential-free.
    pub detail: Option<String>,
}

/// A captured browser artifact's identity (bytes stay with the evidence
/// plane).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserArtifact {
    /// Artifact name; credential-free.
    pub name: String,
    /// Media type; credential-free.
    pub media_kind: String,
    /// Origin the artifact came from, when known.
    pub origin: Option<String>,
}

/// What an approval decision was about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserApprovalSubject {
    /// Access to an origin.
    OriginAccess {
        /// The origin.
        origin: String,
    },
    /// A download from an origin.
    Download {
        /// The origin.
        origin: String,
    },
    /// An upload to an origin.
    Upload {
        /// The origin.
        origin: String,
    },
    /// Full CDP access on an origin.
    FullCdpAccess {
        /// The origin.
        origin: String,
    },
    /// Take-over of a browser session.
    SessionTakeOver {
        /// The session identifier.
        session_id: String,
    },
}

/// An approval decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserApprovalDecision {
    /// Approved.
    Approved,
    /// Denied.
    Denied,
}

/// A recorded approval flow decision. Codex approvals remain the
/// authority; the adapter only records what the native flow decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserApproval {
    /// What was approved or denied.
    pub subject: BrowserApprovalSubject,
    /// The decision.
    pub decision: BrowserApprovalDecision,
    /// Optional approver label (never a credential).
    pub approver: Option<String>,
}

/// A verification step performed against browser state (evidence only).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserVerification {
    /// Verification name.
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// Additional detail; must be credential-free.
    pub detail: Option<String>,
}

/// A recovery note recorded after the host performs a recovery action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BrowserRecoveryNote {
    /// Recovery strategy name.
    pub strategy: String,
    /// Credential-free detail.
    pub detail: String,
}

/// How the execution session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionEnd {
    /// Completed successfully.
    Completed,
    /// Completed with failure.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

/// The closed session's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionOutcome {
    /// The final binding identity.
    pub final_binding: BindingIdentity,
    /// All evidence records produced by the session.
    pub records: Vec<EvidenceRecord>,
    /// Advisory mapping of the session end onto the contract instance
    /// status. Instance lifecycle remains control-plane authority.
    pub suggested_instance_status: WorkflowInstanceStatus,
    /// How many interruption/recovery/take-over records were produced.
    pub recovery_count: usize,
}

/// Advisory mapping from a session end to the contract instance status.
pub fn suggested_instance_status(end: SessionEnd) -> WorkflowInstanceStatus {
    match end {
        SessionEnd::Completed => WorkflowInstanceStatus::Succeeded,
        SessionEnd::Failed => WorkflowInstanceStatus::Failed,
        SessionEnd::Cancelled => WorkflowInstanceStatus::Cancelled,
    }
}

#[derive(Serialize)]
struct ActionRecord<'a> {
    action: &'a BrowserAction,
    result: &'a BrowserActionResult,
}

#[derive(Serialize)]
struct InterruptionRecord {
    interruption: InterruptionKind,
    binding_fingerprint: String,
}

#[derive(Serialize)]
struct RebindRecord {
    takeover: bool,
    previous_fingerprint: String,
    new_fingerprint: String,
}

/// One execution turn bound to a concrete browser session. Opens via
/// [`crate::adapter::BrowserUseAdapter::begin_execution`].
pub struct BrowserExecutionSession {
    binding: BindingIdentity,
    binding_fingerprint_short: String,
    next_sequence: u64,
    records: Vec<EvidenceRecord>,
    recoveries: usize,
}

impl BrowserExecutionSession {
    pub(crate) fn new(binding: BindingIdentity) -> Self {
        let binding_fingerprint_short: String =
            binding.fingerprint_hex().chars().take(12).collect();
        Self {
            binding,
            binding_fingerprint_short,
            next_sequence: 0,
            records: Vec::new(),
            recoveries: 0,
        }
    }

    /// The session's binding identity.
    pub fn binding(&self) -> &BindingIdentity {
        &self.binding
    }

    /// All evidence records produced so far, in order.
    pub fn records(&self) -> &[EvidenceRecord] {
        &self.records
    }

    /// How many interruption/recovery/take-over records were produced.
    pub fn recovery_count(&self) -> usize {
        self.recoveries
    }

    fn push_record<T: Serialize + ?Sized>(&mut self, kind: EvidenceKind, tag: &str, payload: &T) {
        self.next_sequence += 1;
        let locator = format!(
            "codex-browser-use/{}/{tag}/{:04}",
            self.binding_fingerprint_short, self.next_sequence
        );
        self.records
            .push(EvidenceRecord::from_payload(kind, locator, payload));
    }

    /// Records a browser observation as `EvidenceKind::Observation`.
    pub fn record_observation(&mut self, observation: &BrowserObservation) {
        self.push_record(EvidenceKind::Observation, "observation", observation);
    }

    /// Records a browser action and its result as `EvidenceKind::Trace`.
    pub fn record_action(&mut self, action: &BrowserAction, result: &BrowserActionResult) {
        self.push_record(
            EvidenceKind::Trace,
            "action",
            &ActionRecord { action, result },
        );
    }

    /// Records a captured artifact as `EvidenceKind::Artifact`.
    pub fn record_artifact(&mut self, artifact: &BrowserArtifact) {
        self.push_record(EvidenceKind::Artifact, "artifact", artifact);
    }

    /// Records an approval decision as `EvidenceKind::Approval`.
    pub fn record_approval(&mut self, approval: &BrowserApproval) {
        self.push_record(EvidenceKind::Approval, "approval", approval);
    }

    /// Records a verification as `EvidenceKind::TestResult`.
    pub fn record_verification(&mut self, verification: &BrowserVerification) {
        self.push_record(EvidenceKind::TestResult, "verification", verification);
    }

    /// Records a host-performed recovery action as `EvidenceKind::Recovery`.
    pub fn record_recovery_note(&mut self, note: &BrowserRecoveryNote) {
        self.recoveries += 1;
        self.push_record(EvidenceKind::Recovery, "recovery", note);
    }

    pub(crate) fn record_interruption(&mut self, kind: InterruptionKind) {
        self.recoveries += 1;
        let payload = InterruptionRecord {
            interruption: kind,
            binding_fingerprint: self.binding.fingerprint_hex(),
        };
        self.push_record(EvidenceKind::Recovery, "interruption", &payload);
    }

    pub(crate) fn rebind(
        &mut self,
        previous: &BindingIdentity,
        next: BindingIdentity,
        takeover: bool,
    ) {
        self.recoveries += 1;
        let payload = RebindRecord {
            takeover,
            previous_fingerprint: previous.fingerprint_hex(),
            new_fingerprint: next.fingerprint_hex(),
        };
        self.binding = next;
        let tag = if takeover { "takeover" } else { "rebind" };
        self.push_record(EvidenceKind::Recovery, tag, &payload);
    }

    /// Closes the session and produces its outcome.
    pub fn finish(self, end: SessionEnd) -> ExecutionOutcome {
        ExecutionOutcome {
            suggested_instance_status: suggested_instance_status(end),
            final_binding: self.binding,
            records: self.records,
            recovery_count: self.recoveries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::{
        BROWSER_USE_CAPABILITY_ID, BindingIdentity, BrowserAdapterKind, BrowserSessionIdentity,
    };

    fn test_binding() -> BindingIdentity {
        BindingIdentity {
            capability_id: BROWSER_USE_CAPABILITY_ID.to_string(),
            adapter: BrowserAdapterKind::Native,
            session: BrowserSessionIdentity {
                profile: None,
                session_id: "s1".to_string(),
                tabs: vec!["tab-1".to_string()],
                takeover_of: None,
            },
            origin_scope: vec!["https://example.com".to_string()],
        }
    }

    #[test]
    fn artifacts_normalize_into_contract_evidence_kinds_in_order() {
        let mut session = BrowserExecutionSession::new(test_binding());
        session.record_observation(&BrowserObservation {
            origin: "https://example.com".to_string(),
            url: Some("https://example.com/a".to_string()),
            title: Some("Example".to_string()),
            tab_ids: vec!["tab-1".to_string()],
            redacted: true,
        });
        session.record_action(
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
        session.record_artifact(&BrowserArtifact {
            name: "download-report.pdf".to_string(),
            media_kind: "application/pdf".to_string(),
            origin: Some("https://example.com".to_string()),
        });
        session.record_approval(&BrowserApproval {
            subject: BrowserApprovalSubject::Download {
                origin: "https://example.com".to_string(),
            },
            decision: BrowserApprovalDecision::Approved,
            approver: Some("approver-label".to_string()),
        });
        session.record_verification(&BrowserVerification {
            name: "title-check".to_string(),
            passed: true,
            detail: None,
        });
        session.record_recovery_note(&BrowserRecoveryNote {
            strategy: "retry".to_string(),
            detail: "attempt 2".to_string(),
        });

        let records = session.records();
        let kinds: Vec<EvidenceKind> = records.iter().map(|record| record.kind).collect();
        assert_eq!(
            kinds,
            [
                EvidenceKind::Observation,
                EvidenceKind::Trace,
                EvidenceKind::Artifact,
                EvidenceKind::Approval,
                EvidenceKind::TestResult,
                EvidenceKind::Recovery,
            ]
        );
        let mut locators: Vec<&str> = records
            .iter()
            .map(|record| record.locator.as_str())
            .collect();
        locators.sort_unstable();
        let before = locators.len();
        locators.dedup();
        assert_eq!(locators.len(), before);
        for record in records {
            assert_eq!(record.digest_hex.len(), 64);
        }
        assert_eq!(session.recovery_count(), 1);
    }

    #[test]
    fn identical_payloads_digest_identically() {
        let observation = BrowserObservation {
            origin: "https://example.com".to_string(),
            url: None,
            title: None,
            tab_ids: Vec::new(),
            redacted: true,
        };
        let mut first = BrowserExecutionSession::new(test_binding());
        let mut second = BrowserExecutionSession::new(test_binding());
        first.record_observation(&observation);
        second.record_observation(&observation);
        assert_eq!(
            first.records()[0].digest_hex,
            second.records()[0].digest_hex
        );
    }

    #[test]
    fn finish_maps_end_to_suggested_instance_status() {
        assert_eq!(
            suggested_instance_status(SessionEnd::Completed),
            WorkflowInstanceStatus::Succeeded
        );
        assert_eq!(
            suggested_instance_status(SessionEnd::Failed),
            WorkflowInstanceStatus::Failed
        );
        assert_eq!(
            suggested_instance_status(SessionEnd::Cancelled),
            WorkflowInstanceStatus::Cancelled
        );
    }
}
