//! Binding policy.
//!
//! Binding policy controls which environments and binding classes
//! resolution may select. It is an *input* to binding decisions, not an
//! authority over workflow semantics: the frozen invariant *execution
//! capability does not imply permission to execute* means policy is
//! checked during resolution and again at authorization, and a binding
//! can never be selected merely because it exists and is ready.
//!
//! The default policy is conservative: every peer environment is in scope,
//! but human fallback is forbidden. Routing work to a human silently is
//! never the default; it must be explicitly permitted.

use serde::Deserialize;
use serde::Serialize;

use crate::BindingClass;
use crate::ExecutionContractError;
use crate::ExecutionEnvironment;

/// Which environments resolution may select.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum EnvironmentScope {
    /// Every peer environment is in scope.
    All,
    /// Only the listed environments are in scope.
    ///
    /// The list must be non-empty and must not contain reserved
    /// environments.
    Only(Vec<ExecutionEnvironment>),
}

/// Whether human-fallback bindings may be selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum HumanFallbackPolicy {
    /// Human-fallback bindings may be selected when explicitly permitted
    /// by the run.
    Allowed,
    /// Human-fallback bindings are rejected during resolution.
    Forbidden,
}

/// The binding policy applied during capability resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingPolicy {
    /// Environments resolution may select.
    pub environments: EnvironmentScope,
    /// Whether the human fallback class may be selected.
    pub human_fallback: HumanFallbackPolicy,
}

impl Default for BindingPolicy {
    fn default() -> Self {
        Self {
            environments: EnvironmentScope::All,
            human_fallback: HumanFallbackPolicy::Forbidden,
        }
    }
}

impl BindingPolicy {
    /// Validates the policy's structural rules.
    ///
    /// - an `Only` scope must be non-empty, duplicate-free, and free of
    ///   reserved environments.
    pub fn validate(&self) -> Result<(), ExecutionContractError> {
        if let EnvironmentScope::Only(environments) = &self.environments {
            if environments.is_empty() {
                return Err(ExecutionContractError::InvalidPolicy {
                    reason: "environment scope `Only` must list at least one environment"
                        .to_owned(),
                });
            }
            let mut sorted = environments.clone();
            sorted.sort();
            let distinct = sorted.len();
            sorted.dedup();
            if sorted.len() != distinct {
                return Err(ExecutionContractError::InvalidPolicy {
                    reason: "environment scope lists the same environment twice".to_owned(),
                });
            }
            if let Some(reserved) = environments.iter().find(|env| env.is_reserved()) {
                return Err(ExecutionContractError::InvalidPolicy {
                    reason: format!("environment scope includes reserved environment {reserved}"),
                });
            }
        }
        Ok(())
    }

    /// Whether `environment` is in scope for this policy.
    pub fn allows_environment(&self, environment: ExecutionEnvironment) -> bool {
        match &self.environments {
            EnvironmentScope::All => true,
            EnvironmentScope::Only(environments) => environments.contains(&environment),
        }
    }

    /// Whether `class` is permitted by this policy.
    ///
    /// Only the human fallback class is policy-gated today; every other
    /// class is evaluated by environment scope and readiness.
    pub fn allows_class(&self, class: BindingClass) -> bool {
        match class {
            BindingClass::HumanFallback => self.human_fallback == HumanFallbackPolicy::Allowed,
            BindingClass::CodexNative | BindingClass::Compatible => true,
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
