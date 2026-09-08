//! Compiler inference outputs: capability hints, binding proposals, and
//! trigger intents.
//!
//! These are **proposals only**. The compiler is environment- and
//! provider-neutral: it never binds a concrete tool, model, browser, or
//! MCP server, and it never writes contract capability requirements
//! itself. Binding resolution belongs to the execution plane and is
//! confirmed by explicit approval.

use codex_workflow_contracts::IrNodeId;
use serde::Deserialize;
use serde::Serialize;

/// The plane a binding proposal addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BindingKind {
    /// A semantic capability requirement for a step.
    Capability,
    /// A resource requirement for a step.
    Resource,
    /// A skill dependency.
    Skill,
    /// A plugin dependency.
    Plugin,
    /// An MCP server dependency.
    McpServer,
    /// A pinned subworkflow dependency.
    Subworkflow,
}

impl BindingKind {
    /// Stable lowercase name for diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Resource => "resource",
            Self::Skill => "skill",
            Self::Plugin => "plugin",
            Self::McpServer => "mcp-server",
            Self::Subworkflow => "subworkflow",
        }
    }
}

/// A lexical capability hint derived from one step's intent.
///
/// Hints are derived deterministically from the step text and carry no
/// binding authority.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityInference {
    /// The step the hint applies to.
    pub node_id: IrNodeId,
    /// A neutral hint label, for example `verb:navigate`.
    pub requirement_hint: String,
    /// Why the hint was produced.
    pub rationale: String,
}

/// A proposed execution binding that must be resolved and approved before
/// publication.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingProposal {
    /// The step the binding applies to.
    pub node_id: IrNodeId,
    /// Which plane the binding addresses.
    pub kind: BindingKind,
    /// A neutral proposed reference, never a concrete environment binding.
    pub proposed_reference: String,
    /// Why the binding is proposed.
    pub rationale: String,
    /// Always `true`: bindings are never applied without approval.
    pub requires_approval: bool,
}

/// A trigger suggestion derived from instruction statements that begin
/// with "when". Triggers are control-plane constructs; this is a hint
/// carried for review, never a declared trigger.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerIntent {
    /// Neutral class hint; always `unresolved` until the control plane
    /// normalizes it.
    pub class_hint: String,
    /// The instruction text the intent was derived from.
    pub description: String,
}
