//! Resource bindings.
//!
//! Resources are typed execution dependencies (a browser profile, a GitHub
//! account, a human approver). Workflow semantics declare *requirements*
//! (WO-003 [`ResourceRequirement`]); the execution plane binds them to
//! concrete instances at install or execution time.
//!
//! A resource binding never carries credentials. It names a resource
//! instance by an opaque [`crate::ResourceId`] and the environment class
//! that owns it; secret material stays inside the adapter that owns the
//! resource and is resolved only at execution time. This is the frozen
//! invariant *credentials never enter workflow source or ordinary
//! evidence* expressed at the binding boundary.

use serde::Deserialize;
use serde::Serialize;

use crate::ExecutionContractError;
use crate::ExecutionEnvironment;
use crate::ResourceId;
use codex_workflow_contracts::ResourceTypeId;

/// The binding of one resource type to a concrete resource instance.
///
/// Rebinding (replacing the active instance for a type) is a control-plane
/// operation that creates a new binding record; it never rewrites
/// published workflow semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBinding {
    /// The resource type being bound.
    pub resource_type: ResourceTypeId,
    /// The concrete resource instance, opaque to the contract.
    pub resource: ResourceId,
    /// The environment class that owns and interprets the instance.
    pub holder: ExecutionEnvironment,
}

impl ResourceBinding {
    /// Validates the resource binding's invariants.
    ///
    /// - the holder environment must not be reserved.
    ///
    /// Credentials are structurally absent: the record only carries typed
    /// identities.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if self.holder.is_reserved() {
            return Err(ExecutionContractError::ReservedEnvironment {
                environment: self.holder,
                reason: "reserved environments cannot hold resources yet",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "resource_tests.rs"]
mod tests;
