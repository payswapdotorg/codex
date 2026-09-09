//! Publication composition: approved teaching output into an immutable,
//! installable workflow version.
//!
//! The teaching compiler (WO-008) deliberately emits **binding proposals**,
//! never declared capability requirements: binding authority belongs to the
//! execution plane and is confirmed by review. This module is the
//! publisher-side resolution: a host-reviewed [`StepCapabilityBindings`]
//! map turns the compiler's per-step proposals into declared
//! [`CapabilityRequirement`]s on the step nodes. The requirements then
//! become part of the definition content, which is digested into the
//! version identity — so binding resolution is pinned by the immutable
//! version and any later change necessarily produces a new version, never
//! a silent mutation.
//!
//! Sealing, integrity, releases, and installation all delegate to the
//! frozen crates: [`WorkflowVersion::seal`] and
//! [`WorkflowVersion::verify_integrity`] (WO-003 contracts), and
//! `publish_release`/`InstallRegistry` (WO-009 forge).

use std::collections::BTreeMap;

use codex_teaching_compiler::ApprovedWorkflow;
use codex_workflow_contracts::CapabilityRequirement;
use codex_workflow_contracts::DependencyLock;
use codex_workflow_contracts::ImmutableSourceRevision;
use codex_workflow_contracts::IrNodeId;
use codex_workflow_contracts::SemanticVersion;
use codex_workflow_contracts::WorkflowDefinition;
use codex_workflow_contracts::WorkflowIrNode;
use codex_workflow_contracts::WorkflowProvenance;
use codex_workflow_contracts::WorkflowRepositoryId;
use codex_workflow_contracts::WorkflowVersion;
use codex_workflow_forge::InstallRegistry;
use codex_workflow_forge::InstalledWorkflow;
use codex_workflow_forge::PublishedVersionRef;
use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowAppError;

/// Step-node to declared capability requirements, resolved by review from
/// the teaching compiler's binding proposals.
///
/// Keys must reference step nodes that exist in the approved definition;
/// every listed requirement becomes part of the digested, immutable
/// version content.
pub type StepCapabilityBindings = BTreeMap<IrNodeId, Vec<CapabilityRequirement>>;

/// A publish request: the approved workflow plus the publication anchors.
pub struct PublishRequest {
    /// The approved, digest-pinned output of the teaching compiler.
    pub approved: ApprovedWorkflow,
    /// The repository the version is published from.
    pub repository: WorkflowRepositoryId,
    /// The immutable source revision the version is anchored at.
    ///
    /// The host resolves a development ref to a commit through the forge
    /// (`WorkflowForge::resolve_ref`); moving branches afterwards cannot
    /// change the published version.
    pub source_revision: ImmutableSourceRevision,
    /// The semantic version of this revision.
    pub semantic_version: SemanticVersion,
    /// The reviewed step bindings to declare on the definition.
    pub bindings: StepCapabilityBindings,
    /// The resolved dependency lock (may be empty when the workflow
    /// declares no dependencies).
    pub dependency_lock: DependencyLock,
    /// Publication provenance: attribution, license, upgrade policy.
    pub provenance: Option<WorkflowProvenance>,
}

/// The audit record of binding resolution: which step nodes received
/// which declared capability requirements, and from which approved
/// content digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingResolution {
    /// Digest of the approved definition the resolution started from.
    pub approved_digest: String,
    /// Digest of the executable definition the resolution produced.
    pub executable_digest: String,
    /// Declared requirements per step node.
    pub bindings: BTreeMap<String, Vec<CapabilityRequirement>>,
}

/// The output of a successful publication.
#[derive(Clone, Debug)]
pub struct PublishedArtifact {
    /// The sealed, immutable workflow version.
    pub version: WorkflowVersion,
    /// The published-version reference (identity tuple + digest) used by
    /// releases, pins, and installs.
    pub reference: PublishedVersionRef,
    /// The binding-resolution audit record.
    pub resolution: BindingResolution,
}

/// Publishes an approved workflow as an immutable version.
///
/// Steps:
///
/// 1. re-verify the approved content digest (the compiler already proved
///    byte-stability; this guards the hand-off);
/// 2. apply the reviewed bindings to the step nodes and validate the
///    executable definition;
/// 3. seal the immutable version through the frozen contract
///    (definition digest, dependency-lock coverage, identity tuple);
/// 4. project the published-version reference.
pub fn publish(request: PublishRequest) -> Result<PublishedArtifact, WorkflowAppError> {
    let approved = request.approved;
    let recomputed = approved.definition.digest()?;
    if recomputed != approved.digest {
        return Err(WorkflowAppError::VersionIntegrity {
            version: approved.digest.to_string(),
            reason: "approved definition content does not match the hand-off digest".to_string(),
        });
    }
    let executable = apply_bindings(&approved.definition, &request.bindings)?;
    executable.validate()?;
    let version = WorkflowVersion::seal(
        executable.clone(),
        request.repository,
        request.source_revision,
        request.semantic_version,
        request.dependency_lock,
        request.provenance,
    )?;
    let resolution = BindingResolution {
        approved_digest: approved.digest.to_string(),
        executable_digest: executable.digest()?.to_string(),
        bindings: request
            .bindings
            .iter()
            .map(|(node, requirements)| (node.to_string(), requirements.clone()))
            .collect(),
    };
    let reference = PublishedVersionRef::of(&version);
    Ok(PublishedArtifact {
        version,
        reference,
        resolution,
    })
}

/// Installs a published artifact through the forge's install registry.
///
/// Installation is the boundary between collaboration and execution: an
/// installed workflow is pinned to an immutable published version, and
/// future instances resolve the installed version identity. The host then
/// mirrors the full version record into its [`crate::port::WorkflowVersionStore`].
pub fn install_version(
    registry: &mut InstallRegistry,
    artifact: &PublishedArtifact,
) -> Result<InstalledWorkflow, WorkflowAppError> {
    Ok(registry.install(&artifact.reference)?)
}

/// Applies reviewed bindings to the step nodes of a definition.
///
/// Only step nodes may carry bindings, and every step keeps at most the
/// declared list (replacing any earlier content): the result is a new
/// candidate definition, never a mutation of a published one.
fn apply_bindings(
    definition: &WorkflowDefinition,
    bindings: &StepCapabilityBindings,
) -> Result<WorkflowDefinition, WorkflowAppError> {
    let mut executable = definition.clone();
    for (node_id, requirements) in bindings {
        match executable.ir.nodes.get_mut(node_id) {
            Some(WorkflowIrNode::Step(step)) => {
                step.capabilities = requirements.clone();
            }
            Some(_) => {
                return Err(WorkflowAppError::Contract(
                    codex_workflow_contracts::WorkflowContractError::MalformedIr {
                        reason: format!(
                            "binding resolution targets node `{node_id}`, which is not a step node"
                        ),
                    },
                ));
            }
            None => {
                return Err(WorkflowAppError::Contract(
                    codex_workflow_contracts::WorkflowContractError::MalformedIr {
                        reason: format!(
                            "binding resolution targets node `{node_id}`, which does not exist"
                        ),
                    },
                ));
            }
        }
    }
    Ok(executable)
}
