//! In-memory implementations of the distribution seams: the reference
//! marketplace (the access-policy double lives in [`crate::access`]
//! and the entitlement registry in [`crate::entitlement`]).
//!
//! [`InMemoryMarketplace`] implements [`DistributionPort`] with pure
//! record collections — no network, no execution, no durable storage
//! — and is the reference semantics for every host implementation:
//! the gate order, the no-silent-change rules, and the
//! re-verify-before-record discipline are exactly what the trait
//! documents. The Codex application persists these records through
//! the workflow control plane.

use std::collections::BTreeMap;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::ForkLineage;
use codex_workflow_contracts::LicenseInfo;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;

use crate::AccessDecision;
use crate::AccessPolicy;
use crate::AccessRequest;
use crate::CommercialTerms;
use crate::DistributionPort;
use crate::DistributionState;
use crate::EntitlementDecision;
use crate::EntitlementPort;
use crate::EntitlementRequest;
use crate::ForkRequest;
use crate::MarketplaceInstall;
use crate::MarketplaceInstallRequest;
use crate::MarketplaceListing;
use crate::MarketplacePrincipal;
use crate::MarketplaceQuery;
use crate::PublicationMetadata;
use crate::PublicationScope;
use crate::PublishSubmission;
use crate::SourceLineage;
use crate::UpgradeDecision;
use crate::UpgradePolicySetting;
use crate::UpgradeProposal;
use crate::UpgradeRecord;
use crate::WorkflowDistributionError;
use crate::discovery::listing_matches;
use crate::discovery::listing_visible;
use crate::discovery::release_installable;
use crate::state::DistributionTransition;
use crate::state::validate_transition;

/// One release entry in the in-memory marketplace.
#[derive(Clone, Debug)]
struct ReleaseEntry {
    metadata: PublicationMetadata,
    state: DistributionState,
    audience: Vec<MarketplacePrincipal>,
    commercial: Option<CommercialTerms>,
}

impl ReleaseEntry {
    fn listing(&self) -> MarketplaceListing {
        MarketplaceListing {
            metadata: self.metadata.clone(),
            state: self.state,
            commercial: self.commercial.as_ref().map(|terms| terms.policy),
        }
    }
}

/// The in-memory marketplace: the reference [`DistributionPort`]
/// implementation.
///
/// The gate order and record discipline are the port's documented
/// contract:
///
/// - publish re-verifies the version, the metadata hash chain, the
///   metadata's compatibility claims, and any commercial terms before
///   recording anything;
/// - install runs visibility → integrity → already-installed → access
///   → entitlement before recording, and a denial records nothing;
/// - upgrades are surfaced as pure proposals and advance only through
///   an explicit decision whose gates re-run;
/// - scope transitions change only the entry's state and audience.
pub struct InMemoryMarketplace {
    access: Box<dyn AccessPolicy>,
    entitlements: Box<dyn EntitlementPort>,
    entries: BTreeMap<WorkflowVersionId, ReleaseEntry>,
    versions: BTreeMap<WorkflowVersionId, WorkflowVersion>,
    installs: BTreeMap<WorkflowDefinitionId, MarketplaceInstall>,
    upgrade_history: Vec<UpgradeRecord>,
}

impl InMemoryMarketplace {
    /// Creates an empty marketplace over the given gate seams.
    pub fn new(access: Box<dyn AccessPolicy>, entitlements: Box<dyn EntitlementPort>) -> Self {
        Self {
            access,
            entitlements,
            entries: BTreeMap::new(),
            versions: BTreeMap::new(),
            installs: BTreeMap::new(),
            upgrade_history: Vec::new(),
        }
    }

    /// The full upgrade history, oldest first.
    pub fn upgrade_history(&self) -> &[UpgradeRecord] {
        &self.upgrade_history
    }

    /// Loads the entry for a release, checking that it publishes
    /// `workflow`.
    fn entry_for(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<&ReleaseEntry, WorkflowDistributionError> {
        let entry =
            self.entries
                .get(version)
                .ok_or_else(|| WorkflowDistributionError::UnknownRelease {
                    workflow: workflow.to_string(),
                    version: version.to_string(),
                })?;
        if &entry.metadata.release.identity.workflow != workflow {
            return Err(WorkflowDistributionError::UnknownRelease {
                workflow: workflow.to_string(),
                version: version.to_string(),
            });
        }
        Ok(entry)
    }

    /// Loads and re-verifies the sealed version record for a release.
    fn verified_version(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<WorkflowVersion, WorkflowDistributionError> {
        let record = self.versions.get(version).ok_or_else(|| {
            WorkflowDistributionError::UnknownRelease {
                workflow: workflow.to_string(),
                version: version.to_string(),
            }
        })?;
        if &record.identity.workflow != workflow {
            return Err(WorkflowDistributionError::UnknownRelease {
                workflow: workflow.to_string(),
                version: version.to_string(),
            });
        }
        record.verify_integrity()?;
        Ok(record.clone())
    }

    /// Runs the access and entitlement gates for one principal over
    /// one release entry.
    fn run_gates(
        &mut self,
        principal: &MarketplacePrincipal,
        entry: &ReleaseEntry,
    ) -> Result<(AccessDecision, Option<EntitlementDecision>), WorkflowDistributionError> {
        let access = self.access.evaluate(&AccessRequest {
            principal: principal.clone(),
            release: entry.metadata.release.clone(),
            licensing: entry.metadata.licensing.clone(),
        })?;
        let entitlement = match &entry.commercial {
            None => None,
            Some(terms) => {
                let decision = self.entitlements.check(&EntitlementRequest {
                    principal: principal.clone(),
                    release: entry.metadata.release.version_id.clone(),
                    policy: terms.policy,
                })?;
                Some(decision)
            }
        };
        Ok((access, entitlement))
    }
}

impl DistributionPort for InMemoryMarketplace {
    fn publish(
        &mut self,
        submission: PublishSubmission,
    ) -> Result<MarketplaceListing, WorkflowDistributionError> {
        let version = submission.version;
        version.verify_integrity()?;
        submission.metadata.verify()?;
        submission.metadata.validate_against_version(&version)?;
        if let Some(terms) = &submission.commercial {
            terms.verify()?;
            if terms.release != version.version_id {
                return Err(WorkflowDistributionError::InvalidCommercialTerms {
                    reason: "commercial terms are bound to a different release".to_owned(),
                });
            }
        }
        let version_id = version.version_id.clone();
        if self.entries.contains_key(&version_id) {
            return Err(WorkflowDistributionError::AlreadyPublished {
                workflow: version.identity.workflow.to_string(),
                version: version_id.to_string(),
            });
        }
        let audience = match &submission.scope {
            PublicationScope::Shared { audience } => audience.clone(),
            _ => Vec::new(),
        };
        let entry = ReleaseEntry {
            metadata: submission.metadata,
            state: submission.scope.entry_state(),
            audience,
            commercial: submission.commercial,
        };
        let listing = entry.listing();
        self.entries.insert(version_id.clone(), entry);
        self.versions.insert(version_id, version);
        Ok(listing)
    }

    fn search(
        &self,
        query: &MarketplaceQuery,
    ) -> Result<Vec<MarketplaceListing>, WorkflowDistributionError> {
        let mut listings = Vec::new();
        for entry in self.entries.values() {
            if listing_visible(
                &entry.metadata,
                entry.state,
                &entry.audience,
                query.requesting.as_ref(),
            ) && listing_matches(&entry.metadata, query)
            {
                listings.push(entry.listing());
            }
        }
        Ok(listings)
    }

    fn fetch_version(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<Option<WorkflowVersion>, WorkflowDistributionError> {
        match self.versions.get(version) {
            None => Ok(None),
            Some(record) if &record.identity.workflow != workflow => Ok(None),
            Some(record) => {
                record.verify_integrity()?;
                Ok(Some(record.clone()))
            }
        }
    }

    fn fetch_listing(
        &self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
    ) -> Result<Option<MarketplaceListing>, WorkflowDistributionError> {
        match self.entry_for(workflow, version) {
            Err(WorkflowDistributionError::UnknownRelease { .. }) => Ok(None),
            Err(error) => Err(error),
            Ok(entry) => {
                entry.metadata.verify()?;
                Ok(Some(entry.listing()))
            }
        }
    }

    fn fork(
        &mut self,
        request: ForkRequest,
    ) -> Result<MarketplaceListing, WorkflowDistributionError> {
        let upstream_ref = request.upstream;
        upstream_ref.verify()?;
        let upstream_workflow = upstream_ref.identity.workflow.clone();
        let upstream = self.verified_version(&upstream_workflow, &upstream_ref.version_id)?;
        let upstream_entry = self
            .entry_for(&upstream_workflow, &upstream_ref.version_id)?
            .clone();
        upstream_entry.metadata.verify()?;

        if request.carried_attribution.is_empty() {
            return Err(WorkflowDistributionError::InvalidRecord {
                reason: "a fork must carry upstream attribution".to_owned(),
            });
        }
        if request.fork_repository == upstream.identity.repository {
            return Err(WorkflowDistributionError::InvalidRecord {
                reason: "a fork's repository must differ from its upstream's".to_owned(),
            });
        }

        // The fork's authorship: upstream attribution carried forward
        // plus the fork's owner. Both the sealed version's provenance
        // and the metadata document record it.
        let mut fork_authors = request.carried_attribution.clone();
        fork_authors.push(Attribution {
            name: request.owner.to_string(),
            contact: None,
        });
        let provenance = WorkflowProvenance {
            authors: fork_authors.clone(),
            forked_from: Some(ForkLineage {
                upstream: upstream.identity.repository.clone(),
                forked_at: Some(upstream.identity.source_revision.commit_sha.clone()),
            }),
            license: Some(LicenseInfo {
                spdx: request.licensing.identifier.clone(),
                url: request.licensing.url.clone(),
            }),
            upgrade_policy: Some(request.upgrade_policy.to_contract()),
        };
        let derived = WorkflowVersion::seal(
            upstream.definition.clone(),
            request.fork_repository,
            request.fork_revision,
            request.fork_version,
            upstream.dependency_lock,
            Some(provenance),
        )?;
        let derived_id = derived.version_id.clone();
        if self.entries.contains_key(&derived_id) {
            return Err(WorkflowDistributionError::AlreadyPublished {
                workflow: derived.identity.workflow.to_string(),
                version: derived_id.to_string(),
            });
        }
        let derived_ref = PublishedVersionRef::of(&derived);
        let metadata = PublicationMetadata::seal(
            derived_ref,
            fork_authors,
            request.owner,
            request.licensing,
            SourceLineage {
                forked_from: Some(upstream_ref),
                carried_attribution: request.carried_attribution,
            },
            upstream_entry.metadata.compatibility,
            request.upgrade_policy,
        )?;
        metadata.validate_against_version(&derived)?;
        let entry = ReleaseEntry {
            metadata,
            state: DistributionState::Forked,
            audience: Vec::new(),
            commercial: None,
        };
        let listing = entry.listing();
        self.entries.insert(derived_id.clone(), entry);
        self.versions.insert(derived_id, derived);
        Ok(listing)
    }

    fn install(
        &mut self,
        request: MarketplaceInstallRequest,
    ) -> Result<MarketplaceInstall, WorkflowDistributionError> {
        let entry = self.entry_for(&request.workflow, &request.version)?.clone();
        if !release_installable(
            &entry.metadata,
            entry.state,
            &entry.audience,
            &request.principal,
        ) {
            return Err(WorkflowDistributionError::ReleaseNotVisible {
                workflow: request.workflow.to_string(),
                version: request.version.to_string(),
            });
        }
        let version = self.verified_version(&request.workflow, &request.version)?;
        if let Some(installed) = self.installs.get(&request.workflow) {
            return Err(WorkflowDistributionError::AlreadyInstalled {
                workflow: request.workflow.to_string(),
                version: installed.installed.version_id.to_string(),
            });
        }
        let (access, entitlement) = self.run_gates(&request.principal, &entry)?;
        if !access.allowed
            || entitlement
                .as_ref()
                .is_some_and(|decision| !decision.entitled)
        {
            return Err(WorkflowDistributionError::InstallRefused {
                workflow: request.workflow.to_string(),
                access,
                entitlement,
            });
        }
        let install = MarketplaceInstall {
            workflow: request.workflow.clone(),
            principal: request.principal,
            installed: entry.metadata.release.clone(),
            version,
            access,
            entitlement,
            upgrade_policy: entry.metadata.upgrade_policy,
            at_unix_ms: request.at_unix_ms,
        };
        self.installs.insert(request.workflow, install.clone());
        Ok(install)
    }

    fn apply_transition(
        &mut self,
        workflow: &WorkflowDefinitionId,
        version: &WorkflowVersionId,
        transition: DistributionTransition,
    ) -> Result<MarketplaceListing, WorkflowDistributionError> {
        let entry = self.entries.get_mut(version).ok_or_else(|| {
            WorkflowDistributionError::UnknownRelease {
                workflow: workflow.to_string(),
                version: version.to_string(),
            }
        })?;
        if &entry.metadata.release.identity.workflow != workflow {
            return Err(WorkflowDistributionError::UnknownRelease {
                workflow: workflow.to_string(),
                version: version.to_string(),
            });
        }
        let target = validate_transition(entry.state, &transition)?;
        entry.state = target;
        if let DistributionTransition::Share { audience } = transition {
            entry.audience = audience;
        }
        Ok(entry.listing())
    }

    fn evaluate_upgrade(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<UpgradeProposal>, WorkflowDistributionError> {
        let install =
            self.installs
                .get(workflow)
                .ok_or_else(|| WorkflowDistributionError::NotInstalled {
                    workflow: workflow.to_string(),
                })?;
        if install.upgrade_policy == UpgradePolicySetting::Pin {
            return Ok(None);
        }
        let newest = self
            .entries
            .values()
            .filter(|entry| {
                &entry.metadata.release.identity.workflow == workflow
                    && entry.metadata.release.version_id != install.installed.version_id
            })
            .max_by_key(|entry| entry.metadata.release.identity.semantic_version.clone());
        Ok(newest.map(|entry| UpgradeProposal {
            workflow: workflow.clone(),
            from: install.installed.version_id.clone(),
            to: entry.metadata.release.clone(),
            policy: UpgradePolicySetting::Follow,
        }))
    }

    fn decide_upgrade(
        &mut self,
        proposal: &UpgradeProposal,
        decision: UpgradeDecision,
        at_unix_ms: u64,
    ) -> Result<UpgradeRecord, WorkflowDistributionError> {
        let workflow = proposal.workflow.clone();
        let install = self.installs.get(&workflow).ok_or_else(|| {
            WorkflowDistributionError::NotInstalled {
                workflow: workflow.to_string(),
            }
        })?;
        if install.installed.version_id != proposal.from {
            return Err(WorkflowDistributionError::StaleUpgradeProposal {
                workflow: workflow.to_string(),
                expected: proposal.from.to_string(),
                actual: install.installed.version_id.to_string(),
            });
        }
        if decision == UpgradeDecision::Reject {
            let record = UpgradeRecord {
                workflow: workflow.clone(),
                from: proposal.from.clone(),
                to: proposal.from.clone(),
                applied: false,
                at_unix_ms,
            };
            self.upgrade_history.push(record.clone());
            return Ok(record);
        }

        // Approval path: re-verify the target release and re-run every
        // install gate for it. An upgrade the gates would refuse is
        // refused with the decision data and changes nothing.
        proposal.to.verify()?;
        let entry = self.entry_for(&workflow, &proposal.to.version_id)?.clone();
        let version = self.verified_version(&workflow, &proposal.to.version_id)?;
        let principal = install.principal.clone();
        let (access, entitlement) = self.run_gates(&principal, &entry)?;
        if !access.allowed
            || entitlement
                .as_ref()
                .is_some_and(|decision| !decision.entitled)
        {
            return Err(WorkflowDistributionError::UpgradeRefused {
                workflow: workflow.to_string(),
                access,
                entitlement,
            });
        }
        let upgrade_policy = entry.metadata.upgrade_policy;
        let to = proposal.to.version_id.clone();
        let updated = MarketplaceInstall {
            workflow: workflow.clone(),
            principal,
            installed: proposal.to.clone(),
            version,
            access,
            entitlement,
            upgrade_policy,
            at_unix_ms,
        };
        self.installs.insert(workflow.clone(), updated);
        let record = UpgradeRecord {
            workflow,
            from: proposal.from.clone(),
            to,
            applied: true,
            at_unix_ms,
        };
        self.upgrade_history.push(record.clone());
        Ok(record)
    }

    fn installed(
        &self,
        workflow: &WorkflowDefinitionId,
    ) -> Result<Option<MarketplaceInstall>, WorkflowDistributionError> {
        Ok(self.installs.get(workflow).cloned())
    }
}
