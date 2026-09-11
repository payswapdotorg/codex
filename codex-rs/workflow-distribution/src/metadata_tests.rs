//! Publication metadata document tests: hash-chain integrity, binding
//! validation, and credential-free principal rules.

use codex_workflow_contracts::Attribution;
use codex_workflow_contracts::CapabilityId;
use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::ExecutionVersionIdentity;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::RevisionSha;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_forge::PublishedVersionRef;
use pretty_assertions::assert_eq;

use super::CapabilityClass;
use super::CompatibilitySpec;
use super::LicenseTerms;
use super::MarketplacePrincipal;
use super::PublicationMetadata;
use super::SourceLineage;
use super::UpgradePolicySetting;
use crate::WorkflowDistributionError;

fn release_ref(seed: u64) -> PublishedVersionRef {
    let identity = ExecutionVersionIdentity {
        workflow: WorkflowDefinitionId::parse("triage-report").unwrap(),
        semantic_version: SemanticVersion::new(1, 0, 0),
        repository: WorkflowRepositoryId::parse("https://github.com/acme/ops-workflows").unwrap(),
        source_revision: ImmutableSourceRevision::pin_commit(
            RevisionSha::parse(format!("{seed:040x}")).unwrap(),
        ),
        definition_digest: ContentDigest::of(&format!("definition:{seed}")).unwrap(),
        dependency_lock_digest: ContentDigest::of(&format!("lock:{seed}")).unwrap(),
    };
    let version_id = identity.version_id().unwrap();
    PublishedVersionRef {
        identity,
        version_id,
    }
}

fn sealed_document(release: PublishedVersionRef) -> PublicationMetadata {
    PublicationMetadata::seal(
        release,
        vec![Attribution {
            name: "acme-ops".to_owned(),
            contact: None,
        }],
        MarketplacePrincipal::parse("acme-ops").unwrap(),
        LicenseTerms {
            identifier: "Apache-2.0".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage::default(),
        CompatibilitySpec {
            minimum_runtime: SemanticVersion::new(0, 2, 0),
            required_capabilities: vec![CapabilityId::parse("navigate_web").unwrap()],
            capability_classes: vec![CapabilityClass::parse("browser-automation").unwrap()],
            required_resources: Vec::new(),
        },
        UpgradePolicySetting::Follow,
    )
    .unwrap()
}

#[test]
fn sealed_documents_verify() {
    let document = sealed_document(release_ref(1));
    assert!(document.verify().is_ok());
    // The digest is stable: sealing identical content twice yields the
    // same hash chain.
    let again = sealed_document(release_ref(1));
    assert_eq!(document, again);
}

#[test]
fn tampering_any_covered_field_breaks_the_hash_chain() {
    let release = release_ref(2);
    let mut document = sealed_document(release.clone());
    document.licensing.identifier = "MIT".to_owned();
    assert!(matches!(
        document.verify(),
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));

    let mut ownership = sealed_document(release.clone());
    ownership.ownership = MarketplacePrincipal::parse("mallory").unwrap();
    assert!(matches!(
        ownership.verify(),
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));

    let mut compatibility = sealed_document(release.clone());
    compatibility.compatibility.required_capabilities.clear();
    assert!(matches!(
        compatibility.verify(),
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));

    let mut policy = sealed_document(release);
    policy.upgrade_policy = UpgradePolicySetting::Pin;
    assert!(matches!(
        policy.verify(),
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));
}

#[test]
fn tampered_release_references_never_seal() {
    let mut tampered = release_ref(3);
    tampered.version_id = release_ref(4).version_id;
    assert!(matches!(
        PublicationMetadata::seal(
            tampered,
            Vec::new(),
            MarketplacePrincipal::parse("acme-ops").unwrap(),
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
            UpgradePolicySetting::Pin,
        ),
        Err(WorkflowDistributionError::Forge(
            codex_workflow_forge::WorkflowForgeError::VersionIdentityMismatch { .. }
        ))
    ));
}

#[test]
fn forks_must_carry_upstream_attribution() {
    let result = PublicationMetadata::seal(
        release_ref(5),
        Vec::new(),
        MarketplacePrincipal::parse("alice").unwrap(),
        LicenseTerms {
            identifier: "LicenseRef-Proprietary".to_owned(),
            url: None,
            custom_terms_digest: None,
        },
        SourceLineage {
            forked_from: Some(release_ref(4)),
            carried_attribution: Vec::new(),
        },
        CompatibilitySpec {
            minimum_runtime: SemanticVersion::new(0, 2, 0),
            required_capabilities: Vec::new(),
            capability_classes: Vec::new(),
            required_resources: Vec::new(),
        },
        UpgradePolicySetting::Pin,
    );
    assert!(matches!(
        result,
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));
}

#[test]
fn license_identifiers_must_be_non_empty() {
    let result = PublicationMetadata::seal(
        release_ref(6),
        Vec::new(),
        MarketplacePrincipal::parse("acme-ops").unwrap(),
        LicenseTerms {
            identifier: "  ".to_owned(),
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
        UpgradePolicySetting::Pin,
    );
    assert!(matches!(
        result,
        Err(WorkflowDistributionError::InvalidMetadata { .. })
    ));
}

#[test]
fn principals_are_credential_free_by_construction() {
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
    assert_eq!(
        MarketplacePrincipal::parse("alice.ops").unwrap().as_ref(),
        "alice.ops"
    );
}
