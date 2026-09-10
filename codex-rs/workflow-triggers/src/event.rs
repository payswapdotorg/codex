//! Inbound trigger events and their idempotency keys.
//!
//! Triggers enter the plane as normalized, digest-bearing envelopes.
//! External event payloads are untrusted input: an envelope carries the
//! trigger class, an idempotency key, attribution/routing labels, and at
//! most a **digest** of the external payload — never the payload content.
//! The control plane authenticates, deduplicates, and evaluates the
//! envelope before any durable transition; no event ever mutates
//! workflow meaning.

use codex_workflow_contracts::TriggerClass;
use codex_workflow_contracts::WorkflowDefinitionId;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowTriggerError;

/// The idempotency key of one trigger event.
///
/// The key identifies the *event* (not the workflow): the control-plane
/// ledger deduplicates on `(workflow, key)`, so the same event may
/// legitimately start several different installed workflows once each,
/// but can never start the same workflow twice. Schedule-derived fires
/// use deterministic keys (`schedule:{schedule-id}:{occurrence-ms}`), so
/// a re-polled or crashed scheduler is naturally idempotent.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TriggerEventKey(String);

impl TriggerEventKey {
    /// Parses a trigger event key.
    ///
    /// Keys are non-empty, at most 128 characters, and restricted to
    /// letters, digits, `_`, `-`, `.`, and `:` so they can appear in
    /// control-plane records, logs, and evidence locators without
    /// escaping. Credentials never appear here by construction.
    pub fn parse(value: impl Into<String>) -> Result<Self, WorkflowTriggerError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 128
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(WorkflowTriggerError::InvalidEventKey {
                value,
                reason: "must be non-empty, at most 128 characters, restricted to [A-Za-z0-9_-.:]"
                    .to_string(),
            })
        }
    }
}

impl TryFrom<String> for TriggerEventKey {
    type Error = WorkflowTriggerError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<TriggerEventKey> for String {
    fn from(value: TriggerEventKey) -> Self {
        value.0
    }
}

impl AsRef<str> for TriggerEventKey {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// A normalized inbound trigger event.
///
/// The envelope is the untrusted-input boundary of the trigger plane.
/// Host ingress (a webhook endpoint, a connector subscription, a browser
/// or desktop event bridge, the application UI) normalizes what it
/// received into this shape before handing it over; the plane then
/// deduplicates, gates, and — only if everything passes — starts or
/// resumes exactly one control-plane transition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IncomingTrigger {
    /// The normalized trigger class.
    pub trigger: TriggerClass,
    /// The event's idempotency key (deduplicated per workflow).
    pub key: TriggerEventKey,
    /// When the event occurred, as host-normalized unix milliseconds.
    pub occurred_at_unix_ms: u64,
    /// Attribution label of the event source (for example
    /// `webhook:ops-hook`, `connector:github`, `user:alice`). Attribution
    /// only — never a credential.
    pub source: String,
    /// The opaque routing endpoint the event arrived on (a webhook path,
    /// connector subscription id, or session label), when applicable.
    ///
    /// Routing matches against the installation's
    /// [`crate::TriggerBinding`] endpoints: an event with `None` routing
    /// matches direct (non-endpoint) bindings only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<String>,
    /// The workflow this event explicitly targets, when the ingress
    /// knows it.
    ///
    /// When `None`, the plane routes by the installed trigger bindings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<WorkflowDefinitionId>,
    /// Hex digest of the untrusted external payload, recorded for audit
    /// without carrying content into workflow records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<String>,
}

impl IncomingTrigger {
    /// Builds an externally-routed event (webhook, connector, browser,
    /// computer, workflow, or human ingress).
    ///
    /// The event carries no target: the plane resolves the destination by
    /// the installed trigger bindings matching `class` and `endpoint`.
    pub fn external(
        trigger: TriggerClass,
        key: impl Into<String>,
        occurred_at_unix_ms: u64,
        source: impl Into<String>,
        endpoint: Option<String>,
        payload_digest: Option<String>,
    ) -> Result<Self, WorkflowTriggerError> {
        Ok(Self {
            trigger,
            key: TriggerEventKey::parse(key)?,
            occurred_at_unix_ms,
            source: source.into(),
            routing: endpoint,
            target: None,
            payload_digest,
        })
    }

    /// Builds the deterministic envelope of one schedule occurrence.
    ///
    /// The idempotency key is derived from the schedule identity and the
    /// occurrence timestamp, so re-polling the same occurrence is a
    /// recorded no-op rather than a second fire.
    pub(crate) fn scheduled(
        workflow: &WorkflowDefinitionId,
        schedule_id: &str,
        occurrence_unix_ms: u64,
    ) -> Result<Self, WorkflowTriggerError> {
        Ok(Self {
            trigger: TriggerClass::Schedule,
            key: TriggerEventKey::parse(format!("schedule:{schedule_id}:{occurrence_unix_ms}"))?,
            occurred_at_unix_ms: occurrence_unix_ms,
            source: format!("schedule:{schedule_id}"),
            routing: None,
            target: Some(workflow.clone()),
            payload_digest: None,
        })
    }
}
