//! Distribution-state transition table tests.

use pretty_assertions::assert_eq;

use super::DistributionState;
use super::DistributionTransition;
use super::state_label;
use super::validate_transition;
use crate::MarketplacePrincipal;
use crate::WorkflowDistributionError;

fn share(audience: &[&str]) -> DistributionTransition {
    DistributionTransition::Share {
        audience: audience
            .iter()
            .map(|value| MarketplacePrincipal::parse(*value))
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
    }
}

#[test]
fn legal_transitions_produce_their_target_states() {
    for from in [
        DistributionState::Private,
        DistributionState::Shared,
        DistributionState::Forked,
        DistributionState::Published,
        DistributionState::Public,
    ] {
        let share = validate_transition(from, &share(&["alice"]));
        if from == DistributionState::Shared {
            // share -> share is a refused no-op, not a legal transition.
            assert!(matches!(
                share,
                Err(WorkflowDistributionError::IllegalTransition { .. })
            ));
        } else {
            assert_eq!(share.ok(), Some(DistributionState::Shared));
        }

        let make_public = validate_transition(from, &DistributionTransition::MakePublic);
        if from == DistributionState::Public {
            assert!(matches!(
                make_public,
                Err(WorkflowDistributionError::IllegalTransition { .. })
            ));
        } else {
            assert_eq!(make_public.ok(), Some(DistributionState::Public));
        }

        let make_private = validate_transition(from, &DistributionTransition::MakePrivate);
        if from == DistributionState::Private {
            assert!(matches!(
                make_private,
                Err(WorkflowDistributionError::IllegalTransition { .. })
            ));
        } else {
            assert_eq!(make_private.ok(), Some(DistributionState::Private));
        }
    }
}

#[test]
fn installed_records_never_transition() {
    for transition in [
        share(&["alice"]),
        DistributionTransition::MakePublic,
        DistributionTransition::MakePrivate,
    ] {
        let refused = validate_transition(DistributionState::Installed, &transition);
        assert!(matches!(
            refused,
            Err(WorkflowDistributionError::IllegalTransition { .. })
        ));
    }
}

#[test]
fn sharing_requires_a_named_audience() {
    let refused = validate_transition(DistributionState::Private, &share(&[]));
    assert!(matches!(
        refused,
        Err(WorkflowDistributionError::InvalidRecord { .. })
    ));
}

#[test]
fn state_labels_are_stable_lowercase() {
    assert_eq!(state_label(DistributionState::Private), "private");
    assert_eq!(state_label(DistributionState::Shared), "shared");
    assert_eq!(state_label(DistributionState::Public), "public");
    assert_eq!(state_label(DistributionState::Forked), "forked");
    assert_eq!(state_label(DistributionState::Installed), "installed");
    assert_eq!(state_label(DistributionState::Published), "published");
}
