//! The teaching session: a bounded, append-only trajectory recorder.

use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;
use crate::evidence::TeachingEvidence;
use crate::mode::TeachingMode;
use crate::trajectory::RecordOrigin;
use crate::trajectory::TrajectoryEvent;
use crate::trajectory::TrajectoryRecord;

/// Default maximum number of records a session accepts.
pub const DEFAULT_MAX_RECORDS: usize = 256;

/// Policy governing a teaching session and the candidate it produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionPolicy {
    /// Whether deterministic, provably equivalence-preserving optimization
    /// proposals may be applied to the candidate. Defaults to `false`;
    /// enabling it is an explicit policy decision.
    pub allow_optimization: bool,
    /// Maximum number of trajectory records the session accepts.
    pub max_records: usize,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            allow_optimization: false,
            max_records: DEFAULT_MAX_RECORDS,
        }
    }
}

/// A bounded teaching session in one of the three teaching modes.
///
/// The session records validated, structured events. It performs no
/// interpretation beyond bound and mode checks: interpretation happens in
/// the compiler, and semantics live only in the compiled IR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingSession {
    mode: TeachingMode,
    policy: SessionPolicy,
    records: Vec<TrajectoryRecord>,
    next_sequence: u64,
    closed: bool,
}

impl TeachingSession {
    /// Opens a session with the default policy.
    pub fn new(mode: TeachingMode) -> Self {
        Self::with_policy(mode, SessionPolicy::default())
    }

    /// Opens a session with an explicit policy.
    pub fn with_policy(mode: TeachingMode, policy: SessionPolicy) -> Self {
        Self {
            mode,
            policy,
            records: Vec::new(),
            next_sequence: 0,
            closed: false,
        }
    }

    /// The session's teaching mode.
    pub fn mode(&self) -> TeachingMode {
        self.mode
    }

    /// The session's policy.
    pub fn policy(&self) -> &SessionPolicy {
        &self.policy
    }

    /// The recorded trajectory.
    pub fn records(&self) -> &[TrajectoryRecord] {
        &self.records
    }

    /// Number of recorded records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no records have been recorded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Whether the session is closed for recording.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Records one event with attached evidence and returns its sequence
    /// number.
    ///
    /// Recording fails if the session is closed, the mode does not accept
    /// the origin, the origin and the event kind disagree, a bound is
    /// exceeded, or the event or evidence is invalid. The trajectory is
    /// append-only: records are never rewritten or removed.
    pub fn record(
        &mut self,
        origin: RecordOrigin,
        event: TrajectoryEvent,
        evidence: Vec<TeachingEvidence>,
    ) -> Result<u64, TeachingCompilerError> {
        if self.closed {
            return Err(TeachingCompilerError::InvalidState {
                current: "closed".to_string(),
                required: "open".to_string(),
            });
        }
        let origin_allowed = match origin {
            RecordOrigin::Demonstration => self.mode.accepts_demonstration(),
            RecordOrigin::Instruction => self.mode.accepts_instruction(),
        };
        if !origin_allowed {
            return Err(TeachingCompilerError::ModeOriginMismatch {
                mode: self.mode.as_str().to_string(),
                origin: origin.as_str().to_string(),
            });
        }
        let origin_is_instruction = matches!(origin, RecordOrigin::Instruction);
        let event_is_instruction = matches!(event, TrajectoryEvent::Instruction { .. });
        if origin_is_instruction != event_is_instruction {
            return Err(TeachingCompilerError::InvalidEvent {
                reason: format!(
                    "a {} record cannot carry a {} event",
                    origin.as_str(),
                    event.kind_label()
                ),
            });
        }
        if self.records.len() >= self.policy.max_records {
            return Err(TeachingCompilerError::TrajectoryBoundExceeded {
                limit: self.policy.max_records,
            });
        }
        let sequence = self.next_sequence;
        let record = TrajectoryRecord::new(sequence, origin, event, evidence)?;
        self.records.push(record);
        self.next_sequence += 1;
        Ok(sequence)
    }

    /// Closes the session. A closed session can be compiled; it can no
    /// longer record events. Closing is idempotent.
    pub fn close(&mut self) {
        self.closed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_MAX_RECORDS;
    use super::RecordOrigin as Origin;
    use super::SessionPolicy;
    use super::TeachingSession;
    use crate::error::TeachingCompilerError;
    use crate::mode::TeachingMode;
    use crate::trajectory::TrajectoryEvent;

    #[test]
    fn enforces_mode_and_origin_pairing() {
        let mut demonstrate = TeachingSession::new(TeachingMode::Demonstrate);
        assert!(matches!(
            demonstrate.record(
                Origin::Instruction,
                TrajectoryEvent::Instruction {
                    text: "Do it.".to_string()
                },
                Vec::new()
            ),
            Err(TeachingCompilerError::ModeOriginMismatch { .. })
        ));
        assert!(matches!(
            demonstrate.record(
                Origin::Demonstration,
                TrajectoryEvent::Instruction {
                    text: "Do it.".to_string()
                },
                Vec::new()
            ),
            Err(TeachingCompilerError::InvalidEvent { .. })
        ));

        let mut instruct = TeachingSession::new(TeachingMode::Instruct);
        assert!(matches!(
            instruct.record(
                Origin::Demonstration,
                TrajectoryEvent::Action {
                    text: "Click.".to_string()
                },
                Vec::new()
            ),
            Err(TeachingCompilerError::ModeOriginMismatch { .. })
        ));

        let mut hybrid = TeachingSession::new(TeachingMode::Hybrid);
        hybrid
            .record(
                Origin::Demonstration,
                TrajectoryEvent::Action {
                    text: "Click.".to_string(),
                },
                Vec::new(),
            )
            .expect("demonstration record");
        hybrid
            .record(
                Origin::Instruction,
                TrajectoryEvent::Instruction {
                    text: "Then save.".to_string(),
                },
                Vec::new(),
            )
            .expect("instruction record");
        assert_eq!(hybrid.len(), 2);
    }

    #[test]
    fn enforces_the_trajectory_bound() {
        let mut session = TeachingSession::with_policy(
            TeachingMode::Demonstrate,
            SessionPolicy {
                allow_optimization: false,
                max_records: 2,
            },
        );
        for index in 0..2 {
            session
                .record(
                    Origin::Demonstration,
                    TrajectoryEvent::Action {
                        text: format!("Step {index}."),
                    },
                    Vec::new(),
                )
                .expect("record");
        }
        assert!(matches!(
            session.record(
                Origin::Demonstration,
                TrajectoryEvent::Action {
                    text: "One too many.".to_string(),
                },
                Vec::new()
            ),
            Err(TeachingCompilerError::TrajectoryBoundExceeded { limit: 2 })
        ));
    }

    #[test]
    fn closed_sessions_reject_records_and_sequences_are_dense() {
        let mut session = TeachingSession::new(TeachingMode::Demonstrate);
        assert_eq!(
            session
                .record(
                    Origin::Demonstration,
                    TrajectoryEvent::Action {
                        text: "A.".to_string()
                    },
                    Vec::new()
                )
                .expect("record"),
            0
        );
        assert_eq!(
            session
                .record(
                    Origin::Demonstration,
                    TrajectoryEvent::Action {
                        text: "B.".to_string()
                    },
                    Vec::new()
                )
                .expect("record"),
            1
        );
        session.close();
        assert!(session.is_closed());
        session.close();
        assert!(matches!(
            session.record(
                Origin::Demonstration,
                TrajectoryEvent::Action {
                    text: "C.".to_string()
                },
                Vec::new()
            ),
            Err(TeachingCompilerError::InvalidState { .. })
        ));
        assert_eq!(DEFAULT_MAX_RECORDS, 256);
    }

    #[test]
    fn round_trips_through_json() {
        let mut session = TeachingSession::new(TeachingMode::Hybrid);
        session
            .record(
                Origin::Demonstration,
                TrajectoryEvent::Action {
                    text: "A.".to_string(),
                },
                Vec::new(),
            )
            .expect("record");
        session.close();
        let encoded = serde_json::to_string(&session).expect("serialize");
        let decoded: TeachingSession = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(session, decoded);
    }
}
