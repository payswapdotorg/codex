//! Structured trigger-plane diagnostics.
//!
//! Diagnostics make the trigger plane's rejection paths observable and
//! diagnosable: every reason a trigger was not eligible, an installation
//! was refused, or an instantiation failed is a structured record with a
//! stable code and a bounded, human-readable message. They are recorded
//! durably through the [`crate::TriggerLedger`] settlement and returned
//! from installation validation — never swallowed.

use serde::Deserialize;
use serde::Serialize;

/// One structured trigger-plane diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerDiagnostic {
    /// The stable diagnostic code.
    pub code: TriggerDiagnosticCode,
    /// Bounded, human-readable detail (untrusted-derived content is
    /// redacted before it reaches diagnostics).
    pub message: String,
}

impl TriggerDiagnostic {
    /// Builds a diagnostic from a code and message.
    pub fn new(code: TriggerDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Stable codes for trigger-plane diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerDiagnosticCode {
    /// No installation exists for the workflow.
    NotInstalled,
    /// The configuration and the forge installation pin different
    /// versions (an explicit update was applied but the configuration was
    /// not reconfigured).
    VersionDrift,
    /// The configured version record is missing from the version store.
    VersionUnavailable,
    /// The version record failed end-to-end integrity verification: the
    /// tampered or corrupted record must never execute.
    VersionIntegrity,
    /// The installation binds no trigger for this class and routing
    /// endpoint.
    TriggerNotBound,
    /// The workflow version declares its triggers and does not declare
    /// this class.
    TriggerNotDeclared,
    /// A declared resource requirement has no configured binding.
    ResourceMissing,
    /// A resource binding targets a type the workflow version does not
    /// declare.
    ResourceNotDeclared,
    /// The authorization plane refused a resource or account binding.
    ResourceNotAuthorized,
    /// A locked dependency has no configured binding.
    DependencyUnbound,
    /// A configured dependency binding does not match the version's
    /// immutable dependency lock (a silent upgrade attempt).
    DependencyDrift,
    /// Capability, policy, or binding-plan readiness was refused during
    /// instantiation (the instance settled `Failed` with diagnostics).
    ReadinessDenied,
    /// Instantiation failed for another reason (seam failure after
    /// acceptance).
    InstantiationFailed,
}
