//! The access-policy seam: license and sharing-term evaluation for
//! distribution and installation.
//!
//! [`AccessPolicy`] decides whether a principal may access and
//! install a release under its license terms. Its shape deliberately
//! reuses the WO-011 trigger-plane authorization discipline (see
//! `codex_workflow_triggers::ResourceAuthorizer` and its
//! `ResourceAuthorization` decision record): a synchronous record
//! seam, a decision that is *data* (a denial is a recorded outcome
//! attributed to a source mechanism, never a credential), and an
//! in-memory, default-deny double for tests. The concrete WO-011
//! port authorizes **resource bindings** for installed workflows;
//! this port authorizes **distribution access** — different
//! questions, the same decision discipline, with no duplicated
//! authorization semantics.
//!
//! Denials gate distribution and installation only: they never
//! change executable workflow semantics, and a refused install
//! records nothing.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;

use codex_workflow_forge::PublishedVersionRef;

use crate::LicenseTerms;
use crate::MarketplacePrincipal;
use crate::WorkflowDistributionError;

/// A request to evaluate one distribution access decision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessRequest {
    /// The principal seeking access.
    pub principal: MarketplacePrincipal,
    /// The release being accessed.
    pub release: PublishedVersionRef,
    /// The license terms of the release.
    pub licensing: LicenseTerms,
}

/// The decision of the access policy for one request.
///
/// The decision is data: `allowed: false` is a recorded outcome that
/// refuses installation, not an error and never a semantic change.
/// `source` labels the deciding mechanism for audit; credentials
/// never appear.
///
/// This shape deliberately reuses the WO-011
/// `codex_workflow_triggers::ResourceAuthorization` decision shape.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessDecision {
    /// Whether access is allowed.
    pub allowed: bool,
    /// Attribution label of the deciding mechanism.
    pub source: String,
    /// Why the decision was reached, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The access-policy seam: license and sharing-term evaluation for
/// installs.
///
/// Host implementations bind this to their real license acceptance
/// and sharing surfaces. Denials gate distribution and installation
/// only; they never change executable semantics.
pub trait AccessPolicy: Send + Sync {
    /// Evaluates whether `request`'s principal may access and install
    /// the release under its license terms.
    fn evaluate(
        &mut self,
        request: &AccessRequest,
    ) -> Result<AccessDecision, WorkflowDistributionError>;
}

/// An in-memory access policy: the explicit-allow, default-deny
/// double for license/sharing evaluation.
///
/// The shape mirrors the WO-011 in-memory resource authorizer: grants
/// are seeded by the host, decisions are attributed to a source
/// label, and every evaluation is recorded for audit. Credentials
/// never appear.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAccessPolicy {
    state: Arc<Mutex<AccessState>>,
}

/// The backing state of [`InMemoryAccessPolicy`].
#[derive(Clone, Debug, Default)]
struct AccessState {
    allowed: BTreeMap<(String, String), String>,
    requests: Vec<AccessRequest>,
}

impl InMemoryAccessPolicy {
    /// Creates a default-deny policy with an empty allow list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allows one principal to install one release, attributed to
    /// `source` (for example `host-license-acceptance`).
    pub fn allow(
        &mut self,
        principal: impl AsRef<str>,
        release: impl AsRef<str>,
        source: impl Into<String>,
    ) {
        self.lock().allowed.insert(
            (principal.as_ref().to_string(), release.as_ref().to_string()),
            source.into(),
        );
    }

    /// Revokes a previously allowed install grant.
    pub fn revoke(&mut self, principal: impl AsRef<str>, release: impl AsRef<str>) {
        self.lock()
            .allowed
            .remove(&(principal.as_ref().to_string(), release.as_ref().to_string()));
    }

    /// Snapshot of the access requests seen so far, in order.
    pub fn requests(&self) -> Vec<AccessRequest> {
        self.lock().requests.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, AccessState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl AccessPolicy for InMemoryAccessPolicy {
    fn evaluate(
        &mut self,
        request: &AccessRequest,
    ) -> Result<AccessDecision, WorkflowDistributionError> {
        let lookup = (
            request.principal.as_ref().to_string(),
            request.release.version_id.to_string(),
        );
        let mut state = self.lock();
        let decision = match state.allowed.get(&lookup) {
            Some(source) => AccessDecision {
                allowed: true,
                source: source.clone(),
                reason: None,
            },
            None => AccessDecision {
                allowed: false,
                source: "in-memory-access-policy".to_string(),
                reason: Some(format!(
                    "principal `{}` has no license acceptance recorded for this release",
                    request.principal
                )),
            },
        };
        state.requests.push(request.clone());
        Ok(decision)
    }
}
