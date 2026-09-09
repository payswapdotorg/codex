//! Workflow packages: manifests and dependency locks with subworkflow
//! pins to immutable revisions.
//!
//! A workflow package is the installable unit of collaboration: the
//! workflows one repository declares, snapshotted at an immutable source
//! revision, plus a dependency lock that pins every subworkflow
//! dependency to immutable published content.
//!
//! Invariants enforced here:
//!
//! - Package manifests are read at an **immutable source revision** (the
//!   field type guarantees a full commit SHA; moving branches are
//!   development state only).
//! - Subworkflow pins anchor to immutable revisions; when a pin
//!   references a published version, the version must be part of the
//!   referenced release, be anchored at the release's target revision,
//!   and verify against its recomputed identity digest.
//! - Lock digests cover exactly the identity-bearing pin fields.
//!   Provenance-only fields (such as `resolved_from`) are excluded, so
//!   repinning identical executable content yields the identical lock
//!   digest — mirroring the frozen version identity semantics.
//!
//! Whether a candidate version *satisfies* a declared dependency pin
//! (ranges, compatibility rules) is a control-plane decision; this
//! module records and verifies the resolved, immutable pin.

use codex_workflow_contracts::ContentDigest;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::SubworkflowDependencyId;
use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowManifest;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRelease;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::collaboration::PublishedVersionRef;
use crate::WorkflowForgeError;

/// Current workflow package manifest format version.
pub const PACKAGE_MANIFEST_FORMAT_VERSION: u32 = 1;

/// Current workflow package lock format version.
pub const PACKAGE_LOCK_FORMAT_VERSION: u32 = 1;

/// A subworkflow dependency pinned to immutable published content.
///
/// A pin is either content-level (pinned to an immutable source revision,
/// [`SubworkflowPin::from_revision`]) or version-level (pinned to a
/// published version cut by a release, [`SubworkflowPin::from_release`]).
/// In both cases the execution anchor is the full commit SHA inside
/// `source_revision`; `resolved_from` inside it is provenance only.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubworkflowPin {
    /// The dependency being pinned, by its declared dependency id.
    pub dependency: SubworkflowDependencyId,
    /// Repository the dependency was resolved from.
    pub source: WorkflowRepositoryId,
    /// The immutable revision the dependency is pinned to.
    pub source_revision: ImmutableSourceRevision,
    /// The published version the pin resolved to, when the dependency is
    /// pinned to a released version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<PublishedVersionRef>,
}

impl SubworkflowPin {
    /// Pins a dependency to an immutable source revision without a
    /// release.
    ///
    /// The revision must already be resolved to a full commit SHA; the
    /// `resolved_from` provenance inside it is preserved unchanged.
    pub fn from_revision(
        dependency: SubworkflowDependencyId,
        source: WorkflowRepositoryId,
        revision: ImmutableSourceRevision,
    ) -> Result<Self, WorkflowForgeError> {
        Ok(Self {
            dependency,
            source,
            source_revision: revision,
            version: None,
        })
    }

    /// Pins a dependency to a published version of a release.
    ///
    /// Validation: the version must verify against its recomputed
    /// identity digest, must be listed by the release, and must be
    /// anchored exactly at the release's immutable target revision.
    /// The pin then copies the version's repository and source revision,
    /// so the pin and the version can never drift apart.
    pub fn from_release(
        dependency: SubworkflowDependencyId,
        release: &WorkflowRelease,
        version: &PublishedVersionRef,
    ) -> Result<Self, WorkflowForgeError> {
        version.verify()?;
        if !release.versions.iter().any(|id| id == &version.version_id) {
            return Err(WorkflowForgeError::VersionNotInRelease {
                dependency,
                version: version.version_id.clone(),
            });
        }
        if version.identity.source_revision.commit_sha != release.target.commit_sha {
            return Err(WorkflowForgeError::VersionNotAnchoredAtTarget {
                workflow: version.identity.workflow.clone(),
                expected: release.target.commit_sha.clone(),
                actual: version.identity.source_revision.commit_sha.clone(),
            });
        }
        Ok(Self {
            dependency,
            source: version.identity.repository.clone(),
            source_revision: version.identity.source_revision.clone(),
            version: Some(version.clone()),
        })
    }

    /// The workflow the pin resolves to, when pinned to a published
    /// version.
    pub fn workflow(&self) -> Option<&WorkflowDefinitionId> {
        self.version
            .as_ref()
            .map(|version| &version.identity.workflow)
    }

    /// The semantic version the pin resolves to, when pinned to a
    /// published version.
    pub fn semantic_version(&self) -> Option<&SemanticVersion> {
        self.version
            .as_ref()
            .map(|version| &version.identity.semantic_version)
    }

    /// The published version identity the pin resolves to, when pinned
    /// to a published version.
    pub fn version_id(&self) -> Option<&WorkflowVersionId> {
        self.version.as_ref().map(|version| &version.version_id)
    }

    /// Verifies the pin's internal consistency.
    ///
    /// A versioned pin must still verify its digest, and its recorded
    /// source repository and revision must match the version's identity
    /// tuple, so a pin cannot be silently repointed after sealing.
    pub fn verify(&self) -> Result<(), WorkflowForgeError> {
        if let Some(version) = &self.version {
            version.verify()?;
            if version.identity.repository != self.source {
                return Err(WorkflowForgeError::PinInconsistent {
                    dependency: self.dependency.clone(),
                    reason: "pin source does not match the pinned version repository",
                });
            }
            if version.identity.source_revision.commit_sha != self.source_revision.commit_sha {
                return Err(WorkflowForgeError::PinInconsistent {
                    dependency: self.dependency.clone(),
                    reason: "pin revision does not match the pinned version revision",
                });
            }
        }
        Ok(())
    }
}

/// The manifest of a workflow package: which workflows a repository
/// provides at one immutable source revision.
///
/// The package manifest wraps the repository's frozen contract
/// manifests. Every manifest must declare exactly this repository, and
/// the snapshot revision is an immutable commit SHA by type — packages
/// are never cut from moving branches.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowPackageManifest {
    /// Package manifest serialization format version.
    pub package_version: u32,
    /// Package display name.
    pub name: String,
    /// Repository the package is cut from.
    pub repository: WorkflowRepositoryId,
    /// Immutable revision the package snapshot was read at.
    pub source_revision: ImmutableSourceRevision,
    /// The workflow manifests included in the package.
    pub workflows: Vec<WorkflowManifest>,
    /// Publication provenance: attribution, license, upgrade policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<WorkflowProvenance>,
}

impl WorkflowPackageManifest {
    /// Creates and validates a package manifest.
    ///
    /// Validation: supported format version, non-empty name, at least
    /// one workflow, every manifest validates and declares exactly this
    /// repository.
    pub fn new(
        name: impl Into<String>,
        repository: WorkflowRepositoryId,
        source_revision: ImmutableSourceRevision,
        workflows: Vec<WorkflowManifest>,
        provenance: Option<WorkflowProvenance>,
    ) -> Result<Self, WorkflowForgeError> {
        let manifest = Self {
            package_version: PACKAGE_MANIFEST_FORMAT_VERSION,
            name: name.into(),
            repository,
            source_revision,
            workflows,
            provenance,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Re-validates the manifest record.
    pub fn validate(&self) -> Result<(), WorkflowForgeError> {
        if self.package_version != PACKAGE_MANIFEST_FORMAT_VERSION {
            return Err(WorkflowForgeError::UnsupportedFormatVersion {
                kind: "workflow package manifest",
                version: self.package_version,
            });
        }
        if self.name.trim().is_empty() {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "package names must be non-empty".to_owned(),
            });
        }
        if self.workflows.is_empty() {
            return Err(WorkflowForgeError::InvalidRecord {
                reason: "a workflow package must declare at least one workflow".to_owned(),
            });
        }
        for workflow in &self.workflows {
            workflow.validate()?;
            if workflow.repository != self.repository {
                return Err(WorkflowForgeError::ManifestRepositoryMismatch {
                    manifest_repository: workflow.repository.clone(),
                    expected_repository: self.repository.clone(),
                });
            }
        }
        Ok(())
    }

    /// Finds the manifest declaring `workflow`, when present.
    pub fn manifest_for(&self, workflow: &WorkflowDefinitionId) -> Option<&WorkflowManifest> {
        self.workflows
            .iter()
            .find(|manifest| &manifest.workflow == workflow)
    }
}

/// The dependency lock of a workflow package: every subworkflow
/// dependency pinned to immutable content.
///
/// Locks may be empty — a package whose workflows declare no subworkflow
/// dependencies locks nothing. Each dependency id may be pinned at most
/// once.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowPackageLock {
    /// Lock serialization format version.
    pub lock_version: u32,
    /// The immutable subworkflow pins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pins: Vec<SubworkflowPin>,
}

impl WorkflowPackageLock {
    /// Seals a lock from verified pins.
    pub fn new(pins: Vec<SubworkflowPin>) -> Result<Self, WorkflowForgeError> {
        let lock = Self {
            lock_version: PACKAGE_LOCK_FORMAT_VERSION,
            pins,
        };
        lock.verify()?;
        Ok(lock)
    }

    /// Finds the pin for a declared dependency id.
    pub fn pin(&self, dependency: &SubworkflowDependencyId) -> Option<&SubworkflowPin> {
        self.pins.iter().find(|pin| &pin.dependency == dependency)
    }

    /// Verifies the lock: supported format version, every pin
    /// internally consistent, no dependency pinned twice.
    pub fn verify(&self) -> Result<(), WorkflowForgeError> {
        if self.lock_version != PACKAGE_LOCK_FORMAT_VERSION {
            return Err(WorkflowForgeError::UnsupportedFormatVersion {
                kind: "workflow package lock",
                version: self.lock_version,
            });
        }
        for (index, pin) in self.pins.iter().enumerate() {
            pin.verify()?;
            if self.pins[..index]
                .iter()
                .any(|other| other.dependency == pin.dependency)
            {
                return Err(WorkflowForgeError::DuplicateSubworkflowPin {
                    dependency: pin.dependency.clone(),
                });
            }
        }
        Ok(())
    }

    /// Computes the lock's content digest.
    ///
    /// The digest covers exactly the identity-bearing pin fields, with
    /// source revisions reduced to their commit SHAs: provenance-only
    /// data (notably `resolved_from`) is excluded, mirroring how the
    /// frozen version identity excludes publication provenance. Two
    /// locks pinning identical executable content therefore digest
    /// identically, regardless of how the revisions were resolved.
    pub fn digest(&self) -> Result<ContentDigest, WorkflowForgeError> {
        let anchored = AnchoredLock::project(self);
        Ok(ContentDigest::of(&anchored)?)
    }
}

/// Identity-covered projection of a lock: provenance-stripped, mirroring
/// the frozen `AnchoredIdentity` discipline from the contracts crate.
#[derive(Serialize)]
struct AnchoredLock<'a> {
    lock_version: u32,
    pins: Vec<AnchoredPin<'a>>,
}

/// Identity-covered projection of one pin.
#[derive(Serialize)]
struct AnchoredPin<'a> {
    dependency: &'a SubworkflowDependencyId,
    source: &'a WorkflowRepositoryId,
    commit_sha: &'a codex_workflow_contracts::RevisionSha,
    version: Option<AnchoredVersion<'a>>,
}

/// Identity-covered projection of a pinned version.
#[derive(Serialize)]
struct AnchoredVersion<'a> {
    workflow: &'a WorkflowDefinitionId,
    semantic_version: &'a SemanticVersion,
    repository: &'a WorkflowRepositoryId,
    commit_sha: &'a codex_workflow_contracts::RevisionSha,
    definition_digest: &'a ContentDigest,
    dependency_lock_digest: &'a ContentDigest,
    version_id: &'a WorkflowVersionId,
}

impl<'a> AnchoredLock<'a> {
    /// Projects a lock into its identity-covered form.
    fn project(lock: &'a WorkflowPackageLock) -> Self {
        Self {
            lock_version: lock.lock_version,
            pins: lock
                .pins
                .iter()
                .map(|pin| AnchoredPin {
                    dependency: &pin.dependency,
                    source: &pin.source,
                    commit_sha: &pin.source_revision.commit_sha,
                    version: pin.version.as_ref().map(|version| AnchoredVersion {
                        workflow: &version.identity.workflow,
                        semantic_version: &version.identity.semantic_version,
                        repository: &version.identity.repository,
                        commit_sha: &version.identity.source_revision.commit_sha,
                        definition_digest: &version.identity.definition_digest,
                        dependency_lock_digest: &version.identity.dependency_lock_digest,
                        version_id: &version.version_id,
                    }),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use codex_workflow_contracts::DevelopmentRef;
    use codex_workflow_contracts::ImmutableSourceRevision;

    use crate::test_support::manifest;
    use crate::test_support::published_version;
    use crate::test_support::repo_id;
    use crate::test_support::sha;
    use crate::test_support::workflow_id;

    use super::SubworkflowPin;
    use super::WorkflowPackageLock;
    use super::WorkflowPackageManifest;
    use super::PACKAGE_LOCK_FORMAT_VERSION;
    use crate::WorkflowForgeError;

    fn sample_release() -> codex_workflow_contracts::WorkflowRelease {
        let repository = repo_id("github.com/acme/ops-workflows");
        let target = sha(30);
        let version = published_version(
            "teardown",
            "github.com/acme/ops-workflows",
            &target,
            (1, 4, 0),
            "teardown-definition",
            "teardown-lock",
        );
        let target_revision =
            ImmutableSourceRevision::pin(target, Some(DevelopmentRef::tag("v1.4.0").unwrap()));
        crate::collaboration::publish_release(
            &repository,
            &[],
            "v1.4.0",
            &target_revision,
            std::slice::from_ref(&version),
        )
        .unwrap()
    }

    fn sample_version() -> crate::collaboration::PublishedVersionRef {
        published_version(
            "teardown",
            "github.com/acme/ops-workflows",
            &sha(30),
            (1, 4, 0),
            "teardown-definition",
            "teardown-lock",
        )
    }

    #[test]
    fn pins_subworkflows_to_released_versions() {
        let release = sample_release();
        let version = sample_version();
        let dependency =
            codex_workflow_contracts::SubworkflowDependencyId::parse("deploy_needs_teardown")
                .unwrap();
        let pin = SubworkflowPin::from_release(dependency.clone(), &release, &version).unwrap();
        assert_eq!(pin.dependency, dependency);
        assert_eq!(pin.source, repo_id("github.com/acme/ops-workflows"));
        assert_eq!(pin.source_revision.commit_sha, release.target.commit_sha);
        assert_eq!(pin.version_id(), Some(&version.version_id));
        assert_eq!(pin.workflow(), Some(&workflow_id("teardown")));
        assert!(pin.verify().is_ok());
    }

    #[test]
    fn release_pins_reject_versions_outside_the_release() {
        let release = sample_release();
        let foreign = published_version(
            "teardown",
            "github.com/acme/ops-workflows",
            &sha(30),
            (1, 5, 0), // different content, not listed by the release
            "teardown-definition-2",
            "teardown-lock",
        );
        let dependency = codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap();
        assert!(matches!(
            SubworkflowPin::from_release(dependency, &release, &foreign),
            Err(WorkflowForgeError::VersionNotInRelease { .. })
        ));
    }

    #[test]
    fn release_pins_reject_unanchored_versions() {
        // A release cut at revision A cannot pin a version anchored at
        // revision B, even if the version id were listed.
        let repository = repo_id("github.com/acme/ops-workflows");
        let target = sha(30);
        let unanchored = published_version(
            "teardown",
            "github.com/acme/ops-workflows",
            &sha(31),
            (1, 4, 0),
            "teardown-definition",
            "teardown-lock",
        );
        let rejected = crate::collaboration::publish_release(
            &repository,
            &[],
            "v1.4.1",
            &ImmutableSourceRevision::pin_commit(target),
            std::slice::from_ref(&unanchored),
        )
        .unwrap_err();
        // publish_release itself refuses the mismatched anchoring, which
        // is the invariant from_release leans on.
        assert!(matches!(
            rejected,
            WorkflowForgeError::VersionNotAnchoredAtTarget { .. }
        ));
    }

    #[test]
    fn pins_subworkflows_to_immutable_revisions_without_releases() {
        let dependency = codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap();
        let revision =
            ImmutableSourceRevision::pin(sha(50), Some(DevelopmentRef::branch("main").unwrap()));
        let pin = SubworkflowPin::from_revision(
            dependency.clone(),
            repo_id("github.com/acme/ops-workflows"),
            revision,
        )
        .unwrap();
        assert_eq!(pin.version, None);
        assert_eq!(pin.version_id(), None);
        assert_eq!(pin.source_revision.commit_sha, sha(50));
        assert!(pin.verify().is_ok());
    }

    #[test]
    fn locks_reject_duplicate_dependency_pins() {
        let dependency = codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap();
        let pin = SubworkflowPin::from_revision(
            dependency,
            repo_id("github.com/acme/ops-workflows"),
            ImmutableSourceRevision::pin_commit(sha(50)),
        )
        .unwrap();
        let duplicate = pin.clone();
        assert!(matches!(
            WorkflowPackageLock::new(vec![pin, duplicate]),
            Err(WorkflowForgeError::DuplicateSubworkflowPin { .. })
        ));
    }

    #[test]
    fn lock_digests_ignore_provenance_only_fields() {
        // Two pins with identical executable content but different
        // resolution provenance digest identically.
        let dependency = codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap();
        let from_branch = SubworkflowPin::from_revision(
            dependency.clone(),
            repo_id("github.com/acme/ops-workflows"),
            ImmutableSourceRevision::pin(sha(60), Some(DevelopmentRef::branch("main").unwrap())),
        )
        .unwrap();
        let from_tag = SubworkflowPin::from_revision(
            dependency,
            repo_id("github.com/acme/ops-workflows"),
            ImmutableSourceRevision::pin(sha(60), Some(DevelopmentRef::tag("v9").unwrap())),
        )
        .unwrap();
        let branch_lock = WorkflowPackageLock::new(vec![from_branch]).unwrap();
        let tag_lock = WorkflowPackageLock::new(vec![from_tag]).unwrap();
        assert_eq!(branch_lock.digest().unwrap(), tag_lock.digest().unwrap());

        // Different executable content digests differently.
        let other = SubworkflowPin::from_revision(
            codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap(),
            repo_id("github.com/acme/ops-workflows"),
            ImmutableSourceRevision::pin_commit(sha(61)),
        )
        .unwrap();
        let other_lock = WorkflowPackageLock::new(vec![other]).unwrap();
        assert_ne!(branch_lock.digest().unwrap(), other_lock.digest().unwrap());

        // Lock records themselves differ (provenance is preserved as
        // data), only the identity digest is provenance-free.
        assert_ne!(branch_lock, tag_lock);
    }

    #[test]
    fn locks_verify_format_versions() {
        let pin = SubworkflowPin::from_revision(
            codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap(),
            repo_id("github.com/acme/ops-workflows"),
            ImmutableSourceRevision::pin_commit(sha(70)),
        )
        .unwrap();
        let mut lock = WorkflowPackageLock::new(vec![pin]).unwrap();
        assert_eq!(lock.lock_version, PACKAGE_LOCK_FORMAT_VERSION);
        lock.lock_version = 99;
        assert!(matches!(
            lock.verify(),
            Err(WorkflowForgeError::UnsupportedFormatVersion { .. })
        ));
    }

    #[test]
    fn tampered_pins_fail_verification() {
        let release = sample_release();
        let version = sample_version();
        let dependency = codex_workflow_contracts::SubworkflowDependencyId::parse("dep").unwrap();
        let mut pin = SubworkflowPin::from_release(dependency, &release, &version).unwrap();
        // Repoint the pin at a different revision than its version.
        pin.source_revision = ImmutableSourceRevision::pin_commit(sha(99));
        assert!(matches!(
            pin.verify(),
            Err(WorkflowForgeError::PinInconsistent { .. })
        ));
    }

    #[test]
    fn package_manifests_declare_their_workflows() {
        let repository = repo_id("github.com/acme/ops-workflows");
        let revision = ImmutableSourceRevision::pin_commit(sha(80));
        let deploy = manifest(
            "deploy",
            "github.com/acme/ops-workflows",
            "workflows/deploy.toml",
        );
        let package = WorkflowPackageManifest::new(
            "ops-workflows",
            repository.clone(),
            revision.clone(),
            vec![deploy],
            None,
        )
        .unwrap();
        assert_eq!(package.name, "ops-workflows");
        assert_eq!(package.source_revision, revision);
        assert!(package.manifest_for(&workflow_id("deploy")).is_some());
        assert!(package.manifest_for(&workflow_id("absent")).is_none());

        // Empty packages, foreign manifests, and stale manifest format
        // versions are rejected.
        assert!(matches!(
            WorkflowPackageManifest::new("ops", repository.clone(), revision.clone(), vec![], None),
            Err(WorkflowForgeError::InvalidRecord { .. })
        ));
        let foreign = manifest("deploy", "github.com/other/repo", "workflows/deploy.toml");
        assert!(matches!(
            WorkflowPackageManifest::new(
                "ops",
                repository.clone(),
                revision.clone(),
                vec![foreign],
                None
            ),
            Err(WorkflowForgeError::ManifestRepositoryMismatch { .. })
        ));
        let mut stale = manifest(
            "deploy",
            "github.com/acme/ops-workflows",
            "workflows/deploy.toml",
        );
        stale.manifest_version = 0;
        assert!(matches!(
            WorkflowPackageManifest::new("ops", repository, revision, vec![stale], None),
            Err(WorkflowForgeError::Contract(_))
        ));
    }
}
