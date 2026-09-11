//! Durable control-plane stores (MWO-001).
//!
//! This crate is the **durable backing** of the workflow control plane:
//! file-backed implementations of the six existing store ports so
//! workflow versions, instance records, evidence, trigger idempotency,
//! installations, and resume directives survive process restart. It
//! implements the ports exactly as the in-memory doubles define their
//! observable behavior — no new semantics, no second engine, no host
//! integration beyond the stores themselves.
//!
//! ```text
//! workflow-contracts (WO-003)  -> frozen records: versions, instances,
//!                                 evidence references, digests
//! workflow-app (WO-010)        -> ports: WorkflowVersionStore,
//!                                 WorkflowInstanceStore, EvidenceStore
//! workflow-triggers (WO-011)   -> ports: TriggerLedger,
//!                                 InstallationStore, InstanceControl
//! workflow-durable (this)      -> std-only file-backed durability:
//!                                 atomic snapshots + fsynced journals
//! ```
//!
//! ## What this crate is
//!
//! - [`DurableStores`]: a facade opening all six stores over one root
//!   directory, plus the individual store structs
//!   ([`DurableVersionStore`], [`DurableInstanceStore`],
//!   [`DurableEvidenceStore`], [`DurableTriggerLedger`],
//!   [`DurableInstallationStore`], [`DurableInstanceControl`]) so hosts
//!   adopt them piecemeal.
//! - **Atomic snapshots** for current-state records (versions,
//!   instances, installations): every write serializes the whole
//!   ordered map deterministically, goes to a temp file, fsyncs, and
//!   renames — the rename is the commit point.
//! - **Append-only JSONL journals** for event-shaped state (evidence
//!   payloads, trigger acceptances/settlements, installation audits,
//!   resume directives): every append writes the full line and fsyncs
//!   before the port method returns.
//! - **Crash/restart parity**: reloading replays exactly the committed
//!   records — locators, digests, `evt-<n>` event ids, settlements,
//!   await registrations, instance statuses, and installations all
//!   re-observe identically. A trigger event key accepted once keeps
//!   answering `Duplicate` forever, across restarts.
//!
//! ## Durability policies
//!
//! - **Single-writer contract.** Every port takes `&mut self`; the host
//!   serializes writers through that `&mut`, exactly as with the
//!   in-memory doubles. The stores are shared-state *handles* (clones
//!   observe the same backing files) with an internal mutex only
//!   because sharing a handle is what lets a host audit a store while
//!   the lifecycle or trigger plane owns the boxed port — the same
//!   shape as the in-memory doubles. There is no locking beyond that
//!   correctness requirement: no cross-process locks, no background
//!   threads, one writer per root directory is the host's contract.
//! - **Torn-tail policy (journals).** A crash between an append's write
//!   and its fsync can leave a partial final line. At open time the
//!   bytes after the last newline are dropped, a final line that fails
//!   to parse is dropped (it is the least-committed write), and the
//!   healthy prefix is rewritten atomically so later appends can never
//!   land behind uncommitted bytes. A parse failure in any *committed*
//!   (earlier) line, or invalid UTF-8 inside the committed region, is
//!   structural corruption: a deterministic
//!   [`DurableStoreError::Corrupt`] from the constructor — never a
//!   silent wrong state.
//! - **Load-time integrity.** Every workflow version is re-verified
//!   with `WorkflowVersion::verify_integrity` on every
//!   `WorkflowVersionStore::load` call; a tampered record surfaces as
//!   `WorkflowAppError::VersionIntegrity` and never executes.
//! - **Error projection.** The frozen port error types predate
//!   durability and have no i/o variant; store failures project as
//!   documented in the `error` module — i/o and serialization
//!   failures surface through the existing `Serialization` variants
//!   with full context, identity failures reuse the existing identity
//!   variants verbatim.
//!
//! ## What this crate deliberately is not
//!
//! - **No new semantics.** Ports, records, and error contracts are the
//!   frozen ones; the durable stores reproduce the in-memory doubles'
//!   observable behavior (locator minting, event id allocation, dedupe
//!   outcomes, legal resume transitions) and add durability only.
//! - **No database engine.** `std`-only file I/O: no rusqlite/sqlx, no
//!   new external dependencies, no network, no background threads.
//! - **No credentials.** Stored records are the credential-free
//!   contract records themselves (attribution labels and opaque
//!   resource identities only, by construction of the frozen
//!   contracts); fixtures in this crate's tests scan for
//!   credential-shaped content.
//! - **No host integration beyond the stores.** Wiring the stores into
//!   the Codex application's real control plane is the host's work;
//!   this crate only supplies the port implementations.
//!
//! ## Module map
//!
//! - `error` — the store error type and its documented projections
//!   into the frozen port error enums.
//! - `atomic` — atomic snapshot writes (temp → fsync → rename).
//! - `journal` — append-only JSONL journals and the torn-tail policy.
//! - `version_store` — durable workflow version records.
//! - `instance_store` — durable workflow instance records.
//! - `evidence_store` — durable evidence payloads.
//! - `trigger_ledger` — the durable trigger idempotency/audit ledger.
//! - `installation_store` — durable installed configurations and
//!   rebind audits.
//! - `instance_control` — the durable `Paused -> Running` resume seam.
//! - `facade` — [`DurableStores`] over one root directory.
//!
//! ## Ordinary Codex compatibility
//!
//! Nothing in this crate executes unless a host opens a store and hands
//! it to a lifecycle or trigger plane. Ordinary coding-agent behavior
//! of Codex is untouched: no code was added to any existing crate.

#![deny(missing_docs)]

mod atomic;
mod error;
mod evidence_store;
mod facade;
mod installation_store;
mod instance_control;
mod instance_store;
mod journal;
mod trigger_ledger;
mod version_store;

#[cfg(test)]
#[path = "testutil.rs"]
mod testutil;

pub use error::DurableStoreError;
pub use evidence_store::DurableEvidenceStore;
pub use facade::DurableStores;
pub use installation_store::DurableInstallationStore;
pub use instance_control::DurableInstanceControl;
pub use instance_store::DurableInstanceStore;
pub use trigger_ledger::DurableTriggerLedger;
pub use version_store::DurableVersionStore;
