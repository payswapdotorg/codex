//! Atomic snapshot write tests.

use pretty_assertions::assert_eq;

use super::read_snapshot;
use super::write_snapshot;
use crate::testutil::cleanup;
use crate::testutil::temp_root;

#[test]
fn snapshot_writes_replace_content_and_leave_no_temporary() {
    let root = temp_root("atomic-replace");
    let path = root.join("state.json");

    write_snapshot(&path, b"{\"first\":true}").expect("first write");
    assert_eq!(
        read_snapshot(&path).expect("read"),
        Some(b"{\"first\":true}".to_vec())
    );
    assert_eq!(
        std::fs::read_dir(&root).expect("list root").count(),
        1,
        "only the snapshot file exists after the first write"
    );

    write_snapshot(&path, b"{\"second\":true}").expect("second write");
    assert_eq!(
        read_snapshot(&path).expect("read"),
        Some(b"{\"second\":true}".to_vec())
    );
    assert_eq!(
        std::fs::read_dir(&root).expect("list root").count(),
        1,
        "the temporary file never survives a completed write"
    );

    cleanup(&root);
}

#[test]
fn missing_snapshots_read_as_empty() {
    let root = temp_root("atomic-missing");
    assert_eq!(
        read_snapshot(&root.join("absent.json")).expect("read"),
        None
    );
    cleanup(&root);
}
