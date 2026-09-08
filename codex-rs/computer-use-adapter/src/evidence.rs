//! Normalization of adapter records into workflow evidence.
//!
//! Adapter evidence records are the adapter-side half of
//! `codex_workflow_contracts::EvidenceReference`: they carry the contract
//! `EvidenceKind`, an opaque adapter-allocated locator, and a sha-256 content
//! digest. The evidence plane (execution plane, WO-005) mints the final
//! contract reference — including the contract `ContentDigest` and instance
//! attachment via `WorkflowInstance::record_evidence` — from
//! [`AdapterEvidenceRecord::promotion`]; this crate never fabricates contract
//! identifiers it does not own.

use codex_workflow_contracts::EvidenceKind;
use serde::Deserialize;
use serde::Serialize;

use crate::action::NormalizedActionResult;
use crate::capability::CapabilityBinding;
use crate::observation::ScreenObservation;
use crate::policy::AuthorizationRecord;
use crate::session::ComputerUseSessionId;
use crate::session::RecoveryRecord;
use crate::util::canonical_digest;
use crate::util::sha256_hex;

/// Locator prefix for every adapter evidence record.
pub const EVIDENCE_LOCATOR_PREFIX: &str = "computer-use";

/// Adapter-side evidence record ready for promotion into a contract
/// `EvidenceReference` by the evidence plane.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterEvidenceRecord {
    /// Contract evidence kind.
    pub kind: EvidenceKind,
    /// Adapter-allocated opaque locator.
    pub locator: String,
    /// Lowercase hex sha-256 digest of the record payload.
    pub digest: String,
    /// Capability id the record belongs to (mixed-run scoping).
    pub capability_id: String,
    /// Session the record belongs to, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Unix milliseconds of recording (adapter clock).
    pub recorded_at_unix_ms: u64,
}

/// Promotion parts the evidence plane needs to mint a contract
/// `EvidenceReference`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidencePromotion {
    /// Contract evidence kind.
    pub kind: EvidenceKind,
    /// Opaque locator.
    pub locator: String,
    /// Lowercase hex sha-256 digest.
    pub digest_hex: String,
}

impl AdapterEvidenceRecord {
    /// Promotion parts for the evidence plane.
    pub fn promotion(&self) -> EvidencePromotion {
        EvidencePromotion {
            kind: self.kind,
            locator: self.locator.clone(),
            digest_hex: self.digest.clone(),
        }
    }
}

/// Allocates adapter evidence records with monotonic, deterministic locators.
pub(crate) struct EvidenceBuilder {
    capability_id: String,
    next_seq: u64,
}

impl EvidenceBuilder {
    pub(crate) fn new(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            next_seq: 1,
        }
    }

    pub(crate) fn record(
        &mut self,
        kind: EvidenceKind,
        payload_digest: String,
        session: Option<&ComputerUseSessionId>,
        recorded_at_unix_ms: u64,
    ) -> AdapterEvidenceRecord {
        let locator = format!(
            "{}/{}/{}",
            EVIDENCE_LOCATOR_PREFIX,
            slug(kind),
            self.next_seq
        );
        self.next_seq += 1;
        AdapterEvidenceRecord {
            kind,
            locator,
            digest: payload_digest,
            capability_id: self.capability_id.clone(),
            session_id: session.map(|id| id.as_str().to_string()),
            recorded_at_unix_ms,
        }
    }
}

fn slug(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Observation => "observation",
        EvidenceKind::Artifact => "artifact",
        EvidenceKind::Approval => "approval",
        EvidenceKind::Trace => "trace",
        EvidenceKind::TestResult => "test-result",
        EvidenceKind::Recovery => "recovery",
    }
}

/// Normalizes one screen observation into `(kind, digest)` evidence inputs.
pub fn normalize_observation(observation: &ScreenObservation) -> (EvidenceKind, String) {
    (EvidenceKind::Observation, observation.content_digest())
}

/// Normalizes one action result into `(kind, digest)` evidence inputs.
pub fn normalize_action_result(result: &NormalizedActionResult) -> (EvidenceKind, String) {
    (EvidenceKind::Observation, canonical_digest(result))
}

/// Normalizes one authorization record into `(kind, digest)` evidence inputs.
pub fn normalize_authorization(record: &AuthorizationRecord) -> (EvidenceKind, String) {
    (EvidenceKind::Approval, canonical_digest(record))
}

/// Normalizes one binding into `(kind, digest)` trace evidence inputs.
pub fn normalize_binding(binding: &CapabilityBinding) -> (EvidenceKind, String) {
    (EvidenceKind::Trace, canonical_digest(binding))
}

/// Normalizes one recovery record into `(kind, digest)` evidence inputs.
pub fn normalize_recovery(record: &RecoveryRecord) -> (EvidenceKind, String) {
    (EvidenceKind::Recovery, canonical_digest(record))
}

/// Normalizes an adapter diagnostic (readiness, release, takeover, fallback)
/// into `(kind, digest)` trace evidence inputs.
pub fn normalize_diagnostic(label: &str, detail: &str, at_unix_ms: u64) -> (EvidenceKind, String) {
    (
        EvidenceKind::Trace,
        sha256_hex(format!("{label}|{detail}|{at_unix_ms}").as_bytes()),
    )
}

#[cfg(test)]
mod tests {
    use codex_workflow_contracts::EvidenceKind;

    use super::EVIDENCE_LOCATOR_PREFIX;
    use super::EvidenceBuilder;
    use super::normalize_action_result;
    use super::normalize_recovery;
    use super::slug;
    use crate::action::ActionFailureReason;
    use crate::action::ActionStatus;
    use crate::action::NormalizedAction;
    use crate::action::NormalizedActionResult;
    use crate::session::ComputerUseSessionId;
    use crate::session::RecoveryOutcome;
    use crate::session::RecoveryRecord;
    use crate::session::RecoveryStrategy;
    use crate::session::RecoveryTrigger;

    #[test]
    fn kinds_map_to_locator_slugs_and_promotion_parts() {
        assert_eq!(slug(EvidenceKind::Observation), "observation");
        assert_eq!(slug(EvidenceKind::Artifact), "artifact");
        assert_eq!(slug(EvidenceKind::Approval), "approval");
        assert_eq!(slug(EvidenceKind::Trace), "trace");
        assert_eq!(slug(EvidenceKind::TestResult), "test-result");
        assert_eq!(slug(EvidenceKind::Recovery), "recovery");

        let mut builder = EvidenceBuilder::new("codex.computer-use");
        let session = ComputerUseSessionId::generate();
        let record = builder.record(
            EvidenceKind::Trace,
            "digest-1".to_string(),
            Some(&session),
            42,
        );
        assert_eq!(record.locator, format!("{EVIDENCE_LOCATOR_PREFIX}/trace/1"));
        assert_eq!(record.digest, "digest-1");
        assert_eq!(record.session_id.as_deref(), Some(session.as_str()));
        assert_eq!(record.recorded_at_unix_ms, 42);
        let promotion = record.promotion();
        assert_eq!(promotion.kind, EvidenceKind::Trace);
        assert_eq!(promotion.locator, record.locator);
        assert_eq!(promotion.digest_hex, "digest-1");

        let next = builder.record(EvidenceKind::Observation, "digest-2".to_string(), None, 43);
        assert_eq!(
            next.locator,
            format!("{EVIDENCE_LOCATOR_PREFIX}/observation/2"),
            "locator sequence is monotonic"
        );
        assert!(next.session_id.is_none());
    }

    #[test]
    fn normalizers_map_payloads_to_contract_kinds() {
        let action = NormalizedAction::Wait { millis: 5 };
        let result = NormalizedActionResult::succeeded(&action, 7);
        let (kind, digest) = normalize_action_result(&result);
        assert_eq!(kind, EvidenceKind::Observation);
        assert_eq!(digest.len(), 64, "sha-256 hex digest");

        let recovery = RecoveryRecord {
            session_id: ComputerUseSessionId::generate(),
            trigger: RecoveryTrigger::ActionFailed {
                action_id: "a1".to_string(),
                reason: "boom".to_string(),
            },
            strategy: RecoveryStrategy::Escalate,
            outcome: RecoveryOutcome::Escalated,
            at_unix_ms: 9,
        };
        let (kind, _) = normalize_recovery(&recovery);
        assert_eq!(kind, EvidenceKind::Recovery);
    }

    #[test]
    fn evidence_record_serializes_in_contract_shape() {
        let mut builder = EvidenceBuilder::new("codex.computer-use");
        let record = builder.record(EvidenceKind::Approval, "d".to_string(), None, 5);
        let json = serde_json::to_value(&record).expect("serde");
        assert_eq!(json["kind"], "approval");
        assert_eq!(json["capabilityId"], "codex.computer-use");
        assert_eq!(json["locator"], "computer-use/approval/1");
        assert!(json.get("sessionId").is_none());
    }

    #[test]
    fn action_failure_reasons_carry_normalized_kinds() {
        let action = NormalizedAction::Screenshot;
        let failed =
            NormalizedActionResult::failed(&action, ActionFailureReason::TargetNotFound, 1);
        match failed.status {
            ActionStatus::Failed { reason } => {
                assert_eq!(reason, ActionFailureReason::TargetNotFound)
            }
            other => panic!("expected failed status, got {other:?}"),
        }
    }
}
