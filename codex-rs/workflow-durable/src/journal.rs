//! Append-only JSONL journals for the durable control-plane stores.
//!
//! Event-shaped state (evidence payloads, trigger acceptances and
//! settlements, installation audits, resume directives) is stored as
//! one JSON record per line. Every append writes the full line — JSON
//! bytes plus the terminating newline — with a single `write_all` and
//! fsyncs before returning, so a record is either fully committed or
//! absent.
//!
//! ## Torn-tail policy (documented and tested)
//!
//! A crash between the write and the fsync can leave a partial line at
//! the end of the file. At open time the journal is repaired
//! deterministically:
//!
//! - bytes after the last newline are a torn append: dropped;
//! - the final line, if it fails to parse, is the least-committed
//!   write: dropped;
//! - a parse failure in any *earlier* (committed) line, or invalid
//!   UTF-8 inside the committed region, is structural corruption and
//!   surfaces as [`DurableStoreError::Corrupt`] — a deterministic
//!   error, never a silent wrong state.
//!
//! The repaired prefix is rewritten atomically (temp file → fsync →
//! rename) so subsequent appends can never land behind uncommitted
//! bytes.

use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::DurableStoreError;
use crate::atomic::write_snapshot;

/// An open append handle onto one JSONL journal.
///
/// The handle is opened after [`load_journal`] has loaded and repaired
/// the file; every [`Journal::append`] fsyncs before returning, which
/// is what makes the appended record durable. The handle takes `&self`
/// (the file is mutex-guarded) so it can live behind the shared-state
/// handles the stores use, while the host-level `&mut self` port
/// contract keeps writes serialized.
#[derive(Debug)]
pub(crate) struct Journal {
    file: Mutex<fs::File>,
}

impl Journal {
    /// Opens (creating when absent) the journal at `path` for appending.
    pub(crate) fn open(path: &Path) -> Result<Self, DurableStoreError> {
        let file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map_err(|source| DurableStoreError::io("opening a journal for append", source))?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    /// Appends one record as a JSONL line and fsyncs it before
    /// returning.
    pub(crate) fn append<T>(&self, record: &T) -> Result<(), DurableStoreError>
    where
        T: Serialize,
    {
        let mut line = serde_json::to_vec(record)?;
        line.push(b'\n');
        let mut file = self.lock();
        file.write_all(&line)
            .map_err(|source| DurableStoreError::io("appending a journal record", source))?;
        file.sync_all()
            .map_err(|source| DurableStoreError::io("fsyncing a journal record", source))?;
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, fs::File> {
        self.file.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Loads every committed record from the journal at `path`, repairing a
/// torn tail per the documented policy.
///
/// A missing file is an empty journal. See the [module
/// docs](self) for the truncation policy and its corruption boundary.
pub(crate) fn load_journal<T>(path: &Path) -> Result<Vec<T>, DurableStoreError>
where
    T: DeserializeOwned,
{
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(DurableStoreError::io("reading a journal file", source)),
    };
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    // The committed region is everything up to and including the last
    // newline; anything after it is a torn append.
    let committed_end = bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |index| index + 1);
    let committed = &bytes[..committed_end];
    let text = str::from_utf8(committed).map_err(|source| {
        DurableStoreError::corrupt(
            path.display(),
            format!("invalid UTF-8 in the committed journal region: {source}"),
        )
    })?;

    let mut records = Vec::new();
    // How many leading bytes are healthy committed state; a torn tail
    // or dropped final line shrinks this below the file length.
    let mut healthy_end = 0usize;
    if !text.is_empty() {
        let body = text.strip_suffix('\n').unwrap_or(text);
        let lines: Vec<&str> = body.split('\n').collect();
        let total = lines.len();
        for (index, line) in lines.iter().enumerate() {
            match serde_json::from_str::<T>(line) {
                Ok(record) => records.push(record),
                Err(source) => {
                    if index + 1 == total {
                        // The final line is the least-committed write: a
                        // record that fails to parse there is treated as
                        // a torn tail and dropped (documented policy).
                        break;
                    }
                    return Err(DurableStoreError::corrupt(
                        path.display(),
                        format!(
                            "committed journal line {} is not a valid record: {source}",
                            index + 1
                        ),
                    ));
                }
            }
            healthy_end += line.len() + 1;
        }
    }

    if healthy_end < bytes.len() {
        // Rewrite the healthy prefix atomically so later appends can
        // never land behind uncommitted bytes.
        write_snapshot(path, &bytes[..healthy_end])?;
    }
    Ok(records)
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
