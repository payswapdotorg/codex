//! Time-based schedule specifications and due-occurrence computation.
//!
//! Schedules are pure, deterministic data: an anchor instant, a
//! specification, and a consumption cursor. The due-occurrence computation
//! is overflow-safe arithmetic with no calendar dependencies (UTC days
//! are exact multiples of 86_400_000 ms), so the same inputs always
//! produce the same occurrences — which is what makes schedule fires
//! idempotent when combined with deterministic trigger event keys.
//!
//! The scheduler itself owns no authority: it only derives triggers,
//! which flow through the same ingest path as every other event.

use serde::Deserialize;
use serde::Serialize;

use crate::WorkflowTriggerError;

/// Milliseconds in one UTC day.
const MS_PER_DAY: u64 = 86_400_000;
/// Maximum due occurrences one poll may process per schedule entry.
///
/// Bounded catch-up: when a scheduler is down long enough to accumulate
/// more missed occurrences than this, only the most recent ones are
/// processed; older ones are dropped (recorded as consumed by the cursor
/// advance) so a poll can never loop unboundedly.
pub const MAX_OCCURRENCES_PER_POLL: usize = 1_024;

/// A time-based schedule specification.
///
/// The spec deliberately supports two deterministic forms; richer
/// calendars (cron expressions, time zones) are host-side concerns that
/// would have to map onto these or extend the port surface in a bounded
/// follow-up Work Order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ScheduleSpec {
    /// Fires every `period_ms` after the registration anchor.
    Every {
        /// Firing period in milliseconds; must be positive.
        period_ms: u64,
    },
    /// Fires daily at a fixed UTC time.
    DailyAtUtc {
        /// UTC hour of the day (0..24).
        hour_utc: u32,
        /// UTC minute of the hour (0..60).
        minute_utc: u32,
    },
}

impl ScheduleSpec {
    /// Validates the specification's structural rules.
    ///
    /// - `Every` requires a positive period;
    /// - `DailyAtUtc` requires a valid UTC hour and minute.
    pub fn validate(&self) -> Result<(), WorkflowTriggerError> {
        match self {
            Self::Every { period_ms } => {
                if *period_ms == 0 {
                    Err(WorkflowTriggerError::InvalidSchedule {
                        spec: format!("{self:?}"),
                        reason: "period must be positive".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
            Self::DailyAtUtc {
                hour_utc,
                minute_utc,
            } => {
                if *hour_utc >= 24 || *minute_utc >= 60 {
                    Err(WorkflowTriggerError::InvalidSchedule {
                        spec: format!("{self:?}"),
                        reason: "hour must be < 24 and minute must be < 60 (UTC)".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
        }
    }

    /// The spec in compact, human-readable form (for diagnostics).
    pub fn describe(&self) -> String {
        match self {
            Self::Every { period_ms } => format!("every {period_ms}ms"),
            Self::DailyAtUtc {
                hour_utc,
                minute_utc,
            } => format!("daily at {hour_utc:02}:{minute_utc:02} UTC"),
        }
    }
}

/// Computes the due occurrences of one schedule: every occurrence in
/// `(cursor_unix_ms, now_unix_ms]`, anchored at `anchor_unix_ms`.
///
/// The result is capped at [`MAX_OCCURRENCES_PER_POLL`] most-recent
/// occurrences. All arithmetic is checked, so extreme inputs terminate
/// instead of wrapping.
pub fn due_occurrences(
    spec: &ScheduleSpec,
    anchor_unix_ms: u64,
    cursor_unix_ms: u64,
    now_unix_ms: u64,
) -> Vec<u64> {
    match spec {
        ScheduleSpec::Every { period_ms } => {
            let mut occurrences = Vec::new();
            let mut multiplier: u64 = 1;
            while let Some(offset) = multiplier.checked_mul(*period_ms) {
                let Some(occurrence) = anchor_unix_ms.checked_add(offset) else {
                    break;
                };
                if occurrence > now_unix_ms {
                    break;
                }
                if occurrence > cursor_unix_ms {
                    push_bounded(&mut occurrences, occurrence);
                }
                let Some(next) = multiplier.checked_add(1) else {
                    break;
                };
                multiplier = next;
            }
            occurrences
        }
        ScheduleSpec::DailyAtUtc {
            hour_utc,
            minute_utc,
        } => {
            let offset_ms = u64::from(*hour_utc) * 3_600_000 + u64::from(*minute_utc) * 60_000;
            let mut occurrences = Vec::new();
            let mut occurrence = next_daily_boundary(cursor_unix_ms, offset_ms);
            while occurrence <= now_unix_ms {
                push_bounded(&mut occurrences, occurrence);
                let Some(next) = occurrence.checked_add(MS_PER_DAY) else {
                    break;
                };
                occurrence = next;
            }
            occurrences
        }
    }
}

/// Pushes one occurrence, dropping the oldest when the cap is reached.
fn push_bounded(occurrences: &mut Vec<u64>, occurrence: u64) {
    if occurrences.len() == MAX_OCCURRENCES_PER_POLL {
        occurrences.remove(0);
    }
    occurrences.push(occurrence);
}

/// The first daily boundary (day start + `offset_ms`) strictly after
/// `after`.
fn next_daily_boundary(after: u64, offset_ms: u64) -> u64 {
    let day_start = after - (after % MS_PER_DAY);
    let candidate = day_start.saturating_add(offset_ms);
    if candidate > after {
        candidate
    } else {
        candidate.saturating_add(MS_PER_DAY)
    }
}
