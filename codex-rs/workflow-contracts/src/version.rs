//! Immutable published workflow versions.
//!
//! A `WorkflowVersion` is the publishable, executable artifact of a
//! workflow. Its identity is defined by the frozen architecture as:
//!
//! ```text
//! workflow semantic version
//! repository identity
//! immutable source revision
//! workflow definition digest
//! dependency lock digest
//! ```
//!
//! Consequences enforced here:
//!
//! - Development refs never enter version identity; publication resolves a
//!   ref to a commit and pins the commit. Moving or deleting branches
//!   afterwards cannot change a published version.
//! - Mutating any identity-covered field produces a different
//!   `WorkflowVersionId`, so a record claiming an old id with new content
//!   fails `verify_integrity`.
//! - Publication metadata (provenance, resolved-from ref) is excluded from
//!   the identity tuple, so identical executable content always yields the
//!   same version identity regardless of who published it or from which
//!   branch.
//! - Every declared dependency must be resolved by the dependency lock
//!   before a version can be sealed.

use serde::Deserialize;
use serde::Serialize;

use crate::ContentDigest;
use crate::DependencyLock;
use crate::ImmutableSourceRevision;
use crate::RevisionSha;
use crate::SemanticVersion;
use crate::WorkflowContractError;
use crate::WorkflowDefinition;
use crate::WorkflowDefinitionId;
use crate::WorkflowProvenance;
use crate::WorkflowRepositoryId;
use crate::WorkflowVersionId;

/// The identity tuple of a published workflow version, exactly as frozen by
/// the architecture.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionVersionIdentity {
    /// Identity of the workflow being versioned.
    pub workflow: WorkflowDefinitionId,
    /// Semantic version of this revision of the workflow.
    pub semantic_version: SemanticVersion,
    /// Repository the version was published from.
    pub repository: WorkflowRepositoryId,
    /// Immutable source revision the version was published at.
    pub source_revision: ImmutableSourceRevision,
    /// Digest of the frozen workflow definition.
    pub definition_digest: ContentDigest,
    /// Digest of the resolved dependency lock.
    pub dependency_lock_digest: ContentDigest,
}

impl ExecutionVersionIdentity {
    /// Computes the immutable version identity digest for this tuple.
    ///
    /// The digest covers exactly the fields above: semantic version,
    /// repository identity, the immutable commit SHA, the definition
    /// digest, and the dependency lock digest. Provenance such as
    /// `resolved_from` is excluded by construction because the tuple
    /// serializes `ImmutableSourceRevision` with its provenance-stripped
    /// commit anchor only.
    pub fn version_id(&self) -> Result<WorkflowVersionId, WorkflowContractError> {
        let anchored = AnchoredIdentity {
            workflow: &self.workflow,
            semantic_version: &self.semantic_version,
            repository: &self.repository,
            commit_sha: self.source_revision.commit_sha.clone(),
            definition_digest: &self.definition_digest,
            dependency_lock_digest: &self.dependency_lock_digest,
        };
        Ok(WorkflowVersionId::from_digest(ContentDigest::of(
            &anchored,
        )?))
    }
}

/// The identity-covered projection of a version: identical to
/// [`ExecutionVersionIdentity`] except that the source revision is reduced
/// to its commit SHA so provenance cannot influence identity.
#[derive(Serialize)]
struct AnchoredIdentity<'a> {
    workflow: &'a WorkflowDefinitionId,
    semantic_version: &'a SemanticVersion,
    repository: &'a WorkflowRepositoryId,
    commit_sha: RevisionSha,
    definition_digest: &'a ContentDigest,
    dependency_lock_digest: &'a ContentDigest,
}

/// An immutable published workflow version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowVersion {
    /// Identity tuple of the version.
    pub identity: ExecutionVersionIdentity,
    /// Version identity digest recomputed from the tuple.
    pub version_id: WorkflowVersionId,
    /// The frozen workflow definition.
    pub definition: WorkflowDefinition,
    /// Resolved immutable dependency identities.
    pub dependency_lock: DependencyLock,
    /// Publication provenance: attribution, license, upgrade policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<WorkflowProvenance>,
}

impl WorkflowVersion {
    /// Seals a validated candidate into an immutable published version.
    ///
    /// Sealing:
    ///
    /// 1. validates the definition's semantic invariants;
    /// 2. verifies the dependency lock covers every declared dependency;
    /// 3. computes the definition digest and lock digest;
    /// 4. derives the version identity from the immutable execution tuple.
    ///
    /// After sealing, the record is treated as immutable: every mutation of
    /// identity-covered content changes the digest and therefore the
    /// identity, which `verify_integrity` detects.
    pub fn seal(
        definition: WorkflowDefinition,
        repository: WorkflowRepositoryId,
        source_revision: ImmutableSourceRevision,
        semantic_version: SemanticVersion,
        dependency_lock: DependencyLock,
        provenance: Option<WorkflowProvenance>,
    ) -> Result<Self, WorkflowContractError> {
        definition.validate()?;
        dependency_lock.verify_covers(&definition.dependencies)?;
        let definition_digest = definition.digest()?;
        let dependency_lock_digest = dependency_lock.digest()?;
        let identity = ExecutionVersionIdentity {
            workflow: definition.id.clone(),
            semantic_version,
            repository,
            source_revision,
            definition_digest,
            dependency_lock_digest,
        };
        let version_id = identity.version_id()?;
        Ok(Self {
            identity,
            version_id,
            definition,
            dependency_lock,
            provenance,
        })
    }

    /// Verifies the record's integrity end to end.
    ///
    /// A version passes only when:
    ///
    /// - the embedded definition digests to the recorded definition digest;
    /// - the embedded dependency lock digests to the recorded lock digest;
    /// - the dependency lock still covers every declared dependency;
    /// - the recorded version id equals the digest of the identity tuple;
    /// - the definition still satisfies its semantic invariants.
    ///
    /// This is the check that makes published versions effectively
    /// immutable: a tampered record fails recomputation.
    pub fn verify_integrity(&self) -> Result<(), WorkflowContractError> {
        let recomputed_definition_digest = self.definition.digest()?;
        if recomputed_definition_digest != self.identity.definition_digest {
            return Err(WorkflowContractError::VersionIntegrity {
                reason: "definition content does not match the recorded definition digest"
                    .to_owned(),
            });
        }
        let recomputed_lock_digest = self.dependency_lock.digest()?;
        if recomputed_lock_digest != self.identity.dependency_lock_digest {
            return Err(WorkflowContractError::VersionIntegrity {
                reason: "dependency lock does not match the recorded lock digest".to_owned(),
            });
        }
        self.dependency_lock
            .verify_covers(&self.definition.dependencies)?;
        self.definition.validate()?;
        let recomputed_version_id = self.identity.version_id()?;
        if recomputed_version_id != self.version_id {
            return Err(WorkflowContractError::VersionIntegrity {
                reason: "version id does not match the execution identity tuple".to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "version_tests.rs"]
mod tests;
