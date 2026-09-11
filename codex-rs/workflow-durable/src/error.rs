//! Errors surfaced by the durable control-plane stores.
//!
//! The frozen port error types (`WorkflowAppError`,
//! `WorkflowTriggerError`) predate durability and carry no i/o variant,
//! and this Work Order may not edit them. The durable stores therefore
//! keep their own precise error type — [`DurableStoreError`] — for
//! construction-time failures (unreadable roots, unparseable snapshots,
//! corrupt journals), and project runtime failures into the frozen
//! enums through explicit, documented mappings:
//!
//! - i/o and serialization failures during a port write project as
//!   [`WorkflowAppError::Serialization`] carrying the decorated context
//!   message, because "the store could not write its serialized bytes"
//!   is exactly that variant's plane; on the trigger side the existing
//!   `From<WorkflowAppError>` projection surfaces them unchanged.
//! - a workflow version that fails `verify_integrity` on load projects
//!   as [`WorkflowAppError::VersionIntegrity`]: the record could not be
//!   re-verified and must never execute.
//! - identity-shaped failures reuse the existing variants verbatim
//!   (`InstanceAlreadyExists`, `InstanceUnavailable`, `IllegalResume`,
//!   `UnknownTriggerEvent`), matching the in-memory doubles' errors.

use std::fmt;
use std::io;

use codex_workflow_app::WorkflowAppError;
use codex_workflow_triggers::WorkflowTriggerError;
use serde::ser::Error as _;
use thiserror::Error;

/// A failure produced while opening or driving a durable control-plane
/// store.
#[derive(Debug, Error)]
pub enum DurableStoreError {
    /// An i/o failure while reading or writing a durable store file.
    #[error("durable store i/o failure while {context}: {source}")]
    Io {
        /// Which durable operation failed.
        context: &'static str,
        /// The underlying i/o failure.
        source: io::Error,
    },
    /// A stored record could not be serialized or parsed.
    #[error("durable store serialization failure: {source}")]
    Serialization {
        /// The underlying serde failure.
        source: serde_json::Error,
    },
    /// A durable store file is structurally corrupt: a committed
    /// (non-tail) journal line or snapshot region cannot be interpreted.
    ///
    /// This is a deterministic, loud failure — never a silent wrong
    /// state. Torn *tails* (an interrupted append) are not errors: they
    /// are repaired by the documented truncation policy at open time.
    #[error("durable store file `{path}` is corrupt: {reason}")]
    Corrupt {
        /// The file that failed to interpret.
        path: String,
        /// Why the file cannot be interpreted.
        reason: String,
    },
}

impl From<serde_json::Error> for DurableStoreError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

impl DurableStoreError {
    /// Wraps an i/o failure with the durable operation that produced it.
    pub(crate) fn io(context: &'static str, source: io::Error) -> Self {
        Self::Io { context, source }
    }

    /// Labels a structurally corrupt durable file.
    pub(crate) fn corrupt(path: impl fmt::Display, reason: impl Into<String>) -> Self {
        Self::Corrupt {
            path: path.to_string(),
            reason: reason.into(),
        }
    }

    /// Projects the failure into the workflow application error type.
    ///
    /// The mapping is documented in the [module docs](self); it never
    /// invents identity failures — a store failure is surfaced as a
    /// store failure.
    pub(crate) fn into_app_error(self) -> WorkflowAppError {
        match self {
            Self::Io { context, source } => {
                WorkflowAppError::Serialization(json_error(context, source))
            }
            Self::Serialization { source } => WorkflowAppError::Serialization(source),
            Self::Corrupt { path, reason } => WorkflowAppError::Serialization(json_error(
                "interpreting durable state",
                io::Error::other(format!("file `{path}` is corrupt: {reason}")),
            )),
        }
    }

    /// Projects the failure into the workflow trigger error type
    /// through the existing `From<WorkflowAppError>` projection.
    pub(crate) fn into_trigger_error(self) -> WorkflowTriggerError {
        self.into_app_error().into()
    }
}

/// Converts an i/o failure into the `serde_json::Error` the frozen port
/// signatures can carry, with the durable context embedded in the
/// message.
///
/// `serde_json::Error::custom` (serde's public error constructor for
/// serializer-side failures) is used because the frozen enums have no
/// raw i/o variant; the resulting message keeps the full context and
/// cause visible through `WorkflowAppError::Serialization`.
fn json_error(context: &str, source: io::Error) -> serde_json::Error {
    serde_json::Error::custom(format!("{context}: {source}"))
}
