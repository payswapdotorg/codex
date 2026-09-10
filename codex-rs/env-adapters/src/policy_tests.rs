//! Tests for the shared allow/deny policy tables.

use std::collections::BTreeMap;

use pretty_assertions::assert_eq;

use crate::policy::Access;
use crate::policy::AccessDecision;
use crate::policy::AccessTable;

#[test]
fn entries_win_over_the_default() {
    let table = AccessTable {
        default: Some(Access::Allow),
        entries: BTreeMap::from([("ops-remote-host-2".to_string(), Access::Deny)]),
    };
    assert_eq!(table.decide("ops-remote-host-2"), AccessDecision::Denied);
    assert_eq!(table.decide("ops-remote-host-1"), AccessDecision::Allowed);
}

#[test]
fn unspecified_targets_require_explicit_authorization() {
    let table = AccessTable::new();
    assert_eq!(table.decide("any-target"), AccessDecision::Unspecified);
}

#[test]
fn deny_defaults_deny_every_unlisted_target() {
    let table = AccessTable {
        default: Some(Access::Deny),
        entries: BTreeMap::from([("ops-remote-host-1".to_string(), Access::Allow)]),
    };
    assert_eq!(table.decide("ops-remote-host-1"), AccessDecision::Allowed);
    assert_eq!(table.decide("ops-remote-host-9"), AccessDecision::Denied);
}
