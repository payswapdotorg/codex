//! WO-012 end-to-end tests: workflow distribution, marketplace, and
//! monetization.
//!
//! Every scenario is **static**: in-memory doubles stand in for the
//! host seams (access policy, entitlement registry, trigger plane),
//! so no network, payment system, browser, or desktop runtime is
//! touched. The scenarios map to the Work Order's required evidence:
//!
//! - **publish → immutable**: sealed releases publish, re-verify on
//!   every fetch, tampered versions/metadata/terms are rejected, and
//!   republishing is refused;
//! - **fork lineage provenance**: forking derives a new immutable
//!   version whose provenance points back (metadata lineage and the
//!   sealed version's own fork lineage), the upstream is untouched;
//! - **license-gated install**: deny and allow paths through the
//!   access-policy port, denials recording nothing;
//! - **entitlement boundary**: commercial entitlement denial gates
//!   installation only — sealed records stay byte-identical before,
//!   during, and after entitlement evaluation;
//! - **upgrade policy**: pin stays pinned; follow advances only
//!   through an explicit, recorded approval; stale proposals and
//!   gate-refused upgrades change nothing;
//! - **search/filter correctness**: every facet (capability,
//!   capability class, resource, license, provenance, compatibility,
//!   name) and every visibility scope;
//! - **no-credentials invariants**: serialized distribution records
//!   carry no credential-shaped keys or values, principals reject
//!   credential shapes, and repository identity is credential-free by
//!   construction;
//! - **installed identity stays bound to the immutable release**: the
//!   marketplace install hands off to the WO-011 trigger plane, whose
//!   installed configuration pins the same immutable version identity
//!   through the forge install registry.
//!
//! The workspace clippy.toml allows `expect`/`unwrap` in test code.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::ResourceRequirement;
use codex_workflow_contracts::ResourceTypeId;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::StepNode;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowDependencies;
use codex_workflow_contracts::WorkflowIr;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::PublishedVersionRef;
use pretty_assertions::assert_eq;
use serde_json::Value;

use codex_workflow_distribution::CommercialPolicy;
use codex_workflow_distribution::CommercialTerms;
use codex_workflow_distribution::CompatibilitySpec;
use codex_workflow_distribution::DistributionPort;
use codex_workflow_distribution::DistributionState;
use codex_workflow_distribution::DistributionTransition;
use codex_workflow_distribution::ForkRequest;
use codex_workflow_distribution::InMemoryAccessPolicy;
use codex_workflow_distribution::InMemoryEntitlements;
use codex_workflow_distribution::InMemoryMarketplace;
use codex_workflow_distribution::LicenseTerms;
use codex_workflow_distribution::MarketplaceInstallRequest;
use codex_workflow_distribution::MarketplaceListing;
use codex_workflow_distribution::MarketplacePrincipal;
use codex_workflow_distribution::MarketplaceQuery;
use codex_workflow_distribution::PublicationMetadata;
use codex_workflow_distribution::PublicationScope;
use codex_workflow_distribution::PublishSubmission;
use codex_workflow_distribution::SourceLineage;
use codex_workflow_distribution::UpgradeDecision;
use codex_workflow_distribution::UpgradePolicySetting;
use codex_workflow_distribution::WorkflowDistributionError;

const ACME_REPO: &str = "https://github.com/acme/ops-workflows";
const ALICE_FORK_REPO: &str = "https://github.com/alice/ops-workflows";

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn sha(seed: u64) -> RevisionSha {
    RevisionSha::parse(format!("{seed:040x}")).expect("seeded SHA")
}

fn principal(value: &str) -> MarketplacePrincipal {
    MarketplacePrincipal::parse(value).expect("principal")
}

/// The capabilities a sealed version's steps declare.
fn declared_capabilities(version: &WorkflowVersion) -> Vec<CapabilityId> {
    version
        .definition
        .ir
        .nodes
        .values()
        .filter_map(|node| match node {
            WorkflowIrNode::Step(step) => Some(step.capabilities.clone()),
            _ => None,
        })
        .flatten()
        .map(|requirement| requirement.capability)
        .collect()
}

/// The resources a sealed version's dependencies declare.
fn declared_resources(version: &WorkflowVersion) -> Vec<ResourceTypeId> {
    version
        .definition
        .dependencies
        .resources
        .iter()
        .map(|requirement| requirement.resource_type.clone())
        .collect()
}

/// Seals a single-step workflow version with declared capabilities
/// and resources, directly through the frozen contract (the same
/// `WorkflowVersion::seal` primitive the publication pipeline ends
/// with).
fn sealed_version(
    workflow: &str,
    repository: &str,
    commit: u64,
    version: (u64, u64, u64),
    capabilities: &[&str],
    resources: &[&str],
) -> WorkflowVersion {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        node_id("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: capabilities
                .iter()
                .map(|value| CapabilityRequirement {
                    capability: CapabilityId::parse(*value).expect("capability"),
                    purpose: None,
                })
                .collect(),
            roles: Vec::new(),
            description: Some(format!("execute {workflow}")),
            next: None,
        }),
    );
    let definition = WorkflowDefinition {
        id: WorkflowDefinitionId::parse(workflow).expect("workflow id"),
        description: None,
        ir: WorkflowIr {
            ir_format: IR_FORMAT_VERSION,
            entry: node_id("step-001"),
            nodes,
            conditions: BTreeMap::new(),
        },
        roles: BTreeMap::new(),
        triggers: Vec::new(),
        dependencies: WorkflowDependencies {
            resources: resources
                .iter()
                .map(|value| ResourceRequirement {
                    resource_type: ResourceTypeId::parse(*value).expect("resource type"),
                    purpose: None,
                })
                .collect(),
            ..WorkflowDependencies::default()
        },
    };
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse(repository).expect("repository"),
        ImmutableSourceRevision::pin_commit(sha(commit)),
        SemanticVersion::new(version.0, version.1, version.2),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version")
}

/// Builds publication metadata covering exactly what the version
/// declares (so compatibility claims are honest), with taxonomy
/// classes and license supplied by the caller.
fn publication_metadata(
    version: &WorkflowVersion,
    owner: &str,
    license: &str,
    classes: &[&str],
    minimum_runtime: (u64, u64, u64),
    policy: UpgradePolicySetting,
) -> PublicationMetadata {
    PublicationMetadata::seal(
        PublishedVersionRef::of(version),
        vec![Attribution {
            name: owner.to_owned(),
            contact: None,
        }],
        principal(owner),
        LicenseTerms {
            identifier: license.to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage::default(),
        CompatibilitySpec {
            minimum_runtime: SemanticVersion::new(
                minimum_runtime.0,
                minimum_runtime.1,
                minimum_runtime.2,
            ),
            required_capabilities: declared_capabilities(version),
            capability_classes: classes
                .iter()
                .map(|value| {
                    codex_workflow_distribution::CapabilityClass::parse(*value).expect("class")
                })
                .collect(),
            required_resources: declared_resources(version),
        },
        policy,
    )
    .expect("publication metadata")
}

/// Publishes a free release under a scope.
fn publish(
    market: &mut InMemoryMarketplace,
    version: WorkflowVersion,
    metadata: PublicationMetadata,
    scope: PublicationScope,
) -> MarketplaceListing {
    market
        .publish(PublishSubmission {
            version,
            metadata,
            commercial: None,
            scope,
        })
        .expect("publication succeeds")
}

/// A marketplace plus its gate doubles, with an access grant already
/// seeded for `principal` over `version`.
fn marketplace_with_access(
    principal: &str,
    version: &WorkflowVersion,
) -> (
    InMemoryMarketplace,
    InMemoryAccessPolicy,
    InMemoryEntitlements,
) {
    let mut access = InMemoryAccessPolicy::new();
    access.allow(principal, &version.version_id, "host-license-acceptance");
    let entitlements = InMemoryEntitlements::new();
    let market = InMemoryMarketplace::new(Box::new(access.clone()), Box::new(entitlements.clone()));
    (market, access, entitlements)
}

/// A fresh marketplace with default-deny gates.
fn plain_marketplace() -> (
    InMemoryMarketplace,
    InMemoryAccessPolicy,
    InMemoryEntitlements,
) {
    let access = InMemoryAccessPolicy::new();
    let entitlements = InMemoryEntitlements::new();
    let market = InMemoryMarketplace::new(Box::new(access.clone()), Box::new(entitlements.clone()));
    (market, access, entitlements)
}

/// The listing state of one release.
fn state_of(market: &InMemoryMarketplace, version: &WorkflowVersion) -> DistributionState {
    market
        .fetch_listing(&version.identity.workflow, &version.version_id)
        .expect("listing fetches")
        .expect("release published")
        .state
}

/// Version-id strings of search results, sorted, for set comparison.
fn result_releases(market: &InMemoryMarketplace, query: &MarketplaceQuery) -> Vec<String> {
    let mut releases = market
        .search(query)
        .expect("search succeeds")
        .into_iter()
        .map(|listing| listing.metadata.release.version_id.to_string())
        .collect::<Vec<_>>();
    releases.sort();
    releases
}

#[test]
fn publishing_seals_releases_and_rejects_every_tampered_record() {
    let (mut market, _, _) = plain_marketplace();
    let version = sealed_version(
        "invoice-triage",
        ACME_REPO,
        1,
        (1, 0, 0),
        &["send_email"],
        &["slack_workspace"],
    );
    let metadata = publication_metadata(
        &version,
        "acme-ops",
        "Apache-2.0",
        &["document-processing"],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );

    // The golden publish: the listing is a metadata-only view in the
    // requested scope, and every later fetch re-verifies.
    let listing = publish(
        &mut market,
        version.clone(),
        metadata.clone(),
        PublicationScope::Public,
    );
    assert_eq!(listing.state, DistributionState::Public);
    assert_eq!(listing.commercial, None);
    let fetched = market
        .fetch_version(&version.identity.workflow, &version.version_id)
        .expect("version fetches")
        .expect("release published");
    fetched.verify_integrity().expect("integrity holds");
    let fetched_listing = market
        .fetch_listing(&version.identity.workflow, &version.version_id)
        .expect("listing fetches")
        .expect("release published");
    fetched_listing
        .metadata
        .verify()
        .expect("metadata chain holds");
    assert_eq!(fetched_listing, listing);

    // Republishing the same immutable version identity is refused.
    let republish = market.publish(PublishSubmission {
        version: version.clone(),
        metadata: metadata.clone(),
        commercial: None,
        scope: PublicationScope::Private,
    });
    assert!(matches!(
        republish,
        Err(WorkflowDistributionError::AlreadyPublished { .. })
    ));

    // A tampered version record (definition content changed after
    // sealing) never publishes.
    let mut tampered_version = version.clone();
    tampered_version.definition.description = Some("silently rewritten".to_owned());
    let tampered = market.publish(PublishSubmission {
        version: tampered_version,
        metadata: metadata.clone(),
        commercial: None,
        scope: PublicationScope::Public,
    });
    assert!(matches!(
        tampered,
        Err(WorkflowDistributionError::Contract(
            codex_workflow_contracts::WorkflowContractError::VersionIntegrity { .. }
        ))
    ));

    // A tampered metadata document (license rewritten after sealing)
    // fails its hash chain.
    let mut tampered_metadata = metadata.clone();
    tampered_metadata.licensing.identifier = "MIT".to_owned();
    let tampered = market.publish(PublishSubmission {
        version: version.clone(),
        metadata: tampered_metadata,
        commercial: None,
        scope: PublicationScope::Public,
    });
    assert!(matches!(
        tampered,
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));

    // Metadata that understates the version's compatibility claims is
    // refused: marketplace facets must not lie.
    let understating = PublicationMetadata::seal(
        PublishedVersionRef::of(&version),
        vec![Attribution {
            name: "acme-ops".to_owned(),
            contact: None,
        }],
        principal("acme-ops"),
        LicenseTerms {
            identifier: "Apache-2.0".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage::default(),
        CompatibilitySpec {
            minimum_runtime: SemanticVersion::new(0, 2, 0),
            required_capabilities: Vec::new(),
            capability_classes: Vec::new(),
            required_resources: Vec::new(),
        },
        UpgradePolicySetting::Follow,
    )
    .expect("metadata seals");
    let understated = market.publish(PublishSubmission {
        version: version.clone(),
        metadata: understating,
        commercial: None,
        scope: PublicationScope::Public,
    });
    assert!(matches!(
        understated,
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));

    // Tampered or misbound commercial terms never publish.
    let terms = CommercialTerms::seal(version.version_id.clone(), CommercialPolicy::PaidRelease)
        .expect("terms seal");
    let mut tampered_terms = terms;
    tampered_terms.policy = CommercialPolicy::Subscription;
    let misbound = market.publish(PublishSubmission {
        version: version.clone(),
        metadata: metadata.clone(),
        commercial: Some(tampered_terms),
        scope: PublicationScope::Public,
    });
    assert!(matches!(
        misbound,
        Err(WorkflowDistributionError::InvalidCommercialTerms { .. })
    ));
    let other = sealed_version(
        "invoice-triage",
        ACME_REPO,
        2,
        (1, 1, 0),
        &["send_email"],
        &["slack_workspace"],
    );
    let misbound = market.publish(PublishSubmission {
        version: version.clone(),
        metadata,
        commercial: Some(
            CommercialTerms::seal(other.version_id, CommercialPolicy::Subscription)
                .expect("terms seal"),
        ),
        scope: PublicationScope::Public,
    });
    assert!(matches!(
        misbound,
        Err(WorkflowDistributionError::InvalidCommercialTerms { .. })
    ));

    // Scope transitions change only the entry's state: the sealed
    // version and its metadata are untouched, and installs that
    // already pinned the release keep a valid record.
    let before = market
        .fetch_version(&version.identity.workflow, &version.version_id)
        .expect("version fetches")
        .expect("release published");
    market
        .apply_transition(
            &version.identity.workflow,
            &version.version_id,
            DistributionTransition::MakePrivate,
        )
        .expect("transition applies");
    market
        .apply_transition(
            &version.identity.workflow,
            &version.version_id,
            DistributionTransition::Share {
                audience: vec![principal("bob")],
            },
        )
        .expect("transition applies");
    let after = market
        .fetch_version(&version.identity.workflow, &version.version_id)
        .expect("version fetches")
        .expect("release published");
    assert_eq!(before, after);
    assert_eq!(state_of(&market, &version), DistributionState::Shared);

    // From shared, the release can be listed publicly, and making it
    // public again afterwards is a refused no-op: scope changes are
    // observable.
    market
        .apply_transition(
            &version.identity.workflow,
            &version.version_id,
            DistributionTransition::MakePublic,
        )
        .expect("transition applies");
    let no_op = market.apply_transition(
        &version.identity.workflow,
        &version.version_id,
        DistributionTransition::MakePublic,
    );
    assert!(matches!(
        no_op,
        Err(WorkflowDistributionError::IllegalTransition { .. })
    ));
}

#[test]
fn forking_derives_new_immutable_versions_with_lineage_provenance() {
    let (mut market, _, _) = plain_marketplace();
    let upstream = sealed_version(
        "web-research",
        ACME_REPO,
        10,
        (1, 0, 0),
        &["navigate_web"],
        &["browser_profile"],
    );
    let upstream_metadata = publication_metadata(
        &upstream,
        "acme-ops",
        "MIT",
        &["browser-automation"],
        (0, 3, 0),
        UpgradePolicySetting::Follow,
    );
    publish(
        &mut market,
        upstream.clone(),
        upstream_metadata,
        PublicationScope::Public,
    );

    let fork = market
        .fork(ForkRequest {
            upstream: PublishedVersionRef::of(&upstream),
            fork_repository: WorkflowRepositoryId::parse(ALICE_FORK_REPO).expect("fork repo"),
            fork_revision: ImmutableSourceRevision::pin_commit(sha(11)),
            fork_version: SemanticVersion::new(1, 0, 0),
            owner: principal("alice"),
            licensing: LicenseTerms {
                identifier: "MIT".to_owned(),
                url: None,
                custom_terms_digest: None,
            },
            carried_attribution: vec![Attribution {
                name: "acme-ops".to_owned(),
                contact: None,
            }],
            upgrade_policy: UpgradePolicySetting::Pin,
        })
        .expect("fork succeeds");

    // The fork is a NEW immutable version: different identity (its
    // repository and revision are part of the execution tuple),
    // identical frozen semantics (same definition digest).
    assert_ne!(fork.metadata.release.version_id, upstream.version_id);
    assert_eq!(
        fork.metadata.release.identity.definition_digest,
        upstream.identity.definition_digest
    );
    assert_eq!(fork.state, DistributionState::Forked);

    // Lineage provenance points back at the upstream immutable
    // version in the metadata document...
    assert_eq!(
        fork.metadata.provenance.forked_from.as_ref(),
        Some(&PublishedVersionRef::of(&upstream))
    );
    assert_eq!(
        fork.metadata.provenance.carried_attribution,
        vec![Attribution {
            name: "acme-ops".to_owned(),
            contact: None,
        }]
    );
    // ...and in the sealed version's own frozen provenance.
    let fork_record = market
        .fetch_version(
            &upstream.identity.workflow,
            &fork.metadata.release.version_id,
        )
        .expect("fork record fetches")
        .expect("fork published");
    let sealed_lineage = fork_record
        .provenance
        .as_ref()
        .expect("fork carries sealed provenance")
        .forked_from
        .as_ref()
        .expect("sealed fork lineage");
    assert_eq!(
        sealed_lineage.upstream,
        WorkflowRepositoryId::parse(ACME_REPO).expect("upstream repo")
    );
    assert_eq!(
        sealed_lineage.forked_at.as_ref(),
        Some(&upstream.identity.source_revision.commit_sha)
    );
    assert_eq!(
        fork_record
            .provenance
            .as_ref()
            .map(|record| record.authors.clone()),
        Some(vec![
            Attribution {
                name: "acme-ops".to_owned(),
                contact: None
            },
            Attribution {
                name: "alice".to_owned(),
                contact: None
            },
        ])
    );

    // The upstream release is untouched: same record, same listing.
    let upstream_after = market
        .fetch_version(&upstream.identity.workflow, &upstream.version_id)
        .expect("upstream fetches")
        .expect("upstream published");
    assert_eq!(upstream_after, upstream);
    assert_eq!(state_of(&market, &upstream), DistributionState::Public);

    // Forking into the same repository and forking without carried
    // attribution are refused.
    let same_repo = market.fork(ForkRequest {
        upstream: PublishedVersionRef::of(&upstream),
        fork_repository: WorkflowRepositoryId::parse(ACME_REPO).expect("same repo"),
        fork_revision: ImmutableSourceRevision::pin_commit(sha(12)),
        fork_version: SemanticVersion::new(1, 1, 0),
        owner: principal("mallory"),
        licensing: LicenseTerms {
            identifier: "MIT".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        carried_attribution: vec![Attribution {
            name: "acme-ops".to_owned(),
            contact: None,
        }],
        upgrade_policy: UpgradePolicySetting::Pin,
    });
    assert!(matches!(
        same_repo,
        Err(WorkflowDistributionError::InvalidRecord { .. })
    ));
    let uncredited = market.fork(ForkRequest {
        upstream: PublishedVersionRef::of(&upstream),
        fork_repository: WorkflowRepositoryId::parse("https://github.com/mallory/ops-workflows")
            .expect("fork repo"),
        fork_revision: ImmutableSourceRevision::pin_commit(sha(13)),
        fork_version: SemanticVersion::new(1, 1, 0),
        owner: principal("mallory"),
        licensing: LicenseTerms {
            identifier: "MIT".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        carried_attribution: Vec::new(),
        upgrade_policy: UpgradePolicySetting::Pin,
    });
    assert!(matches!(
        uncredited,
        Err(WorkflowDistributionError::InvalidRecord { .. })
    ));

    // The provenance search facet finds the fork, not its upstream.
    market
        .apply_transition(
            &fork.metadata.release.identity.workflow,
            &fork.metadata.release.version_id,
            DistributionTransition::MakePublic,
        )
        .expect("fork lists publicly");
    let forks_of_upstream = result_releases(
        &market,
        &MarketplaceQuery {
            forked_from: Some(upstream.version_id),
            ..MarketplaceQuery::default()
        },
    );
    assert_eq!(
        forks_of_upstream,
        vec![fork.metadata.release.version_id.to_string()]
    );
}

#[test]
fn installs_are_license_gated_with_deny_and_allow_paths() {
    // Allow path: a principal with a seeded license acceptance
    // installs a public release and pins its immutable version.
    let allowed_version = sealed_version(
        "triage-report",
        ACME_REPO,
        20,
        (1, 0, 0),
        &["send_email"],
        &["slack_workspace"],
    );
    let (mut market, _, _) = marketplace_with_access("alice", &allowed_version);
    let metadata = publication_metadata(
        &allowed_version,
        "acme-ops",
        "LicenseRef-Proprietary",
        &["document-processing"],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    publish(
        &mut market,
        allowed_version.clone(),
        metadata,
        PublicationScope::Public,
    );

    let install = market
        .install(MarketplaceInstallRequest {
            workflow: allowed_version.identity.workflow.clone(),
            version: allowed_version.version_id.clone(),
            principal: principal("alice"),
            at_unix_ms: 1_000,
        })
        .expect("licensed install succeeds");
    assert_eq!(install.workflow, allowed_version.identity.workflow);
    assert_eq!(install.installed.version_id, allowed_version.version_id);
    assert_eq!(install.version, allowed_version);
    assert!(install.access.allowed);
    assert_eq!(install.access.source, "host-license-acceptance");
    assert_eq!(install.entitlement, None);

    // Re-installing is refused: installs are pinned and changes flow
    // through the explicit upgrade path.
    let reinstall = market.install(MarketplaceInstallRequest {
        workflow: allowed_version.identity.workflow.clone(),
        version: allowed_version.version_id,
        principal: principal("alice"),
        at_unix_ms: 2_000,
    });
    assert!(matches!(
        reinstall,
        Err(WorkflowDistributionError::AlreadyInstalled { .. })
    ));

    // Deny path (a different workflow, since installs are per
    // workflow): default-deny refuses the install, carries the
    // decision data, and records nothing.
    let (mut market, access, _) = plain_marketplace();
    let gated_version = sealed_version(
        "release-notes",
        ACME_REPO,
        21,
        (1, 0, 0),
        &["send_email"],
        &[],
    );
    let metadata = publication_metadata(
        &gated_version,
        "acme-ops",
        "LicenseRef-Proprietary",
        &["document-processing"],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    publish(
        &mut market,
        gated_version.clone(),
        metadata,
        PublicationScope::Public,
    );

    let denied = market.install(MarketplaceInstallRequest {
        workflow: gated_version.identity.workflow.clone(),
        version: gated_version.version_id.clone(),
        principal: principal("bob"),
        at_unix_ms: 3_000,
    });
    let WorkflowDistributionError::InstallRefused {
        workflow: _,
        access: refusal,
        entitlement: refusal_entitlement,
    } = denied.expect_err("default-deny refuses the install")
    else {
        panic!("expected an install refusal");
    };
    assert!(!refusal.allowed);
    assert_eq!(refusal.source, "in-memory-access-policy");
    assert!(refusal.reason.is_some());
    assert_eq!(refusal_entitlement, None);

    // The refusal recorded no state and mutated nothing.
    assert_eq!(
        market
            .installed(&gated_version.identity.workflow)
            .expect("installed lookup"),
        None
    );
    gated_version.verify_integrity().expect("version untouched");
    let seen = access.requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].principal, principal("bob"));

    // Scope gates: private and fresh-fork releases are not
    // addressable by other principals; unlisted releases are
    // addressable by identity but never discoverable.
    let (mut market, _, _) = plain_marketplace();
    let private_version = sealed_version("secure-signoff", ACME_REPO, 22, (1, 0, 0), &[], &[]);
    let unlisted_version = sealed_version("beta-pipeline", ACME_REPO, 23, (1, 0, 0), &[], &[]);
    let private_metadata = publication_metadata(
        &private_version,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    let unlisted_metadata = publication_metadata(
        &unlisted_version,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    publish(
        &mut market,
        private_version.clone(),
        private_metadata,
        PublicationScope::Private,
    );
    publish(
        &mut market,
        unlisted_version,
        unlisted_metadata,
        PublicationScope::Unlisted,
    );

    let invisible = market.install(MarketplaceInstallRequest {
        workflow: private_version.identity.workflow.clone(),
        version: private_version.version_id,
        principal: principal("bob"),
        at_unix_ms: 4_000,
    });
    assert!(matches!(
        invisible,
        Err(WorkflowDistributionError::ReleaseNotVisible { .. })
    ));
    let anonymous = result_releases(&market, &MarketplaceQuery::default());
    assert_eq!(anonymous, Vec::<String>::new());
}

#[test]
fn entitlement_denial_gates_installation_only_and_never_mutates_semantics() {
    let (mut market, mut access, mut entitlements) = plain_marketplace();
    let version = sealed_version(
        "paid-triage",
        ACME_REPO,
        30,
        (1, 0, 0),
        &["send_email"],
        &[],
    );
    let metadata = publication_metadata(
        &version,
        "acme-ops",
        "LicenseRef-Proprietary",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    let terms = CommercialTerms::seal(version.version_id.clone(), CommercialPolicy::PaidRelease)
        .expect("terms seal");
    access.allow("bob", &version.version_id, "host-license-acceptance");
    market
        .publish(PublishSubmission {
            version: version.clone(),
            metadata,
            commercial: Some(terms),
            scope: PublicationScope::Public,
        })
        .expect("paid release publishes");

    // Denial: bob holds the license acceptance but no entitlement
    // grant. The refusal carries the entitlement decision as data.
    let denied = market.install(MarketplaceInstallRequest {
        workflow: version.identity.workflow.clone(),
        version: version.version_id.clone(),
        principal: principal("bob"),
        at_unix_ms: 5_000,
    });
    let WorkflowDistributionError::InstallRefused {
        workflow: _,
        access: license_gate,
        entitlement: entitlement_gate,
    } = denied.expect_err("entitlement denial refuses the install")
    else {
        panic!("expected an install refusal");
    };
    assert!(license_gate.allowed, "the license gate passed");
    let entitlement = entitlement_gate.expect("the entitlement gate ran");
    assert!(!entitlement.entitled);
    assert_eq!(entitlement.source, "in-memory-entitlements");
    assert!(entitlement.reason.is_some());

    // The denial changed no executable semantics: the sealed record,
    // its listing, and the install registry are all untouched.
    let record_before = market
        .fetch_version(&version.identity.workflow, &version.version_id)
        .expect("version fetches")
        .expect("release published");
    assert_eq!(record_before, version);
    assert_eq!(
        market
            .installed(&version.identity.workflow)
            .expect("installed lookup"),
        None
    );

    // Grant the entitlement (host-owned store) and install again: the
    // record carries the decision, and the sealed version is still
    // byte-identical.
    entitlements.seed_grant(principal("bob"), version.version_id.clone());
    let install = market
        .install(MarketplaceInstallRequest {
            workflow: version.identity.workflow.clone(),
            version: version.version_id.clone(),
            principal: principal("bob"),
            at_unix_ms: 6_000,
        })
        .expect("entitled install succeeds");
    let decision = install.entitlement.expect("entitlement recorded");
    assert!(decision.entitled);
    assert!(install.access.allowed);
    let record_after = market
        .fetch_version(&version.identity.workflow, &version.version_id)
        .expect("version fetches")
        .expect("release published");
    assert_eq!(record_after, version);

    // Entitlement gates also guard upgrades: a follow-policy
    // installation is offered a new paid release the installer is not
    // yet entitled to; approving it is refused with the decision data
    // and the pin stays.
    let upgrade_version = sealed_version(
        "paid-triage",
        ACME_REPO,
        31,
        (2, 0, 0),
        &["send_email"],
        &[],
    );
    let upgrade_metadata = publication_metadata(
        &upgrade_version,
        "acme-ops",
        "LicenseRef-Proprietary",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    access.allow(
        "bob",
        &upgrade_version.version_id,
        "host-license-acceptance",
    );
    market
        .publish(PublishSubmission {
            version: upgrade_version.clone(),
            metadata: upgrade_metadata,
            commercial: Some(
                CommercialTerms::seal(
                    upgrade_version.version_id.clone(),
                    CommercialPolicy::PaidRelease,
                )
                .expect("terms seal"),
            ),
            scope: PublicationScope::Public,
        })
        .expect("upgrade release publishes");
    let proposal = market
        .evaluate_upgrade(&version.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces a proposal");
    assert_eq!(proposal.from, version.version_id);
    assert_eq!(proposal.to.version_id, upgrade_version.version_id);
    let refused_upgrade = market.decide_upgrade(&proposal, UpgradeDecision::Approve, 7_000);
    let WorkflowDistributionError::UpgradeRefused { .. } =
        refused_upgrade.expect_err("unentitled upgrades are refused")
    else {
        panic!("expected an upgrade refusal");
    };
    let pinned = market
        .installed(&version.identity.workflow)
        .expect("installed lookup")
        .expect("still installed");
    assert_eq!(pinned.installed.version_id, version.version_id);
    assert!(market.upgrade_history().is_empty());
}

#[test]
fn pin_stays_pinned_and_follow_advances_only_with_explicit_approval() {
    // Pin policy: new releases never surface as proposals.
    let (mut market, mut access, _) = plain_marketplace();
    let pinned_version = sealed_version("stable-report", ACME_REPO, 40, (1, 0, 0), &[], &[]);
    access.allow(
        "alice",
        &pinned_version.version_id,
        "host-license-acceptance",
    );
    let metadata = publication_metadata(
        &pinned_version,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    publish(
        &mut market,
        pinned_version.clone(),
        metadata,
        PublicationScope::Public,
    );
    market
        .install(MarketplaceInstallRequest {
            workflow: pinned_version.identity.workflow.clone(),
            version: pinned_version.version_id.clone(),
            principal: principal("alice"),
            at_unix_ms: 8_000,
        })
        .expect("install succeeds");
    let newer = sealed_version("stable-report", ACME_REPO, 41, (1, 1, 0), &[], &[]);
    let newer_metadata = publication_metadata(
        &newer,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    publish(&mut market, newer, newer_metadata, PublicationScope::Public);
    assert_eq!(
        market
            .evaluate_upgrade(&pinned_version.identity.workflow)
            .expect("upgrade evaluation"),
        None,
        "pins never surface proposals"
    );
    let still_pinned = market
        .installed(&pinned_version.identity.workflow)
        .expect("installed lookup")
        .expect("installed");
    assert_eq!(still_pinned.installed.version_id, pinned_version.version_id);

    // Follow policy: proposal -> reject keeps the pin and is
    // recorded; proposal -> approve advances it explicitly.
    let (mut market, mut access, _) = plain_marketplace();
    let first = sealed_version("living-report", ACME_REPO, 42, (1, 0, 0), &[], &[]);
    let second = sealed_version("living-report", ACME_REPO, 43, (1, 1, 0), &[], &[]);
    access.allow("alice", &first.version_id, "host-license-acceptance");
    access.allow("alice", &second.version_id, "host-license-acceptance");
    let first_metadata = publication_metadata(
        &first,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    let second_metadata = publication_metadata(
        &second,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    publish(
        &mut market,
        first.clone(),
        first_metadata,
        PublicationScope::Public,
    );
    publish(
        &mut market,
        second.clone(),
        second_metadata,
        PublicationScope::Public,
    );
    market
        .install(MarketplaceInstallRequest {
            workflow: first.identity.workflow.clone(),
            version: first.version_id.clone(),
            principal: principal("alice"),
            at_unix_ms: 9_000,
        })
        .expect("install succeeds");

    let proposal = market
        .evaluate_upgrade(&first.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces a proposal");
    assert_eq!(proposal.from, first.version_id);
    assert_eq!(proposal.to.version_id, second.version_id);
    assert_eq!(proposal.policy, UpgradePolicySetting::Follow);

    // Surfacing a proposal changed nothing.
    assert_eq!(
        market
            .installed(&first.identity.workflow)
            .expect("installed lookup")
            .expect("installed")
            .installed
            .version_id,
        first.version_id
    );

    // Rejection keeps the pin and is still recorded.
    let rejected = market
        .decide_upgrade(&proposal, UpgradeDecision::Reject, 10_000)
        .expect("rejection records");
    assert!(!rejected.applied);
    assert_eq!(rejected.to, first.version_id);
    assert_eq!(
        market
            .installed(&first.identity.workflow)
            .expect("installed lookup")
            .expect("installed")
            .installed
            .version_id,
        first.version_id
    );

    // Approval advances the pin; the previously pinned version stays
    // a valid, fetchable, immutable record.
    let approved = market
        .decide_upgrade(&proposal, UpgradeDecision::Approve, 11_000)
        .expect("approval applies");
    assert!(approved.applied);
    assert_eq!(approved.from, first.version_id);
    assert_eq!(approved.to, second.version_id);
    let advanced = market
        .installed(&first.identity.workflow)
        .expect("installed lookup")
        .expect("installed");
    assert_eq!(advanced.installed.version_id, second.version_id);
    assert_eq!(advanced.version, second);
    first.verify_integrity().expect("old version still valid");
    assert_eq!(
        market
            .fetch_version(&first.identity.workflow, &first.version_id)
            .expect("version fetches")
            .expect("old release published"),
        first
    );
    assert_eq!(market.upgrade_history().len(), 2);
    assert!(!market.upgrade_history()[0].applied);
    assert!(market.upgrade_history()[1].applied);

    // The decided proposal is stale: deciding it again is refused.
    let stale = market.decide_upgrade(&proposal, UpgradeDecision::Approve, 12_000);
    assert!(matches!(
        stale,
        Err(WorkflowDistributionError::StaleUpgradeProposal { .. })
    ));

    // No other releases: follow has nothing to surface.
    let (mut market, _, _) = plain_marketplace();
    let lone = sealed_version("lone-report", ACME_REPO, 44, (1, 0, 0), &[], &[]);
    let metadata = publication_metadata(
        &lone,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    publish(
        &mut market,
        lone.clone(),
        metadata,
        PublicationScope::Public,
    );
    assert!(matches!(
        market.evaluate_upgrade(&lone.identity.workflow),
        Err(WorkflowDistributionError::NotInstalled { .. })
    ));
}

#[test]
fn search_filters_by_every_facet_and_visibility_scope() {
    let (mut market, _, _) = plain_marketplace();

    // Two public releases with distinct facets, one shared, one
    // private, one unlisted, and a public fork.
    let invoice = sealed_version(
        "invoice-triage",
        ACME_REPO,
        50,
        (1, 0, 0),
        &["send_email"],
        &["slack_workspace"],
    );
    let web = sealed_version(
        "web-research",
        ACME_REPO,
        51,
        (1, 0, 0),
        &["navigate_web"],
        &["browser_profile"],
    );
    let team = sealed_version(
        "team-report",
        ACME_REPO,
        52,
        (1, 0, 0),
        &["send_email"],
        &[],
    );
    let secure = sealed_version("secure-signoff", ACME_REPO, 53, (1, 0, 0), &[], &[]);
    let beta = sealed_version("beta-pipeline", ACME_REPO, 54, (1, 0, 0), &[], &[]);
    publish(
        &mut market,
        invoice.clone(),
        publication_metadata(
            &invoice,
            "acme-ops",
            "Apache-2.0",
            &["document-processing"],
            (0, 2, 0),
            UpgradePolicySetting::Pin,
        ),
        PublicationScope::Public,
    );
    publish(
        &mut market,
        web.clone(),
        publication_metadata(
            &web,
            "acme-ops",
            "MIT",
            &["browser-automation"],
            (0, 3, 0),
            UpgradePolicySetting::Follow,
        ),
        PublicationScope::Public,
    );
    publish(
        &mut market,
        team.clone(),
        publication_metadata(
            &team,
            "acme-ops",
            "Apache-2.0",
            &["document-processing"],
            (0, 2, 0),
            UpgradePolicySetting::Pin,
        ),
        PublicationScope::Shared {
            audience: vec![principal("bob")],
        },
    );
    publish(
        &mut market,
        secure.clone(),
        publication_metadata(
            &secure,
            "acme-ops",
            "LicenseRef-Proprietary",
            &[],
            (0, 2, 0),
            UpgradePolicySetting::Pin,
        ),
        PublicationScope::Private,
    );
    publish(
        &mut market,
        beta.clone(),
        publication_metadata(
            &beta,
            "acme-ops",
            "Apache-2.0",
            &[],
            (0, 2, 0),
            UpgradePolicySetting::Pin,
        ),
        PublicationScope::Unlisted,
    );
    let fork = market
        .fork(ForkRequest {
            upstream: PublishedVersionRef::of(&web),
            fork_repository: WorkflowRepositoryId::parse(ALICE_FORK_REPO).expect("fork repo"),
            fork_revision: ImmutableSourceRevision::pin_commit(sha(55)),
            fork_version: SemanticVersion::new(1, 0, 0),
            owner: principal("alice"),
            licensing: LicenseTerms {
                identifier: "MIT".to_owned(),
                url: None,
                custom_terms_digest: None,
            },
            carried_attribution: vec![Attribution {
                name: "acme-ops".to_owned(),
                contact: None,
            }],
            upgrade_policy: UpgradePolicySetting::Pin,
        })
        .expect("fork succeeds");
    market
        .apply_transition(
            &fork.metadata.release.identity.workflow,
            &fork.metadata.release.version_id,
            DistributionTransition::MakePublic,
        )
        .expect("fork lists publicly");
    let fork_release = fork.metadata.release.version_id.to_string();
    let invoice_release = invoice.version_id.to_string();
    let web_release = web.version_id.to_string();
    let team_release = team.version_id.to_string();

    // Capability facet.
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                capability: Some(CapabilityId::parse("navigate_web").expect("capability")),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone(), web_release.clone()]
    );
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                capability: Some(CapabilityId::parse("send_email").expect("capability")),
                ..MarketplaceQuery::default()
            }
        ),
        vec![invoice_release.clone()]
    );

    // Capability-class facet (a marketplace taxonomy, not an
    // execution binding).
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                capability_class: Some(
                    codex_workflow_distribution::CapabilityClass::parse("browser-automation")
                        .expect("class")
                ),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone(), web_release.clone()]
    );

    // Resource facet.
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                resource: Some(ResourceTypeId::parse("browser_profile").expect("resource")),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone(), web_release.clone()]
    );

    // License facet (exact SPDX-style identifier).
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                license: Some("Apache-2.0".to_owned()),
                ..MarketplaceQuery::default()
            }
        ),
        vec![invoice_release.clone()]
    );
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                license: Some("MIT".to_owned()),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone(), web_release.clone()]
    );

    // Provenance facet: direct forks of the web-research release.
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                forked_from: Some(web.version_id),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone()]
    );

    // Compatibility facet: releases whose minimum runtime is at most
    // the querying runtime. web-research and its fork need 0.3.0.
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                minimum_runtime: Some(SemanticVersion::new(0, 2, 0)),
                ..MarketplaceQuery::default()
            }
        ),
        vec![invoice_release.clone()]
    );
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                minimum_runtime: Some(SemanticVersion::new(0, 3, 0)),
                ..MarketplaceQuery::default()
            }
        )
        .len(),
        3
    );

    // Workflow and name-fragment facets.
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                workflow: Some(WorkflowDefinitionId::parse("web-research").expect("workflow")),
                ..MarketplaceQuery::default()
            }
        ),
        vec![fork_release.clone(), web_release.clone()]
    );
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                name_contains: Some("INVOICE".to_owned()),
                ..MarketplaceQuery::default()
            }
        ),
        vec![invoice_release.clone()]
    );

    // Visibility scopes: anonymous searches see public releases only.
    let mut expected_public = vec![
        fork_release.clone(),
        invoice_release.clone(),
        web_release.clone(),
    ];
    expected_public.sort();
    assert_eq!(
        result_releases(&market, &MarketplaceQuery::default()),
        expected_public
    );

    // bob sees the public releases plus the release shared with him.
    let mut expected_bob = vec![
        fork_release.clone(),
        invoice_release,
        team_release,
        web_release,
    ];
    expected_bob.sort();
    assert_eq!(
        result_releases(
            &market,
            &MarketplaceQuery {
                requesting: Some(principal("bob")),
                ..MarketplaceQuery::default()
            }
        ),
        expected_bob
    );

    // The owner sees every entry regardless of scope; alice sees her
    // fork plus the public releases.
    let acme_seen = result_releases(
        &market,
        &MarketplaceQuery {
            requesting: Some(principal("acme-ops")),
            ..MarketplaceQuery::default()
        },
    );
    assert_eq!(acme_seen.len(), 6);
    let alice_seen = result_releases(
        &market,
        &MarketplaceQuery {
            requesting: Some(principal("alice")),
            ..MarketplaceQuery::default()
        },
    );
    assert_eq!(alice_seen.len(), 3);
    assert!(alice_seen.contains(&fork_release));
}

#[test]
fn no_credentials_appear_in_any_distribution_record() {
    // Repository identity is credential-free by construction: URLs
    // with embedded userinfo canonicalize to their bare form.
    assert_eq!(
        WorkflowRepositoryId::parse("https://alice:secret@github.com/acme/ops-workflows")
            .expect("repository canonicalizes")
            .as_str(),
        "github.com/acme/ops-workflows"
    );

    // Principals reject every credential-shaped label.
    for credential_shape in [
        "https://alice:secret@github.com",
        "alice:secret",
        "github.com/alice",
        "has space",
        "",
    ] {
        assert!(
            MarketplacePrincipal::parse(credential_shape).is_err(),
            "`{credential_shape}` must never become a principal"
        );
    }

    // Build a full scenario and serialize every record family; no
    // credential-shaped key or value may appear anywhere.
    let (mut market, mut access, mut entitlements) = plain_marketplace();
    let version = sealed_version(
        "no-secrets-report",
        ACME_REPO,
        60,
        (1, 0, 0),
        &["send_email"],
        &[],
    );
    access.allow("alice", &version.version_id, "host-license-acceptance");
    entitlements.seed_grant(principal("alice"), version.version_id.clone());
    let metadata = publication_metadata(
        &version,
        "acme-ops",
        "LicenseRef-Proprietary",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Follow,
    );
    let listing = market
        .publish(PublishSubmission {
            version: version.clone(),
            metadata,
            commercial: Some(
                CommercialTerms::seal(version.version_id.clone(), CommercialPolicy::Subscription)
                    .expect("terms seal"),
            ),
            scope: PublicationScope::Public,
        })
        .expect("publication succeeds");
    let install = market
        .install(MarketplaceInstallRequest {
            workflow: version.identity.workflow.clone(),
            version: version.version_id.clone(),
            principal: principal("alice"),
            at_unix_ms: 13_000,
        })
        .expect("install succeeds");

    let records: Vec<(&str, Value)> = vec![
        (
            "listing",
            serde_json::to_value(&listing).expect("listing serializes"),
        ),
        (
            "install",
            serde_json::to_value(&install).expect("install serializes"),
        ),
        (
            "upgrade-proposal",
            serde_json::to_value(
                market
                    .evaluate_upgrade(&version.identity.workflow)
                    .expect("upgrade evaluation"),
            )
            .expect("proposal serializes"),
        ),
        (
            "access-request",
            serde_json::to_value(&access.requests()[0]).expect("request serializes"),
        ),
        (
            "entitlement-decision",
            serde_json::to_value(install.entitlement.as_ref().expect("decision recorded"))
                .expect("decision serializes"),
        ),
    ];
    for (name, record) in records {
        assert_no_credentials(name, &record);
    }
}

/// Walks a serialized record asserting that no key is credential-
/// shaped and no string value carries URL userinfo.
fn assert_no_credentials(name: &str, value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let lower = key.to_lowercase();
                for forbidden in [
                    "credential",
                    "secret",
                    "token",
                    "password",
                    "passwd",
                    "apikey",
                    "api_key",
                    "privatekey",
                    "private_key",
                ] {
                    assert!(
                        !lower.contains(forbidden),
                        "`{name}` record has a credential-shaped key `{key}`"
                    );
                }
                assert_no_credentials(name, inner);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_credentials(name, item);
            }
        }
        Value::String(text) => {
            if let Some(rest) = text.split_once("://") {
                assert!(
                    !rest.1.contains('@')
                        || !rest
                            .1
                            .split('@')
                            .next()
                            .is_some_and(|user| user.contains(':')),
                    "`{name}` record value `{text}` carries URL userinfo"
                );
            }
        }
        _ => {}
    }
}

#[test]
fn marketplace_installs_hand_off_to_the_trigger_plane_with_the_same_identity() {
    use codex_workflow_app::InMemoryInstanceStore;
    use codex_workflow_app::InMemoryVersionStore;
    use codex_workflow_forge::InstallRegistry;
    use codex_workflow_triggers::FixedClock;
    use codex_workflow_triggers::InMemoryInstallationStore;
    use codex_workflow_triggers::InMemoryInstanceControl;
    use codex_workflow_triggers::InMemoryPackageCatalog;
    use codex_workflow_triggers::InMemoryResourceAuthorizer;
    use codex_workflow_triggers::InMemoryTriggerLedger;
    use codex_workflow_triggers::InstallRequest;
    use codex_workflow_triggers::TriggerDeps;
    use codex_workflow_triggers::WorkflowTriggerPlane;

    // Publish and marketplace-install a release.
    let (mut market, mut access, _) = plain_marketplace();
    let version = sealed_version("triage-report", ACME_REPO, 70, (1, 0, 0), &[], &[]);
    access.allow("alice", &version.version_id, "host-license-acceptance");
    let metadata = publication_metadata(
        &version,
        "acme-ops",
        "Apache-2.0",
        &[],
        (0, 2, 0),
        UpgradePolicySetting::Pin,
    );
    publish(
        &mut market,
        version.clone(),
        metadata,
        PublicationScope::Public,
    );
    let install = market
        .install(MarketplaceInstallRequest {
            workflow: version.identity.workflow.clone(),
            version: version.version_id.clone(),
            principal: principal("alice"),
            at_unix_ms: 14_000,
        })
        .expect("marketplace install succeeds");

    // The distribution install did not touch the forge install
    // registry: operational pinning belongs to the trigger plane.
    let mut trigger_plane = WorkflowTriggerPlane::new(TriggerDeps {
        ledger: Box::new(InMemoryTriggerLedger::new()),
        catalog: Box::new(InMemoryPackageCatalog::new()),
        authorizer: Box::new(InMemoryResourceAuthorizer::new()),
        control: Box::new(InMemoryInstanceControl::new(InMemoryInstanceStore::new())),
        installations: Box::new(InMemoryInstallationStore::new()),
        clock: Box::new(FixedClock::new(0)),
        versions: Box::new(InMemoryVersionStore::new()),
        installs: InstallRegistry::new(),
    });

    // The hand-off: the marketplace install record carries the sealed
    // version the trigger plane's InstallRequest is built from.
    let request = InstallRequest::new(install.version.clone());
    let configuration = trigger_plane
        .install(request)
        .expect("trigger-plane installation succeeds");
    assert_eq!(configuration.workflow, version.identity.workflow);
    assert_eq!(configuration.version_id, version.version_id);
    assert_eq!(
        configuration.reference.version_id,
        install.installed.version_id
    );
    assert!(
        trigger_plane
            .install_registry()
            .is_installed(&version.identity.workflow)
    );
    assert_eq!(
        trigger_plane
            .install_registry()
            .installed(&version.identity.workflow)
            .expect("forge pin recorded")
            .installed
            .version_id,
        version.version_id
    );
}
