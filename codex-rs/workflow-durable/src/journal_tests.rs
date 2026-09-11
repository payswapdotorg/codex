//! Journal append/load and torn-tail policy tests.

use std::io::Write;

use pretty_assertions::assert_eq;

use super::Journal;
use super::load_journal;
use crate::DurableStoreError;
use crate::testutil::cleanup;
use crate::testutil::temp_root;

/// Appends `records` as JSONL lines through the journal machinery.
fn append_lines(path: &std::path::Path, records: &[serde_json::Value]) {
    let journal = Journal::open(path).expect("open journal");
    for record in records {
        journal.append(record).expect("append record");
    }
}

#[test]
fn journal_round_trips_every_appended_record() {
    let root = temp_root("journal-round-trip");
    let path = root.join("journal.jsonl");
    let records = vec![
        serde_json::json!({ "event": "first", "n": 1 }),
        serde_json::json!({ "event": "second", "n": 2 }),
        serde_json::json!({ "event": "third", "n": 3 }),
    ];
    append_lines(&path, &records);

    let loaded: Vec<serde_json::Value> = load_journal(&path).expect("load journal");
    assert_eq!(loaded, records);

    let text = std::fs::read_to_string(&path).expect("read journal");
    assert!(
        text.ends_with('\n'),
        "the journal always ends on a line boundary"
    );
    cleanup(&root);
}

#[test]
fn missing_journals_load_as_empty() {
    let root = temp_root("journal-missing");
    let loaded: Vec<serde_json::Value> =
        load_journal(&root.join("absent.jsonl")).expect("load journal");
    assert!(loaded.is_empty());
    cleanup(&root);
}

#[test]
fn torn_tail_bytes_are_dropped_and_repaired() {
    let root = temp_root("journal-torn-tail");
    let path = root.join("journal.jsonl");
    append_lines(
        &path,
        &[
            serde_json::json!({ "event": "first" }),
            serde_json::json!({ "event": "second" }),
        ],
    );

    // Chop bytes off the final line, simulating a crash between the
    // append write and its fsync.
    let bytes = std::fs::read(&path).expect("read journal");
    std::fs::write(&path, &bytes[..bytes.len() - 2]).expect("truncate");

    let loaded: Vec<serde_json::Value> = load_journal(&path).expect("load journal");
    assert_eq!(
        loaded,
        vec![serde_json::json!({ "event": "first" })],
        "only the first record was committed"
    );

    let repaired = std::fs::read(&path).expect("read repaired journal");
    assert!(
        repaired.ends_with(b"\n"),
        "the repaired journal ends on a line boundary"
    );
    // Appending after the repair lands on a clean line boundary.
    append_lines(&path, &[serde_json::json!({ "event": "third" })]);
    let reloaded: Vec<serde_json::Value> = load_journal(&path).expect("reload journal");
    assert_eq!(
        reloaded,
        vec![
            serde_json::json!({ "event": "first" }),
            serde_json::json!({ "event": "third" }),
        ]
    );
    cleanup(&root);
}

#[test]
fn whole_file_without_newline_is_a_torn_tail() {
    let root = temp_root("journal-whole-torn");
    let path = root.join("journal.jsonl");
    std::fs::write(&path, b"{\"event\":\"never-fsynced\"}").expect("write partial");

    let loaded: Vec<serde_json::Value> = load_journal(&path).expect("load journal");
    assert!(loaded.is_empty(), "an uncommitted partial file loads empty");

    let repaired = std::fs::read(&path).expect("read repaired journal");
    assert!(repaired.is_empty(), "the file is repaired to empty");
    cleanup(&root);
}

#[test]
fn final_line_that_fails_to_parse_is_dropped() {
    let root = temp_root("journal-garbage-tail");
    let path = root.join("journal.jsonl");
    append_lines(&path, &[serde_json::json!({ "event": "first" })]);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open journal");
    file.write_all(b"not-json-at-all\n")
        .expect("append garbage");
    drop(file);

    let loaded: Vec<serde_json::Value> = load_journal(&path).expect("load journal");
    assert_eq!(
        loaded,
        vec![serde_json::json!({ "event": "first" })],
        "the garbage tail is dropped, the committed record survives"
    );
    let text = std::fs::read_to_string(&path).expect("read repaired journal");
    assert_eq!(
        text, "{\"event\":\"first\"}\n",
        "the repair physically truncates the tail"
    );
    cleanup(&root);
}

#[test]
fn committed_line_corruption_is_a_deterministic_error() {
    let root = temp_root("journal-mid-corrupt");
    let path = root.join("journal.jsonl");
    append_lines(
        &path,
        &[
            serde_json::json!({ "event": "first" }),
            serde_json::json!({ "event": "second" }),
            serde_json::json!({ "event": "third" }),
        ],
    );

    // Corrupt the FIRST (committed) line: this is not a crash tail, so
    // it must never be silently skipped.
    let bytes = std::fs::read(&path).expect("read journal");
    let mut corrupted = Vec::new();
    corrupted.extend_from_slice(b"@@not json@@\n");
    corrupted.extend_from_slice(
        &bytes[bytes
            .iter()
            .position(|&byte| byte == b'\n')
            .expect("first boundary")
            + 1..],
    );
    std::fs::write(&path, corrupted).expect("corrupt");

    let error = load_journal::<serde_json::Value>(&path).expect_err("corruption must fail");
    assert!(
        matches!(error, DurableStoreError::Corrupt { .. }),
        "a committed-line corruption is a deterministic error, got {error:?}"
    );
    cleanup(&root);
}

#[test]
fn invalid_utf8_in_the_committed_region_is_a_deterministic_error() {
    let root = temp_root("journal-utf8-corrupt");
    let path = root.join("journal.jsonl");
    append_lines(&path, &[serde_json::json!({ "event": "first" })]);

    let bytes = std::fs::read(&path).expect("read journal");
    let mut corrupted = bytes;
    corrupted[0] = 0xff;
    corrupted.push(b'\n');
    std::fs::write(&path, &corrupted).expect("corrupt");

    let error = load_journal::<serde_json::Value>(&path).expect_err("corruption must fail");
    assert!(
        matches!(error, DurableStoreError::Corrupt { .. }),
        "invalid UTF-8 in the committed region is a deterministic error, got {error:?}"
    );
    cleanup(&root);
}
