//! Due-occurrence computation for schedule specifications.

use pretty_assertions::assert_eq;

use crate::MAX_OCCURRENCES_PER_POLL;
use crate::ScheduleSpec;
use crate::WorkflowTriggerError;
use crate::due_occurrences;

#[test]
fn interval_schedules_fire_once_per_period_after_the_anchor() {
    let spec = ScheduleSpec::Every { period_ms: 1_000 };
    // Anchor at t=0; nothing is due at the anchor itself.
    assert_eq!(due_occurrences(&spec, 0, 0, 0), Vec::<u64>::new());
    assert_eq!(due_occurrences(&spec, 0, 0, 999), Vec::<u64>::new());
    assert_eq!(due_occurrences(&spec, 0, 0, 1_000), vec![1_000]);
    assert_eq!(due_occurrences(&spec, 0, 0, 2_500), vec![1_000, 2_000]);
    // The cursor consumes occurrences: only new ones are due.
    assert_eq!(due_occurrences(&spec, 0, 1_000, 2_500), vec![2_000]);
    assert_eq!(due_occurrences(&spec, 0, 2_000, 2_500), Vec::<u64>::new());
    // A clock that moved backwards yields nothing (the cursor never
    // rewinds).
    assert_eq!(due_occurrences(&spec, 0, 2_000, 1_500), Vec::<u64>::new());
}

#[test]
fn interval_schedules_anchor_at_registration_not_at_zero() {
    let spec = ScheduleSpec::Every { period_ms: 1_000 };
    // Anchored at t=5_000: occurrences are 6_000, 7_000, ...
    assert_eq!(
        due_occurrences(&spec, 5_000, 5_000, 5_999),
        Vec::<u64>::new()
    );
    assert_eq!(
        due_occurrences(&spec, 5_000, 5_000, 7_500),
        vec![6_000, 7_000]
    );
}

#[test]
fn daily_schedules_fire_at_utc_boundaries() {
    let spec = ScheduleSpec::DailyAtUtc {
        hour_utc: 9,
        minute_utc: 30,
    };
    // Day 0 (epoch) 09:30 UTC = 34_200_000 ms.
    let boundary = 34_200_000u64;
    // Cursor before the boundary on day 0.
    assert_eq!(
        due_occurrences(&spec, 0, 10_000_000, boundary),
        vec![boundary]
    );
    // Two days elapse.
    let day = 86_400_000u64;
    assert_eq!(
        due_occurrences(&spec, 0, 10_000_000, boundary + 2 * day),
        vec![boundary, boundary + day, boundary + 2 * day]
    );
    // A cursor exactly at a boundary consumes it: the next boundary is
    // the following day.
    assert_eq!(
        due_occurrences(&spec, 0, boundary, boundary + day),
        vec![boundary + day]
    );
    // A midnight-aligned cursor still fires later the same day.
    assert_eq!(
        due_occurrences(&spec, 0, 5 * day, 5 * day + boundary),
        vec![5 * day + boundary]
    );
    // ... but not before the boundary has actually been reached.
    assert_eq!(
        due_occurrences(&spec, 0, 5 * day, 5 * day + boundary - 1),
        Vec::<u64>::new()
    );
}

#[test]
fn catch_up_is_bounded_to_the_most_recent_occurrences() {
    // Tiny period, huge gap: the computation must stay bounded and keep
    // only the most recent occurrences.
    let spec = ScheduleSpec::Every { period_ms: 1 };
    let occurrences = due_occurrences(&spec, 0, 0, 10_000);
    assert_eq!(occurrences.len(), MAX_OCCURRENCES_PER_POLL);
    let cap = u64::try_from(MAX_OCCURRENCES_PER_POLL).expect("cap fits u64");
    assert_eq!(
        occurrences,
        ((10_000 - cap + 1)..=10_000).collect::<Vec<u64>>()
    );
}

#[test]
fn extreme_inputs_terminate_instead_of_wrapping() {
    let spec = ScheduleSpec::Every {
        period_ms: u64::MAX,
    };
    // anchor + 1 * u64::MAX overflows: no occurrences, no panic.
    assert_eq!(
        due_occurrences(&spec, u64::MAX / 2, 0, u64::MAX),
        Vec::<u64>::new()
    );
    let daily = ScheduleSpec::DailyAtUtc {
        hour_utc: 23,
        minute_utc: 59,
    };
    // Near the u64 ceiling the day arithmetic saturates and terminates.
    let occurrences = due_occurrences(&daily, 0, u64::MAX - 1, u64::MAX);
    assert!(occurrences.len() <= 1);
}

#[test]
fn specs_validate_their_bounds() {
    assert!(ScheduleSpec::Every { period_ms: 1 }.validate().is_ok());
    assert!(matches!(
        ScheduleSpec::Every { period_ms: 0 }.validate(),
        Err(WorkflowTriggerError::InvalidSchedule { .. })
    ));
    assert!(
        ScheduleSpec::DailyAtUtc {
            hour_utc: 23,
            minute_utc: 59
        }
        .validate()
        .is_ok()
    );
    for invalid in [
        ScheduleSpec::DailyAtUtc {
            hour_utc: 24,
            minute_utc: 0,
        },
        ScheduleSpec::DailyAtUtc {
            hour_utc: 0,
            minute_utc: 60,
        },
    ] {
        assert!(matches!(
            invalid.validate(),
            Err(WorkflowTriggerError::InvalidSchedule { .. })
        ));
    }
}
