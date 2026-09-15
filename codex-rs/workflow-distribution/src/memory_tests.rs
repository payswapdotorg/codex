//! In-memory marketplace upgrade-path guard tests (RWO-009).
//!
//! Two closed findings drive these tests:
//!
//! - **Family I (VWO-009 F10)**: the upgrade path now runs the same
//!   visibility predicate `install` enforces — `evaluate_upgrade`
//!   filters candidates through `release_installable` with the
//!   install's owner as the requesting principal (a private release
//!   never surfaces to a foreign installer), and `decide_upgrade`
//!   re-runs the gate on the target before approval, in the
//!   documented order (visibility → integrity → access →
//!   entitlement).
//! - **Family G engine half (VWO-009 F4)**: an upgrade proposal whose
//!   target is older than the installed pin is refused with the typed
//!   `IllegalDowngrade` error and the pin never moves.
//!
//! Every refusal test deliberately seeds no access grant for the
//! refused target: the access gate would refuse with
//! `UpgradeRefused`, a different error, so asserting the visibility
//! or ordering error specifically proves the new guard fired.

use std::collections::BTreeMap;

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::IR_FORMAT_VERSION;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
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
use codex_workflow_contracts::WorkflowVersionId;
use codex_workflow_forge::PublishedVersionRef;
use pretty_assertions::assert_eq;

use crate::CompatibilitySpec;
use crate::DistributionPort;
use crate::InMemoryAccessPolicy;
use crate::InMemoryEntitlements;
use crate::InMemoryMarketplace;
use crate::LicenseTerms;
use crate::MarketplaceInstallRequest;
use crate::MarketplacePrincipal;
use crate::PublicationMetadata;
use crate::PublicationScope;
use crate::PublishSubmission;
use crate::SourceLineage;
use crate::UpgradeDecision;
use crate::UpgradePolicySetting;
use crate::UpgradeProposal;
use crate::WorkflowDistributionError;

const OWNER_REPO: &str = "https://github.com/acme/ops-workflows";

fn node_id(value: &str) -> IrNodeId {
    IrNodeId::parse(value).expect("node id")
}

fn sha(seed: u64) -> RevisionSha {
    RevisionSha::parse(format!("{seed:040x}")).expect("seeded SHA")
}

fn principal(value: &str) -> MarketplacePrincipal {
    MarketplacePrincipal::parse(value).expect("principal")
}

/// Seals a single-step workflow version with no capability or
/// resource declarations, directly through the frozen contract (the
/// same `WorkflowVersion::seal` primitive the e2e suite uses).
fn sealed_version(workflow: &str, commit: u64, version: (u64, u64, u64)) -> WorkflowVersion {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        node_id("step-001"),
        WorkflowIrNode::Step(StepNode {
            capabilities: Vec::new(),
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
        dependencies: WorkflowDependencies::default(),
    };
    WorkflowVersion::seal(
        definition,
        WorkflowRepositoryId::parse(OWNER_REPO).expect("repository"),
        ImmutableSourceRevision::pin_commit(sha(commit)),
        SemanticVersion::new(version.0, version.1, version.2),
        DependencyLock::default(),
        None,
    )
    .expect("sealed version")
}

/// Honest publication metadata for a no-requirement version, owned by
/// `owner`.
fn publication_metadata(
    version: &WorkflowVersion,
    owner: &str,
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
            identifier: "Apache-2.0".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage::default(),
        CompatibilitySpec {
            minimum_runtime: SemanticVersion::new(0, 0, 0),
            required_capabilities: Vec::new(),
            capability_classes: Vec::new(),
            required_resources: Vec::new(),
        },
        policy,
    )
    .expect("publication metadata")
}

/// Publishes a free follow-policy release under a scope.
fn publish(market: &mut InMemoryMarketplace, version: WorkflowVersion, scope: PublicationScope) {
    market
        .publish(PublishSubmission {
            metadata: publication_metadata(&version, "acme-ops", UpgradePolicySetting::Follow),
            version,
            commercial: None,
            scope,
        })
        .expect("publication succeeds");
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

/// Installs `version` as `installer` with an access grant seeded.
fn install(
    market: &mut InMemoryMarketplace,
    access: &mut InMemoryAccessPolicy,
    version: &WorkflowVersion,
    installer: &str,
    at_unix_ms: u64,
) {
    access.allow(installer, &version.version_id, "host-license-acceptance");
    market
        .install(MarketplaceInstallRequest {
            workflow: version.identity.workflow.clone(),
            version: version.version_id.clone(),
            principal: principal(installer),
            at_unix_ms,
        })
        .expect("install succeeds");
}

/// The version the workflow's pin currently names.
fn pinned(market: &InMemoryMarketplace, workflow: &WorkflowDefinitionId) -> WorkflowVersionId {
    market
        .installed(workflow)
        .expect("installed lookup")
        .expect("installed")
        .installed
        .version_id
        .clone()
}

#[test]
fn evaluate_upgrade_skips_releases_invisible_to_the_installer() {
    // Engine attack 6d, first half: a private release newer than
    // anything visible must never surface to the foreign installer
    // who pinned the workflow. Without the visibility filter the
    // newest candidate would be the private 3.0.0 and its identity
    // would leak; with it, the proposal targets the newest visible
    // release.
    let (mut market, mut access, _) = plain_marketplace();
    let base = sealed_version("guarded-report", 70, (1, 0, 0));
    let private_newest = sealed_version("guarded-report", 71, (3, 0, 0));
    let visible_newer = sealed_version("guarded-report", 72, (2, 0, 0));
    publish(&mut market, base.clone(), PublicationScope::Public);
    publish(&mut market, private_newest, PublicationScope::Private);
    publish(&mut market, visible_newer.clone(), PublicationScope::Public);
    install(&mut market, &mut access, &base, "bob", 21_000);

    let proposal = market
        .evaluate_upgrade(&base.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces the newest visible release");
    assert_eq!(proposal.to.version_id, visible_newer.version_id);
    assert_eq!(proposal.from, base.version_id);
    assert_eq!(
        pinned(&market, &base.identity.workflow),
        base.version_id,
        "surfacing a proposal changes nothing"
    );

    // Positive control: the visible newer target still approves —
    // the new gates do not over-block the legitimate path.
    access.allow("bob", &visible_newer.version_id, "host-license-acceptance");
    let applied = market
        .decide_upgrade(&proposal, UpgradeDecision::Approve, 22_000)
        .expect("visible newer targets approve");
    assert!(applied.applied);
    assert_eq!(applied.to, visible_newer.version_id);
    assert_eq!(
        pinned(&market, &base.identity.workflow),
        visible_newer.version_id
    );
}

#[test]
fn evaluate_upgrade_returns_none_when_all_other_releases_are_invisible() {
    // Engine attack 6d, only-private shape: the sole newer release is
    // private and foreign, so nothing surfaces at all.
    let (mut market, mut access, _) = plain_marketplace();
    let base = sealed_version("hidden-report", 73, (1, 0, 0));
    let private_newer = sealed_version("hidden-report", 74, (2, 0, 0));
    publish(&mut market, base.clone(), PublicationScope::Public);
    publish(&mut market, private_newer, PublicationScope::Private);
    install(&mut market, &mut access, &base, "bob", 23_000);
    assert_eq!(
        market
            .evaluate_upgrade(&base.identity.workflow)
            .expect("upgrade evaluation"),
        None,
        "a private foreign release never surfaces as a proposal"
    );
    assert_eq!(pinned(&market, &base.identity.workflow), base.version_id);
}

#[test]
fn decide_upgrade_refuses_smuggled_proposals_targeting_invisible_releases() {
    // Engine attack 6d, second half: a caller who bypasses
    // evaluate_upgrade and hand-builds a proposal naming a private
    // foreign release is refused with ReleaseNotVisible on the
    // approval path. No access grant is seeded for the private
    // release: if the visibility gate were missing, the access gate
    // would refuse with UpgradeRefused instead, so the asserted error
    // proves the visibility gate fired.
    let (mut market, mut access, _) = plain_marketplace();
    let base = sealed_version("smuggled-report", 75, (1, 0, 0));
    let private_newer = sealed_version("smuggled-report", 76, (2, 0, 0));
    publish(&mut market, base.clone(), PublicationScope::Public);
    publish(
        &mut market,
        private_newer.clone(),
        PublicationScope::Private,
    );
    install(&mut market, &mut access, &base, "bob", 24_000);
    let smuggled = UpgradeProposal {
        workflow: base.identity.workflow.clone(),
        from: base.version_id.clone(),
        to: PublishedVersionRef::of(&private_newer),
        policy: UpgradePolicySetting::Follow,
    };
    let refused = market.decide_upgrade(&smuggled, UpgradeDecision::Approve, 25_000);
    let WorkflowDistributionError::ReleaseNotVisible { workflow, version } =
        refused.expect_err("smuggled private targets are not visible")
    else {
        panic!("expected a visibility refusal");
    };
    assert_eq!(workflow, base.identity.workflow.to_string());
    assert_eq!(version, private_newer.version_id.to_string());
    assert_eq!(
        pinned(&market, &base.identity.workflow),
        base.version_id,
        "the pin is unchanged"
    );
    assert!(
        market.upgrade_history().is_empty(),
        "refused proposals record nothing"
    );
}

#[test]
fn decide_upgrade_refuses_targets_older_than_the_pin() {
    // Engine attack 12 (Family G engine half): the follow path
    // surfaced 1.5.0 from a 2.0.0 pin and an approved decision moved
    // the pin backward. Approval must now refuse with the typed
    // IllegalDowngrade error. No access grant is seeded for the older
    // release, so the asserted error proves the ordering guard fired
    // (the access gate would have said UpgradeRefused).
    let (mut market, mut access, _) = plain_marketplace();
    let pin = sealed_version("monotonic-report", 77, (2, 0, 0));
    let older = sealed_version("monotonic-report", 78, (1, 5, 0));
    publish(&mut market, older.clone(), PublicationScope::Public);
    publish(&mut market, pin.clone(), PublicationScope::Public);
    install(&mut market, &mut access, &pin, "alice", 26_000);

    // Bounded scope: evaluation still surfaces the older release as
    // pure data — the ordering guard lives on the decision path.
    let proposal = market
        .evaluate_upgrade(&pin.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces the only other release");
    assert_eq!(proposal.to.version_id, older.version_id);

    let refused = market.decide_upgrade(&proposal, UpgradeDecision::Approve, 27_000);
    let WorkflowDistributionError::IllegalDowngrade {
        workflow,
        expected_newer_than,
        got,
    } = refused.expect_err("older targets are refused as downgrades")
    else {
        panic!("expected an illegal-downgrade refusal");
    };
    assert_eq!(workflow, pin.identity.workflow.to_string());
    assert_eq!(expected_newer_than, pin.identity.semantic_version);
    assert_eq!(got, older.identity.semantic_version);
    assert_eq!(
        pinned(&market, &pin.identity.workflow),
        pin.version_id,
        "the pin is unchanged"
    );
    assert!(
        market.upgrade_history().is_empty(),
        "refused proposals record nothing"
    );
}

#[test]
fn rejected_downgrade_proposals_still_record_a_rejection() {
    // The ordering guard guards APPROVAL only: an explicit rejection
    // keeps the pin (as any rejection does) and is still recorded,
    // with the record's `to` equal to the pinned version.
    let (mut market, mut access, _) = plain_marketplace();
    let pin = sealed_version("declined-report", 79, (2, 0, 0));
    let older = sealed_version("declined-report", 80, (1, 5, 0));
    publish(&mut market, older, PublicationScope::Public);
    publish(&mut market, pin.clone(), PublicationScope::Public);
    install(&mut market, &mut access, &pin, "alice", 28_000);
    let proposal = market
        .evaluate_upgrade(&pin.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces the only other release");
    let rejected = market
        .decide_upgrade(&proposal, UpgradeDecision::Reject, 29_000)
        .expect("rejection records");
    assert!(!rejected.applied);
    assert_eq!(rejected.to, pin.version_id);
    assert_eq!(market.upgrade_history().len(), 1);
    assert!(!market.upgrade_history()[0].applied);
    assert_eq!(pinned(&market, &pin.identity.workflow), pin.version_id);
}

#[test]
fn equal_semver_targets_are_not_downgrades() {
    // The guard refuses strictly older targets only ("older than",
    // per the work order): a re-sealed release at the SAME semver
    // under a different immutable identity is not a downgrade and
    // proceeds through the gates like any upgrade.
    let (mut market, mut access, _) = plain_marketplace();
    let pin = sealed_version("sideways-report", 81, (1, 0, 0));
    let resealed = sealed_version("sideways-report", 82, (1, 0, 0));
    assert_ne!(pin.version_id, resealed.version_id);
    publish(&mut market, pin.clone(), PublicationScope::Public);
    publish(&mut market, resealed.clone(), PublicationScope::Public);
    install(&mut market, &mut access, &pin, "alice", 30_000);
    let proposal = market
        .evaluate_upgrade(&pin.identity.workflow)
        .expect("upgrade evaluation")
        .expect("follow surfaces the only other release");
    assert_eq!(proposal.to.version_id, resealed.version_id);
    access.allow("alice", &resealed.version_id, "host-license-acceptance");
    let applied = market
        .decide_upgrade(&proposal, UpgradeDecision::Approve, 31_000)
        .expect("equal-semver targets are not refused as downgrades");
    assert!(applied.applied);
    assert_eq!(pinned(&market, &pin.identity.workflow), resealed.version_id);
}
