//! Teaching modes.

use serde::Deserialize;
use serde::Serialize;

/// How a workflow is being taught.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingMode {
    /// The user demonstrates the workflow by performing it; steps are
    /// derived from recorded observations, actions, results, and
    /// recoveries.
    Demonstrate,
    /// The user describes the workflow; steps are derived from structured
    /// instruction statements.
    Instruct,
    /// Both interleaved: demonstrated steps and instructed steps are
    /// merged in recording order.
    Hybrid,
}

impl TeachingMode {
    /// Whether this mode accepts demonstration-origin records.
    pub fn accepts_demonstration(self) -> bool {
        matches!(self, Self::Demonstrate | Self::Hybrid)
    }

    /// Whether this mode accepts instruction-origin records.
    pub fn accepts_instruction(self) -> bool {
        matches!(self, Self::Instruct | Self::Hybrid)
    }

    /// Stable lowercase name, used in error messages and diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Demonstrate => "demonstrate",
            Self::Instruct => "instruct",
            Self::Hybrid => "hybrid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TeachingMode;

    #[test]
    fn modes_gate_record_origins() {
        assert!(TeachingMode::Demonstrate.accepts_demonstration());
        assert!(!TeachingMode::Demonstrate.accepts_instruction());
        assert!(!TeachingMode::Instruct.accepts_demonstration());
        assert!(TeachingMode::Instruct.accepts_instruction());
        assert!(TeachingMode::Hybrid.accepts_demonstration());
        assert!(TeachingMode::Hybrid.accepts_instruction());
    }
}
