//! Atomic snapshot writes for the durable control-plane stores.
//!
//! Current-state records (versions, instances, installations) are stored
//! as whole snapshot files. Every snapshot write is atomic with respect
//! to crashes and concurrent readers: the bytes go to a sibling
//! temporary file, that file is fsynced, and only then is it renamed
//! over the snapshot path — the rename is the commit point. The
//! containing directory is fsynced afterwards on platforms that allow
//! it (Unix) so the rename itself survives a power loss. A crash at any
//! point leaves either the previous snapshot or the new one, never a
//! partial file.

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use crate::DurableStoreError;

/// The suffix of the temporary file used while replacing a snapshot.
const TEMP_SUFFIX: &str = ".tmp";

/// Atomically replaces `path` with `bytes`.
///
/// The write is durable before returning: the temporary file is fsynced
/// before the rename and the parent directory is fsynced after it
/// (where the platform allows opening a directory for synchronization).
pub(crate) fn write_snapshot(path: &Path, bytes: &[u8]) -> Result<(), DurableStoreError> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(TEMP_SUFFIX);
    let temporary = PathBuf::from(temporary);
    let result = write_temporary_then_rename(&temporary, path, bytes);
    if result.is_err() {
        // Never leave a stray temporary behind a failed write.
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// The temp→fsync→rename core of [`write_snapshot`].
fn write_temporary_then_rename(
    temporary: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<(), DurableStoreError> {
    let mut file = fs::File::create(temporary)
        .map_err(|source| DurableStoreError::io("creating a snapshot temporary file", source))?;
    file.write_all(bytes)
        .map_err(|source| DurableStoreError::io("writing a snapshot temporary file", source))?;
    file.sync_all()
        .map_err(|source| DurableStoreError::io("fsyncing a snapshot temporary file", source))?;
    drop(file);
    fs::rename(temporary, path)
        .map_err(|source| DurableStoreError::io("renaming a snapshot into place", source))?;
    sync_parent_directory(path);
    Ok(())
}

/// Serializes `state` and atomically replaces the snapshot at `path`.
///
/// The serialization is deterministic by construction: the stores keep
/// ordered maps (`BTreeMap`) of serde records, so the same state always
/// produces the same bytes.
pub(crate) fn write_state<T>(path: &Path, state: &T) -> Result<(), DurableStoreError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(state)?;
    write_snapshot(path, &bytes)
}

/// Reads the snapshot bytes at `path`, when the file exists.
///
/// A missing file is an empty snapshot (a store that has never been
/// written); any other i/o failure is surfaced with context.
pub(crate) fn read_snapshot(path: &Path) -> Result<Option<Vec<u8>>, DurableStoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(DurableStoreError::io("reading a snapshot file", source)),
    }
}

/// Best-effort parent-directory fsync after a rename.
///
/// Unix allows opening a directory and fsyncing it so the rename is
/// durable; other platforms either forbid directory fsync or provide it
/// through platform-specific APIs, so the call is skipped there. The
/// rename itself remains atomic everywhere.
fn sync_parent_directory(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent()
        && let Ok(directory) = fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
#[path = "atomic_tests.rs"]
mod tests;
