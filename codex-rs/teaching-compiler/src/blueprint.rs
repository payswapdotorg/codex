//! Deterministic extraction of step blueprints from a closed teaching
//! session.
//!
//! Blueprints are ordered step intents plus the evidence supporting them.
//! Extraction is a pure function of the trajectory: identical sessions
//! always produce identical blueprints, which is what makes the compiled
//! IR — and its digest — reproducible.

use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;
use crate::evidence::TeachingEvidence;
use crate::session::TeachingSession;
use crate::trajectory::TrajectoryEvent;

/// Where a compiled step came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepOrigin {
    /// Derived from a demonstrated action.
    Observed,
    /// Derived from an instruction statement.
    Instructed,
}

impl StepOrigin {
    /// Stable lowercase name for diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Instructed => "instructed",
        }
    }
}

/// One ordered step intent extracted from a teaching trajectory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepBlueprint {
    intent: String,
    origin: StepOrigin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<TeachingEvidence>,
}

impl StepBlueprint {
    /// The step's human-facing intent, carried into the IR step
    /// description.
    pub fn intent(&self) -> &str {
        &self.intent
    }

    /// Where the step came from.
    pub fn origin(&self) -> StepOrigin {
        self.origin
    }

    /// Evidence supporting the step.
    pub fn evidence(&self) -> &[TeachingEvidence] {
        &self.evidence
    }
}

/// Extracts ordered step blueprints from a closed session.
///
/// Demonstration records: an `Action` opens a step; a preceding
/// `Observation` prefixes its description; a following `Result` or
/// `Recovery` record annotates the most recent step. Instruction records
/// each become one step. `Note` records are provenance only. Extraction
/// never reorders or merges records: the compiled ordering is exactly the
/// recorded ordering.
pub(crate) fn extract_blueprints(
    session: &TeachingSession,
) -> Result<Vec<StepBlueprint>, TeachingCompilerError> {
    if session.is_empty() {
        return Err(TeachingCompilerError::EmptySession);
    }
    if !session.is_closed() {
        return Err(TeachingCompilerError::SessionNotClosed);
    }
    let mut blueprints: Vec<StepBlueprint> = Vec::new();
    let mut pending_observation: Option<String> = None;
    for record in session.records() {
        match record.event() {
            TrajectoryEvent::Observation { text } => {
                pending_observation = Some(text.clone());
            }
            TrajectoryEvent::Action { text } => {
                let intent = match pending_observation.take() {
                    Some(observation) => format!("Observed context: {observation}. Action: {text}"),
                    None => text.clone(),
                };
                blueprints.push(StepBlueprint {
                    intent,
                    origin: StepOrigin::Observed,
                    evidence: record.evidence().to_vec(),
                });
            }
            TrajectoryEvent::Result { text } => {
                annotate_last(&mut blueprints, format!("Outcome: {text}"))?;
            }
            TrajectoryEvent::Recovery { text } => {
                annotate_last(&mut blueprints, format!("Recovery: {text}"))?;
            }
            TrajectoryEvent::Instruction { text } => {
                pending_observation = None;
                blueprints.push(StepBlueprint {
                    intent: text.clone(),
                    origin: StepOrigin::Instructed,
                    evidence: record.evidence().to_vec(),
                });
            }
            TrajectoryEvent::Note { .. } => {}
        }
    }
    if blueprints.is_empty() {
        return Err(TeachingCompilerError::NoCompilableSteps {
            reason: format!(
                "the {} session contained no demonstration actions or instruction statements",
                session.mode().as_str()
            ),
        });
    }
    Ok(blueprints)
}

/// Attaches an annotation to the most recent blueprint.
fn annotate_last(
    blueprints: &mut [StepBlueprint],
    annotation: String,
) -> Result<(), TeachingCompilerError> {
    let last = blueprints
        .last_mut()
        .ok_or_else(|| TeachingCompilerError::InvalidEvent {
            reason: "a result or recovery record appeared before any demonstrated action"
                .to_string(),
        })?;
    last.intent = format!("{}. {annotation}", last.intent);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::StepOrigin;
    use super::extract_blueprints;
    use crate::error::TeachingCompilerError;
    use crate::mode::TeachingMode;
    use crate::session::TeachingSession;
    use crate::trajectory::RecordOrigin;
    use crate::trajectory::TrajectoryEvent;

    #[test]
    fn rejects_empty_open_and_accepts_closed_sessions() {
        let mut session = TeachingSession::new(TeachingMode::Instruct);
        assert!(matches!(
            extract_blueprints(&session),
            Err(TeachingCompilerError::EmptySession)
        ));
        session
            .record(
                RecordOrigin::Instruction,
                TrajectoryEvent::Instruction {
                    text: "Do it.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        assert!(matches!(
            extract_blueprints(&session),
            Err(TeachingCompilerError::SessionNotClosed)
        ));
        session.close();
        let blueprints = extract_blueprints(&session).expect("blueprints");
        assert_eq!(blueprints.len(), 1);
        assert_eq!(blueprints[0].origin(), StepOrigin::Instructed);
        assert_eq!(blueprints[0].intent(), "Do it.");
    }

    #[test]
    fn rejects_sessions_with_no_compilable_steps() {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Note {
                    text: "Provenance only, never a step.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();
        assert!(matches!(
            extract_blueprints(&session),
            Err(TeachingCompilerError::NoCompilableSteps { .. })
        ));
    }

    #[test]
    fn rejects_outcome_records_before_any_demonstrated_action() {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        session
            .record(
                RecordOrigin::Demonstration,
                TrajectoryEvent::Result {
                    text: "An outcome with no preceding action.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();
        assert!(matches!(
            extract_blueprints(&session),
            Err(TeachingCompilerError::InvalidEvent { .. })
        ));
    }
}
