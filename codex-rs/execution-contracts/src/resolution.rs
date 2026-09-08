//! Resolution support for the capability registry.
//!
//! This module holds the pure ordering, skip-recording, and diagnostic
//! helpers used by [`crate::CapabilityRegistry::resolve`]: the frozen
//! fallback-chain ordering, readiness ranking, preferred-candidate
//! projection, and the conversion of a skipped candidate into a
//! resolution diagnostic. Keeping them separate from the registry's
//! stateful surface makes the resolution rules reviewable in isolation.

use crate::AdapterId;
use crate::BindingDiagnostic;
use crate::DiagnosticCode;
use crate::FallbackReason;
use crate::ReadinessState;
use crate::RegisteredBinding;
use crate::SelectedBinding;
use codex_workflow_contracts::CapabilityId;

/// Orders resolution candidates: fallback class first, then readiness
/// (bindings that already passed the gates win), then adapter identity
/// for determinism.
pub(super) fn compare_candidates(
    left: &RegisteredBinding,
    right: &RegisteredBinding,
) -> std::cmp::Ordering {
    left.binding
        .class
        .cmp(&right.binding.class)
        .then(readiness_rank(right.readiness.state()).cmp(&readiness_rank(left.readiness.state())))
        .then(left.binding.adapter.cmp(&right.binding.adapter))
}

/// Ranks readiness for selection: bindings that passed more gates rank
/// higher; failure states rank lowest.
pub(super) fn readiness_rank(state: ReadinessState) -> i8 {
    match state {
        ReadinessState::Declared => 0,
        ReadinessState::Available => 1,
        ReadinessState::Ready => 2,
        ReadinessState::Authorized => 3,
        ReadinessState::Bound => 4,
        ReadinessState::Executing => 5,
        ReadinessState::Failed | ReadinessState::Unavailable => -1,
    }
}

/// Records the best (highest-preference) skipped candidate.
pub(super) fn record_skipped(
    best_skipped: &mut Option<(SelectedBinding, FallbackReason)>,
    candidate: &RegisteredBinding,
    reason: FallbackReason,
) {
    if best_skipped.is_none() {
        *best_skipped = Some((selected_of(candidate), reason));
    }
}

/// Projects a registered binding into the selection record.
pub(super) fn selected_of(registered: &RegisteredBinding) -> SelectedBinding {
    let binding = &registered.binding;
    SelectedBinding {
        binding: binding.binding.clone(),
        adapter: binding.adapter.clone(),
        environment: binding.environment,
        class: binding.class,
    }
}

/// Converts the best skipped candidate into a resolution diagnostic.
pub(super) fn skipped_diagnostic(
    capability: &CapabilityId,
    best_skipped: Option<(SelectedBinding, FallbackReason)>,
) -> BindingDiagnostic {
    match best_skipped {
        Some((skipped, reason)) => {
            let (code, detail) = match &reason {
                FallbackReason::PreferredNotReady { state, diagnostic } => {
                    let code = diagnostic
                        .as_ref()
                        .map_or(DiagnosticCode::NotInstalled, |diagnostic| diagnostic.code);
                    let detail = format!(
                        "preferred binding `{}` for capability `{}` is in state {state}",
                        skipped.binding,
                        capability.as_ref()
                    );
                    (code, detail)
                }
                FallbackReason::PreferredOutsideEnvironmentScope => (
                    DiagnosticCode::PolicyDenied,
                    format!(
                        "preferred binding `{}` routes to environment {}, which is outside the policy scope",
                        skipped.binding, skipped.environment
                    ),
                ),
                FallbackReason::PreferredHumanFallbackForbidden => (
                    DiagnosticCode::PolicyDenied,
                    format!(
                        "preferred binding `{}` is human fallback, which policy forbids",
                        skipped.binding
                    ),
                ),
                FallbackReason::PreferredMissingResources { missing } => (
                    DiagnosticCode::ResourceMissing,
                    format!(
                        "preferred binding `{}` requires unbound resources: {}",
                        skipped.binding,
                        missing
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ),
            };
            BindingDiagnostic::new(code, detail).in_environment(skipped.environment)
        }
        None => BindingDiagnostic::new(
            DiagnosticCode::NotRegistered,
            format!(
                "capability `{}` has no selectable binding",
                capability.as_ref()
            ),
        ),
    }
}

/// Builds a default not-installed diagnostic for an adapter.
pub(super) fn not_installed(adapter: &AdapterId) -> BindingDiagnostic {
    BindingDiagnostic::new(
        DiagnosticCode::NotInstalled,
        format!("adapter `{adapter}` reports the capability is not installed"),
    )
}
