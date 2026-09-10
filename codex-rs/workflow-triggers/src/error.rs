//! Errors surfaced by the workflow trigger plane.
//!
//! Trigger-plane failures name the plane that owns them: installation and
//! configuration checks produce structured
//! [`InstallDiagnostics`](crate::TriggerDiagnostic) records, control-plane
//! record mismatches produce identity errors, and failures of the frozen
//! crates below (forge, application lifecycle, contracts) project
//! transparently. Nothing here panics or invents semantics.

use codex_workflow_app::WorkflowAppError;
use codex_workflow_contracts::WorkflowContractError;
use codex_workflow_forge::WorkflowForgeError;
use thiserror::Error;

use crate::TriggerDiagnostic;

/// A failure produced while driving the workflow trigger plane.
#[derive(Debug, Error)]
pub enum WorkflowTriggerError {
    /// No installation exists for the workflow.
    #[error("workflow `{workflow}` is not installed")]
    NotInstalled {
        /// The workflow that has no installation.
        workflow: String,
    },
    /// The version a reconfiguration requested does not match the forge
    /// installation record.
    ///
    /// This is the silent-upgrade guard: after an explicit forge update
    /// is applied, the configuration must be explicitly reconfigured
    /// against exactly the installed version before further fires.
    #[error(
        "version `{requested}` was requested for `{workflow}` but the forge registry installs `{installed}`"
    )]
    VersionDrift {
        /// The workflow whose installation is being reconfigured.
        workflow: String,
        /// The version the request carried.
        requested: String,
        /// The version the forge registry installed.
        installed: String,
    },
    /// The configured version record is not present in the version store.
    #[error("workflow version `{version}` is not available in the version store")]
    VersionUnavailable {
        /// The missing version identity.
        version: String,
    },
    /// The instance record referenced by a resume is not present.
    #[error("workflow instance `{instance}` is not present in the instance store")]
    InstanceUnavailable {
        /// The missing instance identity.
        instance: String,
    },
    /// An instance cannot be resumed from its current status.
    #[error("workflow instance `{instance}` cannot be resumed from `{current}`")]
    IllegalResume {
        /// The instance that was not paused.
        instance: String,
        /// The instance's current status.
        current: String,
    },
    /// Installation or configuration validation failed with structured
    /// diagnostics.
    #[error("installation for `{workflow}` failed: {summary}")]
    InstallDiagnostics {
        /// The workflow that failed validation.
        workflow: String,
        /// One-line summary of the first diagnostic.
        summary: String,
        /// Every diagnostic produced by the validation.
        diagnostics: Vec<TriggerDiagnostic>,
    },
    /// A schedule specification is structurally invalid.
    #[error("invalid schedule `{spec}`: {reason}")]
    InvalidSchedule {
        /// The invalid specification, as written.
        spec: String,
        /// Why the specification is invalid.
        reason: String,
    },
    /// A trigger event key is malformed.
    #[error("invalid trigger event key `{value}`: {reason}")]
    InvalidEventKey {
        /// The malformed key.
        value: String,
        /// Why the key is invalid.
        reason: String,
    },
    /// The ledger was asked to settle an event id it never allocated.
    #[error("trigger event `{event_id}` is unknown to the ledger")]
    UnknownTriggerEvent {
        /// The unknown event identity.
        event_id: String,
    },
    /// A workflow-forge failure (install/update semantics).
    #[error(transparent)]
    Forge(#[from] WorkflowForgeError),
    /// A workflow-application failure (lifecycle seams).
    #[error(transparent)]
    App(#[from] WorkflowAppError),
    /// A workflow-contracts failure (integrity, validation).
    #[error(transparent)]
    Contract(#[from] WorkflowContractError),
}

impl WorkflowTriggerError {
    /// Builds an [`WorkflowTriggerError::InstallDiagnostics`] from the
    /// diagnostics produced by one validation pass.
    ///
    /// The summary mirrors the first diagnostic so the error renders
    /// usefully in logs while the full list stays structured.
    pub(crate) fn install_diagnostics(
        workflow: impl Into<String>,
        diagnostics: Vec<TriggerDiagnostic>,
    ) -> Self {
        let summary = diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "no diagnostics were produced".to_string());
        Self::InstallDiagnostics {
            workflow: workflow.into(),
            summary,
            diagnostics,
        }
    }
}
