//! Execution environment classes.
//!
//! The frozen architecture models execution environments as *peers*: a
//! workflow step's semantics never name an environment; the runtime binds
//! each semantic capability requirement to a compatible environment
//! adapter. `BROWSER`, `COMPUTER`, `TERMINAL`, `API`, `TOOL`, `MCP`, and
//! `HUMAN` are the seven peer environments defined today.
//!
//! `MOBILE` and `REMOTE_DESKTOP` are **reserved**: they exist in the enum so
//! the wire contract is stable, but no adapter for them may be registered
//! until the Work Order that owns that environment (WO-015) activates it.
//! Registration and resource binding for reserved environments fail with
//! [`crate::ExecutionContractError::ReservedEnvironment`] so a premature
//! binding is an explicit diagnostic, never a silent behavior change.
//!
//! Additional future environments (for example `DEVICE`) are added as new
//! enum variants by a future Work Order. Because workflow semantics never
//! reference environment identities, adding a variant changes no workflow
//! contract and no published workflow revision.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::ExecutionContractError;

/// The class of execution environment a capability binding routes work to.
///
/// All peers have equal standing: no environment class implies authority
/// over workflow semantics, and no environment class is privileged during
/// binding resolution. Preference between bindings is expressed per binding
/// by [`crate::BindingClass`], not by this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionEnvironment {
    /// Web browser interaction (preferred adapter: Codex Browser Use).
    Browser,
    /// Desktop/application interaction (preferred adapter: Codex Computer
    /// Use).
    Computer,
    /// Terminal/shell execution through Codex sandboxed exec surfaces.
    Terminal,
    /// HTTP/API request execution through environment-owned clients and
    /// connectors.
    Api,
    /// Codex-native tool invocation.
    Tool,
    /// MCP tool/connector invocation.
    Mcp,
    /// Human participation: approvals, gates, and human-fallback execution.
    Human,
    /// Reserved for WO-015; not bindable today.
    Mobile,
    /// Reserved for WO-015; not bindable today.
    RemoteDesktop,
}

impl ExecutionEnvironment {
    /// Every environment known to this contract: the seven peers plus the
    /// two reserved environments.
    pub const ALL: [Self; 9] = [
        Self::Browser,
        Self::Computer,
        Self::Terminal,
        Self::Api,
        Self::Tool,
        Self::Mcp,
        Self::Human,
        Self::Mobile,
        Self::RemoteDesktop,
    ];

    /// The seven peer environments that may carry bindings today.
    pub const PEERS: [Self; 7] = [
        Self::Browser,
        Self::Computer,
        Self::Terminal,
        Self::Api,
        Self::Tool,
        Self::Mcp,
        Self::Human,
    ];

    /// Environments reserved for future Work Orders; never bindable today.
    pub const RESERVED: [Self; 2] = [Self::Mobile, Self::RemoteDesktop];

    /// Whether this environment is reserved and therefore not bindable.
    ///
    /// Reserved environments fail registration and resource binding
    /// explicitly rather than silently changing behavior.
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::Mobile | Self::RemoteDesktop)
    }

    /// The canonical wire name, matching the serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::Computer => "computer",
            Self::Terminal => "terminal",
            Self::Api => "api",
            Self::Tool => "tool",
            Self::Mcp => "mcp",
            Self::Human => "human",
            Self::Mobile => "mobile",
            Self::RemoteDesktop => "remoteDesktop",
        }
    }

    /// Parses a wire name into an environment.
    ///
    /// Only canonical wire names are accepted, so the parsed form always
    /// round-trips through serde unchanged.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, ExecutionContractError> {
        let value = value.as_ref();
        for environment in Self::ALL {
            if environment.as_str() == value {
                return Ok(environment);
            }
        }
        Err(ExecutionContractError::InvalidIdentifier {
            kind: "execution environment",
            value: value.to_owned(),
            reason: "must be a known environment name",
        })
    }
}

impl fmt::Display for ExecutionEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
