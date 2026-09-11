//! Commercial terms: the deliberately separate commercial boundary.
//!
//! A [`CommercialTerms`] document binds a published release to a
//! [`CommercialPolicy`] — whether installing the release requires a
//! paid release entitlement, an active subscription, or a license
//! grant — and is digest-bound to the release exactly like publication
//! metadata. It carries **no prices, no payment credentials, no
//! settlement data, and no payment semantics**: monetization logic,
//! settlement, and credential handling are host/marketplace concerns
//! (intentionally unfrozen by the architecture lock), and entitlement
//! *grants* live in a host-owned store behind
//! [`crate::EntitlementPort`], which performs checks only.
//!
//! The invariant this separation buys: denial of an entitlement gates
//! distribution and installation only. It can never change — or be
//! interpreted as — executable workflow semantics, because nothing in
//! this document or its evaluation touches workflow source, evidence,
//! or execution records.

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowDistributionError;

/// Format version of the commercial terms document.
pub const COMMERCIAL_TERMS_FORMAT_VERSION: u32 = 1;

/// The commercial policy of a published release: what kind of
/// entitlement an installer must hold.
///
/// The policy is deliberately coarse. Prices, billing periods, trial
/// windows, and every other commercial detail belong to the host's
/// marketplace and are deliberately **not representable** here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum CommercialPolicy {
    /// Free: no entitlement is required to install.
    Free,
    /// A paid release: a one-time entitlement grant is required.
    PaidRelease,
    /// A subscription: an active subscription entitlement is required.
    Subscription,
    /// A specific license grant is required.
    LicenseGrant,
}

impl CommercialPolicy {
    /// Whether the policy requires an entitlement check at install
    /// time.
    pub fn requires_entitlement(self) -> bool {
        !matches!(self, Self::Free)
    }
}

/// Commercial terms bound to an immutable release.
///
/// The terms document is separate from
/// [`crate::PublicationMetadata`] by design: non-commercial
/// distribution metadata and commercial policy are distinct concerns
/// with distinct lifecycles, and conflating them would pull payment
/// semantics into workflow distribution. The `terms_digest` binds the
/// document to the release's immutable version identity, so a
/// marketplace cannot silently reprice a release's *policy class*
/// after publication — changing the policy requires publishing a new
/// immutable version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommercialTerms {
    /// Document format version.
    pub format_version: u32,
    /// The immutable release the terms bind to.
    pub release: WorkflowVersionId,
    /// The commercial policy.
    pub policy: CommercialPolicy,
    /// Digest over the release identity and the policy: the
    /// hash-chain seal of the terms document.
    pub terms_digest: ContentDigest,
}

impl CommercialTerms {
    /// Seals commercial terms against an immutable release.
    ///
    /// Free releases may carry terms too (making the free policy
    /// explicit and tamper-evident); paid, subscription, and
    /// license-grant policies are what the entitlement boundary
    /// checks.
    pub fn seal(
        release: WorkflowVersionId,
        policy: CommercialPolicy,
    ) -> Result<Self, WorkflowDistributionError> {
        let terms_digest = ContentDigest::of(&ChainedTerms {
            format_version: COMMERCIAL_TERMS_FORMAT_VERSION,
            release: &release,
            policy,
        })
        .map_err(|error| WorkflowDistributionError::InvalidCommercialTerms {
            reason: error.to_string(),
        })?;
        Ok(Self {
            format_version: COMMERCIAL_TERMS_FORMAT_VERSION,
            release,
            policy,
            terms_digest,
        })
    }

    /// Verifies the terms document: current format, and the recorded
    /// digest equals the recomputed hash chain over the release and
    /// policy.
    pub fn verify(&self) -> Result<(), WorkflowDistributionError> {
        if self.format_version != COMMERCIAL_TERMS_FORMAT_VERSION {
            return Err(WorkflowDistributionError::InvalidCommercialTerms {
                reason: format!(
                    "unsupported commercial terms format version {} (expected {COMMERCIAL_TERMS_FORMAT_VERSION})",
                    self.format_version
                ),
            });
        }
        let recomputed = ContentDigest::of(&ChainedTerms {
            format_version: self.format_version,
            release: &self.release,
            policy: self.policy,
        })
        .map_err(|error| WorkflowDistributionError::InvalidCommercialTerms {
            reason: error.to_string(),
        })?;
        if recomputed != self.terms_digest {
            return Err(WorkflowDistributionError::InvalidCommercialTerms {
                reason: "terms digest does not match the hash chain over the release and policy"
                    .to_owned(),
            });
        }
        Ok(())
    }
}

/// The digest-covered projection of the commercial terms document.
#[derive(Serialize)]
struct ChainedTerms<'a> {
    format_version: u32,
    release: &'a WorkflowVersionId,
    policy: CommercialPolicy,
}
