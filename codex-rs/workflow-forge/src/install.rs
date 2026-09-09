//! Installed workflows and explicit, reviewable updates.
//!
//! Installation is the boundary between collaboration and execution:
//! an installed workflow is pinned to an immutable published version,
//! and running instances pin their own version identities. The
//! invariants enforced here:
//!
//! - **No silent changes.** Installing over an existing installation is
//!   rejected — upgrades must go through an explicit plan and decision.
//! - **Updates are reviewable.** Every decision (apply or reject)
//!   produces an audit record; the from/to version identities and notes
//!   are preserved in history.
//! - **Stale plans are refused.** A plan whose `from` no longer matches
//!   the installation was superseded and must be re-proposed and
//!   re-reviewed.
//! - **Running instances are never touched.** Upgrading an installation
//!   changes which version *future* instances use; instances already
//!   running keep the version identity they pinned at start, which
//!   remains a valid, verifiable immutable record.
//!
//! The registry is an in-memory record collection; durable storage and
//! instance lifecycle belong to the workflow control plane.

use std::collections::HashMap;

use codex_workflow_contracts::WorkflowDefinitionId;
use codex_workflow_contracts::WorkflowVersionId;
use serde::Deserialize;
use serde::Serialize;

use crate::collaboration::PublishedVersionRef;
use crate::WorkflowForgeError;

/// An installed workflow, pinned to an immutable published version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledWorkflow {
    /// The installed workflow.
    pub workflow: WorkflowDefinitionId,
    /// The immutable published version the installation is pinned to.
    pub installed: PublishedVersionRef,
}

/// A proposed update of an installed workflow.
///
/// A plan is pure data: proposing it never changes the installation.
/// Applying it requires an explicit [`UpdateDecision`] through
/// [`InstallRegistry::decide_update`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowUpdatePlan {
    /// The workflow to update.
    pub workflow: WorkflowDefinitionId,
    /// The version the plan upgrades from.
    pub from: WorkflowVersionId,
    /// The version the plan upgrades to.
    pub to: PublishedVersionRef,
    /// Review notes summarizing what changes, for reviewers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// An explicit decision on a proposed update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum UpdateDecision {
    /// Apply the update: future instances run the new version.
    Apply,
    /// Reject the update: keep the current version.
    Reject,
}

/// The audit record of a decided update: immutable history that makes
/// installed-workflow changes reviewable after the fact.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowUpdateRecord {
    /// The workflow that was (not) updated.
    pub workflow: WorkflowDefinitionId,
    /// The version the update started from.
    pub from: WorkflowVersionId,
    /// The version the update moved to (identical to `from` when
    /// rejected).
    pub to: WorkflowVersionId,
    /// Whether the update was applied.
    pub applied: bool,
    /// Review notes carried from the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// In-memory registry of installed workflows and their update history.
///
/// This registry owns no execution and no durable storage: the Codex
/// application persists these records through the workflow control
/// plane. The semantics here are what make installed updates explicit
/// and reviewable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallRegistry {
    installed: HashMap<WorkflowDefinitionId, InstalledWorkflow>,
    history: Vec<WorkflowUpdateRecord>,
}

impl InstallRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs a workflow at an immutable published version.
    ///
    /// The version reference must verify against its recomputed identity
    /// digest. Installing a workflow that is already installed is
    /// rejected: changes to an installation must be explicit updates.
    pub fn install(
        &mut self,
        version: &PublishedVersionRef,
    ) -> Result<InstalledWorkflow, WorkflowForgeError> {
        version.verify()?;
        let workflow = version.identity.workflow.clone();
        if self.installed.contains_key(&workflow) {
            return Err(WorkflowForgeError::AlreadyInstalled { workflow });
        }
        let record = InstalledWorkflow {
            workflow: workflow.clone(),
            installed: version.clone(),
        };
        self.installed.insert(workflow, record.clone());
        Ok(record)
    }

    /// Proposes an update of an installed workflow. Proposing never
    /// changes the installation.
    ///
    /// The target version must verify, and must differ from the
    /// installed version — no-op updates are rejected. Explicit
    /// downgrades (rollbacks) follow the same reviewable path.
    pub fn propose_update(
        &self,
        workflow: &WorkflowDefinitionId,
        to: &PublishedVersionRef,
        notes: Option<String>,
    ) -> Result<WorkflowUpdatePlan, WorkflowForgeError> {
        let current =
            self.installed
                .get(workflow)
                .ok_or_else(|| WorkflowForgeError::NotInstalled {
                    workflow: workflow.clone(),
                })?;
        to.verify()?;
        if to.version_id == current.installed.version_id {
            return Err(WorkflowForgeError::UpdateIsNoOp {
                workflow: workflow.clone(),
                version: to.version_id.clone(),
            });
        }
        Ok(WorkflowUpdatePlan {
            workflow: workflow.clone(),
            from: current.installed.version_id.clone(),
            to: to.clone(),
            notes,
        })
    }

    /// Decides a proposed update explicitly.
    ///
    /// The plan must still match the installation (its `from` equals the
    /// installed version); a superseded plan is refused rather than
    /// applied blindly. The target version is re-verified at decision
    /// time. Rejections keep the installation and are still recorded.
    pub fn decide_update(
        &mut self,
        plan: &WorkflowUpdatePlan,
        decision: UpdateDecision,
    ) -> Result<WorkflowUpdateRecord, WorkflowForgeError> {
        let current =
            self.installed
                .get(&plan.workflow)
                .ok_or_else(|| WorkflowForgeError::NotInstalled {
                    workflow: plan.workflow.clone(),
                })?;
        if current.installed.version_id != plan.from {
            return Err(WorkflowForgeError::StaleUpdatePlan {
                workflow: plan.workflow.clone(),
                expected_from: plan.from.clone(),
                actual_from: current.installed.version_id.clone(),
            });
        }
        plan.to.verify()?;
        let applied = matches!(decision, UpdateDecision::Apply);
        if applied {
            self.installed.insert(
                plan.workflow.clone(),
                InstalledWorkflow {
                    workflow: plan.workflow.clone(),
                    installed: plan.to.clone(),
                },
            );
        }
        let record = WorkflowUpdateRecord {
            workflow: plan.workflow.clone(),
            from: plan.from.clone(),
            to: plan.to.version_id.clone(),
            applied,
            notes: plan.notes.clone(),
        };
        self.history.push(record.clone());
        Ok(record)
    }

    /// The installed record for a workflow, when installed.
    pub fn installed(&self, workflow: &WorkflowDefinitionId) -> Option<&InstalledWorkflow> {
        self.installed.get(workflow)
    }

    /// Whether the workflow is installed.
    pub fn is_installed(&self, workflow: &WorkflowDefinitionId) -> bool {
        self.installed.contains_key(workflow)
    }

    /// The full update history, oldest first.
    pub fn history(&self) -> &[WorkflowUpdateRecord] {
        self.history.as_slice()
    }

    /// The update history for one workflow, oldest first.
    pub fn history_for(&self, workflow: &WorkflowDefinitionId) -> Vec<&WorkflowUpdateRecord> {
        self.history
            .iter()
            .filter(|record| &record.workflow == workflow)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::published_version;
    use crate::test_support::sha;
    use crate::test_support::workflow_id;

    use super::InstallRegistry;
    use super::UpdateDecision;
    use crate::WorkflowForgeError;

    fn version_a() -> crate::collaboration::PublishedVersionRef {
        published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &sha(10),
            (1, 2, 0),
            "definition-v1",
            "lock-v1",
        )
    }

    fn version_b() -> crate::collaboration::PublishedVersionRef {
        published_version(
            "deploy",
            "github.com/acme/ops-workflows",
            &sha(20),
            (1, 3, 0),
            "definition-v2",
            "lock-v2",
        )
    }

    #[test]
    fn installs_verified_versions_only() {
        let mut registry = InstallRegistry::new();
        let installed = registry.install(&version_a()).unwrap();
        assert_eq!(installed.workflow, workflow_id("deploy"));
        assert_eq!(installed.installed.version_id, version_a().version_id);
        assert!(registry.is_installed(&workflow_id("deploy")));

        // A tampered reference fails digest recomputation.
        let mut tampered = version_b();
        tampered.version_id = version_a().version_id;
        assert!(matches!(
            registry.install(&tampered),
            Err(WorkflowForgeError::VersionIdentityMismatch { .. })
        ));
    }

    #[test]
    fn installing_twice_requires_the_update_path() {
        let mut registry = InstallRegistry::new();
        registry.install(&version_a()).unwrap();
        assert!(matches!(
            registry.install(&version_b()),
            Err(WorkflowForgeError::AlreadyInstalled { .. })
        ));
    }

    #[test]
    fn proposing_updates_requires_an_installation_and_a_real_change() {
        let mut registry = InstallRegistry::new();
        let workflow = workflow_id("deploy");
        assert!(matches!(
            registry.propose_update(&workflow, &version_b(), None),
            Err(WorkflowForgeError::NotInstalled { .. })
        ));

        registry.install(&version_a()).unwrap();
        assert!(matches!(
            registry.propose_update(&workflow, &version_a(), None),
            Err(WorkflowForgeError::UpdateIsNoOp { .. })
        ));
        let plan = registry
            .propose_update(
                &workflow,
                &version_b(),
                Some("upgrade to v1.3.0".to_owned()),
            )
            .unwrap();
        assert_eq!(plan.from, version_a().version_id);
        assert_eq!(plan.to.version_id, version_b().version_id);
        assert_eq!(plan.notes.as_deref(), Some("upgrade to v1.3.0"));
        // Proposing did not change the installation.
        assert_eq!(
            registry.installed(&workflow).unwrap().installed.version_id,
            version_a().version_id
        );
    }

    #[test]
    fn applying_updates_is_explicit_and_recorded() {
        let mut registry = InstallRegistry::new();
        let workflow = workflow_id("deploy");
        registry.install(&version_a()).unwrap();
        let plan = registry
            .propose_update(&workflow, &version_b(), Some("upgrade".to_owned()))
            .unwrap();
        let record = registry
            .decide_update(&plan, UpdateDecision::Apply)
            .unwrap();
        assert!(record.applied);
        assert_eq!(record.from, version_a().version_id);
        assert_eq!(record.to, version_b().version_id);
        assert_eq!(
            registry.installed(&workflow).unwrap().installed.version_id,
            version_b().version_id
        );
        assert_eq!(registry.history().len(), 1);
        assert_eq!(registry.history_for(&workflow).len(), 1);
    }

    #[test]
    fn rejecting_updates_keeps_the_installation() {
        let mut registry = InstallRegistry::new();
        let workflow = workflow_id("deploy");
        registry.install(&version_a()).unwrap();
        let plan = registry
            .propose_update(&workflow, &version_b(), None)
            .unwrap();
        let record = registry
            .decide_update(&plan, UpdateDecision::Reject)
            .unwrap();
        assert!(!record.applied);
        assert_eq!(
            registry.installed(&workflow).unwrap().installed.version_id,
            version_a().version_id
        );
        // Rejections are still part of the reviewable history.
        assert_eq!(registry.history().len(), 1);
    }

    #[test]
    fn stale_plans_are_refused() {
        let mut registry = InstallRegistry::new();
        let workflow = workflow_id("deploy");
        registry.install(&version_a()).unwrap();
        let first = registry
            .propose_update(&workflow, &version_b(), None)
            .unwrap();
        registry
            .decide_update(&first, UpdateDecision::Apply)
            .unwrap();
        // `first` no longer matches the installation and must be
        // re-proposed, never applied blindly.
        assert!(matches!(
            registry.decide_update(&first, UpdateDecision::Apply),
            Err(WorkflowForgeError::StaleUpdatePlan { .. })
        ));
    }

    #[test]
    fn running_instances_keep_their_pinned_version() {
        let mut registry = InstallRegistry::new();
        let workflow = workflow_id("deploy");
        registry.install(&version_a()).unwrap();

        // A running instance pins the version identity at start.
        let running_instance_version = registry
            .installed(&workflow)
            .unwrap()
            .installed
            .version_id
            .clone();

        // Later, an upstream upgrade happens and is explicitly applied.
        let plan = registry
            .propose_update(&workflow, &version_b(), None)
            .unwrap();
        registry
            .decide_update(&plan, UpdateDecision::Apply)
            .unwrap();

        // The installation moved ...
        assert_eq!(
            registry.installed(&workflow).unwrap().installed.version_id,
            version_b().version_id
        );
        // ... the previously-running version is unchanged and still a
        // valid immutable record ...
        assert_ne!(
            registry.installed(&workflow).unwrap().installed.version_id,
            running_instance_version
        );
        assert!(version_a().verify().is_ok());
        assert_eq!(version_a().version_id, running_instance_version);
        // ... and the history keeps the old version id for review.
        assert_eq!(registry.history()[0].from, running_instance_version);
    }
}
