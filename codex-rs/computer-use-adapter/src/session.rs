//! Session ownership, takeover, and recovery records.
//!
//! Sessions are owned by opaque, credential-free owner tokens supplied by the
//! execution plane (for example a workflow instance + node reference pair).
//! Ownership, takeover rules, and recovery records are explicit first-class
//! data; the workflow control plane stays the only lifecycle authority for
//! workflow instances and merely consumes these records.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

/// Injectable wall clock returning unix milliseconds; injection keeps
/// lifecycle and evidence timestamps deterministic under test.
pub type UnixMsClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Returns the process wall clock as unix milliseconds.
pub fn system_clock() -> UnixMsClock {
    Arc::new(|| {
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    })
}

/// Identifier of one computer use session.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputerUseSessionId(String);

impl ComputerUseSessionId {
    /// Allocates a fresh session identifier.
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Opaque identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComputerUseSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque, credential-free owner token supplied by the execution plane.
///
/// Tokens must never carry credential material; they exist so takeover and
/// owner-mismatch boundaries are explicit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerToken(String);

impl OwnerToken {
    /// Creates a token from any credential-free string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Opaque token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Takeover mode requested by a claimant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TakeoverMode {
    /// Take over only while the session is idle; the adapter executes steps
    /// synchronously, so `Bound` is the idle state.
    Graceful,
    /// Take over unconditionally; requires a recorded reason and a policy
    /// that allows forced takeover.
    Forced,
}

/// Receipt of a successful takeover.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeoverReceipt {
    /// Session that changed ownership.
    pub session_id: ComputerUseSessionId,
    /// Owner recorded before the takeover.
    pub previous_owner: OwnerToken,
    /// Owner recorded after the takeover.
    pub new_owner: OwnerToken,
    /// Mode used.
    pub mode: TakeoverMode,
    /// Recorded reason (required for forced takeovers).
    pub reason: Option<String>,
    /// Unix milliseconds of the takeover.
    pub at_unix_ms: u64,
}

/// Why a recovery was initiated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RecoveryTrigger {
    /// An action failed on the bridge.
    #[serde(rename_all = "camelCase")]
    ActionFailed {
        /// Adapter-local action fingerprint reference.
        action_id: String,
        /// Failure reason text.
        reason: String,
    },
    /// The bridge became unavailable mid-step.
    BridgeLost,
    /// A forced takeover displaced the previous owner.
    #[serde(rename_all = "camelCase")]
    ForcedTakeover {
        /// Reason recorded by the claimant.
        reason: String,
    },
}

/// Recovery strategy chosen by the adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryStrategy {
    /// Release the session and readiness so the step can be retried end to
    /// end after lazy re-provisioning and re-authorization.
    ReleaseAndRebind,
    /// Surface the failure to the execution plane without retry.
    Escalate,
    /// Ownership handed to the takeover claimant with a recorded trigger.
    Handover,
}

/// Outcome of a recovery attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryOutcome {
    /// The adapter recovered on its own.
    Recovered,
    /// Recovery escalated to the execution plane.
    Escalated,
}

/// A recorded recovery, normalized into workflow evidence as
/// `EvidenceKind::Recovery`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryRecord {
    /// Session the recovery applies to.
    pub session_id: ComputerUseSessionId,
    /// What triggered recovery.
    pub trigger: RecoveryTrigger,
    /// Strategy applied.
    pub strategy: RecoveryStrategy,
    /// Outcome observed.
    pub outcome: RecoveryOutcome,
    /// Unix milliseconds of the recovery record.
    pub at_unix_ms: u64,
}

/// Recorded ownership of one adapter session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionOwnership {
    /// Session identifier.
    pub session_id: ComputerUseSessionId,
    /// Current owner.
    pub owner: OwnerToken,
    /// Unix milliseconds when the current owner acquired the session.
    pub acquired_at_unix_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::ComputerUseSessionId;
    use super::OwnerToken;
    use super::RecoveryOutcome;
    use super::RecoveryRecord;
    use super::RecoveryStrategy;
    use super::RecoveryTrigger;
    use super::TakeoverMode;
    use super::TakeoverReceipt;
    use super::system_clock;

    #[test]
    fn session_ids_are_opaque_and_unique() {
        let first = ComputerUseSessionId::generate();
        let second = ComputerUseSessionId::generate();
        assert_ne!(first, second);
        assert!(!first.as_str().is_empty());
        assert!(first.to_string().contains(first.as_str()));
    }

    #[test]
    fn owner_tokens_compare_by_value() {
        assert_eq!(OwnerToken::new("a/b"), OwnerToken::new("a/b"));
        assert_ne!(OwnerToken::new("a/b"), OwnerToken::new("b/a"));
    }

    #[test]
    fn recovery_and_takeover_records_serialize_camel_case() {
        let session = ComputerUseSessionId::generate();
        let recovery = RecoveryRecord {
            session_id: session.clone(),
            trigger: RecoveryTrigger::ForcedTakeover {
                reason: "ops".to_string(),
            },
            strategy: RecoveryStrategy::Handover,
            outcome: RecoveryOutcome::Recovered,
            at_unix_ms: 12,
        };
        let json = serde_json::to_value(&recovery).expect("serde");
        assert_eq!(json["sessionId"], session.as_str());
        assert_eq!(json["strategy"], "handover");
        assert_eq!(json["trigger"]["reason"], "ops");

        let receipt = TakeoverReceipt {
            session_id: session,
            previous_owner: OwnerToken::new("a"),
            new_owner: OwnerToken::new("b"),
            mode: TakeoverMode::Graceful,
            reason: None,
            at_unix_ms: 1,
        };
        let json = serde_json::to_value(&receipt).expect("serde");
        assert_eq!(json["mode"], "graceful");
        assert!(
            json.get("reason").is_some(),
            "absent reason serializes as null"
        );
    }

    #[test]
    fn system_clock_moves_forward() {
        let clock = system_clock();
        let first = clock();
        let second = clock();
        assert!(second >= first);
        assert!(first > 0);
    }
}
