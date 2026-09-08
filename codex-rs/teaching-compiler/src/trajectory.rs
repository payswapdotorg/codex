//! Bounded teaching trajectory events.
//!
//! A teaching trajectory is an append-only, size-bounded sequence of
//! structured records. Records are provenance and evidence, never semantic
//! authority: semantics are produced only by compiling a validated IR.

use serde::Deserialize;
use serde::Serialize;

use crate::error::TeachingCompilerError;
use crate::evidence::TeachingEvidence;

/// Maximum byte length of a record's text payload.
pub const MAX_TEXT_BYTES: usize = 4096;
/// Maximum number of evidence references attached to one record.
pub const MAX_EVIDENCE_PER_RECORD: usize = 8;

/// The origin plane of a trajectory record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordOrigin {
    /// Recorded from a demonstration (the user performing the workflow).
    Demonstration,
    /// Recorded from an instruction statement.
    Instruction,
}

impl RecordOrigin {
    /// Stable lowercase name for diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Demonstration => "demonstration",
            Self::Instruction => "instruction",
        }
    }
}

/// One structured, bounded teaching event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TrajectoryEvent {
    /// Observed environment state during a demonstration.
    Observation {
        /// Bounded human-facing text describing the observation.
        text: String,
    },
    /// A concrete action performed during a demonstration.
    Action {
        /// Bounded human-facing text describing the action.
        text: String,
    },
    /// The observed outcome of a demonstration action.
    Result {
        /// Bounded human-facing text describing the outcome.
        text: String,
    },
    /// A recovery from failure observed during a demonstration.
    Recovery {
        /// Bounded human-facing text describing the recovery.
        text: String,
    },
    /// A structured instruction statement.
    Instruction {
        /// Bounded human-facing instruction text.
        text: String,
    },
    /// A clarifying note; recorded as provenance only.
    Note {
        /// Bounded human-facing note text.
        text: String,
    },
}

impl TrajectoryEvent {
    /// The record's bounded text payload.
    pub fn text(&self) -> &str {
        match self {
            Self::Observation { text }
            | Self::Action { text }
            | Self::Result { text }
            | Self::Recovery { text }
            | Self::Instruction { text }
            | Self::Note { text } => text,
        }
    }

    /// A stable lowercase kind label for diagnostics.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Observation { .. } => "observation",
            Self::Action { .. } => "action",
            Self::Result { .. } => "result",
            Self::Recovery { .. } => "recovery",
            Self::Instruction { .. } => "instruction",
            Self::Note { .. } => "note",
        }
    }

    fn validate(&self) -> Result<(), TeachingCompilerError> {
        let text = self.text();
        if text.is_empty() {
            return Err(TeachingCompilerError::InvalidEvent {
                reason: format!("{} record text must be non-empty", self.kind_label()),
            });
        }
        if text.len() > MAX_TEXT_BYTES {
            return Err(TeachingCompilerError::InvalidEvent {
                reason: format!(
                    "{} record text exceeds {} bytes",
                    self.kind_label(),
                    MAX_TEXT_BYTES
                ),
            });
        }
        Ok(())
    }
}

/// One append-only trajectory entry: a validated event with origin,
/// sequence number, and attached evidence references.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrajectoryRecord {
    sequence: u64,
    origin: RecordOrigin,
    event: TrajectoryEvent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<TeachingEvidence>,
}

impl TrajectoryRecord {
    pub(crate) fn new(
        sequence: u64,
        origin: RecordOrigin,
        event: TrajectoryEvent,
        evidence: Vec<TeachingEvidence>,
    ) -> Result<Self, TeachingCompilerError> {
        event.validate()?;
        if evidence.len() > MAX_EVIDENCE_PER_RECORD {
            return Err(TeachingCompilerError::InvalidEvent {
                reason: format!(
                    "a record carries at most {MAX_EVIDENCE_PER_RECORD} evidence references"
                ),
            });
        }
        Ok(Self {
            sequence,
            origin,
            event,
            evidence,
        })
    }

    /// The record's position in the trajectory, starting at zero.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// The record's origin plane.
    pub fn origin(&self) -> RecordOrigin {
        self.origin
    }

    /// The record's event.
    pub fn event(&self) -> &TrajectoryEvent {
        &self.event
    }

    /// Evidence references attached to the record.
    pub fn evidence(&self) -> &[TeachingEvidence] {
        &self.evidence
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_EVIDENCE_PER_RECORD;
    use super::MAX_TEXT_BYTES;
    use super::RecordOrigin;
    use super::TrajectoryEvent;
    use super::TrajectoryRecord;
    use crate::evidence::TeachingEvidence;

    fn evidence(label: &str) -> TeachingEvidence {
        TeachingEvidence::new(label, "rollout://x", "ab".repeat(32)).expect("evidence")
    }

    #[test]
    fn rejects_empty_and_oversized_text() {
        let empty = TrajectoryEvent::Action {
            text: String::new(),
        };
        assert!(TrajectoryRecord::new(0, RecordOrigin::Demonstration, empty, Vec::new()).is_err());
        let oversized = TrajectoryEvent::Instruction {
            text: "x".repeat(MAX_TEXT_BYTES + 1),
        };
        assert!(
            TrajectoryRecord::new(0, RecordOrigin::Instruction, oversized, Vec::new()).is_err()
        );
    }

    #[test]
    fn rejects_too_many_evidence_references() {
        let event = TrajectoryEvent::Action {
            text: "Do a thing.".to_string(),
        };
        let evidence: Vec<_> = (0..MAX_EVIDENCE_PER_RECORD + 1)
            .map(|index| evidence(&format!("e{index}")))
            .collect();
        assert!(TrajectoryRecord::new(0, RecordOrigin::Demonstration, event, evidence).is_err());
    }

    #[test]
    fn round_trips_through_json() {
        let record = TrajectoryRecord::new(
            7,
            RecordOrigin::Demonstration,
            TrajectoryEvent::Action {
                text: "Open the list.".to_string(),
            },
            vec![evidence("open")],
        )
        .expect("record");
        let encoded = serde_json::to_string(&record).expect("serialize");
        let decoded: TrajectoryRecord = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(record, decoded);
        assert_eq!(decoded.sequence(), 7);
        assert_eq!(decoded.event().kind_label(), "action");
    }
}
