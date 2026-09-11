//! The commercial entitlement boundary: checks only.
//!
//! The [`EntitlementPort`] answers exactly one question: *does this
//! principal hold the entitlement this release's commercial policy
//! requires?* It performs **checks only** — entitlement grants live in
//! a host-owned store, seeded through the host's real commercial
//! systems. This plane never mints grants, never processes payments,
//! never sees credentials, and never prices anything: denial of an
//! entitlement gates distribution and installation only, and never
//! changes executable workflow semantics.
//!
//! The port shape deliberately reuses the WO-011 trigger-plane
//! authorization discipline (see
//! `codex_workflow_triggers::ResourceAuthorizer` and its
//! `ResourceAuthorization` decision record): a synchronous record
//! seam, a decision that is *data* (a denial is a recorded outcome
//! attributed to a source mechanism, never a credential), and an
//! in-memory, default-deny double for tests. The concrete WO-011 port
//! authorizes **resource bindings** for installed workflows; this port
//! checks **commercial entitlements** for releases — different
//! questions, the same decision discipline, with no duplicated
//! authorization semantics.

use std::sync::Arc;
use std::sync::Mutex;

use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::CommercialPolicy;
use crate::MarketplacePrincipal;
use crate::WorkflowDistributionError;

/// A request to check one commercial entitlement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntitlementRequest {
    /// The principal whose entitlement is checked.
    pub principal: MarketplacePrincipal,
    /// The release being gated.
    pub release: WorkflowVersionId,
    /// The commercial policy being evaluated.
    pub policy: CommercialPolicy,
}

/// The decision of the entitlement boundary for one request.
///
/// The decision is data: `entitled: false` is a recorded outcome that
/// refuses installation, not an error and never a semantic change.
/// `source` labels the deciding mechanism (for example
/// `host-marketplace`) for audit; credentials never appear.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntitlementDecision {
    /// Whether the principal holds the required entitlement.
    pub entitled: bool,
    /// Attribution label of the deciding mechanism.
    pub source: String,
    /// Why the decision was reached, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The commercial entitlement boundary: checks only.
///
/// Host implementations bind this to their real entitlement stores
/// (license servers, subscription systems). Implementations must not
/// mint grants through this seam, must not process payments, and must
/// not return credentials in decisions — the boundary is a read-only
/// check over host-owned grants.
pub trait EntitlementPort: Send + Sync {
    /// Checks whether `request`'s principal holds the entitlement the
    /// request's policy requires.
    fn check(
        &mut self,
        request: &EntitlementRequest,
    ) -> Result<EntitlementDecision, WorkflowDistributionError>;
}

/// An in-memory entitlement registry: the host-owned grant store as a
/// test double.
///
/// Grants are **seeded** by the host (test or application seeding
/// standing in for real commercial systems); the double performs
/// checks only and is default-deny. It mirrors the
/// explicit-allow/default-deny shape of the WO-011 in-memory resource
/// authorizer.
#[derive(Clone, Debug, Default)]
pub struct InMemoryEntitlements {
    grants: Arc<Mutex<Vec<SeededGrant>>>,
}

/// One seeded entitlement grant (host-owned data).
#[derive(Clone, Debug, PartialEq, Eq)]
struct SeededGrant {
    principal: MarketplacePrincipal,
    release: WorkflowVersionId,
}

impl InMemoryEntitlements {
    /// Creates a default-deny registry with no grants.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds one entitlement grant, standing in for the host's real
    /// commercial system issuing the grant.
    pub fn seed_grant(&mut self, principal: MarketplacePrincipal, release: WorkflowVersionId) {
        self.lock().push(SeededGrant { principal, release });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<SeededGrant>> {
        self.grants
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl EntitlementPort for InMemoryEntitlements {
    fn check(
        &mut self,
        request: &EntitlementRequest,
    ) -> Result<EntitlementDecision, WorkflowDistributionError> {
        let entitled = if request.policy.requires_entitlement() {
            self.lock().iter().any(|grant| {
                grant.principal == request.principal && grant.release == request.release
            })
        } else {
            true
        };
        Ok(EntitlementDecision {
            entitled,
            source: "in-memory-entitlements".to_owned(),
            reason: if entitled {
                None
            } else {
                Some(format!(
                    "no {} grant for `{}` on this release",
                    policy_label(request.policy),
                    request.principal
                ))
            },
        })
    }
}

/// Stable label of a commercial policy for decisions and records.
fn policy_label(policy: CommercialPolicy) -> &'static str {
    match policy {
        CommercialPolicy::Free => "free",
        CommercialPolicy::PaidRelease => "paid-release",
        CommercialPolicy::Subscription => "subscription",
        CommercialPolicy::LicenseGrant => "license-grant",
    }
}
